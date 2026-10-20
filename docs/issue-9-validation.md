# Issue #9 player UI validation

Performed on 2026-10-20 in the assigned worktree. The change stays within the
existing HTML, CSS, TypeScript, browser tests, and frontend documentation. No Rust
core, bundled cartridge, dependencies, build configuration, or deployment workflow
changed. No push, PR, merge, comment, or deployment was performed.

## Checks performed

| Check | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo test --workspace --locked` | Pass: 11 unit tests and 4 cartridge integration tests, plus adapter/doc tests |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass |
| `npm ci` in `web` | Pass, committed lockfile unchanged |
| `npm run build:wasm` in `web` | Pass, wasm-pack 0.14.0 with rustup compiler |
| `npm run build` in `web` | Pass: strict TypeScript check and production Vite build |
| `npm run test:browser` against production preview | Pass in headless Chromium with Playwright 1.64.1 |
| `git diff --check` | Pass |

The owned preview used port 4176 because the usual ports were occupied. The final
browser command, from `web`, was:

```sh
WS_PREVIEW_URL=http://127.0.0.1:4176/wonderswan-rust/ \
WS_SMOKE_ARTIFACTS=/tmp/wonderswan-issue-9 npm run test:browser
```

Browser coverage uses real Wasm execution and rendered canvas pixels:

- Fresh page: exactly three models, SwanCrystal default, 224×144 internal canvas,
  prominent demo action, hidden help/debug, desktop touch controls hidden.
- Complete Beacon Trail, observe each score and win lettering, and restart through
  cartridge Enter/Start on all three models. UI restart, pause/resume, WASD, and
  model switching with a held key retain their semantics.
- Play a local ROM through to completion without network requests, reject an empty
  file, reload the same file, and recover. A checksum-valid unsupported-opcode
  cartridge produces a visible stopped state and recoverable execution error.
- Open Settings by pointer and keyboard, verify focus entry and native modal Tab
  behavior, block gameplay input, close through Close and Escape, restore focus,
  clear held keys, and resume only a previously running, non-background-paused game.
- Debug Mode starts off, enables factual model/name/131072-byte size/state/core
  frame counter, collapses runtime details, closes, disables, and stays off after
  reload. Runtime updates happen outside the animation callback, at most twice a
  second; no registers, FPS, or accuracy values are invented.
- Native Chromium fullscreen entry/exit preserves aspect ratio and pixelated
  rendering; Settings also works there. Synthetic unsupported/rejected API cases
  show feedback without stopping gameplay.
- Delayed Wasm initialization, initialization failure/retry, demo HTTP failure and
  retry, a load finishing behind Settings, Settings' Play demo action, and a load
  finishing after a background pause all recover with the correct focus/state.
- At 375×812, touch events exercise all directions, simultaneous directions,
  holding/repeating, release, cancel, blur, Start, and no page scrolling during
  control interaction. A/B and keyboard Z/X are tested through a checksum-valid,
  in-memory variant of the original CC0 demo that reads the action keypad row for
  movement. This fixture is never written over the bundled ROM.
- Production `/wonderswan-rust/` Wasm/ROM paths, GET-only normal requests, the CC0
  notice, and absence of unexpected browser errors are checked. Injected network
  failures run in an isolated context and are expected failures.

Visibility changes, fullscreen failure cases, and network faults are deterministic
simulations. Native fullscreen and Chromium touch input were exercised, but this
is not a claim of physical tab-switch, touch-device, or cross-browser testing.

## Rendered UI review

Opened the generated desktop and mobile screenshots, including empty, playing,
settings, debug, and fullscreen states. Representative final screenshots are
kept here for review:

- [Desktop player, 1440×900](images/issue-9-desktop.png)
- [Mobile player, 375×812](images/issue-9-mobile.png)

Observed and fixed during review:

- The desktop frame was initially two pixels short of allowing integer 4× scaling.
  Its width now accommodates an 896×576 canvas. With the debug sidebar open, the
  canvas uses integer 3× scaling; native desktop fullscreen uses integer 5× scaling.
- Narrow layouts preserve the exact 224:144 ratio using nearest-neighbor scaling
  when integer 2× would not fit. The canvas is about 325×209 CSS pixels at 375px.
- The model selector and primary controls fit both widths, without horizontal
  overflow or clipped actions. Touch buttons have at least 44×44px targets. Debug
  information moves below the player on narrow screens.
- Long help content scrolls inside the modal; a sticky heading keeps Close visible.
  Native mobile tap highlights are suppressed in favor of the explicit active and
  focus styles, avoiding a transient highlight over the modal.
- The normal player has no compatibility paragraphs, license list, or diagnostics.
  These remain reachable from Settings. The screen and demo action dominate idle
  presentation; loaded play keeps all primary actions directly below the screen.

No imagery, fonts, markup, or code was copied from the referenced player sites.
There is no added UI framework, ROM upload endpoint, catalog, or gameplay logic.
The adapter's existing keypad and frame counter APIs supply input and diagnostics.

## Remaining verification

Optional additional browsers and a real touch device were not tested.

Required post-merge verification remains **pending**:

1. Confirm the `main` merge commit's GitHub Pages workflow succeeds; record its
   Actions run URL, commit, and outcome.
2. Open the public player without authentication and verify demo play, model
   selection, Settings/Help, Debug Mode, ROM/source/license links, and production
   JS/CSS/Wasm/ROM assets. Record any deployment-only defects.

These checks require the later merge and deployment. Local preview results do not
establish that the public site has been updated.
