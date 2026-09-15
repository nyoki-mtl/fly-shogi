"""Download and verify the three official MaleCNS inputs (about 1.11 GB)."""
import argparse
import json
from pathlib import Path
import urllib.request

from runtime import ROOT, digest


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-root", type=Path, default=ROOT)
    args = parser.parse_args()
    lock = json.loads((ROOT / "config/data-sources.lock.json").read_text())
    raw = args.data_root / "data/raw/malecns-v1.0"
    raw.mkdir(parents=True, exist_ok=True)
    for name, spec in lock["files"].items():
        path = raw / spec["url"].rsplit("/", 1)[-1]
        if not path.exists():
            temporary = path.with_suffix(".partial")
            print(f"Downloading {name}: {spec['bytes']:,} bytes", flush=True)
            with urllib.request.urlopen(spec["url"], timeout=180) as response, temporary.open("wb") as stream:
                while chunk := response.read(8 * 1024 * 1024):
                    stream.write(chunk)
            if temporary.stat().st_size != spec["bytes"] or digest(temporary) != spec["sha256"]:
                raise ValueError(f"Download checksum mismatch: {name}")
            temporary.replace(path)
        if path.stat().st_size != spec["bytes"] or digest(path) != spec["sha256"]:
            raise ValueError(f"Source checksum mismatch: {name}")
        print(f"Verified {name}", flush=True)


if __name__ == "__main__":
    main()
