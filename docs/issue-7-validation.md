# Issue #7 pre-merge validation

Performed on 2026-10-20 in the assigned Git worktree, against the implemented
cartridge and emulator. No push, pull request, merge, Issue comment or production
deployment was performed.

## Automated results

| Check | Result |
| --- | --- |
| `python3 demo/build.py --check` | Pass: generated and committed ROM match byte-for-byte |
| ROM size / SHA-256 | 131072 bytes / `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19` |
| `cargo fmt --all -- --check` | Pass |
| `cargo test --workspace --locked` | Pass: 11 hardware unit tests and 4 cartridge integration tests; adapter and doc tests also pass |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `npm ci` in `web` | Pass with committed lockfile |
| `npm run build:wasm` in `web` | Pass using rustup compiler and wasm-pack 0.14.0 |
| `npm run build` in `web` | Pass: TypeScript and Vite production build |
| Production assets | ROM and notice copied into `web/dist/demo`; JS/CSS/Wasm URLs use `/wonderswan-rust/` |
| `git diff --check` | Pass |

The native integration tests execute the bundled cartridge as instructions from
FFFF:0000. They check changed tile pixels, directional input changing RAM and
video, each score increment, completion, stopped movement after completion,
cartridge Start restart, reset clearing input, all three models, deterministic
frames, movement bounds, validation failures, color-header restrictions and
unsupported-code faults remaining latched. Unit tests independently exercise
arithmetic flags, byte/word register and ModR/M addressing, segment overrides,
stack/CALL/RET, LOOP/condition codes, repeated strings, I/O, bank mapping, keypad
rows, scanlines, tile planes, palettes, flips and scroll.

## Observed browser smoke test

Used the production build, served with:

```sh
cd web
npx vite preview --host 127.0.0.1 --port 4287 --strictPort
```

Port 4173 was already occupied; this check used its own preview at
`http://127.0.0.1:4287/wonderswan-rust/`. In another terminal:

```sh
cd web
WS_PREVIEW_URL=http://127.0.0.1:4287/wonderswan-rust/ \
WS_SMOKE_SCREENSHOT=/tmp/ws-issue7-win.png npm run test:browser
```

**Passed in headless Chromium (Playwright 1.64.1), fresh unauthenticated context.**
The test sends browser keyboard events and reads rendered canvas pixels, rather
than changing game position or score. The resulting full-page win screenshot was
also opened and visually inspected: the cartridge display showed **3/3**, the
player square, borders, controls, and **ALL FOUND!**. No screenshot-only assertion
is used as the emulation acceptance test; the native tests also verify CPU-driven
machine state and input effects.

Observed steps/results:

1. Wasm initialized, SwanCrystal was initially selected, exactly three model
   choices were present, and the canvas had 224×144 internal resolution.
2. Clicked **Play demo** and observed the initial player/score. On SwanCrystal,
   WonderSwan mono and WonderSwan Color, moved Right ×6, Up ×3, Right ×9, observed
   scores 1/3, 2/3, 3/3 and the completion message, then restarted using Enter.
3. Moved, paused, attempted movement (pixels unchanged), resumed, moved again,
   and restarted using the UI. Switching models during a held key cleared the
   input and restored the initial game. WASD movement was also checked.
4. The visibility handler was tested with a **synthetic** document-hidden change:
   it paused and required explicit Resume after becoming visible. A physical tab
   switch was not tested; this distinction is intentional.
5. Selected `web/public/demo/beacon-trail.ws` through the local file picker and
   played it to completion. File selection generated no network requests.
6. Selected an empty ROM and observed a recoverable error, then selected the
   original local cartridge again and successfully restarted it.
7. Verified ROM/Wasm loading under the production base path, the deployed CC0
   notice, no page exceptions or HTTP errors, and GET-only network traffic.

The initial Wasm attempt found Homebrew Rust without the Wasm target. Selecting
rustup using the README's PATH command resolved it. An early hand-encoded CPU
unit-test jump displacement was corrected; the complete suite then passed.
Neither setup issue remains a blocker.

## Provenance and unperformed checks

The bundled game binary is generated only from original assembly and hand-authored
tiles in `demo/`; its byte identity is checked above. The added game asset inventory
is one `.ws` cartridge and a text notice. CC0 covers source, binary and graphics;
its full legal text is included. No BIOS, commercial cartridge, copied font/game
asset, third-party game binary, audio, or emulator dependency was added.

Physical hardware comparison, other browsers and mobile devices were **not
performed** (optional). Full commercial compatibility and hardware-perfect CPU,
video, timing and model behavior are not claimed.

Required post-merge verification remains **pending**, separate from these results:

1. Record the `main` Pages workflow Actions run URL, deployed commit and final
   deployment result.
2. Play the public site unauthenticated to completion and restart; verify JS,
   Wasm and demo paths, and load the downloaded cartridge through the picker.
3. Confirm the live compatibility statement and provenance/license link; report
   any publication problems as follow-up work.
