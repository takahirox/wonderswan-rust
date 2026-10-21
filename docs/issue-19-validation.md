# Sound DMA / Hyper Voice validation — Issue #19

Pre-merge checks performed locally on 2026-10-21 for
[Issue #19](https://github.com/takahirox/wonderswan-rust/issues/19).
The final checks below passed. No push, pull request, merge, issue comment or
production deployment was performed.

| Check | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | All nine CC0 fixtures reproduce; six previous fixtures unchanged |
| `python3 demo/build.py --check` | Existing silent CC0 demo bytes unchanged |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --locked` | 69 core unit tests and 7 integration tests passed; web/doc tests passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `npm ci` in `web` | Passed; lockfile unchanged |
| `npm run build:wasm` in `web` | Release Wasm compilation and wasm-opt passed using installed rustup stable on PATH |
| `npm run build` in `web` | TypeScript checks and Vite production build passed |
| `npm run test:browser` in `web` | Existing production Chromium gameplay/UI/mobile suite passed on all three models |
| `npm run test:compat` in `web` | Existing save/audio/GDMA checks plus all three new sound probes passed |
| `git diff --check` | Passed |

Browser suites used the production preview at
`http://127.0.0.1:4289/wonderswan-rust/`, selected with `WS_PREVIEW_URL`.
The system/Homebrew compiler initially lacked the Wasm target. The successful
build used the installed rustup stable toolchain's `bin` directory first on
PATH, following the README. Native checks used the existing system Cargo
toolchain. An initial Clippy single-element-loop finding was fixed. A new
browser assertion initially read `AudioParam.value` immediately after suspension;
Chromium can retain the preceding rendered value then. The corrected check
verifies the actual zero-gain command and that all playback sources stop, followed
by a full successful compatibility run. Native fixture assertions allow short
stereo transitions because separate CPU OUT instructions may straddle an output
sample boundary; steady values are still checked exactly.

## Native evidence

`crates/ws-core/src/sound_tests.rs` adds independent hardware vectors for:

- All four SDMA rates, clock phase 117 mod 128 and 6 + N reservations even on
  rate-divided/hold ticks; no CPU memory-transfer double charges.
- Increment/decrement, one-shot completion, repeat, hold, zero-length enable
  failure/active edits, disable/re-enable, individual live/shadow-byte edits across
  carry/borrow, 20-bit lengths and source wrap.
- Internal RAM, byte-wide/slow ROM, fast/slow SRAM, bank/region crossings, common
  and extended mapper windows, including flash-mapped reads.
- CPU stalls before instruction fetch, interrupt entry and GDMA; bounded progress
  across frames without CPU starvation; chunked versus single-clock equality;
  scanline/timer/interrupt/audio progress and pending EEPROM programming.
- Every Hyper Voice rate divisor, enable/frozen output behavior, reset/alternating
  selector, all DMA layouts and CPU input ignoring mono layout.
- Signed, unsigned, unsigned-negated and ignored extension, all shifts, exact
  output floats, direct 16-bit writes, headphone wrap, speaker isolation,
  speaker shift/volume and 10/11-bit legacy mixer readbacks.
- Model-specific ports/masks, unused counter bytes, reserved-bit masking,
  diagnostics for unknown 6B low bits and unsupported direct input port 68.

Existing four-wave/voice/sweep/noise tests, GDMA/save probes and complete playable
demo tests still pass, including mono and both color models.

`crates/ws-core/tests/sound.rs` executes all three original cartridge files through
V30MZ instructions on all three models. It checks their RAM assertions/success
pixels, measured non-silence and independently calculated exact steady PCM,
stereo routing, sample count/rate/bounds, mono silence, drain-once behavior and
deterministic restart without retained samples.

## Browser/Wasm evidence

The compatibility suite loads the same original cartridges through the real
player's file chooser. Success requires the cartridge's green framebuffer and
no execution error. It observes actual `AudioBufferSourceNode` buffers before
the existing high-pass filter, with exact expected sample values:

| Probe | Left/right input to Web Audio |
| --- | --- |
| SDMA → Channel 2 | 0.03125 / 0.21875, same on both sides |
| SDMA → Hyper Voice | −0.125 / +0.125, same on both sides |
| CPU → Hyper Voice (8/16-bit) | ±0.125 / ±0.25, opposite stereo except brief CPU-write transitions |

Each path runs on Color and Crystal, verifies the 24 kHz/bounded buffer contract,
measures nonzero analyser input and output/input gain ratio at 35%, and exercises
mute/unmute, pause/resume, restart and synthetic visibility pause/resume with
source cleanup, no buffers while stopped, a reused single context and bounded
scheduled sources. Mono runs each same file and emits silence without unsupported
I/O. The unchanged suite also measures 0/35/100% gain, validates the intentionally
silent demo, gesture activation, mute/volume persistence, saves, error recovery
and browser-local storage isolation. No new synthesis or frontend audio graph
was introduced. These are headless Chromium measurements, not audible or
physical-speaker observations. Visibility and failure policies are injected;
actual audio nodes, cartridge CPU execution and generated sample buffers are real.

## Provenance, limits and follow-up

All added source and cartridge data are original CC0 work; PCM is generated from
constant bytes by the included assembly/Python/NASM pipeline. No commercial ROM,
boot firmware, downloaded sample, borrowed emulator code or ROM-specific behavior
was used. The generated 131072-byte cartridges have SHA-256:

| File | SHA-256 |
| --- | --- |
| `sound-channel2.ws` | `e58f47e7035eca236fd29a9b1e87f1d456ebf8e9d2b1e19a54bfeda69136b5bc` |
| `sound-hyper.ws` | `af386887772718eb81e7c4df3dfe8337a821b87b8ca647ab50fb53b55592772d` |
| `sound-direct.ws` | `6b4c18e947f53da6caf79397af9b67fc452b77483f16a4d7c9e2e3af72b23668` |

Hardware references were WSdev's [DMA](https://ws.nesdev.org/wiki/DMA),
[Hyper Voice](https://ws.nesdev.org/wiki/Hyper_Voice),
[Sound](https://ws.nesdev.org/wiki/Sound) and
[memory map](https://ws.nesdev.org/wiki/Memory_map), not emulator source.
The wiki's raw-text endpoint was used when the rendered endpoint returned 403.
See [hardware semantics and explicit approximations](hardware-subset.md#sound-dma-semantics-and-shared-timing):
instruction/word-granular CPU stalls, atomic transfers at the documented event
phase, deterministic unspecified divider/readback details, and approximate speaker
main-volume/analogue response. Test port 95, direct writes to internal latch 68
and unknown 6B controls remain explicit gaps. Physical-console comparisons,
other browsers and listening on real speakers were optional and not performed.
There is no claim of perfect analogue or complete commercial-game compatibility.

## Required post-merge verification — pending

1. After merge, record the existing `main` GitHub Pages workflow run URL and
   deployed commit, and confirm build/deployment succeeds.
2. Open the unauthenticated public page, load the original CC0 sound probes,
   and verify activation, PCM, mute/volume/pause/restart and model selection with
   no production-only errors. Distinguish audible observations from measured
   browser instrumentation.

Local production-preview checks establish neither post-merge result. No
post-merge action was attempted in this implementation node.
