"""Optimize existing MBON input synapses using cached full-circuit activity."""
import argparse
import json
from pathlib import Path
import shutil
import subprocess

import numpy as np
import scipy
from scipy.optimize import minimize

from runtime import ROOT, DL_SUISHO_SHA256, build_binary, digest


def save(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n', encoding='utf-8')


def compact(report):
    return {k: v for k, v in report.items() if k not in ('gradient', 'diagonal')}


class Worker:
    def __init__(self, binary, graph, model, cache, temperature, workers, log):
        self.process = subprocess.Popen(
            [str(binary), str(graph), str(model), str(cache), str(temperature), str(workers), '0'],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=log,
            text=True, encoding='utf-8', bufsize=1)

    def read(self):
        line = self.process.stdout.readline()
        if not line:
            raise RuntimeError(f'Objective worker exited: {self.process.poll()}; see worker.log')
        return json.loads(line)

    def evaluate(self, gains, split='train', gradient=False, limit=0):
        self.process.stdin.write(json.dumps(dict(gains=np.asarray(gains).tolist(),
                                                split=split, gradient=gradient, limit=limit)) + '\n')
        self.process.stdin.flush()
        return self.read()

    def close(self):
        if self.process.poll() is None:
            self.process.stdin.close()
            try:
                self.process.wait(timeout=5)
            except subprocess.TimeoutExpired:
                self.process.terminate()
                self.process.wait()


def gradient_check(worker, initial):
    x = np.clip(initial, 0.15, 2.9)
    grad = np.asarray(worker.evaluate(x, gradient=True, limit=16)['gradient'])
    rng = np.random.default_rng(231)
    checks = []
    for _ in range(3):
        direction = rng.choice([-1., 1.], len(x))
        step = 1e-5
        actual = (worker.evaluate(x + step*direction, limit=16)['loss']
                  - worker.evaluate(x - step*direction, limit=16)['loss']) / (2*step)
        expected = float(grad @ direction)
        error = abs(actual - expected)
        if error > 1e-7 + 1e-5*abs(expected):
            raise AssertionError((actual, expected, error))
        checks.append(dict(finite_difference=actual, analytic=expected, absolute_error=error))
    return checks


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-root', type=Path, default=ROOT)
    parser.add_argument('--initial', type=Path, default=ROOT / 'work_dir/circuit/initial.json')
    parser.add_argument('--teacher-data', type=Path, default=ROOT / 'work_dir/teacher')
    parser.add_argument('--output', type=Path, default=ROOT / 'work_dir/lbfgs-training')
    parser.add_argument('--maxiter', type=int, default=100)
    parser.add_argument('--workers', type=int, default=8)
    parser.add_argument('--temperature', type=float, default=0.02)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Choose a new --output directory')
    if args.maxiter < 1 or args.workers < 1 or not np.isfinite(args.temperature) or args.temperature <= 0:
        parser.error('maxiter, workers and temperature must be positive')
    manifest = json.loads((args.teacher_data / 'manifest.json').read_text())
    records = args.teacher_data / 'teacher.jsonl'
    if manifest['teacher_sha256'] != DL_SUISHO_SHA256:
        parser.error('Teacher data must be prepared with public DL Suisho 15b')
    if digest(records) != manifest['output_sha256']:
        parser.error('Teacher records differ from manifest')
    if any(manifest['counts'][split] <= 0 for split in ('train', 'validation')):
        parser.error('Both train and validation positions are required')
    model = json.loads(args.initial.read_text())
    if model['version'] != 'mbon-input-v1':
        parser.error('Use prepare_circuit.py --method lbfgs to initialize this trainer')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    cache_bytes = sum(manifest['counts'].values()) * len(model['sources']) * model['time_bins']
    if shutil.disk_usage(args.output.parent).free < cache_bytes + 512 * 1024**2:
        parser.error(f'Need space for {cache_bytes} cache bytes plus 512 MiB of working files')
    print(json.dumps(dict(cache_bytes=cache_bytes, cache_loaded_into_ram=True)), flush=True)
    cache_binary = build_binary('cache-activity')
    objective_binary = build_binary('synapse-objective')
    evaluator = build_binary('evaluate-circuit')
    graph_dir = args.data_root / 'work_dir/malecns-csr'
    graph = graph_dir / 'graph.bin'
    args.output.mkdir()
    parent = args.output / 'initial.json'
    shutil.copyfile(args.initial, parent)
    cache = args.output / 'cache'
    subprocess.run([str(cache_binary), str(graph), str(graph_dir / 'metadata.json'),
                    str(parent), str(records), str(cache)], check=True)
    save(args.output / 'provenance.json', dict(
        method='L-BFGS-B with fixed presynaptic activity', teacher=manifest,
        initial_sha256=digest(parent), graph_sha256=digest(graph),
        cache_sha256=digest(cache / 'spikes.bin'), temperature=args.temperature,
        bounds=[0.05, 3.0], maxiter=args.maxiter, scipy=scipy.__version__, numpy=np.__version__))
    worker = None
    try:
        with (args.output / 'worker.log').open('w', encoding='utf-8') as log:
            worker = Worker(objective_binary, graph, parent, cache, args.temperature, args.workers, log)
            print(json.dumps(worker.read()), flush=True)
            x0 = np.asarray(model['gains'], dtype=np.float64)
            save(args.output / 'gradient-check.json', gradient_check(worker, x0))
            initial = worker.evaluate(x0, gradient=True)
            valid = worker.evaluate(x0, 'validation')
            save(args.output / 'baseline.json', dict(train=compact(initial), validation=compact(valid)))
            scale = np.clip(1 / np.sqrt(np.maximum(initial['diagonal'], 1e-12)), 0.1, 100.)
            history = []
            best = dict(iteration=0, correct=valid['correct'], loss=valid['loss'])
            shutil.copyfile(parent, args.output / 'best.json')

            def candidate(gains, iteration):
                return dict(model, gains=np.clip(gains.astype(np.float32).astype(np.float64), 0.05, 3.).tolist(),
                            samples=model['samples'] + iteration * initial['records'],
                            updates=model['updates'] + iteration)

            def objective(y):
                report = worker.evaluate(np.clip(x0 + scale*y, 0.05, 3.), gradient=True)
                return report['loss'], np.asarray(report['gradient']) * scale

            def checkpoint(y):
                iteration = len(history) + 1
                model_at_step = candidate(np.clip(x0 + scale*y, 0.05, 3.), iteration)
                # Select using the exact float32 gains that inference will read.
                validation = worker.evaluate(model_at_step['gains'], 'validation')
                report = dict(iteration=iteration, validation=compact(validation))
                history.append(report)
                save(args.output / 'latest.json', model_at_step)
                save(args.output / 'history.json', history)
                if (validation['correct'], -validation['loss']) > (best['correct'], -best['loss']):
                    best.update(iteration=iteration, correct=validation['correct'], loss=validation['loss'])
                    save(args.output / 'best.json', model_at_step)
                print(json.dumps(report), flush=True)

            result = minimize(objective, np.zeros_like(x0), jac=True, method='L-BFGS-B',
                              bounds=list(zip((0.05-x0)/scale, (3.-x0)/scale)), callback=checkpoint,
                              options=dict(maxiter=args.maxiter, maxfun=max(120, args.maxiter*3),
                                           maxcor=20, ftol=1e-10, gtol=1e-6))
            save(args.output / 'final.json', candidate(np.clip(x0 + scale*result.x, 0.05, 3.), int(result.nit)))
            save(args.output / 'optimizer.json', dict(success=bool(result.success), message=str(result.message),
                 iterations=int(result.nit), evaluations=int(result.nfev), loss=float(result.fun), best=best))
            worker.close()
            worker = None
        # Cached scores approximate training only. Re-run the full circuit for evaluation.
        subprocess.run([str(evaluator), str(graph), str(graph_dir / 'metadata.json'),
                        str(args.output / 'best.json'), str(records), str(manifest['counts']['validation']),
                        str(args.output / 'full-validation.json')], check=True)
        print(json.dumps(dict(stage='complete', best_sha256=digest(args.output / 'best.json'))), flush=True)
    finally:
        if worker is not None:
            worker.close()


if __name__ == '__main__':
    main()
