# Issue #37 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`60114160772924f4c2d501a88f19e765754032e1` (workflow checkpoints on evaluation
baseline `3869f9e63a853bf75825c504c448fb0be8084fdf`). The implementation coordinator
read open Issue #37 and the development/review guidance. No remote Issue, PR,
branch or comment was modified. No push or merge is part of this node.

## Correction and bounded source reconciliation

Before implementation, inspected the pinned Wonderful `display.xml` and pinned
ARES PPU scheduler, screen, memory/palette and DAC files through the GitHub
connector. The primary DISPLAY_LINE definition establishes final translation
one line later: translation while DISPLAY_LINE is L targets row L−1. The old
AI-authored immediate-RGBA assumption therefore targeted the wrong raster row.
[Reconciliation and compatibility policies](lcd-timing.md#one-line-shadecolor-translation-issue-37)
record the original assumption, correction, source links, rationale, state split
and remaining uncertainty. Ares is implementation context, not independent proof
of precise phases; no implementation was copied. Previously inaccessible
WSdev/WSman material is not used as established evidence.

Tile/map/OAM, scroll, windows, transparency and priority are composition state.
Mono palette ports resolve to shade identities at composition; the shade LUT is
sampled at translation. Color composition retains palette/index identities;
RGB12 palette RAM is read at translation. Backdrop identity and mode type are
latched at composition; LCD enable is sampled at translation. These sampling
and mode-transition choices are explicitly compatibility policies beyond the
source-established row offset, not measured silicon behavior.

A pending visible row translates at the end of its visited successor line,
including row 143 during line 144, before the next bus line transition. At actual
raster wrap any remaining pending row is discarded. Thus short T<144 frames
translate rows 0 through T−1; T=0 translates none. Unvisited rows and rows lacking
a successor retain their previous pixels. Runtime shortening/extension and odd
totals retain bounded progress. No line/IRQ/timer position, device clock, frame
completion or instruction/DMA overshoot rule is moved. No analog panel, exact
fetch/write phases, OAM-copy accuracy or commercial-title result is claimed.

## Implementation and independent evidence

The core retains one bounded row of typed shade/color identities, never completed
RGBA or references to old tile/map/OAM. The next line boundary translates that
row using current final lookup values, then composes the current visible row.
Reset/restart/model changes/successful replacement use an empty pipeline and white
framebuffer. Rejected operations preserve the running pipeline. Native execution
and the production Wasm/player use this same core path without new public APIs.

Six new core tests provide independent literal pixels across mono on all three
models and 2bpp planar/4bpp planar/4bpp packed color on Color/Crystal. They cover:

- Row 0 remains untouched at composition; final palette changes affect it only
  during line 1. Destruction of tile/map/OAM, layer enables, mono palette mappings
  and backdrop after composition cannot alter pending identities. Overlapping
  layers, transparent pixels, early low-priority sprites masking later sprites,
  front sprites and backdrop all retain their decisions. Adjacent/completed rows
  are checked separately, so delaying RGBA or refetching cannot pass.
- LCD disable/enable and mono/color runtime transitions follow the documented
  sampling policies, including retained color RAM after switching to mono.
- Row 143 remains pending through entry to 144. Literal clocks, IRQ/timer bits,
  OAM latch and frame completion stay fixed at totals 156/158/180 over repeated
  frames. A separate last-row sprite test changes latched OAM at VBlank and
  proves old identity survives; the next frame uses the new sprite palette.
- Short/odd totals 0/1/2/142/143/144/145/155/157/255, repeated frames and runtime
  shortening/extension preserve translated and unvisited rows. Existing exhaustive
  byte-total, line-counter/IRQ/timer, instruction and DMA overshoot checks remain.
- Reset, restart, model changes and successful replacements clear pending state;
  malformed/incompatible loads and incompatible model changes preserve it.

The existing programmable-raster rendering regression now advances the real
machine pipeline and expects a shade-LUT write in line 50 to begin at row 49.
It also checks that total 1 translates only row 0 while completing the frame.
This replaces the old immediate whole-line assertion while retaining its purpose.

`fixtures/lcd-output.asm` and `generated/lcd-output.ws` are original CC0-1.0 work.
The existing NASM/Python builder supplies the normal 128 KiB image, reset footer,
header, checksum and 32 KiB SRAM. It polls lines 40/144 to change shade/RGB12
values, overlaps Screen 2 and a sprite, and edits tile row 7 after row 39 was
composed. Native and production browser checks compare fifteen independent pixel
literals plus every pixel of nine complete output rows (0/38/39/40/46/47/48/142/143)
on every physical model and after restart. The cartridge writes fresh retained
boot counts, completed raster-pass counts, observed write lines 40/144 and 600D
completion markers into RAM/SRAM. Browser checks use the normal file chooser,
model/Restart controls, actual canvas and player SRAM flushes, with zero subsequent
network requests carrying ROM/save data. No framework, dependency, proprietary
asset, firmware, copied emulator code or game-specific path was introduced.

Cartridge SHA-256:
`0fc51374871dc198415692c7b86abe8947de574632416f4dd470afe0661138fe`.
All 25 existing cartridges and the demo remain byte-identical.

### Baseline failure demonstration actually executed

Exported the unchanged evaluation baseline to a temporary directory and added
only the new cartridge and native integration test. Ran the test separately on
each model; all exited 101 at output pixel (0,39):

| Model | Immediate baseline output | Required delayed output |
| --- | --- | --- |
| Mono | `[255,255,255,255]` | `[170,170,170,255]` |
| Color | `[0,255,0,255]` | `[255,0,255,255]` |
| Crystal | `[0,255,0,255]` | `[255,0,255,255]` |

The implemented pipeline passes those same cartridge vectors on all models.
The temporary baseline export was removed; the assigned worktree was not reset.
During focused test development, corrected two test-setup expectations: Mono
must not write color register 60, and the transparent half after the sprite ends
is x20 rather than x16. Neither requires weakening the delay requirement.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 26 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 110 core unit tests, 16 integration tests, one adapter unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass, zero warnings. |
| `git diff --check` | Pass; final record checked again before commit. |
| `npm run build:wasm` in `web` | Pass, using README Rustup PATH selection. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium suite, including complete-row output and fresh restart checks on all three models. |

`npm ci` restored locked dependencies without lockfile changes. Production preview
is `http://127.0.0.1:4297/wonderswan-rust/`, supplied through `WS_PREVIEW_URL`.
Both production builds finished before running the complete smoke suite, then
the complete compatibility suite sequentially. The complete compatibility suite
also passed a second run after strengthening the output probe from individual
pixels to complete-row checks.
No native production code changed after the production builds. Focused tests and
JavaScript syntax checks pass; all 27 relative documentation file/anchor links pass.

## Coordinator acceptance review

The implementation coordinator reviewed the code, independent unit/cartridge
vectors, assembly/image, browser checks, documentation and recorded results against
every Issue acceptance criterion:

- The source-established L→L−1 offset is implemented, with composition versus
  translation state explicitly reconciled before coding. Sampling/mode/backdrop/
  LCD/short-total uncertainties remain labeled policies, not invented precision.
- Mono across the family and all three supported color layouts preserve winning
  identities and priority across the delay; later map/tile/OAM changes cannot
  recompute pending output, and later final palettes affect the correct row.
- Independent literals distinguish composition and translation, check adjacent
  rows, backgrounds/layers/transparency/sprites and the final visible row. Actual
  unchanged-baseline runs fail the new delay vector on every physical model.
- Normal/short/odd/runtime totals, row 0/143, repeated frames, VBlank/OAM ordering,
  lifecycle clears/preservation, counters/IRQs/timers, actual completion and machine
  clocks are covered without changing device scheduling or existing overshoot.
- The original source-included CC0 cartridge executes through ordinary paths;
  native and actual production Wasm/player checks verify all models, restart,
  fresh CPU markers and independent rows. Ownership/provenance stays local.
- Complete native and production suites retain model gates, firmware-free startup,
  audio/PCM, GDMA/SDMA/Hyper Voice, SRAM/cartridge/internal EEPROM, browser
  persistence, input, orientation, frame continuity and invalid-load recovery.
- Required fixture/demo, format, workspace tests/lint, whitespace, production
  builds and sequential Chromium suites are recorded above. Documentation and
  stale comments describe the correction and its bounded limits. No unrelated
  subsystem, asset, dependency, framework or title-specific dispatch was added.

All mandatory checks passed, including the complete-row browser rerun. No
blocking review findings remain. No remote approval/merge action is requested or
performed.

Optional another-browser-engine, additional lawful ROM experiments, physical
console and commercial-game checks were not performed. Required post-merge
verification: none. No usage-limit reset, allowance purchase or provider/model
switch was performed.
