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

## Narrow portrait fullscreen resize follow-up

With the same emulated insets, a 320 CSS pixel viewport has 232 CSS pixels of
safe width. In native Chromium fullscreen with the CC0 keypad fixtures, the
horizontal game stays compact from 320×376 to 320×377, keeping a 98×63 picture.
The vertical game stays compact from 320×349 to 320×350, keeping a 72×112
picture. At 320×385 the horizontal game stacks and grows to 112×72. At 320×351
the vertical game stacks and grows to 81×126. The browser regression checks
resizes in both directions and at adjacent widths. The picture, all eleven
game buttons and Exit fullscreen stay inside the measured safe area without
clipping, overlap or scrolling at both transitions.

At 232 CSS pixels of safe width, the compact horizontal grid fits down to
197 CSS pixels of safe height and the compact vertical grid fits down to
241 CSS pixels (320×226 and 320×270 viewports). One pixel below either limit,
bottom controls are partially clipped at rest. Scrolling the fullscreen safe
area by that pixel reaches their full targets, which remain at least 44×44 CSS
pixels.

## Fullscreen review boundary follow-up

Native fullscreen in headless Chromium 156.0.8078.4 used the original CC0
keypad fixtures and emulated insets of 44 CSS pixels on each side, 8 at the
top, and 21 at the bottom. Measurements used the fullscreen content bounds
after padding. At these sizes, the picture, all eleven game buttons, and
Exit fullscreen fit without overlap or scrolling. All buttons retained at
least 44×44 CSS pixels.

| Game | Viewport (CSS px) | Safe area (CSS px) | Picture (CSS px) |
| --- | --- | --- | --- |
| Horizontal | 315×226 | 227×197 | 70×45 |
| Horizontal | 315×528 | 227×499 | 224×144 |
| Vertical | 454–455×243 and 456–457×245 | 366–367×214 and 368–369×216 | 72×112 |
| Vertical | 463–464×243 and 463–464×245 | 375–376×214 and 375–376×216 | 72×112 |
| Horizontal | 812×262 → 812×263 | 724×233 → 724×234 | 350×225 → 364×234 |
| Horizontal | 568×220 → 568×221 | 480×191 → 480×192 | 182×117 at both sizes |

The narrow horizontal grid fits at 224×197 CSS pixels of safe area
(312×226 viewport). At a safe width of 223, one pixel of horizontal scrolling
reveals the clipped edge. At a safe width of 227 and height of 196, one
pixel of vertical scrolling reveals the bottom of B. At the same width,
the compact vertical grid fits at a safe height of 241 (315×270 viewport)
and needs one pixel of vertical scrolling at 240. Scrolling reaches the
full targets at their original sizes.

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
