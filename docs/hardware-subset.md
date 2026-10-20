# Hardware implemented for Beacon Trail (Issue #7)

The emulator is written from scratch. The references below describe hardware;
no emulator implementation was used or ported. The cartridge source and the
browser-independent CPU/bus/video core remain separate: the core has no game
identifier checks, score/position logic, generated game frames, or embedded ROM.
The browser supplies bytes and a keypad mask and copies the Rust RGBA output.

## Cartridge and firmware-free entry

The [ROM footer](https://ws.nesdev.org/wiki/ROM_header) occupies the final 16 bytes:
far jump, maintenance byte, publisher, model flag, game ID/version, size, save type,
flags, mapper, and little-endian additive checksum excluding its own two bytes.
We validate the jump opcode, maintenance low bits, model flag, file/declared size,
checksum, and supported hardware flags. The supported subset is power-of-two
128 KiB–4 MiB images with mapper 2001, no save hardware, and standard orientation/
ROM width/wait flags. Other formats are rejected explicitly. Color-marked images
are rejected in the mono model, but accepting their header on color models does
not promise support for color graphics or their instructions.

[Memory map](https://ws.nesdev.org/wiki/Memory_map) and
[mapper](https://ws.nesdev.org/wiki/Mapper): physical addresses wrap at 20 bits.
The mono model has 16 KiB RAM, color models 64 KiB. Unmapped memory reads FF.
The no-save region 10000–1FFFF reads FF and ignores writes. ROM is read-only;
C2/C3 select 64 KiB banks at 20000/30000, and C0's low nibble selects the upper
bank bits for 40000–FFFFF. Power-of-two ROMs mirror by their physical size.

We approximate a post-firmware handoff at **CS:IP = FFFF:0000** (physical FFFF0),
with boot ROM absent/locked out, C0=0F, C2=C3=FF, other registers zero, FLAGS=0002,
RAM zeroed, and display disabled. This is deliberately not the exact register,
RAM, palette or EEPROM state left by real [boot firmware](https://ws.nesdev.org/wiki/Boot_ROM).
The demo initializes everything it uses, including stack and segments. Neither
firmware code nor a proprietary BIOS image is present. Model identity at A0 is
85 (mono) / 87 (color). Authentication, EEPROM, RTC and saves are absent.

## CPU

The [NEC V30MZ](https://ws.nesdev.org/wiki/NEC_V30MZ) implements the 80186 instruction
architecture. This milestone implements only the instruction families required
by the cartridge, with general register/ModR/M operand decoding rather than
special treatment of this program:

- 8/16-bit MOV: registers, memory, immediates and segment registers; AL/AH etc.
- ADD, OR, AND, SUB, XOR, CMP: accumulator, register/memory and immediate groups
  (including sign-extended imm8). TEST register/memory and accumulator forms.
- Register and memory INC/DEC; carry preserved. CF/PF/AF/ZF/SF/OF arithmetic
  behavior; logical AF is deterministically cleared (undefined on hardware).
- Register and segment PUSH/POP, near relative CALL/RET, short/near relative
  and far immediate JMP, all short conditional branches, and LOOP.
- MOVSB/W and STOSB/W, with REP and DF; REP is executed one element per bounded
  step. Segment overrides and 16-bit effective addresses, BP defaulting to SS.
- Byte/word IN/OUT via immediate port or DX, NOP, CLI, CLD/STD, CLC/STC, HLT.

Everything else, including unsupported group extensions, REP combinations,
excessive prefixes and unknown ports, returns an explicit error. Execution faults
remain latched until reset/reload. Interrupt delivery, most of the instruction
set, exact bus/cycle timing, and hardware exceptions remain unimplemented. HLT
stays halted until reset because interrupts are not implemented.

## Display and input

[Screen 1](https://ws.nesdev.org/wiki/Display),
[2bpp tile data](https://ws.nesdev.org/wiki/Display/Tile_Data), and
[display ports](https://ws.nesdev.org/wiki/Display/IO_Ports): the framebuffer is
224×144 RGBA pixels. It is decoded from the 32×32 little-endian tilemap (base
selected by 07), 512 interleaved planar tiles at 2000, palette indices in tile
attributes, horizontal/vertical flips, wrapping X/Y scroll (10/11), display
control 00/01, LCD enable 14, mono palettes 20–3F and shade LUT 1C–1F. Shade
nibbles map to linear grayscale. The game uses only Screen 1 and palette zero.
Other display layers, sprites, color modes, LCD response and scanline effects
are not implemented; attempts to enable unsupported layers/map modes fail.

[Keypad](https://ws.nesdev.org/wiki/Keypad) port B5 selects matrix rows using bits
4–6; the low nibble reads active-high selected buttons, combining rows if needed.
The public button mask follows `yyyy xxxx bas0`. The browser maps arrows/WASD to
X1(up)/X2(right)/X3(down)/X4(left), Enter to Start, and Z/X to A/B. Game movement
and restart occur only when cartridge instructions read B5. Blur, pause, hidden
tabs, loading and model resets clear input. The keyboard ignores editable and
native form controls and prevents scrolling for active gameplay bindings.

## Scheduling and limits

[Timing reference](https://ws.nesdev.org/wiki/Timing): 3.072 MHz, 256 clocks per
line, 159 lines per frame, 144 visible lines, about 75.472 Hz. Port 02 advances
through these scanlines; the ROM polls it to pace updates. Each decoded CPU
instruction/REP element currently costs **four approximate clocks**, regardless
of actual V30MZ timing. Video is rendered at the end of each 40,704-clock frame;
raster effects and pixel-level scheduling are absent. This is sufficient for the
small polling loop; it is not a timing accuracy claim.

The browser uses requestAnimationFrame with elapsed-time accumulation, at most
four emulated frames per callback, and bounded catch-up. It stops scheduling
when paused, idle, failed, or hidden. Returning to a hidden tab requires Resume.
All three model choices run the same mono hardware subset; neither successful
playthroughs nor the selected identity establish full model accuracy. The demo
is silent. Full commercial compatibility is explicitly outside this milestone.
