# wonderswan-rust

A from-scratch Rust WonderSwan emulator project targeting **WonderSwan (mono)**,
**WonderSwan Color**, and **SwanCrystal**, with **SwanCrystal as the default**.
No existing emulator implementation is copied, ported, vendored, or used as a dependency.

The current application is an **initial, non-playable prototype**. It creates a
Rust machine for the chosen model, displays its model identity, accepts arbitrary
nonempty local files as raw cartridge bytes, and reports the filename and stored
byte count. Empty files are rejected without discarding a previously loaded ROM.
Switching models or clicking **Reset machine** clears the ROM and resets status.
Files stay in browser memory; there is no upload endpoint, server-side storage,
bundled ROM, or external ROM download.

A blank, opaque black **224 × 144 RGBA framebuffer** originates in the Rust core,
passes through WebAssembly, and is copied to a canvas with pixelated CSS scaling.
It is not fabricated game graphics or simulated LCD output. The three models
currently differ only in their recorded identity.

**ROMs cannot execute and games cannot play yet.** CPU execution, boot ROMs,
cartridge mapping or format validation, interrupts, timers, video rendering and
timing, palettes, audio, keypad input, save states, persistence, compatibility
detection, and physical LCD differences are outside this prototype.

## Public browser prototype and deployment

Intended public URL: **https://takahirox.github.io/wonderswan-rust/**. Once the
first deployment succeeds, anyone can open it in a modern browser without
installing tools or signing in. The hosted app is the same **non-playable
prototype**: ROMs cannot execute and games cannot play yet. Local file bytes
stay in the browser; the site is entirely static.

One-time repository administrator prerequisite: open **Settings → Pages → Build
and deployment → Source: GitHub Actions**. This setting has not been verified or
changed as part of adding the workflow. If deployment reports that Pages is not
enabled or lacks permission, an administrator must confirm that source setting
and check the repository's Actions and `github-pages` environment restrictions.
No personal access token or custom repository secret is needed.

Every push to `main`, including a merged PR, starts the
[Pages deployment workflow](.github/workflows/deploy-pages.yml). It installs Rust
and Node.js 24, uses the committed lockfiles, builds the current Rust core to
WebAssembly, then builds the frontend and deploys the complete `web/dist`.
Overlapping runs cancel the older run so stale builds do not publish afterward.
For a manual retry, open
[Actions → Deploy browser prototype to Pages](https://github.com/takahirox/wonderswan-rust/actions/workflows/deploy-pages.yml)
and choose **Run workflow** with branch **main**. Other branches do not deploy.

Inspect that Actions page for build/deployment logs and failures. Inspect
[Settings → Pages](https://github.com/takahirox/wonderswan-rust/settings/pages)
and the repository's **Deployments → github-pages** environment for the
publication status and deployed commit.

Required post-merge verification (pending until performed; a successful public
deployment is not a pre-merge gate):

1. Confirm the merge/push to `main` triggers the workflow and it succeeds; record
   the run URL and any failure.
2. Open the public URL without authentication. In browser developer tools, check
   that HTML, CSS, JavaScript, and Wasm load under `/wonderswan-rust/` with no
   404s or startup errors. Check SwanCrystal defaults, all three model selections,
   and the visible 224 × 144 canvas. Load a small original dummy file and confirm
   its exact byte count, no upload request, and the visible non-playable disclaimer.
3. On a subsequent real push to `main`, confirm another successful deployment
   updates the public site; record the run URL, commit, and result.

Report any Pages setting, permission, or deployment blocker and the needed
administrator action instead of marking these checks passed.

## Local development

Prerequisites:

- Stable Rust and Cargo (install through [rustup](https://rustup.rs/)).
- The `wasm32-unknown-unknown` target: `rustup target add wasm32-unknown-unknown`.
- [wasm-pack](https://github.com/wasm-bindgen/wasm-pack):
  `cargo install wasm-pack --locked` (validated with 0.14.0).
- Node.js 22.12+ or 24+ and npm, plus a modern browser with WebAssembly and canvas.

Ensure `rustc` and `cargo` on `PATH` use the rustup toolchain with the Wasm target.
If another Rust installation takes precedence, select the rustup binaries first:

```sh
export PATH="$(dirname "$(rustup which rustc)"):$PATH"
```

From the repository root:

```sh
# Browser-independent core tests, plus the thin adapter's native build/tests.
cargo test --workspace --locked

cd web
npm ci
# Compile Rust to Wasm and generate the browser ES module and TypeScript types.
npm run build:wasm
# Start the local Vite development server and open the browser.
npm run dev
```

Vite prints the local URL (normally `http://127.0.0.1:5173`). This is a static
frontend served locally, with no application backend. Stop it with Ctrl+C.
Rerun `npm run build:wasm` and reload the page after Rust changes; Vite handles
TypeScript/CSS updates. Generated `web/pkg`, `web/dist`, and dependency/build
directories are ignored. Both lockfiles are committed for reproducible installs.

To build and preview the production frontend, from `web`:

```sh
npm run build:wasm
npm run build
npm run preview
```

`npm run build` runs the TypeScript check and produces static assets (including
Wasm) in `web/dist`. Serve that directory over HTTP; opening HTML via `file://`
is insufficient for loading Wasm. `npm run preview` prints its local URL and
opens the production build at `http://127.0.0.1:4173/wonderswan-rust/` by default.
Production and preview assets use the `/wonderswan-rust/` base required by GitHub
Pages; the development server still uses `/`. No deployment is required for this
local preview.

The equivalent standalone Wasm compilation command, from the repository root, is:

```sh
wasm-pack build crates/ws-web --target web --out-dir ../../web/pkg --locked
```

## Layout and validation

- `crates/ws-core`: dependency-free, browser-independent model, machine, raw ROM
  storage, and framebuffer. Unit tests cover defaults, all model selections,
  resets, byte ownership/counts, empty input, and framebuffer dimensions/content.
- `crates/ws-web`: thin `wasm-bindgen` adapter; machine state stays in the core.
- `web`: framework-free TypeScript, HTML, and CSS, built with Vite.

Run `cargo fmt --all -- --check`, `cargo test --workspace --locked`, and
`cargo clippy --workspace --all-targets --locked -- -D warnings` from the root.
Run both Wasm compilation and the frontend production build above.

Browser smoke check (use an original dummy file; no game ROM is needed):

1. Open the local page. Check SwanCrystal is selected, the active model reported
   by Rust is SwanCrystal, and the canvas has internal resolution 224 × 144.
2. Select WonderSwan (mono), WonderSwan Color, then SwanCrystal. Verify the Rust
   model status follows each selection and there are exactly three options.
3. Choose a nonempty file. Verify its filename and exact byte count appear, along
   with the statement that games cannot run yet.
4. Reset, reload the file, and switch models. Each reset/switch must show no ROM
   loaded and zero stored bytes. The same file can be selected again.
5. Choose a zero-byte file. Verify a recoverable empty-data error, then choose the
   nonempty file again. A file-read failure should also report an error without
   disabling the UI (it can be simulated with a rejected `File.arrayBuffer()` in
   browser developer tools).
6. Resize the page. The internal canvas stays 224 × 144 and its CSS uses pixelated
   scaling. Optionally inspect the Network panel: file selection sends no requests.

## Contributing

- [Development flow and English collaboration policy](docs/development-flow.md)
- [Review and merge guidelines](docs/review-guidelines.md)
- [Issue template](.github/ISSUE_TEMPLATE/issue.md)
- [Pull request template](.github/pull_request_template.md)
- [AI agent entry point](AGENTS.md)
