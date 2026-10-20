# Implemented WonderSwan hardware

This is a from-scratch Rust core with a thin Wasm adapter. No existing emulator
has been ported/copied, and no proprietary firmware or commercial ROM is present.
The [original CC0 probes](../fixtures/README.md) and [Beacon Trail](../demo/README.md)
are the only cartridge fixtures. No ROM identity, title, publisher or file hash
changes emulated behavior. SHA-256 is used only by the browser save adapter.
This milestone expands a hardware subset; it does not promise that arbitrary
commercial software will run or that timing matches a physical console.

## Support matrix

| Component | Implemented | Limitations / explicit rejection |
| --- | --- | --- |
| Models | WonderSwan mono; WonderSwan Color; SwanCrystal (default) | Mono has 16 KiB internal RAM; color-enabled models have 64 KiB. WSC/Crystal share graphics architecture. A0 reports 85/87; 62 distinguishes WSC/Crystal. LCD panel characteristics are not simulated. |
| Header | Named entry/maintenance/publisher/model/ID/version/protection/size/save/orientation/ROM width/wait/mapper/checksum fields | Independent errors include bad field/value, declared/file size or checksum. Reserved flags and unknown model/save/mapper codes fail. Firmware authentication is absent. |
| ROM | Power-of-two 128 KiB–16 MiB common interface; up to 64 MiB with mapper 01 | 3/6 MiB unpadded images rejected with leading-padding explanation. ROM is read-only. Header 00 identifies a common interface, not proof of a physical 2001 chip (KARNAK also uses 00). Pocket Challenge board features unsupported. |
| Common banking | C0 linear 1 MiB bank, C1 save 64 KiB bank, C2/C3 64 KiB ROM banks | Address bus wraps at 20 bits. Unconnected ROM/save address bits mirror physical power-of-two sizes. Only upper 768 KiB of each linear bank is visible at 40000–FFFFF. |
| SRAM | Codes 01/02: 32 KiB; 03: 128 KiB; 04: 256 KiB; 05: 512 KiB; FF initialization, byte reads/writes and C1 banking | Cartridge SRAM is battery-backed, separate from zero-initialized internal volatile RAM. Code 01 follows the current hardware survey (32 KiB), not older 8 KiB descriptions. No-save region reads FF and ignores writes. |
| External EEPROM | 10: 128 B, 50: 1024 B, 20: 2048 B; C4/C5 data, C6/C7 command, C8/C9 control/status | Word organization, separate read/write buffers, WEN/WDS, READ/WRITE/ERASE, WRAL/ERAL, abort/invalid controller combinations, write latch and busy/done status. Fixed 32-clock read / 3072-clock programming latency, not analogue-accurate. Writes start disabled. 2001's stale read-done quirk is retained. Internal console EEPROM is unsupported. |
| Bandai 2003 | CA/CB RTC transport with no attached RTC; CC/CD four GPO directions/data; CE read-only ROM mapping into save area; CF alias of C0; D0/D2/D4 low banks alias C1/C2/C3; D1/D3/D5 high two bits | RTC commands 10–1B transfer documented payload lengths, busy/ready handshake at byte granularity (64 clocks), and FF from absent RTC pull-up. No S-3511A date/time emulation, alarms, attached GPO peripherals, flash programming or lockout handshake. Invalid commands/control values and self-flash writes fail explicitly. |
| Nonvolatile browser saves | localStorage per SHA-256 of complete ROM bytes, automatic once/second, on pause/hide/pagehide, before replacing a ROM | Reset/model changes preserve in-memory saves. Reload/reopen requires reselecting local ROM. No ROM/save upload or server storage. Corrupt/size-mismatched slots are preserved and not overwritten; denied/full storage shows a notice. Browser profile clearing/private-session expiry removes browser data. No import/export UI. |
| CPU | Generic 8/16-bit register, ModR/M, segment, stack and string operations below | Unsupported opcodes/group extensions/ports/prefix combinations stop execution visibly, with CS:IP for unknown opcodes. No silent NOP fallback. |
| Interrupts | IF, vector table and stack entry, INT/INT3/INTO/IRET, divide faults, BOUND fault; hardware priority highest source first; B0 vector/request, B2 enable, B4 status, B6 acknowledgement | Enable gates request latching; changing enable does not clear latched status. Key edges, line match, VBlank and both timer sources generated. UART/cartridge IRQ sources, single-step trap and low-battery NMI not generated. STI and MOV/POP SS inhibit delivery for the next instruction. HLT wakes for eligible maskable IRQs. |
| Timers | A2 control, A4–A7 reload, A8–AB counter; HBlank and VBlank count sources, one-shot/repeat, reload-on-write and disabled-count-at-one IRQ quirk | Test modes at A3 unsupported. Word writes to A2 with high byte zero allowed. No programmable LCD line total (16 must remain 158). |
| Timing | Shared 3.072 MHz clock, 256 clocks/line, 159 lines/frame, 144 visible; deterministic frame progression | Per-instruction execution cost + fetch/data/I/O transfers, ROM/SRAM/cartridge-I/O wait flags, aligned 16-bit ROM transfers, branch/shift/multiply/divide/interrupt costs. Approximate, not cycle-table/prefetch accurate. Instructions are atomic; events/raster output occur at instruction granularity. |
| Backgrounds | Screen 1 and 2 maps, independent wrapping scroll, flips, palettes, color tile banks; inclusive inside/outside Screen 2 window | Video is rendered scanline-by-scanline from actual RAM/ports. No subpixel effects or one-line LCD pipeline delay. |
| Sprites | 128-entry VBlank-latched table, first/count, 32 per line including clipped sprites, early-entry priority, behind/in-front Screen 2, palette 8–15, flips, wrapped coordinates, inclusive inside/outside window | Table copies atomically at line 144 rather than word-by-word over that line; mid-copy effects unsupported. Tile bank 0 only as on hardware. |
| Mono video | 2bpp interleaved planar tiles at 2000–3FFF; palette ports 20–3F and shade LUT 1C–1F; LCD enable 14; grayscale RGBA 224×144 | No LCD ghosting, icons, contrast/panel response or gamma simulation. |
| Color video | 60=80: 2bpp planar, 1024 tiles at 2000–5FFF; C0: 4bpp planar; E0: 4bpp packed, 1024 tiles at 4000–BFFF; FE00–FFFF 12-bit RGB palettes | Packed nibbles are left/high, right/low. 2bpp palettes 4–7/12–15 make index 0 transparent; 4bpp index 0 always transparent. Mono rejects color register writes; WSC/Crystal extra RAM requires color enable. Invalid mode combinations rejected. |
| Sound | Four 32×4-bit wavetable channels; channel 2 unsigned PCM/voice volume; channel 3 signed sweep with wrap; channel 4 eight LFSR tap modes/reset | 80–94 register surface, 96–9B mixer readback, 9E speaker volume; 24 kHz stereo clock, bounded 200 ms queue. Speaker volume uses a linear approximation. Sound test port 95, Hyper Voice, sound/general DMA unsupported and rejected. Mixing/output filters are approximate. |
| Browser sound | Opt-in Enable/Mute sound, Web Audio buffers, high-pass DC removal, bounded queue, reused single AudioContext | Pause/restart/hide clears scheduled sources; failures show a notice and keep gameplay usable. No output loop while paused. Headless buffer validation does not certify physical speaker quality. Original demo stays silent. |
| Input | B5 active-high keypad matrix (yyyy xxxx bas0); keyboard/touch adapters | No serial/UART, link cable, sound/power buttons or accessories. |

