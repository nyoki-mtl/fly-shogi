"""Train existing KC->MBON synapses with scalar teacher-probability rewards."""
import argparse
import json
from pathlib import Path
import subprocess

from runtime import ROOT, DL_SUISHO_SHA256, build_binary, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-root', type=Path, default=ROOT)
    parser.add_argument('--initial', type=Path, default=ROOT / 'work_dir/dopamine-circuit/initial.json')
    parser.add_argument('--channels', type=Path, default=ROOT / 'work_dir/dopamine-circuit/channels.json')
    parser.add_argument('--teacher-data', type=Path, default=ROOT / 'work_dir/teacher')
    parser.add_argument('--output', type=Path, default=ROOT / 'work_dir/dopamine-training')
    parser.add_argument('--epochs', type=int, default=1)
    parser.add_argument('--eta', type=float, default=0.3)
    parser.add_argument('--resume', action='store_true', help='Resume a checkpoint alongside its settings.json')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Choose a new --output directory')
    manifest = json.loads((args.teacher_data / 'manifest.json').read_text())
    if manifest['teacher_sha256'] != DL_SUISHO_SHA256:
        parser.error('Teacher data must be prepared with public DL Suisho 15b')
    records = args.teacher_data / 'teacher.jsonl'
    if digest(records) != manifest['output_sha256']:
        parser.error('Teacher records differ from manifest')
    if manifest['counts']['train'] <= 0:
        parser.error('No training positions supplied')
    binary = build_binary('train-dopamine')
    schedule = json.loads((ROOT / 'config/dopamine.json').read_text())
    schedule['resume_local'] = args.resume
    args.output.parent.mkdir(parents=True, exist_ok=True)
    # Native training creates OUTPUT and copies its exact inputs into it.
    import tempfile
    with tempfile.TemporaryDirectory() as temp:
        schedule_path = Path(temp) / 'schedule.json'
        schedule_path.write_text(json.dumps(schedule))
        graph = args.data_root / 'work_dir/malecns-csr'
        subprocess.run([str(binary), str(graph / 'graph.bin'), str(graph / 'metadata.json'),
                        str(args.initial), str(args.channels), str(records), str(args.output),
                        str(args.epochs), str(args.eta), 'policy-reward', str(schedule_path)], check=True)
    provenance = dict(teacher=manifest, initial_sha256=digest(args.initial),
                      channels_sha256=digest(args.channels),
                      graph_sha256=digest(graph / 'graph.bin'),
                      final_sha256=digest(args.output / ('stopped.json' if (args.output / 'stop.json').exists() else 'final.json')))
    (args.output / 'provenance.json').write_text(json.dumps(provenance, indent=2) + '\n', encoding='utf-8')


if __name__ == '__main__':
    main()
