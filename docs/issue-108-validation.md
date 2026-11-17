# Issue #108: local-ROM-only player validation

Validated in the assigned worktree against upstream `33eff9a` on 2026-11-17.
This record covers local pre-merge acceptance; it does not establish deployment.

## Result and retained coverage

The player starts with no cartridge, a neutral placeholder, local `.ws` / `.wsc`
instructions and one primary **Load ROM** action. Cartridge selection uses the
existing local file input. The supplied game, its source/builder/license assets,
launcher, network path, download links and game-specific tests are removed.
No emulator implementation, fixture assembly or generated fixture bytes changed.

The smoke suite now loads the independent CC0 keypad fixture from `fixtures/`
through the file chooser. It checks zero ROM bytes/frames before selection,
empty/running picker cancellation, three-model CPU input, pause/resume/restart,
model input clearing, invalid-file preservation and same-file recovery. Gated
local Blob reads cover failures while empty/running/paused, modal/background
completion, stale reads after restart/model changes, and newer selections winning
an older read race. It retains execution errors, dialog focus/isolation,
conditional resume, Debug Mode, fullscreen/API failures, initialization retry,
and 375×812 touch/layout checks. The `.wsc` selection path is also exercised.

Compatibility retains actual Wasm CPU, ROM validation, local saves, console
EEPROM, RTC, sound and display coverage. Audio activation now uses a local fixture
and verifies that a trusted Load ROM click resumes audio before the chooser/read.
LCD presentation/fallback tests also use an independent local fixture. Existing
exhaustive keypad tests cover both orientations, all buttons, mixed sources and
release/lifecycle behavior. No assertions or tolerances in those hardware and
presentation checks were weakened.

The scheduler equivalence test uses the existing monochrome keypad fixture in
place of the deleted game. Benchmark scripts retain only independent SRAM/audio
fixtures. Prior Issue validation and performance measurements remain historical;
performance reports explicitly distinguish archived results from current inputs.

## Executed checks

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass; all remaining unit/integration/doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `python3 fixtures/build.py --check` | Pass; every independent fixture reproduces. |
| In `web`: `npm ci` | Pass; no vulnerabilities reported. |
| In `web`: `npm run build:wasm` | Pass with the documented rustup toolchain PATH. |
| In `web`: `npm run build` | Pass; TypeScript and Vite production build. |
| In `web`: `npm run test:browser` | Complete pass against production preview. |
| In `web`: `npm run test:compat` | Complete pass against production preview, including shaders, portrait/keypad, raw ROMs, saves and console storage. |
| Modified JavaScript/Python syntax, relative Markdown file links and `git diff --check` | Pass. |
| Active-source/reference and production-artifact audit | Pass; details below. |

Browser suites ran sequentially in Playwright's installed headless Chromium with
`WS_PREVIEW_URL=http://127.0.0.1:4287/wonderswan-rust/`. Desktop/mobile empty,
playing, debug, settings and fullscreen screenshots were captured by the smoke
suite; desktop/mobile empty screenshots were visually inspected. This is emulated
mobile viewport coverage, not a physical mobile browser test.

The first Wasm build attempt selected Homebrew Rust, which lacked the Wasm target.
Using the README's rustup PATH selection resolved this environment issue; the
release Wasm build and subsequent browser checks passed.

Short benchmark-script preflights also passed: native release throughput followed
by `tools/perf/compare.py` with two warmup frames, two measured frames and one
repeat, with and without `WS_PERF_INPUT_REPLAY=1`; and
`tools/perf/wasm-throughput.mjs` using the same built package for both labels with
two measured frames and one repeat. Both retained fixture paths, success markers,
metadata and adapter output checks work. These are script/dependency checks,
not new performance measurements or cross-core comparisons.

## Artifact and local-data audit

The final `web/dist` contains exactly four files: `index.html`, one CSS file,
one JavaScript file and the Wasm module. It contains no `.ws` / `.wsc` files,
fixture copies, game/download directory or cartridge asset. Active application,
core/test, benchmark, workflow and maintained documentation files have no removed
game name, launcher identifier or deleted-asset/build-path references. Remaining
mentions occur only in historical Issue validation and archived performance
records; maintained Markdown file targets exist.

Browser request observations show only GET asset requests and no ROM URL requests;
local file selection does not increase the request count. Save compatibility
checks verify SHA-256 identity, renamed/same-name isolation and same-ROM restoration
after reload/reopen without network activity. Fixtures remain test/development
inputs under `fixtures/`, outside the published site.

## Pending post-merge verification

1. After merge and Issue closure, verify the normal Pages deployment on `main`
   succeeds and record the Actions run URL and deployed commit.
2. On the published URL, verify empty startup, the local file-picker flow, absence
   of supplied game/download links and no ROM uploads/network loading. Track any
   actual publication defect as a linked follow-up.

These production checks were not performed. Optional physical-mobile validation
was not performed. No push, PR, merge or Issue comment was made by this node.

## Publication integration validation

During publication, `main` had advanced to `150516c` (the Retro CRT default
change). Merged that revision into the PR branch and resolved the overlapping
display test by retaining the new default-shader assertions while loading the
independent local keypad fixture. No bundled cartridge path was restored.

On the combined tree, `npm ci`, the release Wasm build with the documented rustup
PATH, `npm run build`, the complete `npm run test:browser` and complete
`npm run test:compat` all passed. Both browser suites ran sequentially against
production preview at `http://127.0.0.1:4287/wonderswan-rust/`. JavaScript syntax
and Git whitespace checks passed, and the production artifact still contains
only HTML, CSS, JavaScript and Wasm, with no ROM files. Rust and fixture content
did not change during integration; their checks above were reused.

PR #109 publishes this work. No merge or production verification was performed;
the required post-merge checks remain pending.
