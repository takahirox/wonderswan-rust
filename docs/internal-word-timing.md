# Internal CPU word transfers (Issue #39)

This transfer table was recorded before implementation. It replaces the old
assumption that every CPU word costs two byte transfers except selected ROM
reads. Costs below are bus clocks, not complete instruction execution times.

| Region / operation | Byte | Even word | Odd word | Eligibility / evidence |
| --- | --- | --- | --- | --- |
| Accessible internal RAM, read/write/operand fetch | 1 | 1 | 2 | 00000–03FFF in every mode; 04000–0FFFF additionally in Color/Crystal color mode |
| Documented internal word-register pairs, IN/OUT | 1 | 1 | 2 | Explicit pairs below; existing byte-lane semantics still apply |
| Inaccessible internal RAM | 1 | 2 | 2 | Retain FF/ignored-write policy; physical unconnected/alias timing unresolved |
| SRAM / cartridge self-map | 1 + existing wait | 2 × byte | 2 × byte | Byte-wide; no new external wait assumptions |
| ROM reads, live A0 word width | 1 + A0.3 | 1 + A0.3 | 2 × byte | Existing A0.2 rule; contiguous, nonwrapping pair only |
| ROM reads, live A0 byte width | 1 + A0.3 | 2 × byte | 2 × byte | Existing behavior retained |
| Cartridge I/O | 1 + existing wait | 2 × byte | 2 × byte | C0 and above; no word discount |

The [NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
chapter 4, §§4.1–4.3, Table 4-1 (printed pp. 47–50), defines one-clock
no-wait bus cycles, byte/even-word/odd-word costs 1/1/2, the same alignment rule
for I/O, odd-stack penalties and two aligned vector-table word reads. This is
primary evidence for the CPU interface, not independent proof of console wiring.
The [pinned ares bus classification](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/cpu/cpu.cpp)
corroborates internal RAM/I/O word width and cartridge SRAM/I/O byte width.
Its timing constants are not treated as silicon measurements or copied here.

## Register eligibility and byte lanes

The following explicit pairs come from the pinned Wonderful hardware definitions
at `46783132a59e576c11456f5997620594eca1c390` (community console documentation):

| Source | Even low ports | Directions / lane qualifications |
| --- | --- | --- |
| [Timers](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/timers.xml) | A4, A6; A8, AA | Reload IN/OUT; counters IN only. Each reload byte keeps its existing reload-on-write effect. |
| [Internal EEPROM](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/eeprom.xml) | BA, BC | Data and command IN/OUT, separate data input/output buffers. BE is byte control, BF reserved: no inferred word register. |
| [DMA](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/dma.xml) | 40, 44, 46, 4A, 4E | Color/Crystal only, IN/OUT even in mono mode. Preserve alignment masks and bytewise SDMA shadows. 42/43, 4C/4D, 50/51 are not word registers; high lanes remain reserved. |
| [Sound](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/sound.xml) | 80, 82, 84, 86; 92, 96, 98, 9A | Frequencies IN/OUT, high lane masked to 3 bits. LFSR/output monitors IN only; writes remain unsupported. |
| [Hyper Voice](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/hyper_voice.xml) | 64, 66, 6A | Color/Crystal only, IN/OUT. Existing high-control validation/reset effects retained. |

Register width does not authorize additional byte-lane behavior. Unlisted adjacent
byte registers keep separate charges; broad console decoding and reserved-port
electrical behavior remain unresolved. In particular 16/17, A2/A3 and BE/BF
retain their current data behavior without inventing a word-register discount.
Decide eligibility before dispatch; dispatch low then high exactly once. A failed
lane retains its error and preceding side effects. Unrelated byte instructions
never consolidate, and no consolidation crosses offset, physical or port wraps
or an incompatible/inaccessible region.

## Scheduling limits

The existing explicit fetch approximation remains: opcode/prefix bytes are charged
separately; only existing operand-word helpers consolidate. No prefetch queue or
absolute instruction-cycle accuracy is claimed. CPU steps return execution plus
bus costs to the existing shared clock once. DMA continues to use its separate
mapped accesses and costs. Firmware-free startup, live A0 controls, interrupts,
bounded REP, LCD/audio/device scheduling and browser-local ownership remain in
their existing paths. Commercial games and physical consoles are untested.

Validation and the primary-specification correction to the browser audio
measurement are recorded in [Issue #39 validation](issue-39-validation.md).