## CPU instruction coverage

Implemented with generic operands, not ROM-specific dispatch:

- MOV register/memory/immediate/segment, XCHG, LEA, LES/LDS, XLAT; CBW/CWD.
- ADD/ADC, SUB/SBB, CMP, AND/OR/XOR, TEST; immediate/accumulator/register/memory
  forms including sign-extended imm8; INC/DEC preserve carry, NEG/NOT.
- ROL/ROR/RCL/RCR, SHL/SHR/SAR, byte/word, one/CL/immediate counts; zero-count
  flags preserved, defined one-bit OF, CF/SZP updates as appropriate. Shift /6
  is rejected. Undefined flag values are deterministic, not hardware guarantees.
- Unsigned/signed MUL/IMUL, immediate IMUL, DIV/IDIV with zero/quotient-overflow
  vector 0; AAM zero-base fault. Fault return points at the instruction start.
- DAA/DAS, AAA/AAS, AAM/AAD; CF/PF/AF/ZF/SF/OF arithmetic; logical AF cleared.
- PUSH/POP registers, segments, immediates and memory, PUSHA/POPA, PUSHF/POPF,
  LAHF/SAHF, ENTER/LEAVE.
- Near relative/indirect CALL/JMP and near RET with optional stack adjustment;
  far immediate/indirect CALL/JMP, RETF with optional adjustment; conditional
  branches, LOOP/LOOPE/LOOPNE/JCXZ, INT/INT3/INTO/IRET, BOUND, HLT.
