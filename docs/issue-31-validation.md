# Issue #31 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, based on
`86aba6a037b153e14e3691548ea8f36bae23f4e9` plus workflow checkpoints
(implementation parent `e0ef377dba13257f1f28a7a14873a7693c6701f1`).
The coordinator fetched and read the open source Issue from GitHub before
implementation, along with repository development/review instructions and the
pinned primary definitions, experiment documentation and assembly source.
No remote Issue, PR, branch or comment was modified.

## Source reconciliation and implementation

The [bounded source-to-behavior table](fast-sweep.md) was written before code
changes. It distinguishes inspected experiment assertions, documentation-derived
inference for nonzero fast time, and provisional phase/readback/reset policies.
The obsolete AI-authored requirement rejecting every port-95 access, including
zero writes, is replaced because the pinned WSHWTest source explicitly enables
FAST_SWEEP, asserts frequency progression and later clears the port. Unrelated
unsupported-port checks remain. No further Issue acceptance changes were needed.
The correction already appears in the source Issue; its original assumption,
corrected requirement, primary reference and rationale are retained in that table.

All three models accept 00 and 02 at 95 and read back accepted state, initially
zero. Unsupported bits, including HOLD_CH, report the offending bits and supported
operations before changing sound control, frequency or timer phase. No arbitrary
debug values are accepted. The existing shared 3.072 MHz tick drives separate
8192-clock and programmable counters. FAST_SWEEP bypasses only the former;
time zero updates every clock, time one every two, time 31 every 32 (decimal).
Ordinary periods remain 8192 times those intervals. Signed 8C steps wrap the
channel-3 frequency at 11 bits. Both 90.2 and 90.6 are required; 8D writes reset
both counters. Readback/reset and retained phases across mode/enable changes
remain explicitly labeled emulator policies where unmeasured.

The implementation adds one counter to the existing audio state and a bounded
register interface. No frontend/core public API, sample rate, dependency, clock
source, ownership boundary or persistence mechanism changes.

## Independent vectors and CPU/Wasm probe

Seven new core unit tests cover all three models, positive/negative/zero steps,
both wraps and signed extremes, fast/ordinary time 0/1/31 boundaries, both enable
prerequisites, same-value and changed time resets, repeated control writes,
counter preservation on entering/leaving fast mode and all 254 unsupported
values in each supported control state. A zero-step interval followed by a step
write demonstrates that zero addition still advances the timer. Provisional
enable/transition policies are named in tests and documented.

Single-clock versus irregular chunk advancement crosses the default frame
boundary (40709 clocks), checks independently calculated frequencies 11 and
1604, identical ports/PCM/mixer/raster/frame state, nonzero unrelated-channel
audio, 636 interleaved samples and empty-after-drain. Switching modes and
continuing both machines additionally compares retained hidden phases.

`fixtures/sweep.asm` and its generated 128 KiB cartridge are original CC0-1.0
source/data. CPU OUT/IN self-checks successful clearing, fast frequency
progression, short ordinary stability after clearing and a subsequent ordinary
step. CPU instructions write RAM and SRAM markers, pass pixels and an original
1500 Hz channel-1 square wavetable. Control 95 is deliberately left at 2 with
sweep disabled; restarting must read initial zero. This exercises unrelated
audio even while FAST remains set. Fast snapshots freeze sweep before word IN;
the probe uses a broad measured-delta bound because CPU timing is approximate.
Exact clock behavior is asserted in independent unit vectors.

Native integration tests check all markers, pixels, actual 24 kHz equal-stereo
PCM values 0 and 0.2197265625, 38–40 edges in a 318-sample frame, empty drains,
restart and deterministic first-frame buffers on all models. The complete
production compatibility suite loads the same local image through the real
file chooser and player, reads CPU-produced SRAM markers from actual save
flushes, and measures the Web Audio source buffers. It also runs the same real
production Wasm adapter separately to assert repeatable first-frame PCM,
restart/control reset and empty-after-drain on every model. Neither path mocks
registers, replaces the player's machine, or synthesizes sound in JavaScript.

