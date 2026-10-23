# Issue #27 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, based on the Issue's
`e37a863ad6bf30febeaede058fc052149b1ebe2e` implementation baseline plus workflow
checkpoint commits. Source Issue #27 was fetched from GitHub and read before
implementation. No remote Issue, PR, branch or comment was modified.

## Implementation and reconciled scope

The former fixed-158 restriction is replaced with final-line programming.
The [source reconciliation and policies](lcd-timing.md) were recorded before
scheduler changes. Wonderful's definitions establish programmable totals and
independent STN back porch; even totals are recommended rather than masked.
The existing synthetic total 158 remains on all three models. STN synchronization
starts at 155; Crystal accepts/ignores writes and reads zero as an explicitly
uncertain compatibility policy. No total write changes the synchronization byte.
The inspected WSHWTest tables and pinned ares sources do not settle activation,
shortening, short/odd totals or precise Crystal behavior. The documented line-end
activation and literal short-total policies are tested without claiming silicon
accuracy; the short-total policy intentionally differs from ares's uncertain
minimum-height treatment. No upstream implementation code was copied.

The bus owns current raster line and completed frame count. Readback, event
sources and rendering use that state. Frames complete on actual wraps; CPU
instructions and DMA words retain their normal bounded overshoot. Machine clocks
continue to drive audio, SDMA, Hyper Voice and EEPROM. The thin Wasm adapter
reports current register cadence and browser scheduling rereads it for every
frame decision. Framebuffer dimensions and the default cadence are preserved.

## Tests present and executed

Seven new core unit tests and one native integration test cover:

- Independent byte registers, immediate and DX CPU word OUT/IN, all models,
  standard T-3 STN programming and Crystal zero-read policy.
- Stable totals 156/158/180: exactly 157/159/181 lines and
  40,192/40,704/46,336 clocks per frame; before/at/after line-one, line-144 and
  wrap boundaries, repeated frames, exact HBlank/VBlank countdowns, repeat
  reloads, interrupt acknowledgement/disable retention and sprite latching.
- Increases and decreases during active display and VBlank, including shortening
  below the current line and to zero; unchanged elapsed history; actual
  run_frame completion and monotonic counts; instruction and DMA word overshoot.
- All 256 byte totals, including short/odd/maximum values, bounded wraps,
  unreachable VBlank behavior and chunked versus single-clock progression,
  timer/register state and PCM. Rendering after nondefault frames and runtime
  changes uses the actual row; short frames retain unvisited rows.
- At totals 156/180, actual-clock audio sample counts, Channel 2/Hyper Voice SDMA
  event phase, counters, output latches and stalls across a frame boundary, and
  EEPROM busy/completion at the exact machine-clock deadline with retained data.
  A full steady frame produces 314/362 stereo sample pairs respectively rather
  than a fixed samples-per-frame count.
- The original source-included CC0 LCD cartridge on all models, checked RAM/SRAM
  execution markers, every polled raster increment/wrap, timer/interrupt flags,
  framebuffer pass pixels, runtime input changes, restart and model changes.

The production Chromium compatibility suite includes `web/scripts/lcd.mjs`.
It uses controlled RAF timestamps, the real player and a second independently
executing instance of the exact production Wasm adapter. It checks actual frame
counters across startup and input-driven timing changes, steady cadence,
catch-up bounds, pause/resume, visibility, restart/model changes and failed-load
recovery, plus cartridge SRAM markers and framebuffer output on all three models.
Existing gameplay, audio, mapping, DMA, save and console-persistence suites remain
in place and passed.

A mutation check temporarily changed only the ignored production JS bundle to
use the default cached duration. The LCD browser assertion failed at timestamp
84 ms: player counter 6 versus expected 5. The original production bundle was
restored before running the full browser suites. This demonstrates that the new
assertions detect the reported startup-cached pacing regression.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 17 original cartridges, including LCD. |
| `python3 demo/build.py --check` | Pass: unchanged 131072-byte demo, SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 90 core unit tests, 11 integration tests; Wasm crate and doc tests pass. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in `web` | Pass with the existing installed Rustup toolchain/Wasm target. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium gameplay, three models, desktop/mobile input, lifecycle, recovery and audio suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium compatibility suite including LCD, raw mapping, audio, DMA, saves and console persistence. |

Both full browser suites ran sequentially against this worktree's production
preview at `http://127.0.0.1:4199/wonderswan-rust/`, supplied through
`WS_PREVIEW_URL`. `npm ci` restored locked dependencies without lockfile changes.
The initial Wasm command selected Homebrew Rust, whose sysroot lacks the target;
using the already installed toolchain with
`PATH=/opt/homebrew/opt/rustup/bin:$PATH` resolved it. An early command also ran
npm from the repository root instead of `web`; it was rerun from `web`.
An early repeat-frame test forgot to restore IRQ enable between vectors; its
setup was corrected without changing production behavior or assertions.

The LCD image is 131072 bytes; SHA-256:
`16183c7708d56c4fb4048c2a087356aafb07bfc286309ef9d611d10eb8eae31f`.
Source, generated image, provenance and native/browser tests are committed
inside this repository; no proprietary ROM, firmware, physical console or human
playthrough was needed.

## Source-Issue review

The implementation coordinator compared every acceptance item with the complete
implementation, fixture source, test sources and observed results under the
repository review guidelines. Register access, coherent raster/events/rendering,
actual frame completion, unchanged device clocks, live Wasm/browser pacing,
original native/Chromium cartridge coverage and documentation are covered.
No blocking findings remain. The work preserves core/browser separation,
firmware-free startup, independent implementation ownership and browser-local
data. No PR, push, merge or comment was performed by this node.

Optional software comparisons with other running emulators were not performed.
Physical mid-frame latching, short/odd totals, Crystal readback, analog panel
response and commercial-title compatibility remain unverified as documented.
Required post-merge verification: none.
