# Issue #21 validation and source-Issue review

Implemented the ordinary internal console EEPROM milestone requested by
[Issue #21](https://github.com/takahirox/wonderswan-rust/issues/21), read from GitHub
while open on 2026-10-23. The supplied upstream readiness output had no technical
diagnosis; implementation followed the source Issue and inspected references.
The cartridge EEPROM controller, demo binaries, dependencies and deployment
configuration retain their existing behavior.

## Mandatory checks

| Check | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all eleven original CC0 fixture images reproduce exactly, including both new internal EEPROM probes. |
| `python3 demo/build.py --check` | Pass: unchanged 131072-byte demo, SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 75 core unit tests, eight integration tests; Wasm crate and doc tests also pass. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in `web` | Pass using the already installed Rustup stable toolchain and Wasm target. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass against this worktree's production preview: complete three-model demo, input, lifecycle, recovery, audio and mobile coverage. |
| `npm run test:compat` in `web` | Pass against production preview: existing saves/video/audio/GDMA/SDMA suites plus internal EEPROM CPU/Wasm, console lifecycle and storage-failure checks. |

Preview runs at `http://127.0.0.1:4189/wonderswan-rust/`, with that value supplied
through `WS_PREVIEW_URL` to both suites. The default port 4173 was occupied by
another service; the initial default-port smoke invocation failed before finding
the player. A separate strict-port preview was started for this worktree and the
suites passed there. The first Wasm build selected Homebrew Rust 1.94 without its
Wasm target; prepending `/opt/homebrew/opt/rustup/bin` to PATH selected the existing
Rustup stable 1.99 toolchain with `wasm32-unknown-unknown`, without installing or
switching an agent model/provider. No missing check is represented as a pass.

The first CPU integration assertion sampled frame zero before the probe had
configured its display. Running the next frame verifies its actual pass pixels;
all CPU markers and byte results are checked independently. This was a test
observation correction, not a change to hardware requirements. A later smoke
rerun exposed an existing asynchronous empty-ROM assertion race: the test read
the error before file read/SHA-256 completion. It now waits for the expected error
and retains its exact content assertion; both final browser suites pass with
that correction.

## Acceptance review

The implementation and validation were reviewed against every source-Issue
acceptance item and the repository review guidelines. No blocking findings remain
for the requested ordinary-transfer scope; this is implementation-node source
review, not a GitHub PR approval or merge.

| Source-Issue criterion | Implementation and evidence |
| --- | --- |
| BA–BE transfers and harmless BF; byte/word I/O; separate buffers | Dedicated `internal_eeprom.rs`, routed through `Bus`; byte-level register tests and original CPU `IN/OUT AX` probes on all three models. BF reads zero and ignores writes, including nonzero high bytes and pending transfers. |
| Physical capacities, little endian, command widths and mode changes | Independent 128/2048/2048-byte records; boundary-word read/write vectors, six-bit mono and ten-bit Color layouts; mode changes preserve allocation/bytes/latches. CPU probes read the final Color word through mode transitions. |
| Settings boundary, locking, protection/readback and header policy | WDS/WEN, byte 5E/60 boundary, repeated BE.7, protection while pending and across modes. Load/restart reapplies header protection and enables ordinary writes under a documented firmware-free handoff policy. Both header states are CPU-tested. |
| Bounded shared timing and explicit gaps | READ DONE clears then sets, READY progresses; captured read/write operands survive register/mode edits; failed imports/loads retain pending state. Shared-clock tests verify GDMA copy, SDMA source/length/last-byte, timer IRQ and audio progress while programming completes. Unsupported direct/bulk/mixed/invalid commands leave bytes unchanged and diagnose explicitly. |
| Core/Wasm API and lifecycle retention | Exact-length active-model import/export separate from cartridge saves; native tests cover reset/unload, restart, cartridge replacement, switching away/back, invalid ROM/import and incompatible models. Chromium directly instantiates the exact generated bindings/optimized Wasm to test capacities, failed length imports, cartridge-save independence and retained model records. |
| Browser versioned records and failure handling | Three `wonderswan:console:v1:<model>` slots, restored once before execution; flush before replacing model/ROM and on periodic/pause/hide/pagehide paths. Separate model-named console notice. Production tests disable periodic flush to prove lifecycle writes, inspect distinct model bytes, and preserve corrupt/wrong-length/denied records while cartridge storage keeps working. Quota errors do not stop CPU execution. No console upload requests. |
| Original licensed CPU probes and native/browser execution | `fixtures/internal-eeprom.asm` produces writable-settings and protected-header cartridges, both CC0. Native tests verify markers/pixels/settings/counters across all models and cross-cartridge/export/reopen paths. Production-preview Chromium uses the same images and verifies pass pixels plus storage bytes through cross-cartridge/model changes, reload and new-page reopen. |
| Existing functionality and all required commands | Full workspace checks and both existing Chromium suites pass, preserving gameplay, sound, DMA, SRAM/external EEPROM and browser lifecycle paths. |
| Documentation, provenance and scope review | README, support/API matrix and fixture provenance updated; ordinary transfers remain separate from unresolved direct erase/bulk/silicon details. From-scratch core, thin Wasm adapter, no proprietary software, identity/title-specific behavior or settings-editor UI. |

## References, limits and optional work

The pinned libws header/handle implementation and public experiment source are
linked in [the support document](hardware-subset.md#independent-hardware-references-and-validation).
GitHub's contents API supplied the libws and experiment files after raw-URL web
fetches were unavailable; the rendered libws register definitions were readable.
These are inspected software definitions and experiment vectors, not physical
hardware results. No third-party implementation was copied into the core or CC0
fixtures, and no emulator implementation was used. No primary specification
correction requiring an Issue update was identified.

READ uses 32 clocks, WRITE/WEN/WDS use 3072 clocks as explicit approximations,
including short control commands. BF and reserved BE low bits have deterministic
policies. Direct ERASE, WRAL/ERAL, serial command adaptation, invalid-command
silicon responses, exact protection races and measured busy/DONE timings remain
hardware gaps. Initial owner/settings bytes are synthetic, not factory-accurate.
Pending transfers are discarded on lifecycle reconstruction or successful
administrative import; only completed data is retained. Multi-tab arbitration
and a settings/import/export UI are outside scope.

Storage restrictions, pagehide/visibility events and audio-policy failures in the
browser suites are injected. Cartridge instructions and Wasm execution, pixels,
localStorage records, Web Audio buffers and analyser results are real. Physical
consoles, commercial ROMs, additional independent software runs, human gameplay
and other browser engines were optional and were not used.

## Required post-merge verification

None for Issue #21. No push, PR, merge, Issue comment or deployment was performed
by this implementation node.
