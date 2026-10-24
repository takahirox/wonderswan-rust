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

def build(save, mapper, bits, mode, source='compat.asm', flags=0, defines=(), protection=0x80, size=131072, model=0):
    with tempfile.TemporaryDirectory(prefix='ws-probe-') as directory:
        output = Path(directory) / 'probe.bin'
        subprocess.run(['nasm', '-f', 'bin', f'-DEEPROM_BITS={bits}',
                        f'-DEXTENDED={mapper}', f'-DVIDEO_MODE={mode}',
                        *[f'-D{define}' for define in defines],
                        str(ROOT / source), '-o', str(output)], check=True)
        code = output.read_bytes()
    assert len(code) < 65520
    if source == 'raw-rom.asm':
        rom = bytearray().join(bytes([bank ^ 0x5a, bank ^ 0xa5]) * 32768
                               for bank in range(size // 65536))
    else:
        rom = bytearray([0x31] * size)
    rom[size-65536:size-65536+len(code)] = code
    rom[-16:] = bytes([0xea,0,0,0,0xf0,0,0,model,0,protection,{131072:0, 3145728:5, 6291456:7}[size],save,flags,mapper,0,0])
    rom[-2:] = (sum(rom[:-2]) & 65535).to_bytes(2,'little')
    return rom

if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    images = [(name, build(save, mapper, bits, mode))
              for name, save, mapper, bits, mode in CASES]
    for mib, first in [(3, 0xd0), (6, 0xa0)]:
        for mapper, interface in [(0, 'common'), (1, '2003')]:
            images.append((f'raw{mib}-{interface}',
                           build(2, mapper, 0, 0, 'raw-rom.asm', 4,
                                 defines=(f'FIRST_BANK={first}',), size=mib*1024*1024)))
    images.append(('lcd-output', build(2, 0, 0, 0, 'lcd-output.asm')))
    images.append(('ordinary-io', build(2, 0, 0, 0, 'ordinary-io.asm', 4)))
    images.append(('word-timing', build(2, 0, 0, 0, 'word-timing.asm', 4)))
    images.append(('lcd', build(2, 0, 0, 0, 'lcd.asm')))
    images.append(('sweep', build(2, 0, 0, 0, 'sweep.asm')))
    images.append(('shift-rotate', build(2, 0, 0, 0, 'shift-rotate.asm')))
    images.append(('trap', build(2, 0, 0, 0, 'trap.asm')))
    images.append(('divide', build(2, 0, 0, 0, 'divide.asm')))
    images.append(('conversion', build(2, 0, 0, 0, 'conversion.asm')))
    images.append(('multiply', build(2, 0, 0, 0, 'multiply.asm')))
    images.append(('fpo1-poll', build(2, 0, 0, 0, 'fpo1-poll.asm')))
    images.append(('rtc-calendar', build(2, 1, 0, 0, 'rtc-calendar.asm')))
    images.append(('rtc', build(2, 1, 0, 0, 'rtc.asm')))
    images.append(('push-sp', build(2, 0, 0, 0, 'push-sp.asm')))
    images.append(('buslock', build(2, 0, 0, 0, 'buslock.asm')))
    for model, name in [(0, 'mono'), (1, 'color')]:
        for mapper, interface in [(0, 'common'), (1, '2003')]:
            images.append((f'startup-{name}-{interface}',
                           build(2, mapper, 0, 0, 'startup.asm', 12,
                                 protection=0, model=model)))
    for vertical in [False, True]:
        images.append(('keypad-vertical' if vertical else 'keypad-horizontal',
                       build(2, 0, 0, 0, 'keypad.asm', int(vertical))))
    images.append(('a0', build(0, 0, 0, 0, 'a0.asm', 4)))
    images.append(('gdma', build(0, 0, 0, 0x80, 'gdma.asm', 4)))
    for mode, name in enumerate(['sound-channel2', 'sound-hyper', 'sound-direct']):
        images.append((name, build(0, 0, 0, 0x80, 'sound.asm',
                                   defines=(f'SOUND_PATH={mode}',))))
    for protected in [False, True]:
        images.append(('internal-eeprom' + ('-protected' if protected else ''),
                       build(2, 0, 0, 0, 'internal-eeprom.asm',
                             defines=(f'PROTECTED={int(protected)}',),
                             protection=0 if protected else 0x80)))
    for name, rom in images:
        path = ROOT / 'generated' / (name+'.ws')
        if args.check:
            if path.read_bytes() != rom:
                raise SystemExit(f'{name} differs; run python3 fixtures/build.py')
        else:
            path.parent.mkdir(exist_ok=True)
            path.write_bytes(rom)
        print(name, hashlib.sha256(rom).hexdigest())
