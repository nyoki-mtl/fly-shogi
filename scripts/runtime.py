"""Paths shared by the standalone training and demo commands."""
import hashlib
import os
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]
DL_SUISHO_SHA256 = '4c9e656f01b3ec692478965ee222a2116b73310368ed5aa499f1be1451484c0d'


def digest(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def build_binary(name):
    subprocess.run(["cargo", "build", "--locked", "--release", "--target-dir",
                    str(ROOT / "target"), "--bin", name], cwd=ROOT, check=True)
    return ROOT / "target/release" / (name + (".exe" if os.name == "nt" else ""))