Probe SHA-256: `e92789a0b472c95d5a49e188ebae691636f14531cbb6b3ac1d3aea3d10920e7f`.
All 19 existing cartridges and the original playable demo remain byte-identical.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 20 original cartridges, including sweep. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 97 core unit tests, 13 integration tests, one adapter unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass, including final documentation and staged changes before commit. |
| `npm run build:wasm` in `web` | Pass using the installed Rustup toolchain/Wasm target via the README PATH instruction. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium suite, including all three-model gameplay and existing desktop/mobile, input, lifecycle, fullscreen, audio and recovery paths. |
| `npm run test:compat` in `web` | Pass: complete Chromium suite, including the new all-model port-95 CPU/PCM/reset/drain checks and all existing save/audio/DMA/LCD/keypad/orientation/raw-ROM/internal-EEPROM checks. |

The production preview is `http://127.0.0.1:4289/wonderswan-rust/`, supplied via
`WS_PREVIEW_URL`. The final complete browser suites ran sequentially after the
production build and both exited successfully. `npm ci` restored locked
dependencies without lockfile changes. Syntax checking of the new browser module
and all 44 relative Markdown targets in the five changed documents passed.

Early targeted native probe checks exposed two fixture/test mistakes: asserting
the top-left pixel before its first complete rendered frame, then using the
wrong monochrome pass-palette nibble. The test now observes pixels after a full
frame; the independently authored fixture palette was corrected and rebuilt.
No core requirement or assertion was weakened. Targeted checks then passed on
all models. The initial full workspace run passed 96 core tests; the final
zero-step phase vector increased that to 97 and the complete workspace checks
were repeated successfully. No other Rust/build failures occurred.

The first complete compatibility run reached the new probe after all preceding
save/audio/DMA tests passed, then timed out waiting for a deleted save record.
The driver had removed localStorage outside the player before restart; the
player correctly avoids rewriting identical bytes using its existing save cache.
That invalid expectation was removed. Fresh playback and PCM are required after
restart; CPU marker assertions and the independent real-Wasm reset/drain checks
remain. The corrected targeted production-browser probe passed on every model,
then both complete browser suites were rerun sequentially. No player persistence
code or hardware requirement was changed to accommodate the driver mistake.

## Coordinator source-Issue review and remaining limits

The implementation coordinator completed the source-Issue review against the
final diff, new and existing test sources, original fixture, source reconciliation
and actual check results under the repository review guidelines. Every mandatory
criterion is covered:

- The pre-implementation evidence table and correction record preserve the
  agreed hardware goal, distinguish experimental assertions from inferences and
  policies, and replace only the obsolete blanket rejection requirement.
- All models implement bounded 95 control/readback, non-mutating unsupported-bit
  diagnostics, genuine shared-clock frequency updates, programmable timing,
  signed wrapping, enable prerequisites, time reset and ordinary restoration.
- Independent literal boundary vectors and frame-crossing chunk comparisons
  cover signed/zero steps, wraps, resets, enables and provisional transitions.
- The original source-included CC0 CPU cartridge passes natively and through the
  production Wasm player on every model, with CPU markers, measurable PCM,
  restart/control reset and empty-after-drain checks.
- Complete workspace and both production Chromium suites pass, retaining
  three-model gameplay, legacy wave/voice/noise and speaker/headphone paths,
  GDMA, SDMA/Hyper Voice, shared timing, keypad/orientation, raw cartridge identity,
  SRAM/cartridge EEPROM/internal EEPROM persistence and firmware-free startup.
- Hardware limitations, fixture provenance, README and recorded validation are
  updated without claiming HOLD_CH, complete port-95 or commercial compatibility.

The native fixture and browser-driver findings described above are resolved.
No blocking findings remain. No unrelated refactor, emulator subsystem,
dependency or remote action was added. No mandatory checks were skipped.

Firmware-free startup, from-scratch ownership and browser-local ROM/save handling
are preserved. No third-party emulator implementation code, commercial ROM,
proprietary firmware or external waveform data was used. HOLD_CH and other
debug functions remain unsupported; unmeasured mode-transition phase and
readback/reset behavior are policies. The inspected hardware experiment was not
run on a physical console. No commercial-title benefit or complete port-95
compatibility is claimed.

Optional additional licensed experiments, other browser engines, physical-device
tests and human gameplay were not performed. Required post-merge verification:
none. This node does not push, open a PR, merge or comment.
