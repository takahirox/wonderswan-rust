# Issue #29 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, based on
`a4509d2b751b99fc3d56160c6a3d3a912d1a3e6f` plus workflow checkpoints.
The source Issue was fetched from GitHub and read before implementation.
No remote Issue, PR, branch or comment was modified.

## Implementation

The thin Wasm adapter exposes parsed `orientation()` as `horizontal` (including
no cartridge) or `vertical`. Browser presentation rotates vertical cartridges
counterclockwise: native `(x,y)` becomes `(y,223-x)`. A wrapper reserves the
rotated footprint; the framebuffer and canvas backing remain 224×144. Integer
scaling is preferred, with ratio-preserving fractional steps on smaller screens.
Whole-pixel bounds prevent soft composited edges in rotated fullscreen layouts.
Restart, compatible model changes and rejected replacements preserve metadata;
successful replacement updates it. Cartridge bytes and SHA-256 save identities
are unchanged. The core's input, cartridge, CPU and video implementations needed
no changes.

Arrow/WASD X movement and Z/X/Enter actions are preserved; I/L/K/J select
Y1/Y2/Y3/Y4. Accessible touch controls expose both raw groups and A/B/Start.
Labels and Help explain unchanged hardware identities across rotation. Keyboard
aliases, touch pointers and accessible Space/Enter button activation combine
as a union; one contributor's release retains the others. Touch focuses gameplay
without scrolling; other controls/dialogs cannot receive gameplay bindings.
Existing lifecycle clearing applies to all sources.

## Original probe and regression coverage

`fixtures/keypad.asm` and its horizontal/vertical generated ROMs are original,
source-included CC0-1.0 work. Their CPU instructions select/read all eight B5
combinations (including no row and mixed rows), write RAM/SRAM observations and
render bit tiles. Native tests independently calculate each button's expected
rows, mixed/diagonal combinations and release on Mono, Color and Crystal. They
check unchanged native asymmetric corner tiles and a single bottom-right pixel.

`web/scripts/keypad.mjs`, included in the complete compatibility suite, loads
these ROMs through the production player and real Wasm. It sends actual keyboard
and Chromium touch events, without replacing the player's machine, input handling
or cartridge CPU. It checks every button in both orientations on all models,
WASD aliases, repeat, keyboard/touch overlap, multiple pointers on one button,
independent diagonals plus actions, cancellation, actual pointer capture loss,
and Space/Enter activation overlap. Fresh rendering is required even when an
expected union is unchanged.

Held X+Y+actions are cleared across blur, focus on another control, pause, modal
opening, backgrounding, pagehide, restart, model selection, valid replacement and
invalid replacement. The retained CPU reads released input after failed loading.
Visibility, pagehide and window blur are deterministic synthetic events; this
does not claim physical tab/device testing. The adapter is also exercised in a
separate real Wasm instance for default/load/restart/model/reset metadata and
rejected malformed loads, incompatible model changes and incompatible loads.

Geometry and composited screenshot pixels assert counterclockwise direction,
square pixels, unchanged backing, pixelated rendering, containment, no toolbar
overlap and no horizontal overflow on desktop, 375px and native Chromium
fullscreen layouts in both orientations. Local file selection/saving produces
only GET requests, and the stored save key uses the exact raw vertical ROM hash.
Existing three-model gameplay, programmable LCD timing, audio, DMA, SRAM and
cartridge/console EEPROM regressions remain in the full browser suites.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 19 original cartridges, including both keypad variants. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 90 core unit tests, 12 integration tests, one Wasm adapter unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass; staged diff also checked before commit. |
| `npm run build:wasm` in `web` | Pass with the installed Rustup toolchain/Wasm target. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium suite, three-model gameplay, desktop/mobile input, lifecycle, fullscreen, recovery and audio controls. |
| `npm run test:compat` in `web` | Pass: complete Chromium suite, including new keypad/orientation coverage and all existing LCD timing, audio, DMA, cartridge/console save regressions. |

Production preview: `http://127.0.0.1:4287/wonderswan-rust/`, supplied through
`WS_PREVIEW_URL`. The two complete browser suites ran sequentially after the
final production build and both exited successfully. Documentation relative
targets were checked; a byte comparison confirmed that the two probe images
differ only in the orientation flag and resulting checksum.
`npm ci` restored locked dependencies without lockfile changes. The first Wasm
build selected Homebrew Rust, whose sysroot lacked the Wasm target; the following
frontend build therefore lacked generated bindings. Selecting the already
installed Rustup compiler using the README's PATH command resolved both failures.
No toolchain download, model/provider switch or allowance reset was needed.

Early targeted browser runs exposed test-driver mistakes: individual CDP contact
release must target the ending contacts, and testing capture loss requires
processing the initial capture before releasing it. Those sequences were fixed
without changing input assertions. The new layout/pixel assertions then caught
rounded fullscreen dimensions and fractional composited image edges. Production
sizing was corrected to fit exact available bounds with whole-pixel edges.

Probe SHA-256 values (both 131072 bytes):

- Horizontal: `cf6ac6af1e57691df1511a7d19ee1812e22c184a7b56e8e342907cbf918a7a08`.
- Vertical: `80c8982825655deea39c6765e4d05dd3346fd267649d37230cc2ffa031debbb2`.

## Source-Issue review and remaining limits

The implementation coordinator completed the source-Issue review against the
final diff, fixture/test sources and observed results under the repository review
guidelines. Every mandatory item is covered: thin adapter metadata and preservation,
eleven independent controls, input union and focus/lifecycle handling, upright
portrait presentation with unchanged native pixels, original CPU/Wasm probe
coverage on all models, complete regression checks and updated controls/support/
provenance documentation. The fullscreen geometry/pixel findings were resolved
and the final complete browser suites passed. No blocking findings remain.
No controller framework, extra core subsystem or unrelated behavior was added.

Firmware-free startup, independent implementation ownership and browser-local
ROM/save handling are preserved. No third-party emulator implementation code,
commercial ROM or proprietary firmware was used. No technical acceptance
correction was needed. External implementations cited in the source Issue are
corroboration, not physical-console measurements or code to copy.

Optional additional browser engines, independent homebrew comparisons, physical
console tests and human gameplay were not performed. Commercial compatibility,
exact keypad IRQ timing and runtime orientation signaling remain unverified or
unsupported. Required post-merge verification: none. This node does not push,
open a PR, merge or comment.
