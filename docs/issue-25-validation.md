# Issue #25 validation and source-Issue review

Implemented [Issue #25](https://github.com/takahirox/wonderswan-rust/issues/25),
read from GitHub while open on 2026-10-23. The upstream readiness output supplied
no separate technical diagnosis. The source Issue and repository development and
review instructions define the scope. No remote write, push, PR, comment or merge
was performed by this implementation node.

## Behavior and assumption correction

The old loader required power-of-two file lengths, although size codes 05/07
already described 3/6 MiB, and suggested external leading padding that failed its
own exact declared-size check. Removing only that restriction would also leave
incorrect raw-length masking in ROM reads.

The corrected implementation accepts exact raw lengths 3,145,728/6,291,456 with
unchanged header/checksum validation. `Cartridge::rom` derives rounded capacity C
from raw length L, wraps addresses within C, returns FF below C−L, and indexes raw
bytes above that boundary. Raw bytes are never padded or changed. This one shared
map reaches CPU, reset startup, GDMA, SDMA and 2003 self-map consumers without
changing bank registers, A0 controls, timing, lifecycle or Wasm APIs. Power-of-two
images have a zero gap and retain their original mapping. Reported lengths and
SHA-256 browser save identities remain the exact raw image length/content.

Inspected references:

- [Wonderful linker documentation](https://wonderful.asie.pl/wiki/doku.php?id=wswan:guide:memory_management),
  bank index placement relative to the top of ROM (retrieved through the web tool).
- [Pinned ares loader](https://github.com/ares-emulator/ares/blob/64c835451157de74e418ca3fdbe1b5aec9972af9/ares/ws/cartridge/cartridge.cpp#L31-L38),
  rounded allocation and upper-end placement.
- [Pinned Beetle loader](https://github.com/libretro/beetle-wswan-libretro/blob/0db0832183ad264bbe67bcd7e651365e7dac5389/libretro.c#L554-L568),
  rounded allocation, upper-end placement and explicit FF gap fill.

Pinned raw files were inspected through direct HTTPS after web cache misses.
These corroborate placement policy; they do not establish electrical behavior
at every cartridge's unpopulated addresses. FF is an explicit emulation policy,
not electrical open-bus certification. No third-party emulator code was copied.
No correction beyond the source Issue's existing reconciliation was necessary;
no Issue edit or human approval was needed.

## Mandatory checks

| Check | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all sixteen images reproduce exactly; all twelve prior fixtures unchanged. |
| `python3 demo/build.py --check` | Pass: unchanged 131072-byte demo, SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 83 core unit tests, ten integration tests; Wasm crate and doc tests also pass. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in `web` | Pass with the existing Rustup toolchain and installed Wasm target. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass on isolated rerun: full three-model gameplay, input/lifecycle/recovery, dialogs/debug, audio and mobile suite. |
| `npm run test:compat` in `web` | Pass: full existing compatibility suite and all four raw CPU/Wasm fixtures on all three models. |

Both browser suites use this worktree's production preview at
`http://127.0.0.1:4198/wonderswan-rust/`, passed as `WS_PREVIEW_URL`.
`npm ci` restored locked frontend dependencies without lockfile changes. The first
Wasm invocation selected Homebrew Rust 1.94, which lacks its Wasm target. Selecting
the already installed Rustup toolchain through `/opt/homebrew/opt/rustup/bin`
resolved that build failure. The first gameplay smoke invocation, concurrent with
the compatibility suite, failed a score-tile assertion after timed key taps.
The compatibility suite completed successfully; the smoke suite then passed in
full when rerun alone, without code or assertion changes.
The initial focused native run also exposed a test setup error that attempted a
Color register write on Mono; the corrected vector explicitly checks Mono's DMA
rejection. No production capability gate was changed to accommodate a test.

## Source-Issue acceptance review

The implementation coordinator read the source Issue, full implementation and
fixture sources, diff and observed validation evidence under the repository review
guidelines. All mandatory checks pass; no blocking findings remain for the
requested scope. This is implementation-node review, not a GitHub PR review or merge.

| Criterion | Implementation and evidence |
| --- | --- |
| Exact validation and raw identity | New raw unit vectors test both sizes/interfaces, exact byte/header/checksum preservation, bad fields, truncation/extra bytes with valid final footers, padded mismatch, unsupported length/code, bounds and corruption. Prior field-specific, mapper-size and model validation still pass. Native integration asserts `load_rom` return length and `rom_len` after execution/restart/model changes. |
| Top alignment and mapper coverage | Generated distinct markers cover every bank across 1024 selections and rounded aliases, first/last populated bytes, bank edges/interiors, leading FF, C2/C3, C0 visible windows, 2003 high bits/low aliases and CE self-map reads. Existing power-of-two, large mapper and 64 MiB startup tests pass. |
| Native execution and lifecycle | Four original source-included raw probes execute the final-bank reset jump on Mono/Color/Crystal, assert first/next/interior/leading markers, increment SRAM and draw pass pixels. Native tests inspect RAM/CPU/save/header/length/pixels, restart and compatible model changes. Failure vectors preserve image, live A0/ports, CPU, RAM, cycles, SRAM and all three console EEPROM records across invalid replacements and incompatible changes. |
| DMA | Both Color models/interfaces/sizes transfer distinct populated bytes across C2:FFFE to C3:0000 via GDMA/SDMA. Unit vectors check every delivered byte, 7+2 GDMA clocks, A0 eligibility/rejection, exact 7/8 SDMA event costs, next addresses, remaining counts and stop flags. CPU probes check real byte I/O, copied RAM and final SDMA sample/counters. Mono rejects the hardware; existing A0, arbitration, routing, timing and audio acceptance remains passing. |
| Browser production flow | `raw-rom.mjs`, called by the existing compatibility suite, loads all four exact CC0 files through the local chooser on every model and observes self-checked pass pixels after restart. Independently computed raw SHA-256 keys hold isolated SRAM counters; padded-image keys stay absent. Renamed/same-name files, reload/reopen and separate seeded console records are checked. Local selection/execution/save operations make zero network requests. |
| Architecture, provenance and documentation | Only loader bounds diagnostic/shared ROM mapping changes production Rust behavior. The thin Wasm/browser storage code and firmware-free startup are unchanged. Fixture source/data are original CC0; previous fixtures/demo remain byte-identical. README, support matrix, mapping policy and fixture provenance describe raw sizes, correction, sources and remaining uncertainty. |

## Reproducible raw fixture identities

| Fixture | Bytes | SHA-256 |
| --- | --- | --- |
| `raw3-common.ws` | 3145728 | `7fbea90c2ee082a69ec1d986b60afc3d912a26c4ed998c899ad2380f3ff4bd7b` |
| `raw3-2003.ws` | 3145728 | `6ba4257ca8dec1700cfc8f11b93c5e5698ad2fea122428917705bbb4eb2b6fd5` |
| `raw6-common.ws` | 6291456 | `f7b1da9a518946eb8417633f476757d2417c0f238e91155d89a1deb24ae41e30` |
| `raw6-2003.ws` | 6291456 | `8a61deedf04ecb5f6a4e86f303a491c8cf570d3ef6a3881988c769cc06753c3d` |

## Limits and optional validation

No physical consoles, commercial ROMs, human gameplay, additional independent
software execution or other browser engines were used. Commercial-title
playability, electrical unpopulated-address behavior, RTC, wider mapper
classification, firmware authentication and the existing timing approximations
remain outside this change. Browser storage/audio/visibility failures in the
existing suite are injected; CPU execution, optimized Wasm, framebuffer, saves
and Web Audio observations are real. No usage reset, allowance purchase or
agent model/provider switch was performed.

## Required post-merge verification

None for Issue #25.