- MOVS/STOS/LODS/CMPS/SCAS and INS/OUTS byte/word, DF and segment overrides;
  REP/REPE/REPNE execute one element per bounded step and compare repeats stop
  on the appropriate ZF condition. IN/OUT via immediate port or DX.
- CLC/STC/CMC, CLI/STI, CLD/STD, NOP; segment override and repeat prefixes.
  LOCK is accepted only for supported memory read/modify/write operations;
  instructions are atomic on this single-CPU bus (DMA absent). Invalid lock
  targets, excessive prefixes and REP on non-string instructions fail.

Remaining gaps include NEC extended opcodes/bit and BCD-string operations,
REPC/REPNC, WAIT/ESC coprocessor forms, undocumented aliases (82, shift /6,
SALC), single-step traps/NMI generation, exact NEC undefined-flag behavior,
prefetch and exact instruction/bus cycle tables. Unknown instructions still fail.

## Firmware-free startup and reset

Entry is FFFF:0000, physical FFFF0; no boot ROM is loaded. FLAGS=0002,
other CPU registers/segments zero, internal RAM zero, display/channels disabled,
C0=0F and C2=C3=FF for the common interface. Bandai 2003 uses C0/CF=3F
and ROM bank high bits=3 so even a 64 MiB ROM starts at its final bank;
RAM bank high bits remain zero. Cartridge save bytes start FF
unless restored by the adapter. ROM code must initialize stack, segments,
palettes, sound, timers and interrupts. EEPROM programming starts disabled.

This is a reproducible firmware-free handoff, not the real console's post-BIOS
state. Console internal EEPROM/user settings, authentication, RTC and boot-firmware
services are unavailable. `restart`/`change_model` reset volatile hardware while
preserving the cartridge and completed nonvolatile writes; a model incompatibility
fails before replacing state. `reset` is the explicit unload/clear API; browser
ordinary Restart/model controls use the preserving path. A failed ROM validation
preserves the running machine. Execution errors latch until restart or valid load.

## Independent hardware references and validation

Public hardware descriptions consulted (not emulator source):
[ROM header](https://ws.nesdev.org/wiki/ROM_header),
[memory map](https://ws.nesdev.org/wiki/Memory_map),
[common mapper](https://ws.nesdev.org/wiki/Mapper),
[Bandai 2001](https://ws.nesdev.org/wiki/Bandai_2001),
[Bandai 2003](https://ws.nesdev.org/wiki/Bandai_2003),
[EEPROM](https://ws.nesdev.org/wiki/EEPROM),
[V30MZ](https://ws.nesdev.org/wiki/NEC_V30MZ),
[interrupts](https://ws.nesdev.org/wiki/Interrupts),
[timers](https://ws.nesdev.org/wiki/Timers),
[SoC/modes](https://ws.nesdev.org/wiki/SoC),
[display](https://ws.nesdev.org/wiki/Display),
[display ports](https://ws.nesdev.org/wiki/Display/IO_Ports),
[screens](https://ws.nesdev.org/wiki/Display/Screens),
[windows](https://ws.nesdev.org/wiki/Display/Windows),
[tiles](https://ws.nesdev.org/wiki/Display/Tile_Data),
[sprites](https://ws.nesdev.org/wiki/Display/Sprites),
[palettes](https://ws.nesdev.org/wiki/Display/Palette),
[sound](https://ws.nesdev.org/wiki/Sound).

Native tests exercise hardware vectors and full cartridge paths. Browser smoke
checks use original licensed files only. See [fixture provenance/builds](../fixtures/README.md)
and [Issue #11 validation](issue-11-validation.md). Physical-device comparisons
are optional and have not been performed. Public Pages deployment and live-site
verification belong after merge and remain pending until actually executed.
