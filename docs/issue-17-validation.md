# Issue #17 validation

Source: [Issue #17](https://github.com/takahirox/wonderswan-rust/issues/17).
Implemented in the assigned worktree on 2026-10-21. Changes are confined to
browser output/controls, existing browser tests and documentation. Rust emulation,
ROM/save formats, demo/fixture binaries, dependencies and Pages configuration are
unchanged.

## Mandatory pre-merge checks

| Check | Actual result |
| --- | --- |
| `cargo test --workspace --locked` | Pass: 58 core unit tests and 6 integration tests; web/doc tests also pass. |
| `npm ci` in `web` | Pass; locked dependencies, zero reported vulnerabilities. |
| `npm run build:wasm` in `web`, rustup first on PATH | Pass; release wasm-pack/wasm-opt build. |
| `npm run build` in `web` | Pass; TypeScript checks and Vite production build. |
| `npm run test:browser` in `web` | Pass against production preview; three-model gameplay, desktop/mobile controls, native keyboard/touch, menus/fullscreen/recovery and 375px layout. |
| `npm run test:compat` in `web` | Pass against production preview; real Web Audio gain, activation, mute/persistence/lifecycle and injected failure recovery, plus existing save/video/GDMA checks. |
| `python3 demo/build.py --check` | Pass; original silent demo reproduces unchanged. |
| `python3 fixtures/build.py --check` | Pass; all original CC0 fixtures reproduce unchanged. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `node --check web/scripts/compat.mjs` and `node --check web/scripts/smoke.mjs` | Pass. |
| `git diff --check`, changed documentation relative links | Pass. |

Both browser suites use headless Chromium through the existing Playwright tools,
with `WS_PREVIEW_URL=http://127.0.0.1:4287/wonderswan-rust/`. Screenshots were
captured with `WS_SMOKE_ARTIFACTS` and the mobile controls were visually inspected.
The first Wasm attempt selected Homebrew Rust without a Wasm target; the README's
existing rustup PATH instruction resolved that failure. An early mute assertion
incorrectly expected `AudioParam.value` to change after the context had suspended:
that getter may retain the last rendered value. The corrected check observes the
scheduled zero-gain operation and immediate removal of active sources. Actual
rendered gain is independently measured while running. A build invocation from
the repository root failed because npm commands belong in `web`; the build in
`web` passes.

## Acceptance evidence

- **Preference and activation:** a fresh page shows Mute and 70%, with no
  AudioContext, no claim of playing sound and no redundant enable button.
  Instrumentation verifies a trusted click with active browser user activation
  calls resume before the demo fetch, file chooser and file read. A gated demo
  fetch confirms the attempt does not wait for loading. Local selection of an
  original sound-producing CC0 compatibility probe starts nonzero source buffers
  without clicking a separate enable button. The original Beacon Trail produces
  only zero buffers and is described as deliberately silent.
- **Live gain:** real analysers before and after the output gain measure ratios
  of 0, 0.35 and 1 at slider values 0%, 35% and 100%. The same check covers 70%
  and restored 42%. Nonzero samples before the gain and expected cartridge pixels
  remain present; gameplay keeps running. Unity gain is the maximum; volume
  changes ramp over 10ms, while mute schedules zero immediately and stops queued
  sources. The native labeled slider supports arrow keys; mobile taps, 44px
  control heights and absence of horizontal overflow at 375px pass.
- **Mute and persistence:** mute preserves the slider level and is retained by
  Play demo, local ROM loading, pause/resume, Restart and model change. Slider
  changes while explicitly muted leave output muted. On and muted preferences
  and volume survive reload/reopen; muted game starts create no context. Unmute
  works before any ROM is loaded, while paused and during gameplay. Repeated
  toggles/resets reuse one context and never exceed eight queued sources.
  Audio preferences use `wonderswan:audio:v1`, separate from ROM save slots.
  Malformed JSON, wrong types, out-of-range/nonfinite values and denied storage
  reads/writes fall back safely; existing save identity/persistence checks pass.
- **Lifecycle and recovery:** pause, synthetic visibility hide and pagehide stop
  active sources; visibility return leaves the game paused. An injected late
  resume completion stays suspended after pause. External context suspension
  clears the queue and displays a direct retry. Simulated resolved-but-suspended,
  pending and rejected resume calls, AudioContext creation failure, missing Web
  Audio and playback-buffer failure leave CPU/video playable. Explicit retry
  recovers when the injected failure is removed, preserving sound preferences.
  Sound state has its own status so ROM/save notices cannot erase audio guidance
  or misreport an audio failure as an emulation error.

The normal Chromium runs use real AudioContexts, scheduled buffers, gains and
analysers. Policy restrictions/errors and visibility/pagehide events are injected
for deterministic coverage; these are not observations of physical browser tab
changes or every browser's autoplay policy. Unlock timing follows the
[MDN Web Audio guidance](https://developer.mozilla.org/en-US/docs/Web/API/Web_Audio_API/Best_practices#autoplay_policy):
attempt creation/resume in the initiating event and report actual context state.

## Optional validation

Physical mobile devices, Safari/Firefox and subjective speaker listening were not
tested. Headless measurements verify the Web Audio signal path, not speaker or
operating-system volume or universal browser autoplay behavior.

## Required post-merge verification — pending

1. **Pending:** confirm the existing `main` Pages workflow succeeds for the merged
   commit and record its Actions run URL.
2. **Pending:** open the public player, start an original licensed sound-producing
   fixture with a normal user gesture, verify default-on audio, Mute / Unmute and
   live volume changes, and reload to verify preferences. Record actual browser
   autoplay restrictions. Beacon Trail alone cannot verify audible playback.

No push, pull request, merge, external comment or deployment is performed here.
