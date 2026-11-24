# WonderSwan touch controller and game orientation

The physical layout follows the keypad matrix and the controls illustrated for
the original WonderSwan, WonderSwan Color and SwanCrystal. Horizontal games
stack Y above X on the left. Vertical games follow a counterclockwise quarter
turn of the keypad labels, with Y to the lower left and X to the lower right.
[WSdev keypad matrix](https://ws.nesdev.org/wiki/Keypad),
[WonderSwan manual](https://gamingalexandria.com/highquality/wonderswancolor/GunPey/GunPey%20-%20Manual.pdf),
[WonderSwan Color press kit](https://www.bandai.co.jp/releases/images/3/204.pdf),
[SwanCrystal press kit](https://www.bandai.co.jp/releases/images/3/4092.pdf)

## Production preview

Validation used Playwright 1.64.1 with headless Chromium 156.0.8078.4 on macOS
arm64. Mobile viewports, touch contacts, and safe-area insets were emulated.
The production preview included the remembered-ROM picker from #139
(`main` at `81c4c6f`). Reopening the vertical fixture from My cartridges after
page reload restored its Horizontal override for both the picture and controller.

| Check | Result |
| --- | --- |
| `npm ci` | Passed |
| `npm run build:wasm` | Passed with Rust 1.100.0 on `PATH` for the Wasm target |
| `npm run build` | Passed |
| `npm run test:browser` | Passed with the production preview. Covered eleven touch targets at least 44×44, 375×812 and 320×568 upright layouts, simulated safe areas, short landscape, fullscreen, and error recovery. |
| `npm run test:compat` | Passed with the production preview. Covered all three models, both keypad fixtures, orientation overrides, display effects, audio, and cartridge and console persistence. |
| `git diff --check` | Passed |

The browser checks used both original CC0 keypad cartridges. They sampled the
composited corner pixels to verify rotation direction, checked the native
224×144 canvas backing, and decoded guest CPU input from the framebuffer.
They exercised all eleven physical masks, combined keyboard and touch input,
independent release, pointer cancellation, and Help pause/resume.
Orientation preference checks include opposite-header overrides, a paused LCD
frame, same-name and renamed files, remembered selection after reload, restart,
a compatible model change, Auto clearing, and malformed or denied local storage.
The storage test also verified that changing orientation preserved the existing
progress records.

Rust core and fixture sources were outside the change. Mobile WebKit and
physical devices were outside this run.

## Screenshots

Playwright captured these views on the production preview with the repository’s
original keypad fixtures, using Auto.

| Phone viewport | Horizontal game | Vertical game |
| --- | --- | --- |
| Upright, 375×812 | [Horizontal fixture](images/issue-142-portrait-horizontal.png) | [Vertical fixture](images/issue-142-portrait-vertical.png) |
| Landscape, 812×375 | [Horizontal fixture](images/issue-142-landscape-horizontal.png) | [Vertical fixture](images/issue-142-landscape-vertical.png) |

## Post-merge verification

Pending. Record the Pages workflow URL and deployed merge revision. On the
published player, load both original keypad fixtures and verify Auto and
manual overrides on an upright phone viewport. Resize to landscape, confirm
that the selected orientation persists, and verify guest input from the
player’s buttons. [Issue #142](https://github.com/takahirox/wonderswan-rust/issues/142)
