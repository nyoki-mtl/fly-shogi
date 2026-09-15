"""Create a source-only ZIP without Git, datasets, models or local run outputs."""
import argparse
import json
from pathlib import Path
import zipfile

from runtime import ROOT, digest

ROOT_FILES = ['.gitignore', '.gitattributes', 'Cargo.toml', 'Cargo.lock',
              'pyproject.toml', 'uv.lock', 'package.json', 'package-lock.json',
              'tsconfig.json', 'README.md', 'README_en.md', 'LICENSE', 'NOTICE']
FOLDERS = ['.github', 'config', 'docs', 'licenses', 'scripts', 'src', 'tests', 'ui', 'web']


def source_files():
    files = [ROOT / name for name in ROOT_FILES]
    for folder in FOLDERS:
        files.extend(p for p in (ROOT / folder).rglob('*')
                     if p.is_file() and '__pycache__' not in p.parts and p.suffix != '.pyc')
    for path in files:
        if path.is_symlink() or path.stat().st_size > 5 * 1024 * 1024:
            raise ValueError(f'Unexpected source artifact: {path.relative_to(ROOT)}')
        if path.suffix in {'.onnx', '.bin', '.feather', '.parquet', '.webm', '.mp4', '.pt', '.npz'}:
            raise ValueError(f'Data/model artifact in source: {path.relative_to(ROOT)}')
        if path.suffix == '.jsonl' and path != ROOT / 'tests/positions.jsonl':
            raise ValueError('Only the hand-written test fixture JSONL belongs in source')
    return sorted(files)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    manifest_path = args.output.with_suffix('.manifest.json')
    if args.output.suffix != '.zip' or args.output.exists() or manifest_path.exists():
        parser.error('Choose a new .zip output path')
    files = source_files()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    entries = []
    with zipfile.ZipFile(args.output, 'x', compression=zipfile.ZIP_DEFLATED) as archive:
        for path in files:
            name = path.relative_to(ROOT).as_posix()
            archive.write(path, 'fly-meijin/' + name)
            entries.append(dict(path=name, bytes=path.stat().st_size, sha256=digest(path)))
    manifest = dict(archive_sha256=digest(args.output), files=entries,
                    excluded=['data', 'models', 'work_dir', 'target', '.venv',
                              'node_modules', '.git', 'private training data'])
    manifest_path.write_text(json.dumps(manifest, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(dict(files=len(entries), bytes=args.output.stat().st_size,
                         sha256=manifest['archive_sha256']), indent=2))


if __name__ == '__main__':
    main()
