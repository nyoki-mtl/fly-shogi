"""Label your own SFEN positions with legal DL Suisho policy probabilities."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

from demo_teacher import DLSuishoTeacher
from runtime import ROOT, build_binary, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--teacher', type=Path, required=True)
    parser.add_argument('--positions', type=Path, required=True,
                        help='JSONL with sfen and split (train or validation), optional seed')
    parser.add_argument('--output', type=Path, default=ROOT / 'work_dir/teacher')
    parser.add_argument('--temperature', type=float, default=1.0)
    args = parser.parse_args()
    teacher = DLSuishoTeacher(args.teacher, temperature=args.temperature)
    exporter = build_binary('export-policy-labels')
    args.output.mkdir(parents=True, exist_ok=False)
    counts = {'train': 0, 'validation': 0}
    # Stream both files: the full private corpus never needs to fit in memory.
    with tempfile.TemporaryDirectory() as temp:
        labels = Path(temp) / 'labels.jsonl'
        subprocess.run([str(exporter), str(args.positions.resolve()), str(labels)], check=True)
        with labels.open(encoding='utf-8') as source, (args.output / 'teacher.jsonl').open('x', encoding='utf-8', newline='\n') as out:
            for line_number, line in enumerate(source, 1):
                row = json.loads(line)
                split, seed = row.get('split'), row.get('seed', 101)
                if split not in counts or type(seed) is not int or not 0 <= seed < 2**64:
                    raise ValueError(f'Invalid split or seed on line {line_number}')
                probabilities = teacher.policy(row)
                legal = row['legal_labels']
                best = max(probabilities, key=probabilities.get)
                target = next(c for c, _, usi in legal if usi == best)
                # Do not propagate comments, names, or unrelated source metadata.
                result = dict(sfen=row['sfen'], split=split, seed=seed, target=target,
                              teacher_policy=[[c, probabilities[usi]] for c, _, usi in legal])
                out.write(json.dumps(result) + '\n')
                counts[split] += 1
                if line_number % 128 == 0:
                    print(f'Labeled {line_number} positions', flush=True)
    if not sum(counts.values()):
        raise ValueError('No positions supplied')
    manifest = dict(teacher_sha256=teacher.sha256, positions_sha256=digest(args.positions),
                    teacher_name='DL Suisho 15b (public)', temperature=args.temperature,
                    output_sha256=digest(args.output / 'teacher.jsonl'), counts=counts,
                    policy='2187 logits; rsshogi legal mask; softmax; compact 1496 labels',
                    value_used=False)
    (args.output / 'manifest.json').write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(manifest, indent=2))


if __name__ == '__main__':
    main()
