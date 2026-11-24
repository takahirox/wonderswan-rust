# WonderSwan player layout validation

The player uses the SNES player's compact header, dark screen, adjacent
cartridge slot, and shared toolbar as layout references.
[snes-rust player](https://takahirox.github.io/snes-rust/)

## Production preview

Both suites used Playwright 1.64.1 with headless Chromium 156.0.8078.4 on
macOS arm64. Touch contacts and mobile viewports used browser emulation.

| Check | Result |
| --- | --- |
| `npm ci` | Passed |
| `npm run build:wasm` | Passed with the rustup toolchain on `PATH` |
| `npm run build` | Passed |
| `npm run test:browser` | Passed. Covered a 1280×800 desktop, 744px tablet width, phone portrait and landscape, both cartridge orientations, and native fullscreen. |
| `npm run test:compat` | Passed. Included CPU/Wasm fixtures, composited Clean and LCD pixels, CRT default and fallback, audio recovery, cartridge saves, console EEPROM, and RTC. |
| `git diff --check` | Passed |
| Additional Rust and fixture checks | Skipped. The core, Wasm adapter, and fixture sources were outside the change. |

A focused headless WebKit 27.2 check passed during the initial layout validation
on the same host. It loaded the vertical CC0 fixture at 375×812 and 812×375,
checked the canvas and control layout, confirmed Retro CRT, and dismissed Help
with Escape.
Physical-device testing was outside this run.

Chrome 157.0.8103.52 was outside this run. The LCD Authentic portrait corner
discrepancy on that version is tracked separately. The existing composited
corner pixel assertions and tolerances are included in the compatibility run.
[Issue #100](https://github.com/takahirox/wonderswan-rust/issues/100)

The smoke checks exercise local file selection, display switching with input
held, keyboard focus and Help dismissal, and all eleven controller buttons
through the fixture's CPU-produced bit tiles. Landscape includes simultaneous
X, Y, and A touches and checks that the toolbar and cartridge controls fit
within the first 812×375 viewport. After an invalid ROM selection, the alert
and the focused Load ROM button are both fully visible. The desktop fullscreen
check verifies that an invalid ROM alert leaves the vertical picture's bounds
unchanged. The 375×812 check verifies at least 3:1 contrast between the keyboard
focus outline and the page in Clean and Retro CRT modes.

## Screenshots

Playwright captured the production preview with the original CC0 keypad
fixtures selected as local files. In the 375×812 portrait view, Start, B, and A
are stacked beside the X and Y keypads, with all eleven buttons in view.

- [Desktop empty, 1280×800 viewport](images/issue-136-desktop-empty.png)
- [Desktop with a horizontal cartridge, 1280×800 viewport](images/issue-136-desktop-playing.png)
- [Portrait phone with a vertical cartridge, 375×812 viewport](images/issue-136-mobile-vertical.png)
- [Landscape phone with a horizontal cartridge, 812×375 viewport](images/issue-136-mobile-landscape.png)
- [Landscape phone fullscreen, 812×375 viewport](images/issue-136-mobile-fullscreen.png)

## Post-merge verification

- Pending. Verify the Pages workflow deployment of the merge commit and record
  the successful workflow run URL.
- Pending. Open the public player unauthenticated. Check the empty layout and a
  narrow viewport, load an original CC0 fixture from a local file, and verify
  playback, input, display, audio, and network requests. Record the verification
  against the deployed commit. File a linked follow-up for any production-only
  defect. [Issue #136](https://github.com/takahirox/wonderswan-rust/issues/136)
