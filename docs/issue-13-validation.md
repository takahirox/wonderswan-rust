# General DMA validation — Issue #13

Pre-merge checks performed locally on 2026-10-21 for
[Issue #13](https://github.com/takahirox/wonderswan-rust/issues/13).
All checks below passed. No push, PR, merge, issue comment or deployment was
performed as part of this implementation.

| Check | Result |
| --- | --- |
| `python3 demo/build.py --check` | Existing CC0 demo bytes unchanged |
| `python3 fixtures/build.py --check` | All six original CC0 fixtures reproduce; five existing probes unchanged |
| `cargo fmt --all -- --check` | Passed |
| `cargo test --workspace --locked` | 55 core unit tests and 6 integration tests passed; web/doc tests passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `npm ci` in `web` | Passed, lockfile unchanged |
| `npm run build:wasm` in `web` | Passed with rustup stable and wasm-pack 0.14.0; release Wasm compiled and optimized |
| `npm run build` in `web` | TypeScript checks and Vite production build passed |
| `npm run test:browser` in `web` | Production Chromium gameplay/UI/mobile smoke suite passed on all three models |
| `npm run test:compat` in `web` | Existing save/audio suite and new Color/Crystal GDMA fixture passed |
| `git diff --check` | Passed |

Browser checks used the production preview at
`http://127.0.0.1:4287/wonderswan-rust/`, selected with `WS_PREVIEW_URL`.
The Wasm build used rustup stable's `bin` directory first on `PATH`, as described
in the README; native checks used the existing system Cargo toolchain.

The ten GDMA unit tests cover byte register masks/readback and model/mode gating;
directional RAM overlap; current common/extended ROM banking; source/destination
width and wrap; zero length; SRAM, byte ROM, slow ROM and absent ROM rejection;
partial termination; exact setup/word costs; CPU fetch/IRQ stalling; maximum
transfer progression across a frame boundary; scanline/timer/IRQ/audio clocks;
SRAM preservation; and pending EEPROM programming completing during DMA.
The enabling `OUT` instruction keeps its normal cost, and DMA words add no CPU
byte-access charges.

The original [GDMA cartridge source](../fixtures/gdma.asm) runs actual CPU byte
`OUT`/`IN` instructions, asserts completion registers and RAM words for forward
and reverse transfers, then draws green on success (red on failure). Native
integration tests verify its RAM markers, transferred words and framebuffer on
Color/Crystal; mono skips the color-only ports. The browser compatibility suite
loads the exact generated file through the existing Wasm player's file picker
and observes green output and no execution error on both color models.
The existing demo gameplay and five save probes still pass native tests on all
three models. No frontend product behavior or core dependencies changed.

The fixture is CC0-1.0 and freely redistributable; no commercial ROM, proprietary
BIOS, copied emulator implementation or title-dependent behavior was introduced.
`fixtures/generated/gdma.ws` is 131072 bytes, SHA-256
`469685ed88a118d41afd327d6989fdd600f3d9d1d073754dc188dfdf19070640`.

Implementation references: [WSdev DMA](https://ws.nesdev.org/wiki/DMA),
[Timing](https://ws.nesdev.org/wiki/Timing),
[SoC color enable](https://ws.nesdev.org/wiki/SoC), and
[memory map](https://ws.nesdev.org/wiki/Memory_map).
DMA uses word granularity with setup grouped into the first word; see the
[support matrix and timing limitations](hardware-subset.md#general-dma-semantics).
Optional physical-console comparisons and independent external test vectors were
not performed. They are not pre-merge prerequisites.

## Required post-merge verification — pending

1. Confirm the existing `main` GitHub Pages workflow succeeds after merge; record
   its workflow run URL and deployed commit.
2. Load `fixtures/generated/gdma.ws` through the deployed site's file picker on
   Color and Crystal. Confirm the green success screen and no execution error.

Local production-preview checks do not establish either post-merge result.
