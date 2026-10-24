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
| Models | WonderSwan mono; WonderSwan Color; SwanCrystal (default) | Mono has 16 KiB internal RAM; color-enabled models have 64 KiB. WSC/Crystal share graphics architecture. A0 exposes live ROM controls and fixed model/status bits; 62 distinguishes WSC/Crystal. LCD panel characteristics are not simulated. |
| Header | Named entry/maintenance/publisher/model/ID/version/protection/size/save/orientation/ROM width/wait/mapper/checksum fields | Independent errors include bad field/value, declared/file size or checksum. Reserved flags and unknown model/save/mapper codes fail. Firmware authentication is absent. |
| ROM | Header-declared 128 KiB–16 MiB common interface, including raw 3/6 MiB; up to 64 MiB with mapper 01 | Exact raw file length and checksum required. 3/6 MiB top-align in rounded 4/8 MiB spaces; synthetic leading region reads FF by policy, not electrical certification. ROM is read-only. Header 00 identifies a common interface, not proof of a physical 2001 chip (KARNAK also uses 00). Pocket Challenge board features unsupported. |
| Common banking | C0 linear 1 MiB bank, C1 save 64 KiB bank, C2/C3 64 KiB ROM banks | Address bus wraps at 20 bits. ROM addresses wrap within rounded power-of-two capacity; saves mirror their power-of-two size. Only upper 768 KiB of each linear bank is visible at 40000–FFFFF. |
| SRAM | Codes 01/02: 32 KiB; 03: 128 KiB; 04: 256 KiB; 05: 512 KiB; FF initialization, byte reads/writes and C1 banking | Cartridge SRAM is battery-backed, separate from zero-initialized internal volatile RAM. Code 01 follows the current hardware survey (32 KiB), not older 8 KiB descriptions. No-save region reads FF and ignores writes. |
| External EEPROM | 10: 128 B, 50: 1024 B, 20: 2048 B; C4/C5 data, C6/C7 command, C8/C9 control/status | Word organization, separate read/write buffers, WEN/WDS, READ/WRITE/ERASE, WRAL/ERAL, abort/invalid controller combinations, write latch and busy/done status. Fixed 32-clock read / 3072-clock programming latency, not analogue-accurate. Writes start disabled. 2001's stale read-done quirk is retained. Internal EEPROM uses a separate controller below. |
| Internal EEPROM | Mono 128 B; Color and Crystal independently 2048 B, retained in mono mode; BA/BB data, BC/BD command, BE status/control; ordinary READ/WRITE, WEN/WDS and sticky settings protection | Six-bit mono / ten-bit Color command addresses follow active mode. BF reads zero and ignores writes. Direct ERASE, WRAL/ERAL and silicon edge cases remain unsupported; see detailed policies below. |
| Bandai 2003 | CA/CB RTC transport with independent optional S-3511A attachment; CC/CD four GPO directions/data; CE read-only ROM mapping into save area; CF alias of C0; D0/D2/D4 low banks alias C1/C2/C3; D1/D3/D5 high two bits | RTC commands 10–1B use host byte counts 1,1,1,1,7,7,3,3,2,2,2,2, including reset. Terminal READY contains the final read byte with ACTIVE clear; writes preload their first byte; command start ignores READY. The 64-clock scheduled stage is an approximation. Absent RTC reads FF. See [source reconciliation](rtc-transport.md). Equipped clocks implement retained BCD calendar/status/reset and host-injected time; see [attachment/time/storage policy](rtc-calendar.md). Retail detection, alarm/periodic IRQ delivery and cartridge IRQ wiring remain unavailable, as do attached GPO peripherals, flash programming and lockout handshake. Invalid commands/control values and self-flash writes fail explicitly. |
| Nonvolatile browser saves | localStorage per SHA-256 of complete ROM bytes, automatic once/second, on pause/hide/pagehide, before replacing a ROM | Reset/model changes preserve in-memory saves. Reload/reopen requires reselecting local ROM. No ROM/save upload or server storage. Corrupt/size-mismatched slots are preserved and not overwritten; denied/full storage shows a notice. Browser profile clearing/private-session expiry removes browser data. No import/export UI. |
| Console browser records | Separate `wonderswan:console:v1:mono`, `:color`, `:crystal` localStorage keys, base64 complete physical EEPROM bytes | Restored once per model before execution, independent of cartridge SHA-256 saves. Same periodic/lifecycle flushes as cartridge saves, including before model replacement. Corrupt/denied reads block overwriting that slot; quota failures report separately and leave emulation/cartridge saves usable. Browser-local only; no settings editor. |
| CPU | Generic 8/16-bit register, ModR/M, segment, stack and string operations below; two-byte FPO1 NOP and nonblocking console POLL | Unsupported opcodes/group extensions/ports/prefix combinations stop execution visibly, with CS:IP for unknown opcodes. No silent NOP fallback. |
| Interrupts | IF/TF, vector table and stack entry, INT/INT3/INTO/IRET, divide faults, BOUND fault; vector-1 single-step with POPF/IRET activation and saved-FLAGS continuation; hardware priority highest source first; B0 vector/request, B2 enable, B4 status, B6 acknowledgement | Enable gates request latching; changing enable does not clear latched status. Key edges, line match, VBlank and both timer sources generated. UART/cartridge IRQ sources and low-battery NMI are not generated. MOV/POP SS suppress their TF boundary and defer external INT through the next instruction. STI and IF-enabling POPF/IRET delays apply to external INT. Synchronous interrupts and eligible external INT outrank TF; one selected entry clears live IF/TF. Prefixes are indivisible; REP element/final/zero-count and HALT-completion trap behavior follow the [source reconciliation and explicit inference limits](single-step.md). HLT wakes for a latched IRQ even with IF=0, resuming without vectoring in that case. |
| Timers | A2 control, A4–A7 reload, A8–AB counter; HBlank and VBlank count sources, one-shot/repeat, reload-on-write and disabled-count-at-one IRQ quirk | Test modes at A3 unsupported. Word writes to A2 with high byte zero allowed. Programmable final line at 16; independent STN synchronization at 17. See [LCD policies](lcd-timing.md). |
| Timing | Shared 3.072 MHz clock, 256 clocks/line, programmable T+1 lines/frame (159 by default), 144 visible; actual raster-wrap frame progression | Per-instruction execution cost + fetch/data/I/O transfers, live A0 ROM waits, SRAM/cartridge-I/O waits, aligned internal RAM/documented I/O word pairs and 16-bit ROM reads, branch/shift/multiply/divide/interrupt costs. Ordinary IN/OUT uses six decoded clocks including its normal transfer, plus explicit fetch and additional transfer/wait charges; see [reconciliation and limits](ordinary-io-timing.md). Internal byte/even-word/odd-word costs are 1/1/2; see [eligibility and evidence](internal-word-timing.md). Approximate, not cycle-table/prefetch accurate. Instructions and DMA words are atomic; events/raster output use that granularity. |
| General DMA | Color/Crystal registers 40–42 source, 44–45 destination, 46–47 byte length, 48 control; increment/decrement word copies from RAM or mapped fast 16-bit ROM to internal RAM | Engine requires 60.7 color enable; mono rejects all GDMA register I/O. CPU and IRQ delivery stall for 5 + 2 × copied words on the existing clock. SRAM, byte-wide ROM, wait-state ROM and unmapped ROM stop at the first invalid source word; zero/initially invalid requests cost no DMA clocks. SDMA shares the clock and takes priority over the next GDMA word; see semantics below. |
| Sound DMA | Color/Crystal 4A–4C source, 4E–50 length, 52 control; byte transfers to Channel 2 or Hyper Voice, all four rates, hold/repeat/direction and bytewise repeat shadows | 20-bit counters and current RAM/ROM/SRAM mapper path, including slow/byte-wide sources. Phase 117 mod 128 reserves 6 + source access clocks every enabled tick; CPU/IRQ/GDMA resume at instruction/word boundaries. Reserved 4D/51 read zero, ignore writes. See timing and readback policies below. |
| Hyper Voice | Color/Crystal signed 16-bit output latches 64–67; alternating CPU 8-bit input at 69; DMA stereo/mono routing; 6A conversion/shift/rate/enable and 6B layout/reset | All eight divisors, signed/unsigned/unsigned-negated/ignored extension. Headphone-only 16-bit wrapping addition. Direct CPU write to internal left latch 68 and unknown 6B low-nibble controls rejected. See semantics below. |
| Backgrounds | Screen 1 and 2 maps, independent wrapping scroll, flips, palettes, color tile banks; inclusive inside/outside Screen 2 window | Scanline composition retains pixel identities; final shade/color translation follows one line later, including row 143 during line 144. Sampling and transition policies: [LCD pipeline](lcd-timing.md#one-line-shadecolor-translation-issue-37). No subpixel effects. |
| Sprites | 128-entry VBlank-latched table, first/count, 32 per line including clipped sprites, early-entry priority, behind/in-front Screen 2, palette 8–15, flips, wrapped coordinates, inclusive inside/outside window | Table copies atomically at line 144 rather than word-by-word over that line; mid-copy effects unsupported. Tile bank 0 only as on hardware. |
| Mono video | 2bpp interleaved planar tiles at 2000–3FFF; palette ports 20–3F and shade LUT 1C–1F; LCD enable 14; grayscale RGBA 224×144 | No LCD ghosting, icons, contrast/panel response or gamma simulation. |
| Color video | 60=80: 2bpp planar, 1024 tiles at 2000–5FFF; C0: 4bpp planar; E0: 4bpp packed, 1024 tiles at 4000–BFFF; FE00–FFFF 12-bit RGB palettes | Packed nibbles are left/high, right/low. 2bpp palettes 4–7/12–15 make index 0 transparent; 4bpp index 0 always transparent. Mono rejects color register writes; WSC/Crystal extra RAM requires color enable. Invalid mode combinations rejected. |
| Sound | Four 32×4-bit wavetable channels; channel 2 unsigned PCM/voice volume; channel 3 signed ordinary/fast sweep with wrap; channel 4 eight LFSR tap modes/reset | 80–94 documented bit masks, 95 clearing/FAST_SWEEP control/readback, 96–9B 10/11-bit legacy mixer readbacks, 9E speaker volume; 24 kHz stereo clock, bounded 200 ms queue. Headphone 16-bit and speaker 8-bit adders wrap; speaker main volume uses a linear approximation. HOLD_CH/other debug controls remain rejected. Analogue filters/volume levels are approximate. |
| Browser sound | Default-on sound preference, Mute/Unmute and 0–100% volume, gesture activation, Web Audio buffers, high-pass DC removal, bounded queue, reused single AudioContext | Pause/restart/hide clears scheduled sources; failures show a notice and keep gameplay usable. No output loop while paused. Headless buffer validation does not certify physical speaker quality. Original demo stays silent. |
| Input | B5 active-high keypad matrix (yyyy xxxx bas0); independent X/Y/action keyboard and touch controls, union of simultaneous sources | Exact keypad IRQ timing remains approximate. No serial/UART, link cable, sound/power buttons or accessories. |
| Orientation | Wasm `orientation()` returns `horizontal` by default or `vertical` from parsed header bit 0; browser portrait presentation rotates counterclockwise | Core framebuffer and canvas backing remain 224×144; portrait footprint 144:224, unchanged raw button identities and ROM/save bytes. Header selection only; runtime orientation signaling is unsupported. |

## Keypad controls and portrait presentation

The existing B5 selections are Y=10, X=20 and actions=40 (hex). Selecting several
rows returns their bitwise union. The browser maps X1/X2/X3/X4 to arrows
Up/Right/Down/Left or W/D/S/A, Y1/Y2/Y3/Y4 to I/L/K/J, A/B to Z/X and Start to
Enter. Small/touch screens expose all eleven labeled buttons. Keyboard aliases,
keyboard/touch overlap and separate pointers combine as a union; lifecycle and
focus changes clear input. Help explains that rotation never changes raw identities.

Header flag bit 0 selects vertical presentation on every model. The browser
rotates counterclockwise, mapping native `(x,y)` to `(y,223-x)`, around a wrapper
that reserves the rotated footprint. It keeps native RGBA pixels and 224×144
canvas backing, nearest-neighbor scaling and square-pixel geometry. No cartridge
bytes or content-based save identity change. Restart and compatible model
changes retain orientation; rejected replacements retain the previous display.

The source Issue cites [Wonderful's pinned B5 definitions](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/keypad.xml),
[pinned ares header handling](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/mia/medium/wonderswan.cpp),
and [pinned Beetle presentation handling](https://github.com/libretro/beetle-wswan-libretro/blob/0db0832183ad264bbe67bcd7e651365e7dac5389/libretro.c)
as corroboration. These are not physical-console measurements; no upstream
implementation code is incorporated. Original source-included CC0 CPU probes
check emulator behavior, without establishing commercial-title compatibility.
See [probe provenance](../fixtures/README.md#independent-keypad-and-orientation-probes-issue-29)
and [actual validation](issue-29-validation.md).

## Raw ROM length and mapped capacity

Exact 3,145,728-byte (size code 05) and 6,291,456-byte (07) images load without
padding or header edits on every compatible model and both mapper interfaces.
For raw length L, capacity C = next power of two. Mask the bank/offset address
by C − 1, return FF below C − L, otherwise read raw byte `address − (C − L)`.
Thus common-interface D0 starts raw 3 MiB data and A0 starts raw 6 MiB data;
FF remains the final bank with the original footer/reset jump. The same shared
map serves CPU, startup, General DMA, Sound DMA and 2003 self-map reads.
Power-of-two images have no leading gap and keep their previous mapping.

Raw bytes remain unchanged. Header size/checksum, core/Wasm `load_rom` return
length and `rom_len`, and browser SHA-256 save identity all describe the exact
original image, excluding synthetic fill. Files padded to a different length
while retaining their original size header are rejected with a declared/file
size diagnostic. Unsupported sizes/fields, mapper limits, checksums and model
restrictions retain their existing validation.

The previous power-of-two-only restriction and leading-padding advice could not
load a valid raw 3/6 MiB image. [Wonderful linker placement documentation](https://wonderful.asie.pl/wiki/doku.php?id=wswan:guide:memory_management)
defines bank placement relative to the ROM's upper end. Independently inspected
[pinned ares loader behavior](https://github.com/ares-emulator/ares/blob/64c835451157de74e418ca3fdbe1b5aec9972af9/ares/ws/cartridge/cartridge.cpp#L31-L38)
and [pinned Beetle loader behavior](https://github.com/libretro/beetle-wswan-libretro/blob/0db0832183ad264bbe67bcd7e651365e7dac5389/libretro.c#L554-L568)
corroborate rounded/top-aligned placement; Beetle explicitly fills the gap with FF.
These are placement evidence, not measurements of unpopulated physical cartridge
addresses. FF is an explicit emulation policy; electrical open bus remains
unverified. No emulator code was copied. See the [original raw-image probes](../fixtures/README.md#raw-36-mib-probes-issue-25)
and [Issue #25 validation](issue-25-validation.md). Commercial playability remains
untested; firmware-free startup and other hardware limitations are unchanged.

## Runtime ROM controls (A0)

All three models accept byte IN/OUT at A0. Bits 2 (16-bit ROM transfers) and
3 (one extra ROM wait clock) are writable. Bit 0 (IPL lock) is sticky: writing
zero cannot clear it. Model identification at bit 1 and self-test status at bit
7 ignore writes. Reserved bits 4–6 ignore writes and read zero. The synthetic
handoff keeps lock/status set and reports bit 1 on Color/Crystal only.

Successful cartridge load, restart and compatible model change seed bits 2/3
from the cartridge header's physical ROM width/wait flags. Software then controls
the live register without changing header metadata or ROM bytes. Failed loads
and incompatible model changes preserve live state. An unloaded new/reset bus
retains the prior synthetic 85/87 policy; these are not authentic power-on values.
No IPL mapping, firmware execution or authentication emulation is added.

ROM byte accounting is `1 + A0.3` clocks. An aligned word with adjacent physical
ROM addresses costs the same when A0.2 is set; otherwise it costs two byte
transfers. Instruction operand fetches and data reads share this rule. Odd,
segment-wrapped, 20-bit-wrapped and bank-crossing pairs are not incorrectly
coalesced. Opcode/prefix fetches and instruction execution retain the existing
approximation; there is no prefetch simulation or claim of exact instruction timing.

Configured width is separate from physical cartridge characteristics. With a
width mismatch the current deterministic mapping still returns the same ROM
bytes; only accounting and GDMA eligibility follow live controls. Exact mismatched
width data/electrical behavior is unresolved and is outside hardware-accuracy
assertions. SRAM/self-flash windows remain byte-wide; internal RAM and documented
I/O word pairs follow the [internal transfer table](internal-word-timing.md).
SDMA accepts either ROM wait setting
and samples the live source cost at each event; GDMA rejects slow/byte configuration.

## General DMA semantics

GDMA register bytes are little-endian. Source is 20 bits, destination and byte
length are 16 bits; their low bit and all reserved bits read back as zero.
Control retains direction (bit 6) and enable (bit 7).
Registers are available on Color/Crystal; enabling without color mode stops
without copying or charging setup time. Mono continues to report unsupported I/O.

On Color/Crystal, port 43 is the reserved high byte of the source segment:
writes are ignored and reads return 00, with ordinary I/O access costs and no
DMA work or stored state. Thus word OUT/IN at 42 completes normally; only the
low nibble written to 42 supplies source address bits 16–19, and the byte at
43 is discarded/read as zero. This handling also applies with color mode disabled.
Read-zero is an evidence-based emulation policy; exact electrical readback has
not been confirmed by physical-device tests. Mono still rejects 43. Port 49 remains unsupported on all models; Sound DMA
ports are available only on Color/Crystal as described below.

Each word is read completely before being written, in the selected direction;
overlaps therefore follow word transfer order. Both addresses advance by two,
wrapping source at 20 bits and destination at 16 bits, while length decreases by
two. ROM reads reuse the CPU's current common/extended bank mapping. Live A0.2/3
govern source eligibility for every bounded word (16-bit, no extra wait).
The 10000–1FFFF window is always
rejected, including the 2003 flash mapping. DMA never writes cartridge memory.
Completion or rejection clears enable, retains direction, and exposes the next
addresses and remaining length (unchanged if nothing transferred).

DMA runs before the next CPU instruction or IRQ delivery. The first word costs
seven clocks, subsequent words two; no normal CPU memory-access costs are added.
The existing scanline, timer, interrupt, audio and EEPROM clocks keep running,
including across `run_frame()` calls. The maximum request is 32767 words.
Timing remains approximate within a word: the enabling instruction completes
atomically, setup is grouped with the first word, and each word's memory changes
precede its clock advancement. Per-cycle read/write phases, display fetch
contention and physical-device comparisons are not modeled/verified.

## Sound DMA semantics and shared timing

On all three models, sound-test port 95 accepts 00 (clear) and 02 (FAST_SWEEP).
Bit 1 bypasses the fixed 8192-clock sweep prescaler on the existing 3.072 MHz
machine clock. The five-bit programmable time still applies: an update every
`(8D & 31) + 1` selected ticks, including one update per machine clock at time
zero in fast mode. Updates add signed 8C to the 11-bit channel-3 frequency with
wrap. Both channel-3 and sweep enable bits (90.2 and 90.6) are required; every
8D write resets both counters. Other channels/devices and output sampling keep
their existing clocks. Unknown bits, including HOLD_CH, produce actionable
diagnostics before changing control or timer state.

Readback of the accepted control value, zero on reset/restart, and retention of
counter phases across mode changes are deterministic emulator policies where
unmeasured. Fast mode pauses the ordinary prescaler; clearing resumes it while
retaining the programmable count. Disabling a prerequisite pauses both counters.
See the [source-to-behavior table and correction record](fast-sweep.md) and
[Issue #31 validation](issue-31-validation.md). This does not establish HOLD_CH,
complete debug-port behavior or commercial-title compatibility.

SDMA is Color/Crystal hardware, selected with the typed console model; it is
available regardless of the video mode. Mono rejects 4A–52 and 64–6B and retains
its four legacy channels. Source (4A–4C) and length (4E–50) are little-endian
20-bit byte counters: high nibbles are masked, odd addresses/lengths are valid.
Every CPU register-byte write changes that live byte and only the corresponding
shadow byte. Playback advances only live counters; repeat restores all six
shadow bytes when length reaches zero. Source wraps at 20 bits in either
direction. One-shot completion clears enable and leaves the next source and
zero length visible. Length zero cannot enable and an active zero-length edit
stops at the next controller tick without underflow. Enable toggles preserve
counters and the fractional rate-divider state.

52 retains bits DF: enable 80, decrement 40, Hyper Voice target 10, repeat 08,
hold 04, rate 03. Rates 0/1/2/3 divide the 24 kHz clock by 6/4/2/1 (4/6/12/24
kHz); the first sample is issued after that many enabled ticks. Hold advances
the rate timer and sends zero on sample ticks without consuming source/length.
Channel 2 target writes 89; software must enable voice at 90.5 and choose
left/right voice volume at 94. Hyper Voice target fills the inputs according to
6B layout; software must enable/configure the converter at 6A. SDMA itself does
not implicitly enable either sound generator or headphone output (91.3).

The DMA specification allocates no fields to 4D and 51: they are unused fourth
bytes adjacent to 20-bit counters. Like reserved GDMA port 43, writes have no
state or controller effects and reads return zero, including word I/O to 4C/50.
Read-zero is a deterministic emulation policy; the reference does not specify
the electrical value of these unused ports and no physical-console readback
measurement is claimed. Port 49 and unrelated gaps remain unsupported.

While enabled, SDMA reserves **6 + N clocks every 128 clocks**, beginning at
**117 mod 128**, including hold and ticks between rate-divided samples. N is the
mapped source byte access time (1 normally; 2 for configured SRAM/slow ROM).
Byte-wide SRAM, slow ROM, bank crossings, the extended mapper and its read-only
flash window use the same address map as CPU reads, without GDMA's source
eligibility restrictions. DMA does not write cartridge memory or add normal
CPU access-accounting charges. Disabled/zero-length transfers incur no new
reservation. The global 128-clock phase continues through disable and frames.

Clock advancement splits at every SDMA event, so scanlines, interrupts/timers,
EEPROM, RTC transport and audio advance on the same clock. The RTC calendar
advances only through injected host time. Each event records a CPU stall;
stalls are serviced before another CPU instruction, IRQ entry or GDMA word.
Their own elapsed clocks also advance devices. A clock slice cannot recursively
execute a CPU instruction, and the 7/8-clock reservation leaves CPU time between
events. Transfers/counters/output writes are atomic at phase 117; memory bus
subphases and the exact final latch cycle within the reservation are not modeled.
CPU effects remain instruction-granular: an instruction or GDMA word overlapping
phase 117 completes atomically, then pays the reservation before the next one.
This can delay the CPU-visible stall by the remainder of that instruction/word;
it does not move audio events or charge the transfer twice. Frame boundaries
preserve counters, divider phase and unpaid stalls. No prefetch/contention or
physical-device timing accuracy beyond this model is claimed.

## Hyper Voice conversion and output mixing

64/65 and 66/67 hold little-endian signed 16-bit left/right output, initially
zero. CPU writes update each output byte directly, including with the converter
disabled. 68/69 are left/right 8-bit input latches. CPU writes to 69 alternate
left then right regardless of the DMA layout; DMA layout 6B.5–6 is stereo,
left-only, right-only or both. A one-sided DMA write preserves the other input.
Writing 6B.4 resets the next alternating input to left; it is a strobe, not an
output clear. 6B.7 is reserved/masked and the layout bits read back; the unknown
low nibble is rejected when nonzero, with a diagnostic. The sound diagram shows
68 as the internal left input; direct CPU writes to 68 have no documented
semantics and remain explicitly unsupported. Reads expose the input/output
latches and 6A control; electrical readback/reset-strobe details beyond the
reference are deterministic policies, not independently measured hardware facts.

6A.7 enables conversion, 6A.4–6 selects divisors 1/2/3/4/5/6/8/12 of the existing
24 kHz output clock. Each conversion processes both held inputs; between updates
the outputs persist. Disabling freezes output latches and resets the conversion
divider on the next output tick; it does not clear the samples or alternating
input selector. 6A.2–3 selects extension; 6A.0–1 selects arithmetic right shift
0–3. Extension happens before shifting and retaining the bottom 16 bits:

| Mode | Extended intermediate for byte b | Output |
| --- | --- | --- |
| Unsigned | b × 256 | Shift then wrap to signed 16 bits |
| Unsigned, negated | (b − 256) × 256 | Shift then wrap; this is not arithmetic negation of b |
| Signed | signed8(b) × 256 | Arithmetic shift then signed 16 bits |
| Ignored | b × 256 | Shift ignored; retain bottom 16 bits |

For example E0 with shift 1 yields +28672 unsigned, −4096 unsigned-negated or
signed, and −8192 in ignored mode. Converter/divider startup phase is deterministic
but has not been checked on a physical device. Direct 16-bit writes bypass
conversion and are overwritten if the enabled converter subsequently updates.
Separate CPU writes can straddle a 24 kHz sample boundary; they are not treated
as an atomic stereo pair.

The four legacy unsigned outputs sum into separate 10-bit left/right readbacks
(96–99); their sum is 11 bits (9A/9B). The headphone path shifts each legacy sum
left by five, adds its Hyper Voice signed output, **wraps at 16 bits**, and divides
by 32768 for the existing bounded float sample contract. No adder clamps and no
extra gain is applied. 91.3 selects headphone output. The speaker path (91.0
when headphone output is off) sums only legacy channels, shifts by 91.1–2,
retains eight bits and applies the existing linear 9E.0–1 main-volume approximation.
Hyper Voice never feeds this path; 9E never scales headphones. Disabling both
outputs emits zero. Legacy wavetable, voice priority/volume, sweep and all eight
noise modes remain supported; test port 95 is still a documented gap. The Rust
sample rate remains 24000 Hz with a bounded 200 ms stereo queue. The browser
reuses its existing high-pass filter and gain graph; no JS synthesis was added.

## CPU instruction coverage

Implemented with generic operands, not ROM-specific dispatch:

- MOV register/memory/immediate/segment, XCHG, LEA, LES/LDS, XLAT; CBW/CWD.
- ADD/ADC, SUB/SBB, CMP, AND/OR/XOR, TEST; immediate/accumulator/register/memory
  forms including sign-extended imm8; INC/DEC preserve carry, NEG/NOT.
- ROL/ROR/RCL/RCR, SHL/SHR/SAR, byte/word, one/CL/immediate counts; counts
  masked to five bits. Shifts clear AF and update SZP even for masked-zero
  counts; rotates preserve AF/SZP. Masked-zero counts preserve CF; nonzero
  counts produce the last shifted bit. All counts set left OF to final CF XOR
  result MSB, and right OF to the XOR of the two highest result bits, on all
  three physical models independently of graphics mode. Shift /6 is rejected.
  See [source reconciliation and experimental limits](shift-rotate-flags.md).
- Unsigned/signed MUL/IMUL, immediate IMUL, DIV/IDIV with zero/quotient-overflow
  vector 0; AAM zero-base fault. Fault return points at the instruction start.
  Multiplication clears AF/PF/SF; signed forms (including 69/6B) set ZF on all
  models. Unsigned ZF clears on physical WonderSwan and sets on Color/Crystal,
  independently of graphics mode. Products, CF/OF and control flags retain their
  ordinary rules. See [source reconciliation and experimental limits](multiplication-flags.md).
  Successful word DIV/IDIV clear AF/CF/OF/PF/SF and set ZF only for an exact
  odd quotient on every emulated model, independently of graphics mode.
  Control flags and quotient/remainder remain unchanged by this correction.
  Successful byte IDIV clears AF/CF/OF and derives SF/ZF/PF from quotient AL,
  preserving unrelated FLAGS on every model and across graphics-mode changes.
  Successful byte DIV clears AF/PF/SF, sets ZF for an exact odd quotient and
  restores CF/OF from private explicit-multiplication history, independently of
  live FLAGS and graphics mode on every model. Synthetic startup clears history;
  clones preserve it, successful lifecycle changes replace it and rejected changes
  preserve it. This startup value is policy, not a measured firmware handoff.
  Exceptional-result refinements and implicit conversion-history effects remain
  unresolved. See [word-division source reconciliation](word-division-flags.md),
  [byte-IDIV correction](byte-idiv-flags.md) and
  [byte-DIV evidence, inferences and limits](byte-div-flags.md).
- DAA/DAS, AAA/AAS, AAM/AAD; CF/PF/AF/ZF/SF/OF arithmetic; logical AF cleared.
  Successful AAM clears AF/CF/OF and derives SZP from remainder AL; AAD truncates
  AH × immediate base to a byte, adds original AL, clears AH and takes all six
  arithmetic flags from that final byte ADD. Control/fixed bits and zero-base
  AAM fault handling are preserved on all models and across supported graphics
  modes. See [conversion correction and evidence limits](conversion-flags.md).
  AAA/AAS mask AL before independent byte adjustment, preserving AH byte wrapping.
  PF is set, OF cleared, AF/CF/ZF equal the incoming-AF-or-nibble adjustment
  condition, and SF is its inverse; these are not ordinary result SZP flags.
  Both use nine execution clocks plus existing fetch/wait charges on every model
  and supported graphics mode, preserving control/fixed flags. See
  [AAA/AAS source reconciliation and limits](ascii-adjust.md).
  DAA/DAS select 60/06 adjustments from original AL and incoming CF/AF, preserving
  AH. CF/AF equal those conditions without extra adjustment carry/borrow; SZP
  comes from final AL and OF from byte CMP final AL, original AL. Both use ten
  execution clocks plus existing fetch/wait charges on every supported model/mode,
  preserving control/fixed flags. See [DAA/DAS reconciliation and limits](decimal-adjust.md).
- PUSH/POP registers, segments, immediates and memory, PUSHA/POPA, PUSHF/POPF,
  LAHF/SAHF, ENTER/LEAVE. Both PUSH SP register encodings (54 and FF F4)
  store SP after the wrapping two-byte decrement, independently of model and
  graphics mode. Memory sources retain their pre-write sampling and PUSHA
  saves entry SP. See [assumption correction and evidence limits](push-sp.md).
- Near relative/indirect CALL/JMP and near RET with optional stack adjustment;
  far immediate/indirect CALL/JMP, RETF with optional adjustment; conditional
  branches, LOOP/LOOPE/LOOPNE/JCXZ, INT/INT3/INTO/IRET, BOUND, HLT.
- MOVS/STOS/LODS/CMPS/SCAS and INS/OUTS byte/word, DF and segment overrides;
  REP/REPE/REPNE execute one element per bounded step and compare repeats stop
  on the appropriate ZF condition. IN/OUT via immediate port or DX.
- CLC/STC/CMC, CLI/STI, CLD/STD, NOP; segment override and repeat prefixes.
  D8–DF FPO1 consumes exactly one following byte as a no-op; 9B POLL is
  nonblocking under the effective POLLB-low console profile on all three models.
  See [source reconciliation, fetch costs and limits](fpo1-poll.md).
  F0 BUSLOCK uses ordinary decoding for supported memory and I/O instructions;
  it is ineffective on supported register-only instructions and NOP. Repeated
  BUSLOCK and segment overrides are accepted, with the last segment taking effect.
  Unsupported underlying instructions/ports, excessive prefixes and REP on
  non-string instructions still fail. See the source reconciliation below.

Remaining gaps include electrical POLLB pin behavior, undocumented aliases
(82, shift /6, SALC), NMI generation, exact REP/HALT trap phases, other NEC undefined-flag behavior,
prefetch and exact instruction/bus cycle tables. Unknown instructions still fail.

### BUSLOCK source reconciliation (Issue #35)

The old AI-authored policy required x86-style memory read/modify/write targets.
The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§1.3, printed pp. 14–16, instead describes locking for memory or I/O access.
Without those accesses the lock output stays inactive and bus hold is possible;
executing the supported underlying instruction with an ineffective prefix is our
inference. It guarantees redundant prefixes through seven total. §5.3 excludes
interrupt acknowledgement between prefixes and their instruction.

The decoder consumes F0 without operand lookahead or a separate target validator.
All operands and errors use the existing instruction paths. The existing defensive
limit accepts eight prefixes and rejects the ninth; behavior beyond seven is
emulator policy, with no hardware fidelity claim. IRQ delivery occurs before a
step, never inside prefix decoding. Bounded REP still performs one element per
step and restarts at the first prefix, including after interrupt return.

CPU effects remain instruction-granular; GDMA words and SDMA stalls are serviced
at the existing boundaries. This does not model BUSLOCKB transitions, fetch
suppression, prefetch/pipeline timing or exact bus arbitration. The preliminary
core manual does not establish console DMA wiring or POLL wiring. Issue #67
implements single-step using chapter 5 and separately records REP/HALT inferences
and trap-entry accounting in [the reconciliation](single-step.md). External
IRQ entry uses [32 aligned internal clocks](external-irq-timing.md) plus existing
odd-stack and wait penalties in both delivery paths. [LOOP/LOOPE/LOOPNE](loop-timing.md)
use instruction-specific decoded costs plus explicit fetches and a taken odd-target
clock. [Direct E9/EB/EA JMP](jmp-timing.md) uses four/four/seven decoded
execution clocks plus an odd-target clock and actual fetches.
[Jcc/JCXZ](jcc-timing.md) use four/one decoded taken/fallthrough clocks,
plus one for taken odd destinations and actual byte fetches. CALL, indirect
JMP and general prefetch timing remain separate work. The original
CC0 BUSLOCK cartridge and independent native/browser literals exercise all three models and restart; commercial-title impact is unmeasured.

The adjacent gap list also previously misclassified excluded older NEC operations
and REPC/REPNC as required V30MZ features. §§1.3/3.6.1 exclude them, while retaining
carry/direction operations (CLC/STC/CMC, CLD/STD here). The original Issue #35
assessment left FPO1 byte consumption unresolved because Appendix A depicts
displacement-bearing forms. Issue #41 supersedes that prerequisite using the
reported console two-byte NOP rule and nonblocking POLL behavior; see the
[correction record and remaining uncertainties](fpo1-poll.md). The earlier
[BUSLOCK validation and review](issue-35-validation.md) remains historical evidence.

## Firmware-free startup and reset

Entry is FFFF:0000, physical FFFF0; no boot ROM is loaded. FLAGS=F002
(including MD=1; 8080 emulation is unavailable). PUSHF, interrupt entry and
POPF/IRET keep bits 1 and 12–15 set and reserved bits 3 and 5 clear.
Successful cartridge handoff supplies SP=2000 on all three physical models and
DS=FF00 on WonderSwan or FE00 on WonderSwan Color/SwanCrystal, including when a
color console runs a mono-compatible cartridge. These values apply before the
first footer instruction on load, restart, same-model restart and compatible model
changes. Unloaded `Cpu::default()`/`Machine::new`/`reset`, and restart/model changes
without a cartridge, keep SP and DS zero.

The previous AI-authored assumption that zero SP/DS plus mandatory cartridge
initialization sufficed was incorrect. The pinned
[WSHWTest startup observations](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/README.md#start-registers)
report these values for all boot ROMs/modes, mapping ASWAN to WonderSwan and
SPHINX/SPHINX2 to Color/Crystal. Its
[startup assembly](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/WSHWTest.asm#L49-L74)
captures incoming SP at line 52 before replacing it. The later store at line 63
is commented out, and line 619 reads the preserved capture. DS is captured before
initialization. SS is overwritten first, so this evidence does not establish
incoming SS. The author qualifies
interpretations; these sources were inspected, not executed on physical hardware
here. This bounded correction does not adopt the uncertain startup I/O tables.

Other CPU registers/segments (including SS) and internal RAM are synthetic zeros,
display/channels are disabled, and C0=0F and C2=C3=FF for the common interface.
Bandai 2003 uses C0/CF=3F
and ROM bank high bits=3 so even a 64 MiB ROM starts at its final bank;
RAM bank high bits remain zero. Cartridge save bytes start FF
unless restored by the adapter. ROM code must initialize its working stack, segments,
palettes, sound, timers and interrupts. Cartridge EEPROM programming starts disabled.

A0 width/wait is seeded from header flags under the runtime-control policy above.
This is a reproducible firmware-free handoff with the bounded SP/DS observations
above; all other deterministic defaults remain synthetic unless separately
documented. DS points at the reported boot-ROM base, but no firmware bytes or
services exist there. Full authentic post-BIOS state is not implemented.
Internal EEPROM uses the minimal handoff described below. Authentication and
broader boot-firmware services are unavailable. `restart`/`change_model`
reset volatile hardware while preserving the cartridge and completed nonvolatile
writes; a model incompatibility fails before replacing state. `reset` unloads the cartridge and clears volatile
state but retains all three console EEPROM records; browser ordinary Restart/model
controls use the preserving path. A failed ROM validation
preserves the running machine. Execution errors latch until restart or valid load.

## Internal EEPROM transfers, retention and API

BA/BB have independent little-endian write and read buffers; writes to the data
ports never change readback. BC/BD retain a 16-bit command. Command layout is
`start=1`, two operation bits and six address bits in mono mode, or ten address
bits with Color enabled at 60.7. READ is op 2/control 10, WRITE op 1/control 20,
and WDS/WEN op 0/subcommand 0/3 (top two address bits)/control 40. Mono commands
address the first 128 bytes on every model. Switching active video/command mode
never reallocates bytes or resets write-enable/protection. Pending operations
capture their decoded address, data and write-enable decision when accepted;
completion does not reinterpret them using later register/mode values.

BE bit 0 is READ DONE, cleared at each accepted READ and set on completion.
Bit 1 is READY, clear while an operation is pending and set after it finishes.
Other completed operations leave DONE and the read buffer unchanged. Bit 7 is
settings protection: writing one sets a sticky latch, readback reports it, and
writing zero cannot clear it. It does not abort a transfer. Words at byte 5E
remain writable; words from byte 60 through the physical end reject writes when
protected. WDS separately locks all writes; WEN does not remove protection.
Protection also applies at completion, including if set while a write is pending.
BF, the reserved high byte of word IN/OUT BE, reads zero and ignores every write
without changing controller state. Lower reserved BE bits are ignored.

The shared emulated clock completes READ in 32 clocks and WRITE/WEN/WDS in
3072 clocks. These are **bounded approximations**, not measured internal EEPROM
latencies (including short WEN/WDS commands). Polling consumes normal CPU/I/O
clocks; scanlines, timers, audio and both DMAs keep advancing. Unsupported direct
ERASE, WRAL/ERAL, malformed command formats, mixed operation controls and new
requests while busy produce explicit diagnostics without changing stored bytes.
No external EEPROM bulk/abort behavior is inherited. Serial command adaptation,
invalid-command silicon behavior, protection races and precise busy/DONE timing
outside these ordinary transfers remain unresolved hardware gaps.

Each `Machine` owns one record per physical model. Completed bytes survive
restart, reset/unload, cartridge replacement and switching away/back. Pending
operations are discarded by those lifecycle resets. Failed ROM validation,
incompatible model changes and failed imports preserve existing state. At every
cartridge load/restart/compatible model change, the firmware-free handoff enables
ordinary internal writes and sets settings protection from header byte 9 bit 7:
zero protects, one requests the writable-settings exception. Software may protect
again; a subsequent handoff reapplies the new cartridge's flag. This explicit
policy does not execute firmware, update cartridge/boot counters or synthesize
other firmware services.

New records have FF in bytes 00–5F and zero in bytes 60 through the physical end:
an empty owner name, unknown personal fields and zero counters/options. This is
a deterministic synthetic initial record, **not a factory image** or an assertion
that zero settings implement all firmware expectations. Imported data is opaque.

Core `Machine::console_data() -> &[u8]` exports completed bytes for the active
physical model; `load_console_data(&[u8]) -> Result<(), Error>` accepts exactly
128 bytes for Mono or 2048 for either Color model. Successful administrative
import replaces that model's bytes, cancels pending transfers, clears buffers and
reapplies the current header handoff (or disabled/unprotected state if unloaded).
Failure changes nothing. The thin Wasm equivalents `console_data(): Uint8Array`
and `load_console_data(data: Uint8Array)` copy across the boundary and throw on
length mismatch. These APIs are separate from cartridge `save_data/load_save`;
no ROM is required. `reset(model)` selects/unloads, retaining every record;
`change_model(model)` retains the loaded cartridge and its save when compatible.

The browser restores each model's versioned record once before executing it,
then retains the core's record across cartridge changes. It flushes on the existing
one-second timer, pause, hide/pagehide and before ROM/model replacement. Unreadable
records remain untouched for the session; storage failures have their own notice
and cannot clear cartridge save diagnostics. Clearing browser storage removes
console data; no record or ROM is uploaded. Cross-tab conflict resolution and a
settings/import/export UI are outside this milestone.

## Independent hardware references and validation

Public hardware descriptions consulted (not emulator source):
[ROM header](https://ws.nesdev.org/wiki/ROM_header),
[memory map](https://ws.nesdev.org/wiki/Memory_map),
[common mapper](https://ws.nesdev.org/wiki/Mapper),
[Bandai 2001](https://ws.nesdev.org/wiki/Bandai_2001),
[Bandai 2003](https://ws.nesdev.org/wiki/Bandai_2003),
[EEPROM](https://ws.nesdev.org/wiki/EEPROM),
[V30MZ](https://ws.nesdev.org/wiki/NEC_V30MZ),
[NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)
(§1.3 BUSLOCK, prefix guarantees, excluded instructions and zero-count shifts;
§5.3 interrupt delay, §6.3.1 HALT release, Table 7-2 reset FLAGS),
[interrupts](https://ws.nesdev.org/wiki/Interrupts),
[timers](https://ws.nesdev.org/wiki/Timers),
[General DMA](https://ws.nesdev.org/wiki/DMA),
[timing](https://ws.nesdev.org/wiki/Timing),
[SoC/modes](https://ws.nesdev.org/wiki/SoC),
[display](https://ws.nesdev.org/wiki/Display),
[display ports](https://ws.nesdev.org/wiki/Display/IO_Ports),
[screens](https://ws.nesdev.org/wiki/Display/Screens),
[windows](https://ws.nesdev.org/wiki/Display/Windows),
[tiles](https://ws.nesdev.org/wiki/Display/Tile_Data),
[sprites](https://ws.nesdev.org/wiki/Display/Sprites),
[palettes](https://ws.nesdev.org/wiki/Display/Palette),
[sound](https://ws.nesdev.org/wiki/Sound),
[Hyper Voice](https://ws.nesdev.org/wiki/Hyper_Voice).

For internal EEPROM, the independently authored implementation also consulted
[libws EEPROM definitions and protection documentation](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/e1f2a59c5e61382b1b5339098414c143c2aea5f0/libws/include/ws/eeprom.h),
[active-mode handle selection](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/e1f2a59c5e61382b1b5339098414c143c2aea5f0/libws/src/eeprom/eeprom_handle_internal.c),
and [register definitions](https://wonderful.asie.pl/doc/libws/ports_8h_source.html).
The [public internal EEPROM experiment source](https://github.com/asiekierka/ws-test-suite/blob/7dfa0e2e869d08386b685d6a56df0bcfaf181b47/src/mono/eeprom/internal/main.c)
was inspected for independent buffer/status/locking/protection/mode vectors and
its explicit uncertainty about direct erase. It was not built/run, copied into
our fixtures or reported as a hardware pass. No Ares/emulator implementation or
inaccessible WSdev snippets establish behavior here. The CC0 probes are original.
See [Issue #21 validation and source review](issue-21-validation.md).

Native tests exercise hardware vectors and full cartridge paths. Browser smoke
checks use original licensed files only. See [fixture provenance/builds](../fixtures/README.md)
and [Issue #11 validation](issue-11-validation.md), plus
[General DMA validation](issue-13-validation.md) and
[SDMA/Hyper Voice validation](issue-19-validation.md). Physical-device comparisons
are optional and have not been performed. Public Pages deployment and live-site
verification belong after merge and remain pending until actually executed.

For A0, [pinned Wonderful hardware register definitions](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/soc.xml),
[WSHWTest A0 experiments](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/WSHWTest.asm#L1270-L1295),
[independent GDMA experiments](https://github.com/asiekierka/ws-test-suite/blob/7dfa0e2e869d08386b685d6a56df0bcfaf181b47/src/color/dma/alignment_access/main.c)
and [SDMA experiments](https://github.com/asiekierka/ws-test-suite/blob/7dfa0e2e869d08386b685d6a56df0bcfaf181b47/src/color/dma/sound_dma/main.c)
were inspected. The NEC manual's Table 2-1 and §§4.1–4.3 explain word bus cycles
and added waits; console-specific controls come from the register definitions
and experiments. These correct the old constant-A0/header-only runtime assumption;
header flags seed the synthetic handoff, then software changes live controls.
Experiments are source evidence, not physical-console pass logs. No third-party
code was copied; no corroborating emulator implementation establishes silicon
requirements. See [Issue #23 validation and source review](issue-23-validation.md).
