# wonderswan-rust

A from-scratch Rust WonderSwan emulator with a browser WebAssembly adapter.
Choose **WonderSwan (mono)**, **WonderSwan Color**, or **SwanCrystal** (default).
No existing emulator core is copied, ported, vendored, or used as a dependency.

**Beacon Trail is playable:** an original, silent, freely redistributable cartridge
runs its game logic as CPU instructions, reads the emulated keypad, and draws
through tile RAM and video registers. The Rust core produces the 224×144 RGBA
framebuffer. The browser only provides input/timing and displays those pixels.
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
[Original CC0 hardware probes](fixtures/README.md) verify these paths separately
from the unchanged demo. No broad commercial-ROM compatibility is claimed.

ROM bus width and additional wait clocks are software-controlled through A0 on
all three models. Header flags seed the synthetic firmware-free handoff on
load/restart/model change; subsequent OUT writes change CPU/DMA behavior without
changing cartridge metadata. See [runtime ROM controls and limitations](docs/hardware-subset.md#runtime-rom-controls-a0)
and the [original A0 CPU probe](fixtures/README.md#a0-runtime-control-probe-issue-23).

## UI/UX

These principles apply to the browser player's controls, Settings / Help and
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

1. Click **Play demo**. Move the hollow square to collect the diamond beacon.
   Each collected beacon advances the visible score and places the next target.
   Find all three to see **ALL FOUND!**.
2. **Arrow keys or W A S D** move (the X keypad); **Enter** is Start/restart.
   Z/X map to A/B, although the demo does not use them. Held directions repeat.
   **I / L / K / J** independently select Y1 / Y2 / Y3 / Y4.
   Click the screen to focus gameplay controls if needed.
3. Use **Pause/Resume**, **Restart**, or **Fullscreen** below the display. Changing
   the visible console selector restarts the loaded cartridge and clears keys.
   Hiding the tab pauses; click Resume when returning. There is no emulation loop
   while idle or paused. Small/touch screens show both X and Y groups, A, B, and Start
   buttons; hold directions to repeat and release to stop.
4. **Settings / Help** contains controls, compatibility information, and the
   original demo's download/source/CC0 links. Opening it pauses gameplay and
   clears held inputs. Close or press Escape to return; a previously running
   game resumes unless the tab was hidden. A manually paused game stays paused.
   Keyboard gameplay is active only while the canvas has focus.
5. **Debug Mode** in Settings is off on every page load. Enable it to show the
   actual model, ROM name/byte count, execution state, core frame count, and last
   error. Runtime details collapse; Close or unchecking Debug Mode hides the
   panel. Diagnostics refresh at most twice per second and provide no CPU
   registers or accuracy claims.
6. **Download demo ROM** in Settings saves the same `beacon-trail.ws` used by
   Play demo. Choose **Load ROM** to run it manually. Selected files remain in
   browser memory and are never uploaded. Invalid files report a recoverable
   error without replacing the cartridge. There is no backend or ROM catalog.

Vertical cartridges use their header orientation: the player rotates the native
image counterclockwise into a 144:224 portrait footprint. The core framebuffer
and canvas backing stay 224×144. Pixelated scaling keeps square pixels; controls
keep their raw hardware identities in either orientation. X1/X2/X3/X4 remain
Up/Right/Down/Left (or W/D/S/A); Y1/Y2/Y3/Y4 remain I/L/K/J. Keyboard and multiple
touch presses combine, including two sources holding the same button. Releasing
one source retains the others. Focus changes, blur and player lifecycle changes
clear held inputs. Orientation is header metadata, without runtime rotation signaling.

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
are deterministic synthetic data, not a factory image. Direct erase/bulk commands
and exact programming timing remain hardware gaps; there is no settings editor.
See the [console behavior and core/Wasm API](docs/hardware-subset.md#internal-eeprom-transfers-retention-and-api).

Sound defaults to **on at 70%** after a **Play demo** or **Load ROM** interaction;
there is no audible autoplay on page load. Browser autoplay restrictions may keep
audio suspended: use **Enable audio** to retry directly. The status describes the
actual browser audio state; an audio-ready cartridge can still be silent.
**Beacon Trail is deliberately silent**. **Mute / Unmute** preserves the slider
level, and the labeled **Volume** slider adjusts output immediately from 0–100%
(100% is unity gain). At 0% output is silent; raising the slider restores its level
unless explicitly muted. Mute and volume are remembered locally, separately from
ROM saves; clearing storage resets them. Pause/background/restart clears scheduled
sound, and unavailable audio does not prevent gameplay.

The demo source, generated binary, hand-authored lettering, and graphics are
**CC0-1.0**, allowing play, modification and redistribution. See
[demo provenance/license](web/public/demo/NOTICE.txt), [full license](demo/LICENSE),
and [editable game source and build details](demo/README.md). No commercial ROMs,
copied assets, unlicensed game binaries, or proprietary BIOS/firmware are bundled.

## Build and run locally

Prerequisites:

- Stable Rust/Cargo via [rustup](https://rustup.rs/).
- `rustup target add wasm32-unknown-unknown`.
- `cargo install wasm-pack --version 0.14.0 --locked`.
- Node.js 22.12+ or 24+ and npm; a modern browser supporting Wasm and canvas.
- To rebuild/verify the demo only: Python 3 and [NASM](https://www.nasm.us/)
  (validated with Python 3.13 and NASM 3.02). The deployable ROM is committed;
  normal browser builds do not need an assembler.

If Homebrew/system Rust takes precedence, select rustup before building Wasm:

```sh
export PATH="$(dirname "$(rustup which rustc)"):$PATH"
```

From the repository root:

```sh
python3 demo/build.py --check
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

To rebuild the game after editing `demo/game.asm` or `demo/graphics.py`, from root:

```sh
python3 demo/build.py
python3 demo/build.py --check
cargo test --workspace --locked
```

The deterministic builder creates `web/public/demo/beacon-trail.ws`, a valid
128 KiB cartridge with footer/checksum. Integration tests load that exact asset,
execute CPU frames, inject keypad input, check changed graphics/score, complete
the game and restart on all three models. Unit tests cover CPU arithmetic/flags,
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

The build runs TypeScript checks and produces `web/dist`, including the bundled
ROM and notice. Preview normally opens **http://127.0.0.1:4173/wonderswan-rust/**.
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
loads Wasm and the demo, moves with actual keyboard events, checks score 1/2/3
and the win message from canvas pixels, restarts via Enter, plays on all three
models, tests UI pause/restart and cleared input on model switches, plays a
local file to completion, rejects an empty file, reloads the same file, checks
no upload requests, and verifies production asset paths and the license notice.
It also checks dialog pointer/keyboard entry, Escape, focus/input isolation,
conditional resume, Debug Mode default/on/off, native Chromium fullscreen and
simulated API failure, initialization retry, failed demo recovery, asynchronous
load/menu/visibility races, execution errors, and a 375×812 touch layout. Touch
input is sent through Chromium's touch event API; A/B and Z/X are verified with a
checksum-valid test-only variant of the original CC0 cartridge that reads the
action keypad row. The bundled ROM is unchanged. Visibility events and API/network
failures are synthetic; this does not claim a real device or physical tab test.
For the win screenshot, set `WS_SMOKE_SCREENSHOT=/tmp/beacon-trail-win.png`.
Set `WS_SMOKE_ARTIFACTS=/tmp/wonderswan-player` for desktop/mobile screenshots
including empty, playing, settings, debug, and fullscreen states.
The compatibility suite loads each original SRAM/EEPROM probe through the file
picker, checks background/sprite colors, SHA-256 save isolation, reload/reopen,
reset/model/pause preservation, and real Web Audio sample buffers and bounded
sources. It checks default-on activation before demo fetch/file chooser/file read,
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
The incoming-register CPU probes check SP=2000 and physical-model DS=FF00/FE00
through normal footer/banking paths, with both header types and mapper interfaces,
load, restart, cartridge replacement and compatible model changes. CPU-written
SRAM captures are checked through player persistence and the production Wasm
adapter; see the [bounded startup policy](docs/hardware-subset.md#firmware-free-startup-and-reset).
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
CA/CB. Settings provides an absent-clock override independent of mapper banking.
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
2. Open the public URL unauthenticated. Play the bundled demo to completion,
   restart, and load the same downloaded ROM via the file picker. Confirm ROM,
   JS and Wasm load without production-path errors.
3. Load a CC0 compatibility probe locally on the public page; verify color,
   sound and saves surviving reload, plus Debug Mode/error recovery (Issue #11).
4. Start an original licensed sound-producing fixture on the public player with
   a normal user gesture. Verify default-on audio, mute and live volume changes;
   reload to check remembered preferences and record actual autoplay restrictions
   (Issue #17). Beacon Trail cannot verify audible output because it is silent.
5. Confirm the public model selector, Settings/Help, Debug Mode, compatibility
   notice, and demo download/source/license links. Report deployment-only problems
   as follow-up work.

## Layout and contributing

- `crates/ws-core`: dependency-free CPU, cartridge/save, bus, input, mono/color video and sound.
- `crates/ws-web`: thin wasm-bindgen adapter; machine state stays in Rust core.
- `web`: framework-free TypeScript/HTML/CSS, Vite and browser smoke script.
- `demo`: original cartridge assembly, graphics, deterministic builder and license.
- `fixtures`: CC0 compatibility cartridge source, deterministic builder and generated files.

Follow the [development flow and English collaboration policy](docs/development-flow.md),
[review guidelines](docs/review-guidelines.md), [Issue template](.github/ISSUE_TEMPLATE/issue.md),
[PR template](.github/pull_request_template.md), and [AGENTS.md](AGENTS.md).
