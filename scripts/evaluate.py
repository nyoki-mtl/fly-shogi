"""Measure teacher agreement on user-supplied validation positions."""
import argparse
from pathlib import Path
import subprocess
from runtime import ROOT, build_binary


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data-root', type=Path, default=ROOT)
    parser.add_argument('--model', type=Path, required=True)
    parser.add_argument('--teachers', type=Path, default=ROOT / 'work_dir/teacher/teacher.jsonl')
    parser.add_argument('--count', type=int, required=True)
    parser.add_argument('--output', type=Path, default=ROOT / 'work_dir/evaluation.json')
    args = parser.parse_args()
    binary = build_binary('evaluate-circuit')
    args.output.parent.mkdir(parents=True, exist_ok=True)
    graph = args.data_root / 'work_dir/malecns-csr'
    subprocess.run([str(binary), str(graph / 'graph.bin'), str(graph / 'metadata.json'),
                    str(args.model), str(args.teachers), str(args.count), str(args.output)], check=True)


if __name__ == '__main__':
    main()
