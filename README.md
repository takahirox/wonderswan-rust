# wonderswan-rust

A from-scratch Rust WonderSwan emulator with a browser WebAssembly adapter.
Choose **WonderSwan (mono)**, **WonderSwan Color**, or **SwanCrystal** (default).
No existing emulator core is copied, ported, vendored, or used as a dependency.

**Beacon Trail is playable:** an original, silent, freely redistributable cartridge
runs its game logic as CPU instructions, reads the emulated keypad, and draws
through tile RAM and video registers. The Rust core produces the 224×144 RGBA
framebuffer. The browser only provides input/timing and displays those pixels.
Compatibility remains limited: this is a monochrome hardware subset on all three
models, not a claim of full hardware accuracy or commercial game compatibility.
Sound, interrupts, color graphics, save hardware, and most instructions are not
implemented. Unsupported instructions and I/O stop execution with a visible error.
See the [implemented hardware and limitations](docs/hardware-subset.md).

## Play in the browser

Public URL: **https://takahirox.github.io/wonderswan-rust/**. The new version becomes
public after the existing `main` deployment workflow succeeds; local pre-merge
validation does not establish that deployment has happened.

1. Click **Play demo**. Move the hollow square to collect the diamond beacon.
   Each collected beacon advances the visible score and places the next target.
   Find all three to see **ALL FOUND!**.
2. **Arrow keys or W A S D** move (the X keypad); **Enter** is Start/restart.
   Z/X map to A/B, although the demo does not use them. Held directions repeat.
   Click the screen to focus gameplay controls if needed.
3. Use **Pause/Resume**, **Restart**, or **Fullscreen** below the display. Changing
   the visible console selector restarts the loaded cartridge and clears keys.
   Hiding the tab pauses; click Resume when returning. There is no emulation loop
   while idle or paused. Small/touch screens show direction, A, B, and Start
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
python3 demo/build.py --check   # optional for normal builds; verifies reproducibility
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
See [player UI pre-merge results](docs/issue-9-validation.md) and the
[original cartridge results](docs/issue-7-validation.md).

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
3. Confirm the public model selector, Settings/Help, Debug Mode, compatibility
   notice, and demo download/source/license links. Report deployment-only problems
   as follow-up work.

## Layout and contributing

- `crates/ws-core`: dependency-free CPU, cartridge, bus, input and mono tile video.
- `crates/ws-web`: thin wasm-bindgen adapter; machine state stays in Rust core.
- `web`: framework-free TypeScript/HTML/CSS, Vite and browser smoke script.
- `demo`: original cartridge assembly, graphics, deterministic builder and license.

Follow the [development flow and English collaboration policy](docs/development-flow.md),
[review guidelines](docs/review-guidelines.md), [Issue template](.github/ISSUE_TEMPLATE/issue.md),
[PR template](.github/pull_request_template.md), and [AGENTS.md](AGENTS.md).
