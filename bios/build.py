#!/usr/bin/env python3
# SPDX-License-Identifier: CC0-1.0
"""Assemble the original IPL images and verify the committed bytes and hashes."""

import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent
MODELS = (("mono", 4096, 0), ("color", 8192, 0), ("crystal", 8192, 1))


def build(path, size, crystal):
    with tempfile.TemporaryDirectory(prefix="ws-ipl-") as directory:
        output = Path(directory) / path.name
        subprocess.run(
            ["nasm", "-f", "bin", f"-DIPL_SIZE={size}", f"-DCRYSTAL={crystal}",
             str(ROOT / "boot.asm"), "-o", str(output)],
            check=True,
        )
        data = output.read_bytes()
    if len(data) != size:
        raise SystemExit(f"{path.name}: expected {size} bytes, got {len(data)}")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    generated = ROOT / "generated"
    if not args.check:
        generated.mkdir(exist_ok=True)
    hashes = []
    for model, size, crystal in MODELS:
        path = generated / f"{model}.bin"
        data = build(path, size, crystal)
        hashes.append(f"{hashlib.sha256(data).hexdigest()}  {path.name}\n")
        write_or_check(path, data, args.check)
    manifest = "".join(hashes)
    write_or_check(generated / "SHA256SUMS", manifest.encode("ascii"), args.check)
    print(manifest, end="")


def write_or_check(path, data, check):
    if check:
        if not path.is_file() or path.read_bytes() != data:
            raise SystemExit(f"{path.relative_to(ROOT)} differs; run python3 bios/build.py")
    else:
        path.write_bytes(data)


if __name__ == "__main__":
    main()
