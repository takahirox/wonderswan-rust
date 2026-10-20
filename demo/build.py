#!/usr/bin/env python3
# SPDX-License-Identifier: CC0-1.0
"""Rebuild or verify the bundled 128 KiB cartridge using NASM and Python 3."""
import argparse
import hashlib
from pathlib import Path
import subprocess
import tempfile
from graphics import tiles

ROOT = Path(__file__).resolve().parent.parent
OUTPUT = ROOT / 'web/public/demo/beacon-trail.ws'

def build():
    with tempfile.TemporaryDirectory(prefix='beacon-trail-') as directory:
        work = Path(directory)
        (work / 'tiles.bin').write_bytes(tiles())
        subprocess.run(['nasm', '-f', 'bin', '-I', str(work) + '/',
                        str(ROOT / 'demo/game.asm'), '-o', str(work / 'game.bin')], check=True, cwd=work)
        code = (work / 'game.bin').read_bytes()
    if len(code) > 65520:
        raise ValueError('Game exceeds final cartridge bank')
    rom = bytearray([255] * 131072)
    rom[65536:65536 + len(code)] = code
    # Footer: JMP F000:0000, maintenance, publisher, mono, ID, version,
    # 128 KiB, no save, horizontal/8-bit/zero-wait, Bandai 2001, checksum.
    rom[-16:] = bytes([0xea, 0, 0, 0, 0xf0, 0, 0, 0, 0, 0x80, 0, 0, 0, 0, 0, 0])
    rom[-2:] = (sum(rom[:-2]) & 65535).to_bytes(2, 'little')
    return rom

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true', help='fail if committed ROM differs')
    args = parser.parse_args()
    rom = build()
    if args.check:
        if OUTPUT.read_bytes() != rom:
            raise SystemExit('Bundled ROM differs; run python3 demo/build.py')
    else:
        OUTPUT.parent.mkdir(parents=True, exist_ok=True)
        OUTPUT.write_bytes(rom)
    print(f'{len(rom)} bytes; SHA-256 {hashlib.sha256(rom).hexdigest()}')
