# Issue #23 validation and source-Issue review

Implemented [Issue #23](https://github.com/takahirox/wonderswan-rust/issues/23),
read from GitHub while open on 2026-10-23. The upstream readiness output supplied
no technical diagnosis; the implementation follows the source Issue, inspected
references and repository development/review instructions. This work is bounded
to live A0 state, ROM accounting, existing DMA consumers and regression evidence.
The Wasm adapter remains thin and needs no API changes.

## Mandatory checks

| Check | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all twelve CC0 images reproduce exactly; eleven existing images unchanged. New `a0.ws` SHA-256: `08133afcb3b2086674a9d67fd916e4b1e80156001ae486571d651bc8020fcc79`. |
| `python3 demo/build.py --check` | Pass: unchanged 131072-byte demo, SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 79 core unit tests, nine integration tests; Wasm crate and doc tests also pass. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in `web` | Pass using the existing Rustup toolchain/installed Wasm target. |
| `npm run build` in `web` | Pass: TypeScript check and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete three-model demo/gameplay, input, lifecycle, recovery, audio and mobile suite against this worktree's production preview. |
| `npm run test:compat` in `web` | Pass: existing video/save/audio/GDMA/SDMA/console EEPROM coverage plus the new three-model A0 CPU probe and restart checks against production preview. |

Preview used `http://127.0.0.1:4197/wonderswan-rust/`, supplied as
`WS_PREVIEW_URL` to both Chromium suites. `npm ci` restored the locked frontend
dependencies without changing the lockfile. The first Wasm invocation selected
Homebrew Rust 1.94, which lacks its Wasm target. Prepending the already installed
`/opt/homebrew/opt/rustup/bin` to PATH selected the existing Rustup toolchain and
Wasm target; the required build then passed. No agent model/provider change,
usage reset or allowance purchase was made. No skipped check is reported as passed.

## Acceptance review

The coordinator reviewed the source Issue, complete implementation diff, new
source-included fixture and observed validation results under the repository's
review guidelines. No blocking findings remain for the requested scope. This is
implementation-node source review, not a GitHub PR review, approval or merge.

| Source-Issue criterion | Implementation and evidence |
| --- | --- |
| Live byte A0 on all three models | `Bus` stores A0 with writable width/wait, sticky lock, immutable identification/status, and reserved bits 4–6 read zero. Unit vectors cover every header combination, zero/all-one/repeated/alternating writes, lock clear attempts and the unlocked-to-locked transition. CPU probes execute actual OUT/IN on all models. |
| Synthetic initialization and unchanged metadata | Load/restart/model change seed width/wait from header flags; new/reset bus keeps the prior locked/status policy. Lifecycle tests cover replacement, all four flag combinations, restart, every model, failed/malformed/incompatible loads and incompatible model change. A0 writes preserve header metadata and ROM bytes. |
| CPU ROM costs and boundaries | `access_time` uses live A0.3; `word_transfer` uses A0.2 and canonical aligned adjacency. CPU tests assert byte/word costs through instruction operand fetch and data-read helpers, real MOV immediate execution, odd addresses, segment/20-bit wrap and differently mapped bank windows. Existing opcode wait test now uses OUT instead of mutating a header. Instruction/prefetch approximations remain explicit. |
| GDMA live eligibility and preserved guarantees | Every bounded ROM word checks live fast/16-bit controls; rejection tests use A0 writes. Color/Crystal partial-transfer tests change width/wait at the bus boundary, preserving copied bytes, next addresses and remaining count without normal access charges. Existing matching-width banked/extended ROM, SRAM/flash rejection, direction/overlap/alignment/wrap, zero/absent source, maximum transfer, CPU/IRQ stall, EEPROM/audio/timer and `5 + 2 × words` tests pass. Mono retains its capability gate. |
| SDMA live costs and routing | Both Color models and both Channel 2/Hyper Voice targets transfer distinct ROM bytes as A0 wait changes during the active transfer. Each event checks phase, exact 7/8-clock reservation, source/length, delivered bytes, stop state and zero CPU memory charging. Existing SRAM waits, bank/wrap, all rates, repeat/hold/shadows, cadence/phase, PCM and arbitration tests pass. Shared internal EEPROM/timer/audio/GDMA tests additionally run with a ROM SDMA source and live wait edits. |
| Original CPU probes, native and Chromium/Wasm | `fixtures/a0.asm` is original CC0 source, built reproducibly into `a0.ws`. Control edits run from RAM; matched-width aligned/odd ROM reads and GDMA acceptance/rejection are self-checked. SDMA polling changes wait between two samples while active for both destinations. Native tests inspect RAM markers, bytes, metadata and pixels across model changes/restarts. The production compatibility suite loads the exact same image through the player on all three models and checks pass pixels after restart. |
| Regressions and mandatory commands | Every required command above passes, including the unchanged demo and full existing gameplay, audio, DMA, SRAM/cartridge EEPROM and console-persistence suites. No proprietary software, title-specific dispatch, dependencies or architectural subsystems were added. |
| Documentation, provenance and uncertainty | README, support/startup policies and fixture provenance describe live A0, metadata separation, reset semantics and hardware gaps. This report records sources, old/corrected assumptions, validation and source review. |

## Evidence and assumption correction

The old implementation treated A0 as constant 85/87 and used immutable header
flags as runtime bus controls. The corrected requirement is that header flags
seed the existing synthetic handoff, then A0 writes control ROM width/wait and
applicable DMA behavior without changing the physical cartridge metadata.

Inspected sources, linked in the
[support document](hardware-subset.md#independent-hardware-references-and-validation):

- Pinned Wonderful hardware definitions, `soc.xml`: A0.2 ROM 16-bit transfer,
  A0.3 additional wait clock, bit 0 IPL lock, bit 1 model and bit 7 status.
- Pinned WSHWTest assembly, lines 1270–1295: retained lock/model/status and
  writable controls with reserved-bit masking.
- Pinned independent GDMA alignment/access experiment: toggling A0 wait rejects
  slow ROM with the length unchanged.
- Pinned independent SDMA experiment: ROM source succeeds with either wait setting.
- NEC V30MZ hardware manual, Table 2-1 and §§4.1–4.3: aligned/odd word bus cycles
  and extended waits. It also describes prefetch, which this milestone does not
  implement. Console-specific A0 behavior comes from the preceding definitions
  and experiments, not from the generic processor manual alone.

Raw files were inspected via direct HTTPS after the web fetch cache missed;
the official Renesas PDF was readable through the web tool. References are
specification/experiment evidence, not physical-console pass logs. No third-party
experiment or emulator code was copied into the original core/fixtures. No new
specification correction beyond the source Issue's existing correction was needed;
no Issue edit or other remote write was performed.

## Remaining limits and optional work

Width mismatches keep the existing deterministic ROM byte mapping, with live
controls governing accounting and GDMA eligibility. Accounting unit vectors may
exercise that deterministic mapping, but they do not establish hardware data
accuracy under a mismatch. The new cartridge probe avoids reading mismatched ROM
entirely by running control edits from RAM and restoring 16-bit mode before ROM
reads. Exact mismatched-width data, electrical open bus, prefetch, sub-cycle bus
phases and unrelated RAM/I/O timing remain unresolved. Handoff/status values are
synthetic, not authentic power-on or post-firmware state; IPL mapping,
authentication and firmware execution remain absent.

Physical consoles, commercial ROMs, human gameplay, additional independent
experiment execution and other browser engines were optional and not used.
Storage/audio-policy/visibility failures in existing browser checks are injected;
CPU execution, production Wasm, framebuffer output, localStorage and Web Audio
buffers/analyser observations are real. WSdev/STSWS were not consulted; their
inaccessibility was already recorded in the source Issue.

## Required post-merge verification

None for Issue #23. No push, pull request, merge, Issue comment or deployment was
performed by this implementation node.
