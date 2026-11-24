# wonderswan-rust

A from-scratch Rust WonderSwan emulator with a browser WebAssembly adapter.
Choose **WonderSwan (mono)**, **WonderSwan Color**, or **SwanCrystal** (default).
No existing emulator core is copied, ported, vendored, or used as a dependency.

The player starts empty. Choose **Load ROM** to open a local `.ws` or `.wsc`
file you are entitled to use. No games are supplied, fetched or uploaded. The Rust
core executes the cartridge and produces the 224×144 RGBA framebuffer; the browser
provides input, timing, audio and presentation.
The core now also implements cartridge SRAM/EEPROM, common and Bandai 2003
banking, a broader 80186 instruction set, interrupts/timers, color backgrounds
and sprites, four-channel audio, and Color/Crystal General DMA, Sound DMA and
Hyper Voice PCM with distinct headphone/speaker mixing. Internal console EEPROM
supports ordinary word transfers, settings protection and separate per-model
retention.
Exact raw 3 MiB and 6 MiB cartridges are supported with top-aligned ROM mapping,
unchanged raw-content save identities and no external padding.
LCD total-line programming updates core frame progression and browser cadence; see
[register/model/edge policies](docs/lcd-timing.md).
Sound-test port 95 supports clearing and fast sweep on all three models; see
[evidence and boundary policies](docs/fast-sweep.md). HOLD_CH and other debug
controls remain unsupported.
Compatibility and timing remain limited; see the exact
[support matrix and limitations](docs/hardware-subset.md).
Unsupported instructions and hardware operations stop with a visible diagnostic.
[Original CC0 hardware probes](fixtures/README.md) verify these paths for development and
tests; they are not published with the player. No broad commercial-ROM
compatibility is claimed.

