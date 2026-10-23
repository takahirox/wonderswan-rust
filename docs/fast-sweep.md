# Sound test port 95: evidence and bounded behavior

Recorded before implementation for [Issue #31](https://github.com/takahirox/wonderswan-rust/issues/31).
All numbers below are hexadecimal unless a unit/formula says otherwise.

Primary references inspected at pinned revisions:

- [WonderfulToolchain sound definitions](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/sound.xml).
- [WSHWTest sound-sweep documentation](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/README.md#sound-sweep)
  and [experiment source](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/WSHWTest.asm#L1890-L1950).

“Experiment” means inspected source assertions, not a physical pass observed here.
The experiment author's README also cautions that its interpretations may change.

| Behavior | Source and evidence strength | Implemented requirement / boundary policy |
| --- | --- | --- |
| Clock selection | Definitions identify FAST_SWEEP as bit 1, bypassing the sweep clock divisor. Experiment enables it with time zero and compares frequency increments after CPU instructions. | Shared 3.072 MHz clock; ordinary prescaler is 8192 clocks (375 Hz), fast mode supplies a sweep-counter tick every machine clock. Audio output remains 24 kHz. |
| Programmable time | Definitions describe 8D as ticks between changes minus one. The fast experiment covers only time zero. | Documentation-derived inference: retain the five-bit programmable counter; update after `(8D & 31) + 1` selected ticks in either mode. Thus decimal time 0/1/31 means fast periods 1/2/32 clocks, ordinary periods 8192/16384/262144 clocks. Fast mode does not discard nonzero time. |
| Signed frequency step | Definitions specify signed 8C addition; experiment tests +1, +16 and −1. | Add signed eight-bit 8C to channel 3 frequency (84/85), wrapping at 11 bits. Zero step still advances the timer. |
| Enables | Experiment documentation and source require both channel 3 and sweep enable bits. | Only 90 bits 2 AND 6 enable counter advancement and frequency changes. Provisional policy: both counters pause when either prerequisite is absent, preserving existing ordinary-mode behavior. |
| Time write | Experiment documentation says writes reset the timer; source repeatedly writes time zero each scanline and asserts no sweep. | Every byte write to 8D, even an unchanged value, clears both prescaler and programmable counter in either mode. |
| Enter/leave fast mode | Experiment enables bit 1 and later clears 95. It does not measure partially elapsed prescaler/counter transitions. | Provisional policy: 95 writes preserve both counters. Fast mode freezes/bypasses the ordinary prescaler; the programmable counter advances on fast ticks. Clearing resumes the retained ordinary prescaler and programmable count. Repeated 95 writes do not reset either counter. |
| Readback and restart | The experiment clears 95; startup tables list zero but do not establish all model/read/reset behavior. | Emulator policy on all models: initial/restart/reset state is 00; reads return the last accepted 00 or 02. This is not a measured silicon readback guarantee. |
| Other bits | Definitions name HOLD_CH bit 0, without establishing its behavior here. | Reject any bits outside 02 with an actionable diagnostic, preserving control, frequency, timer phase and other device state. HOLD_CH and other debug functions remain unsupported. |

## Correction and ownership

Old AI-authored assumption: every port-95 access, including a zero write, must
fail. Corrected requirement: clearing and FAST_SWEEP are supported operations.
The pinned experiment explicitly sets FAST_SWEEP, measures frequency changes,
then clears the port, contradicting blanket rejection. The obsolete assertion is
replaced while unrelated unsupported-port assertions remain.

The Issue's ares observation corroborates separating counters; it supplies no
independent transition-phase evidence. No emulator implementation code was used.
The new fixture and vectors are independently authored CC0 work. The native and
production Wasm tests validate these emulator semantics; no commercial-title
benefit or complete port-95/hardware compatibility is established.
