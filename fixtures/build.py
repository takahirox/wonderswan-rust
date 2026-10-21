#!/usr/bin/env python3
# SPDX-License-Identifier: CC0-1.0
"""Build original compatibility probes; --check verifies committed bytes."""
import argparse
from pathlib import Path
import subprocess
import tempfile
import hashlib

ROOT = Path(__file__).resolve().parent
CASES = [
    ('sram32-common', 2, 0, 0, 0xc0),
    ('sram128-2003', 3, 1, 0, 0xc0),
    ('eeprom128', 0x10, 0, 6, 0xc0),
    ('eeprom1024', 0x50, 1, 10, 0xc0),
    ('eeprom2048-packed', 0x20, 1, 10, 0xe0),
]

def build(save, mapper, bits, mode, source='compat.asm', flags=0, defines=()):
    with tempfile.TemporaryDirectory(prefix='ws-probe-') as directory:
        output = Path(directory) / 'probe.bin'
        subprocess.run(['nasm', '-f', 'bin', f'-DEEPROM_BITS={bits}',
                        f'-DEXTENDED={mapper}', f'-DVIDEO_MODE={mode}',
                        *[f'-D{define}' for define in defines],
                        str(ROOT / source), '-o', str(output)], check=True)
        code = output.read_bytes()
    assert len(code) < 65520
    rom = bytearray([0x31] * 131072)
    rom[65536:65536+len(code)] = code
    rom[-16:] = bytes([0xea,0,0,0,0xf0,0,0,0,0,0x80,0,save,flags,mapper,0,0])
    rom[-2:] = (sum(rom[:-2]) & 65535).to_bytes(2,'little')
    return rom

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    images = [(name, build(save, mapper, bits, mode))
              for name, save, mapper, bits, mode in CASES]
    images.append(('gdma', build(0, 0, 0, 0x80, 'gdma.asm', 4)))
    for mode, name in enumerate(['sound-channel2', 'sound-hyper', 'sound-direct']):
        images.append((name, build(0, 0, 0, 0x80, 'sound.asm',
                                   defines=(f'SOUND_PATH={mode}',))))
    for name, rom in images:
        path = ROOT / 'generated' / (name+'.ws')
        if args.check:
            if path.read_bytes() != rom:
                raise SystemExit(f'{name} differs; run python3 fixtures/build.py')
        else:
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(rom)
        print(name, hashlib.sha256(rom).hexdigest())