ROM bus width and additional wait clocks are software-controlled through A0 on
all three models. The built-in IPL applies cartridge header flags on
load, restart and compatible model changes. Subsequent OUT writes change CPU/DMA behavior without
changing cartridge metadata. See [runtime ROM controls and limitations](docs/hardware-subset.md#runtime-rom-controls-a0)
and the [original A0 CPU probe](fixtures/README.md#a0-runtime-control-probe-issue-23).

## UI/UX

These principles apply to the browser player's controls, Help and
diagnostics, as well as documented commands, messages and examples.

1. **Keep it simple.** Show only necessary information and actions; avoid
   unnecessary UI elements.
2. **Show, don't tell.** Prefer layout, spacing and visual hierarchy over
   explanatory labels or prose to communicate structure.
3. **Intuitive and predictable.** Keep appearance and behavior consistent with
   user expectations; don't interrupt the user's work.

Keep necessary control labels, accessible names, error messages and information
needed to use the interface. Simplification must preserve meaningful consent,
destructive-action safeguards and necessary warnings.

## Play in the browser

Public URL: **https://takahirox.github.io/wonderswan-rust/**. The new version becomes
public after the existing `main` deployment workflow succeeds; local pre-merge
validation does not establish that deployment has happened.

1. In **Cartridge**, click **Load ROM** and select a local `.ws` or `.wsc` file.
   No cartridge is preloaded or remembered across page loads. Cancelling keeps the current state;
   invalid files or failed reads report a recoverable error and preserve the current
   cartridge. There is no backend or ROM catalog.
2. **Arrow keys or W A S D** select the X keypad; **Enter** is Start, and **Z/X**
   map to A/B. **I / L / K / J** select Y1 / Y2 / Y3 / Y4 independently.
   Click the screen to focus gameplay controls if needed.
3. The toolbar below the screen groups **Pause/Resume**, **Restart**, **Mute**,
   **Volume**, **Display**, and **Fullscreen**. Use **Enable audio** when the browser
   asks for another gesture. Changing **Console** in the cartridge slot restarts
   the loaded cartridge and clears keys. Hiding the tab pauses gameplay. Choose
   **Resume** when you return.
4. **Display** switches live, including while paused. **Clean** keeps exact
   nearest-neighbor pixels; **LCD Light** adds
   subtle native pixel cells, softness and weak persistence; **LCD Authentic**
   strengthens the matrix and response with modest reflective mono, softer Color,
   and clearer/lower-ghosting SwanCrystal presets. These are aesthetic approximations,
   not measured LCD calibration. **Retro CRT** (the new-visitor default with WebGL2)
   adds a restrained decorative mask and scanlines; a WonderSwan had an LCD, not a
   CRT. Effects change presentation only.
   The choice is saved under `wonderswan:display:v1`, separately from saves, console
   EEPROM, RTC and audio preferences. Unavailable/failed WebGL2 and context loss
   report a nonblocking notice and use the playable 2D Clean display; an explicit
   choice is retained and restored when the context recovers. History updates
   only on emulated frames and is seeded across cartridge/restart/model/mode and
   resume transitions. No additional animation loop runs while idle or paused.
5. **Help** in the header contains the keyboard reference, cartridge clock
   setting, and compatibility, sound, and storage guidance. Opening Help pauses
   gameplay and clears held inputs. Close it or press Escape to return.
   A previously running game resumes unless the tab was hidden.
6. **Debug Mode** in Help is off on every page load. Enable it to show the
   actual model, ROM name/byte count, execution state, core frame count, and last
   error. Runtime details collapse; Close or unchecking Debug Mode hides the
   panel. Diagnostics refresh at most twice per second and provide no CPU
   registers or accuracy claims.

On narrow screens, the toolbar and cartridge slot follow the game screen.
Small and touch screens show both X and Y keypads plus A, B, and Start. On short
landscape touch screens, the keypads flank the screen when there is room.

Vertical cartridges use their header orientation. The player rotates the native
image counterclockwise into a 144:224 portrait footprint. The core framebuffer
and Clean canvas backing stay 224×144; shader history also stays native-sized,
with output capped at 4× resolution. Pixelated scaling keeps square pixels; controls
keep their raw hardware identities in either orientation. X1/X2/X3/X4 remain
Up/Right/Down/Left (or W/D/S/A); Y1/Y2/Y3/Y4 remain I/L/K/J. Keyboard and multiple
touch presses combine, including two sources holding the same button. Releasing
one source retains the others. Focus changes, blur and player lifecycle changes
clear held inputs. Orientation is header metadata, without runtime rotation signaling.

See the [player layout screenshots and production-preview results](docs/issue-136-validation.md).

Cartridge saves are automatic and **browser-local**, keyed by SHA-256 of the
ROM contents (renaming a file keeps its save; different contents get separate
slots). Restart, pause and compatible model changes preserve saves. Reload or
reopen the page and select the same ROM again to restore them. Storage errors
show a notice; unreadable saved data is preserved rather than overwritten. No
ROM or save is uploaded. Browser storage clearing/private-session expiry removes
these saves.

Console EEPROM data is also automatic and **browser-local**, in separate versioned
records for WonderSwan (128 bytes), WonderSwan Color (2048 bytes) and SwanCrystal
(2048 bytes). Different cartridges share that model's console data; switching
away and back, reload and reopening preserve it. Storage failures report a
separate console notice and unreadable records are preserved. Initial settings
are deterministic synthetic data, not a factory image. Direct ERASE supports
word zero under a conservative emulator policy; nonzero ERASE, bulk commands
and exact programming timing remain hardware gaps. There is no settings editor.
See the [console behavior and core/Wasm API](docs/hardware-subset.md#internal-eeprom-transfers-retention-and-api).

Sound defaults to **on at 70%** after a **Load ROM** interaction;
there is no audible autoplay on page load. Browser autoplay restrictions may keep
audio suspended: use **Enable audio** to retry directly. The status describes the
actual browser audio state; an audio-ready cartridge can still be silent.
**Mute / Unmute** preserves the slider
level, and the labeled **Volume** slider adjusts output immediately from 0–100%
(100% is unity gain). At 0% output is silent; raising the slider restores its level
unless explicitly muted. Mute and volume are remembered locally, separately from
ROM saves; clearing storage resets them. Pause/background/restart clears scheduled
sound, and unavailable audio does not prevent gameplay.

Each console automatically boots a local cartridge through its built-in
[original CC0 boot ROM](bios/README.md). Play requires only the cartridge file.
The site bundles the player and its original firmware. The CC0 test cartridges
remain under [fixtures](fixtures/README.md) for development.

## Build and run locally

Prerequisites:

- Stable Rust/Cargo via [rustup](https://rustup.rs/).
- `rustup target add wasm32-unknown-unknown`.
- `cargo install wasm-pack --version 0.14.0 --locked`.
- Node.js 22.12+ or 24+ and npm; a modern browser supporting Wasm and canvas.
- To rebuild or verify the original IPL and development fixtures, install
  Python 3 and [NASM](https://www.nasm.us/). The generated binaries are committed
  so normal builds can use them directly.

If Homebrew/system Rust takes precedence, select rustup before building Wasm:

```sh
export PATH="$(dirname "$(rustup which rustc)"):$PATH"
```

From the repository root:

```sh
python3 bios/build.py --check       # built-in CC0 boot ROMs
python3 fixtures/build.py --check  # rebuildable original CC0 hardware probes
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cd web
npm ci
npm run build:wasm
npm run dev
```

The dev server normally opens `http://127.0.0.1:5173/`. Stop with Ctrl+C. Rebuild
Wasm and reload after Rust changes; Vite handles TypeScript/CSS updates. Generated
`web/pkg`, `web/dist`, dependency/build directories and Python caches are ignored.
Both dependency lockfiles are committed.

For native core throughput and comparisons with pinned Beetle/Oswan builds, see
[the historical performance results](docs/performance-parity.md) and
[the original investigation](docs/performance.md). The current benchmark scripts
use only independent development fixtures, outside the deployable site. Run a headless
measurement
with video and audio generation enabled using:

```sh
cargo run -p ws-core --example throughput --release --locked -- fixtures/generated/sram128-2003.ws 120 3000 crystal
```

For browser playback under low frame delivery and main-thread stalls, see the
[audio stability fix and regression commands](docs/audio-stability-fix.md) and
[the baseline investigation](docs/audio-stability.md).
For reference frontend/device playback strategies and remaining synchronization
work, see the [audio playback comparison](docs/audio-playback-comparison.md).
The [clock-drift correction](docs/audio-clock-drift.md) documents current output
feedback, long-run regressions and rate/pitch tradeoffs.
The [browser playback comparison](docs/browser-playback-parity.md) records
matched core throughput, real Web Audio scheduling and the CRT and stall fixes.

To rebuild development probes after editing their assembly, run
`python3 fixtures/build.py` and `python3 fixtures/build.py --check` from the root.
Native and browser integration tests execute those fixtures through the CPU,
keypad, video, sound and storage paths on the supported models.
Unit tests cover CPU arithmetic/flags,
addressing, branches, stack, strings, I/O, ROM errors, bank mapping, video and input.
There are no emulator dependencies. The only added frontend test tool is
Playwright for the required end-to-end production-preview check.

## Production preview and browser acceptance

From `web`:

```sh
npm run build:wasm
npm run build
npm run preview
```

The build runs TypeScript checks and produces `web/dist`, containing the player
assets and Wasm, with no playable ROM binaries or fixture copies. Preview normally
opens **http://127.0.0.1:4173/wonderswan-rust/**.
Serve over HTTP, not `file://`. Production/preview use the existing
`/wonderswan-rust/` base; the dev server uses `/`.

In a second terminal, from `web`:

```sh
npx playwright install chromium
npm run test:browser
npm run test:compat
```

Use `WS_PREVIEW_URL=http://127.0.0.1:4287/wonderswan-rust/ npm run test:browser`
if preview is on port 4287. The smoke script opens a fresh Chromium context,
checks empty startup, cancellation and local file-picker loading of an independent
CC0 keypad fixture, then verifies CPU input on all three models, pause/restart and
input clearing. It checks no ROM network requests or uploads, invalid-file and
failed-read preservation, same-file reload, asynchronous read/replacement/model/
restart/modal/visibility races, and execution errors. Dialog focus/Escape,
conditional resume, Debug Mode, fullscreen and API failures, and initialization
retry remain covered. Layout checks cover desktop, 375×812 portrait and 812×375
landscape touch viewports with both cartridge orientations. Both keypads and
A/B/Start are observed through CPU-produced bit tiles, including combined presses.
Visibility, read failures and API failures are synthetic, not physical-device tests.
Set `WS_SMOKE_SCREENSHOT=/tmp/wonderswan-playing.png` for a running-player screenshot.
Set `WS_SMOKE_ARTIFACTS=/tmp/wonderswan-player` for desktop/mobile screenshots
including empty, playing, Help, debug, and fullscreen states.
Exact-pixel gameplay and panel-output checks explicitly choose Clean. The LCD
output checks also exercise the production WebGL2 shaders with synthetic flat,
edge and alternating RGBA uploads: native-cell detail, preset/model differences,
softness, per-emulated-frame history, stable paused redraws and texture allocation,
small-scale grid suppression, preference storage, and unavailable/compile/link/lost
context fallback and restoration. Shader tests require working WebGL2 in Chromium
and fail rather than silently skip GPU assertions. The keypad geometry checks
compare both Clean and LCD output in landscape/portrait at desktop/mobile widths
and fullscreen. These headless Chromium checks do not establish Safari/Firefox or
physical mobile GPU behavior.
The compatibility suite loads each original SRAM/EEPROM probe through the file
picker, checks background/sprite colors, SHA-256 save isolation, reload/reopen,
reset/model/pause preservation, and real Web Audio sample buffers and bounded
sources. It checks default-on activation before the local file chooser/read,
measures live output gain at 0%, 35% and 100%, and covers mute/volume persistence,
keyboard/touch controls, denied/malformed preference storage, corrupt saves and
unavailable audio. Suspended, rejected and pending resumes, creation/playback
failures, and visibility/pagehide are simulated; sample buffers and analyser
measurements use real Web Audio nodes. It also loads the original SDMA/Hyper Voice
probes on Color/Crystal, checks exact PCM buffers and gain/lifecycle controls,
and verifies mono silence. The A0 CPU probe checks live ROM controls on all
three models, matching-width ROM data and Color/Crystal DMA paths after restart.
The port-95 CPU probe checks clearing, fast frequency progression, restored
ordinary sweep and control reset on every model, with CPU-written SRAM markers,
measurable real Wasm PCM and empty-after-drain assertions.
The startup CPU probes check the RAM-resident IPL lockout, SP=2000 and
physical-model DS=FF00/FE00 through normal footer/banking paths. They cover both
header types and mapper interfaces, load, restart, cartridge replacement and
compatible model changes. The browser tests check CPU-written SRAM captures
through player persistence and the production Wasm adapter. See the
[startup policy](docs/hardware-subset.md#built-in-ipl-startup-and-reset).
Internal EEPROM CPU probes additionally check settings,
status, write locking/protection, three-model console isolation, cross-cartridge
retention, lifecycle flushes, reload/reopen and corrupt/denied/full console storage
through actual Wasm execution. See [audio control validation](docs/issue-17-validation.md)
and [SDMA/Hyper Voice validation](docs/issue-19-validation.md).
It also runs the original X/Y keypad cartridges on all models in both orientations,
checking each button, mixed B5 rows, simultaneous keyboard/touch sources, release,
lifecycle clearing, and composited asymmetric portrait pixels at desktop, 375px
and fullscreen sizes. See [Issue #29 validation](docs/issue-29-validation.md).
See [hardware milestone results](docs/issue-11-validation.md),
[player UI pre-merge results](docs/issue-9-validation.md) and the
[original cartridge results](docs/issue-7-validation.md).

Equipped raw-01 cartridges also expose a retained S-3511A calendar/status through
CA/CB. Help provides an absent-clock override independent of mapper banking.
The clock progresses while paused/offline and has its own versioned local record
keyed by raw ROM SHA-256. See [RTC policy](docs/rtc-calendar.md) and
[Issue #63 validation](docs/issue-63-validation.md). Retail-board detection and
alarm/periodic IRQ delivery remain unsupported; commercial impact is unmeasured.

## GitHub Pages deployment

The existing [Pages workflow](.github/workflows/deploy-pages.yml) builds and
publishes `web/dist` on pushes to `main`, using locked Rust/npm dependencies,
wasm-pack and Vite. Manual retries use
[Actions → Deploy browser prototype to Pages](https://github.com/takahirox/wonderswan-rust/actions/workflows/deploy-pages.yml)
with branch `main`. Other branches do not deploy. Overlapping runs cancel older
runs. No personal access token or custom repository secret is needed.

One-time administrator prerequisite: **Settings → Pages → Build and deployment →
Source: GitHub Actions**. This setting was not changed here. If deployment fails,
check that setting, Actions permissions and `github-pages` environment restrictions.
Inspect Actions logs and [Settings → Pages](https://github.com/takahirox/wonderswan-rust/settings/pages)
or **Deployments → github-pages** for publication status and deployed commit.

Required **post-merge checks remain pending**:

1. Verify the merge-triggered workflow publishes successfully; record its Actions
   run URL, deployed commit and final deployment result.
2. Open the public URL unauthenticated. Confirm the player starts empty with a
   prominent Load ROM action, local-file instructions, and no supplied game or
   download links. Select an independent local CC0 fixture through the file picker;
   verify it runs without ROM network requests/uploads or production-path errors.
3. Load a CC0 compatibility probe locally on the public page; verify color,
   sound and saves surviving reload, plus Debug Mode/error recovery (Issue #11).
4. Start an original licensed sound-producing fixture on the public player with
   a normal user gesture. Verify default-on audio, mute and live volume changes;
   reload to check remembered preferences and record actual autoplay restrictions
   (Issue #17).
5. Confirm the public console selector, Help, Debug Mode, compatibility
   notice, and the absence of bundled ROMs in the deployed files. Report
   deployment-only problems as follow-up work.

## Layout and contributing

- `crates/ws-core`: dependency-free CPU, cartridge/save, bus, input, mono/color video and sound.
- `crates/ws-web`: thin wasm-bindgen adapter; machine state stays in Rust core.
- `web`: framework-free TypeScript/HTML/CSS, Vite and browser smoke script.
- `fixtures`: CC0 compatibility cartridge source, deterministic builder and generated files.

Follow the [development flow and English collaboration policy](docs/development-flow.md),
[test and CI performance principles](docs/development-flow.md#test-and-ci-performance-principles),
[review guidelines](docs/review-guidelines.md), [Issue template](.github/ISSUE_TEMPLATE/issue.md),
[PR template](.github/pull_request_template.md), and [AGENTS.md](AGENTS.md).
