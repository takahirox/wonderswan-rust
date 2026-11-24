# Player controls, visibility, and fullscreen validation

The local production preview used the Issue #146 implementation on top of
`main` at `f11fde9`. Playwright 1.64.1 ran headless Chromium 156.0.8078.4 on
macOS arm64. Both touch layouts used the original CC0 keypad cartridges.

| Production-preview gate | Result |
| --- | --- |
| `npm ci` | Passed |
| `npm run build:wasm` | Passed with the rustup Rust 1.99.0 toolchain and wasm-pack 0.14.0 |
| `npm run build` | Passed |
| `npm run test:browser` | Passed |
| `npm run test:compat` | Passed |
| `git diff --check` | Passed |

Windowed layout checks measured the gaps above gameplay and below it, verified
eleven 44×44 or larger touch targets, and tested dragging from gameplay to
Reset, cancellation, and repeated taps. At 812×375 the toolbar, picture, and
controller fit together. At 680×340 and 320×568 with simulated safe areas,
normal page scrolling reaches controls below the fold, including the cartridge
slot and recovery actions. Starting a game brings the picture and controller
into view when they fit together.

The visibility tests dispatched simulated `visibilitychange`, `pagehide`,
and `pageshow` events. They observed real Wasm frame counts and guest input,
including manual Pause, Help, pending and stale ROM reads, faults, and empty
startup. Controlled timestamps checked for one producer and presentation chain
and verified that background time was dropped. Web Audio probes checked that
suspension cleared queued sound and that playback resumed with the saved
volume. The suites also covered Reset through the original IPL and cartridge
and console persistence.

Chromium entered actual fullscreen on `#game`. Checks covered Escape, native
API exit, the touch Exit fullscreen button, restored focus and scroll,
Chromium's accessibility tree, paused redraws, and unavailable or rejected
requests. The keypad tests sampled composited corner markers in both game
orientations with Clean and LCD Authentic. Touch contacts, safe-area insets,
and viewport sizes were emulated.

A separate headless Chromium check restored both Running and Paused states
through actual back-forward cache navigation. Switching tabs in that session
left the document visible.

## Screenshots

| Viewport | Horizontal game | Vertical game |
| --- | --- | --- |
| 375×812 windowed | [Horizontal](images/issue-146-portrait-horizontal.png) | [Vertical](images/issue-146-portrait-vertical.png) |
| 812×375 windowed | [Horizontal](images/issue-146-landscape-horizontal.png) | [Vertical](images/issue-146-landscape-vertical.png) |
| 375×812 fullscreen | [Horizontal](images/issue-146-fullscreen-portrait-horizontal.png) | [Vertical](images/issue-146-fullscreen-portrait-vertical.png) |
| 812×375 fullscreen | [Horizontal](images/issue-146-fullscreen-landscape-horizontal.png) | [Vertical](images/issue-146-fullscreen-landscape-vertical.png) |

## Post-merge verification

Pending. Record the Pages workflow URL and deployed revision. On the published
player, use both original CC0 keypad fixtures to verify mobile control
separation, automatic return from a visibility interruption versus manual
Pause, and gameplay-only fullscreen entry and exit. [Issue #146](https://github.com/takahirox/wonderswan-rust/issues/146)
