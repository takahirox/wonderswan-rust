# Issue #53 validation

Implementation and checks performed on 2026-10-24 in the assigned GitWeave
worktree, against evaluation baseline 75f0ebedc424a614660320d9f5a7696f1af1e0b6
(upstream checkpoint 247684c2873d10d72209a42ece5d3e471d1d8bfd).
Issue #53 was read live with
`gh issue view 53 --repo takahirox/wonderswan-rust --json number,title,body,url,state`.
No upstream diagnosis was supplied. The [source reconciliation](rtc-transport.md)
records the old assumptions, corrected requirements, pinned experimental and
library sources, manufacturer reset semantics and inference limits. Raw pinned
sources and the manufacturer PDF were inspected; no external implementation was
copied, and no change to the Issue's already-corrected requirements was necessary.

## Baseline discrimination

The register tests and original cartridge were added before production changes.
`cargo test -p ws-core --lib --locked rtc_ -- --nocapture` against the old
transport failed three tests and passed the existing GPO/invalid-control guard:

| Test | Actual baseline result |
| --- | --- |
| `rtc_polling_consumes_the_terminal_byte_once` | Count 2 versus independent expected 1 for 13. |
| `rtc_ready_bit_is_read_only_on_command_start` | Starting 93 returned `Unsupported RTC command 0x93.` |
| `rtc_all_commands_follow_independent_host_counts_and_status_vectors` | First mismatch at Mono/00/13: count 2 versus 1. Later vectors were not reached on baseline. |

`cargo test -p ws-core --test rtc --locked -- --nocapture` failed on all three
models with the same explicit 93 command error. The cartridge reached that
command in its first phase, before copying final observations to SRAM. Native
restart assertions were therefore not reached on baseline. No baseline
production-browser discrimination was performed.

After the production correction, all focused transport tests passed. During
harness development, two native frames allowed the Color/Crystal RAM result to
complete but stopped during the CPU's SRAM copy (word 424 still contained
poisoned 6D6D). The bounded native run was extended to four frames; no production
change was needed. An initial partition test drained the same DMA stall twice
while comparing three buses; capturing it once fixed that test-source error.

The focused run, `cargo test -p ws-core --locked rtc_ -- --nocapture`,
passed seven register/guard tests and all three cartridge tests. All models
reported 600D on both boots; Mono completed one phase and Color/Crystal seven.
Every one of 466 RAM words and corresponding SRAM words was checked against
independent literals, including unused zero Mono phases. The full workspace
suite also passed these tests.

## Mandatory commands

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 33 source-included CC0 images reproducible. The 32 prior images are byte-identical to HEAD. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 149 core unit tests, 27 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass, including final staged additions (`git diff --cached --check`). |
| `npm run build:wasm` in web | Pass using the installed Rustup stable toolchain and Wasm target. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including the new RTC CPU probe on all three models, runtime mode transitions and fresh restart. |

`npm ci` restored locked dependencies without lockfile changes. The Wasm build
used `PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`;
no toolchain/target installation or allowance change was needed. Both
`node --check web/scripts/rtc.mjs` and `node --check web/scripts/compat.mjs`
passed. Both browser suites use the production preview at
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/`, after both builds;
the complete smoke suite exited zero before compatibility started.

The new image SHA-256 is
`9877e29977646ec062e9e856a42d1da045fc8f505cc57ad795a480e6d5123251`.
Its [source, layout, ownership and provenance](../fixtures/README.md#rtc-transport-probe-issue-53)
are recorded with the existing fixtures.

## Acceptance mapping

| Issue requirement | Implementation and evidence |
| --- | --- |
| Correct supported CA/CB transfers | Reads update remaining count when a stage completes, so final READY exposes the last byte with ACTIVE clear. CB consumption schedules only a remaining byte. Writes retain preloading; all twelve host counts, terminal acknowledgment, command readback, absent FF and 93 masking have independent assertions. Only READY is masked; unsupported commands and common-mapper rejection remain explicit. |
| Independent register tests and baseline failures | Literal count/status vectors for 10–1B, both READY-start variants, consecutive commands and terminal settling on nine model/mode combinations. Separate read-overcount and 93 baseline failures above; corrected passes recorded. Both reset host directions count one without asserting writable chip payload. |
| Models, timing and reset integrity | Mono, Color and Crystal; all supported static modes and runtime mode transitions in the cartridge. Seven-stage read/write tests check 63 then 64 clocks, READY latching through long advances, whole/partitioned/single-clock equivalence of ports, RAM, timers, raster/frame counters, nonzero PCM and SDMA stalls. Actual CPU IN/OUT costs remain 4 clocks or 5 with cartridge I/O wait. Restart/model-change (including same-model) tests cancel newly scheduled, 63-clock, ready and terminal states, preserve all three console EEPROM records plus SRAM or cartridge EEPROM, and execute a fresh transaction. |
| Original CPU cartridge and fresh execution | Original CC0 source and generated cartridge integrated through build.py. Bounded normal IN/OUT polling checks all commands plus 93 in every runtime phase. CPU-written count/status/data observations are checked natively and by the browser harness. SRAM starts poisoned with stale 600D; ROM clears RAM and invalidates saved completion, retains only a boot counter, and records fresh phases. Both boots compare all 466 words on each model. |
| Required checks and existing acceptance | All required native/build commands and both complete production Chromium suites passed sequentially. Existing assertions are retained for CPU, models, startup, video/input/frame, audio/PCM, GDMA/SDMA/Hyper Voice, saves and browser lifecycle. |
| Documentation and limits | Hardware matrix, source reconciliation, original fixture provenance/hash/layout and this actual validation record updated. RTC attachment/detection, calendar progression, persistence, alarms and cartridge IRQ integration remain gaps. |

Implementation review checked the final change against all seven Issue criteria.
The complete smoke suite and then complete compatibility suite both exited zero;
all existing browser assertions were retained. Final native validation was repeated
after strengthening same-model cancellation and partitioned advances through a
full frame (58,254 clocks, one frame, raster line 68). Final working-tree and
staged whitespace checks passed. All 54 relative file links in the four changed
documents resolve, and all 32 prior fixture images match HEAD byte-for-byte.
The downstream Coordinator must review this change against every criterion before
merge; this node does not merge, push, open a PR or comment.

Optional original-console comparison, physical measurements, human gameplay,
commercial-title testing, additional browser engines and baseline production
Chromium discrimination were not performed. No silicon latency, equipped-RTC
support or broad commercial compatibility is claimed. No usage reset, allowance
purchase or model/provider switch occurred. Required post-merge verification:
none.
