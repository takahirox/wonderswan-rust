# Issue #51 validation

Implementation and checks performed on 2026-10-24 in the assigned GitWeave
worktree, against evaluation baseline 9b98c69f59b736e4c31df5414844466e8ba3a099.
The Issue was read live through
`gh issue view 51 --repo takahirox/wonderswan-rust --json number,title,body,state`;
no diagnosis was provided upstream. The [correction record](push-sp.md) distinguishes the old
sampling assumption, corrected requirement, primary encoding reference,
experimental evidence and inference limits.

## Actual baseline discrimination

Tests and the original cartridge were added before changing production decoding.
The first `cargo test -p ws-core --locked push_sp -- --nocapture` attempt stopped
at E0689 in the new guard test: the literal expected value needed an explicit
`u16` annotation before `to_le_bytes`. That test-source error was corrected
before measuring behavior.

`cargo test -p ws-core --lib --locked cpu::push_sp_tests -- --nocapture` then
ran four focused tests against the original decoder: both encoding-vector tests
failed, and the other-push/PUSHA guard and initial scheduler test passed. Each
encoding-vector failure reached the first Mono/mono, SP=2000, FLAGS=F002,
unprefixed case and reported actual bytes `[00,20]` versus `[FE,1F]`. Later
vectors in those two failing tests were not reached. The scheduler test was
subsequently strengthened to assert actual written bytes as well as its existing
clock, frame, sample and CPU-state assertions.

`cargo test -p ws-core --test push_sp --locked -- --nocapture` failed all three
model tests. Actual first-boot diagnostics:

| Model | Result | Completed phases | 54 stored/popped | FF F4 stored/popped |
| --- | --- | --- | --- | --- |
| Mono | 0BAD | 1 | 2000 / 2000 | 2000 / 2000 |
| Color | 0BAD | 7 | 2000 / 2000 | 2000 / 2000 |
| Crystal | 0BAD | 7 | 2000 / 2000 | 2000 / 2000 |

The cartridge deliberately continues after comparisons fail, so both encodings,
all four entry-SP vectors, both flag patterns and every phase executed before
these failures. Color/Crystal reached 00 → 80 → 00 → C0 → 00 → E0 → 00; Mono
reached only its supported mono phase. The test assertions failed on first-boot
result word 0; restart assertions were not reached on baseline. No baseline
Chromium run was performed.

After the operand-identity correction,
`cargo test -p ws-core --locked push_sp -- --nocapture` passed all four focused
unit tests and all three cartridge tests. Every model reported 600D and both
encodings stored/popped 1FFE on the first vector, on both boots. The independent
checks compare all 571 RAM words and all 571 corresponding SRAM words for each
model and boot, including unused zero observations on Mono. Existing 31 fixtures
remain byte-identical; only the new push-sp.ws image was added.

## Mandatory commands

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 32 original CC0 cartridges reproducible. New image SHA-256 92d9347a60409dd83e7eeef7a3d6b15c448f0d199768991c024956d802373c4d. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 143 core unit tests, 24 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass, including final documentation and staged additions (`git diff --cached --check`). |
| `npm run build:wasm` in web | Pass, using the installed Rustup stable toolchain and Wasm target. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including both PUSH SP encodings on every model and fresh restart. |

`npm ci` restored locked dependencies without modifying the lockfile. The Wasm
build used `PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`
to select the installed toolchain; no target/toolchain installation was needed.
Both JavaScript syntax checks (`node --check web/scripts/push-sp.mjs` and
`node --check web/scripts/compat.mjs`) passed. Production preview uses
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/`. Browser suites run
sequentially, smoke before compatibility, after both production builds.

## Acceptance review

The implementation coordinator reviewed the final staged diff against every
Issue checkbox. All seven criteria are satisfied by the implementation and
executed acceptance above. The complete smoke suite exited zero before the
complete compatibility suite started; compatibility also exited zero. Its PUSH
SP probe compared every CPU-written SRAM observation on all models and both
boots, including every supported Color/Crystal graphics mode and returns to
mono, with poisoned stale-success saves. No prior browser assertions were removed.

Final whitespace checks passed for tracked changes and staged additions. A
bounded local script verified all 51 relative file links in the changed docs
and compared all 31 existing fixture images against HEAD byte-for-byte.

The final acceptance mapping is:

| Acceptance | Implementation/evidence |
| --- | --- |
| Both encodings and correction | Shared operand-identity handling preserves the existing SS word-write helper and other value-based push callers. Correction, sources and inference limits in push-sp.md. |
| Literal vectors and state | Both encodings, four SP vectors, mapped wrap, exact little-endian bytes, final SP, subsequent POP AX, continuation/IP, contrasting FLAGS, unrelated registers/segments and every other RAM byte. |
| SS and other stack sources | ES override with distinct segments, non-SP registers equal to entry SP, DS/ES/BP memory values equal to entry SP, destination alias, immediates/segments, PUSHA saved entry SP at SS:1FF6/final 1FF0, poisoned saved-SP POPA slot. Prior POP, calls/returns and interrupt tests retained and pass. |
| Models, modes, accounting and scheduling | Nine static model/mode combinations; live 00/80/00/C0/00/E0/00 sequence on Color/Crystal; literal 3/4 and 4/5 costs; ordinary run_frame clock/frame/sample checks with actual written bytes. |
| Cartridge, fresh execution and discrimination | New original CC0 source/generated image; both encodings and every vector/flag pattern in all runtime phases; clear volatile observations, invalidate saved completion, fresh boot/phase markers, poisoned stale-success saves. Actual pre-fix failures and corrected native passes recorded above; complete production Chromium/Wasm suite also passed all observations on every model and both boots. |
| Required checks and prior acceptance | Native/build checks above retain all existing CPU, three-model, startup, video/input/frame, audio/PCM, DMA/Hyper Voice and save tests. Both complete production Chromium suites passed sequentially, including CPU, three-model startup, video/input/frame, audio/PCM, GDMA/SDMA/Hyper Voice, SRAM/EEPROM and browser save-integrity assertions. |
| Documentation/provenance/final review | Hardware subset, source reconciliation, fixture provenance/layout/hash and command evidence included. Final whitespace checks passed for working-tree and staged changes; all 51 relative file links resolve and all 31 prior images match HEAD byte-for-byte. |

Physical-console comparisons, independently inspected per-model silicon logs,
human gameplay, commercial-title tests, additional browser engines and optional
baseline production-Chromium discrimination were not performed. No commercial
compatibility or absolute hardware timing claim is made. No usage-limit reset,
allowance purchase or model/provider switch occurred. Required post-merge
verification: none. This node does not push, open a PR, merge or comment.
