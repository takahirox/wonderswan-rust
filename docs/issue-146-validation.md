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

## Short fullscreen resize follow-up

Native fullscreen in headless Chromium 156.0.8078.4 used both CC0 keypad
fixtures and simulated insets of 44 CSS pixels on each side, 8 at the top, and
21 at the bottom. The horizontal picture stayed 98×63 CSS pixels across the
568×221 to 568×220 viewport resize. The vertical picture stayed 63×98 CSS
pixels from 390×225 through 390×228, including the 390×226 review case.
The full rectangles of all eleven game buttons, the picture, and Exit
fullscreen fit inside the measured safe area without overlapping or scrolling.
The browser suite also checked adjacent widths, the 272 CSS pixel horizontal
and 264 CSS pixel vertical safe-width breakpoints, and the previous compact
cases.

At a safe width of 480 CSS pixels, the horizontal compact grid fits down to
186 CSS pixels of safe height. At a safe width of 302 CSS pixels, the
vertical compact grid fits down to 194 CSS pixels of safe height. At 185 and
193 CSS pixels of safe height respectively, the bottom targets extend one
pixel beyond the visible safe area at rest. Scrolling inside that area
reveals the full targets at their original size.

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
