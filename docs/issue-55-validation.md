# Issue #55 validation

Implemented on 2026-10-24 in the assigned GitWeave worktree, against evaluation
baseline `296ab20178ce8228581f64ea385d8b472b32b4fb` (upstream checkpoint
`b83262a9684cf8722c0b7dec282498a4b244a87d`). Issue #55 was read live with the
GitHub `github_fetch_issue` tool for `takahirox/wonderswan-rust`, Issue 55.
No upstream technical diagnosis was supplied. The [source reconciliation](conversion-flags.md)
records the old SZP-only assumption, corrected rules, primary manual and pinned
experiment sources, rationale and evidence limits. The raw report/assembly and
relevant manufacturer manual sections were inspected. The Issue already specifies
the corrected requirements; no specification correction or remote edit was needed.

## Baseline discrimination and harness corrections

The CPU tests and original CC0 cartridge were added before editing production
handlers. Against the old handlers:

| Actual command | Actual result |
| --- | --- |
| `cargo test -p ws-core --lib --locked conversion -- --nocapture` | Three failures, two passes (the existing decimal/string test and new zero-base AAM guard). AAM returned AX=0402, FLAGS=F813 instead of F002 for AL=2A, incoming F8D7. AAD returned AX=0010, FLAGS=F002 instead of F012 for AX=0106, incoming F002. Scheduler test also distinguished the AAM flags. |
| `cargo test -p ws-core --test conversion --locked -- --nocapture` | All three model tests failed. Each reached the first AAM vector in mono graphics, wrote AX=0402 / FLAGS=F813 / branch mask=0006 (CF and OF taken), then reported 0BAD with zero completed phases. Expected FLAGS=F002 and branch mask=0000. Later vectors, transitions and restart were not reached on baseline. |

During test authoring, an initial temporary slice of original AH failed Rust
compilation (E0716); binding the one-element array fixed that test-source issue.
An initial cartridge read port 60 on Mono, which correctly rejected the unsupported
input. The probe now avoids that port on Mono; the subsequent baseline run failed
on the intended conversion result on every model as recorded above. Neither
harness correction changed production behavior. A final fixture adjustment made
the second first-vector input F402, so that even the reproduction pair contrasts
set versus clear arithmetic flags while testing preserved DF. It leaves expected
outputs unchanged; the focused cartridge checks were rerun and passed.

After the handler correction, `cargo test -p ws-core --locked conversion --
--nocapture` passed both literal-vector tests, the normal scheduler test, the
zero-base guard, the existing decimal/string test, and all three cartridge tests.
The cartridge reported 600D on both boots of every model: one Mono phase and seven
Color/Crystal phases. Every one of 1145 RAM words and corresponding SRAM words was
compared to independent literals, including unused zero Mono phases. The final
cartridge run also passed after strengthening the first-vector input contrast.

## Required commands

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 34 source-included CC0 images reproducible; all 33 prior images unchanged. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 153 core unit tests, 30 core integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in web | Pass using installed Rustup stable and Wasm target. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including all three conversion models, mode transitions and both boots. |

`npm ci` restored locked dependencies without changing lockfiles. Wasm used
`PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`;
no toolchain/target install was needed. Both `node --check web/scripts/conversion.mjs`
and `node --check web/scripts/compat.mjs` passed. Both browser suites use
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/` and the production Vite
preview after both builds. The complete smoke suite exited zero before starting
the complete compatibility suite; both exited zero. The final fixture/native checks
were rerun after strengthening the first-vector input contrast and also passed.

## Acceptance mapping and review

| Issue acceptance | Implementation and evidence |
| --- | --- |
| Ordinary decoded conversions and state preservation | Only successful D4/D5 handlers change production behavior. AAM clears AF/CF/OF and keeps remainder SZP. AAD masks the product before the existing byte ADD helper, which sets all six flags and returns the zero-extended AL result. Unit tests assert unrelated registers/segments, full control/fixed flags, untouched RAM/ports/SRAM, two-byte consumption, MOV continuation, RAM and mapped-ROM fetch costs (including waits), execution components and no direct clock advance. |
| Independently derived literal vectors | 27 AX/full-FLAGS vectors cover both reproductions, all required immediate bases, AAM ignored AH and quotient/remainder sign/parity boundaries; AAD zero base, truncated product, carry, auxiliary carry, positive/negative signed overflow, zero, sign and parity. CPU tests use three incoming flag profiles and opposing AAM AH values. Expected words call no conversion or ALU helpers. |
| Models, modes, branches, faults and scheduling | Static vectors cover all nine supported model/mode combinations. Normal Machine run_frame tests cover runtime 00/80/00/C0/00/E0/00 transitions and shared clock/raster/frame/PCM progression with the unchanged 18/3 total RAM instruction costs. Cartridge Jcc observations check ZF/CF/OF/SF/PF. The zero-base AAM guard preserves AX and existing stacked full FLAGS/return address, IF/TF clearing and IRET continuation. Existing multiplication/division/shift/rotate/PUSH SP/prefix/interrupt tests remain unchanged and pass in the workspace suite. |
| Original cartridge, native/browser observations and fresh execution | Original CC0 assembly and generated image are integrated into build.py. All three native model tests compare actual CPU-written AX/full FLAGS/five branch outcomes to independent host literals, all seven color mode phases and both boots. Every boot clears volatile observations and invalidates retained success. Native and browser starts deliberately poison SRAM with an old 600D. Browser assertions use the production player and actual persisted SRAM, fresh boot count and real frame pixels, without exporting ROM/save data. |
| Required checks and prior acceptance | All required native/build checks and both complete production Chromium suites passed sequentially as recorded above. Existing startup/firmware-free handoff, video/input/frame, audio/PCM, GDMA/SDMA/Hyper Voice, RTC, SRAM/EEPROM, Wasm lifecycle, ROM identity and browser ownership/save-integrity tests are retained. |
| Documentation and limits | Hardware subset, source reconciliation, fixture provenance/hash/layout, and this actual validation record updated. Conversion hardware evidence remains bounded to inspected experiment sources and instruction-level inference across model/mode configurations. |

The new cartridge SHA-256 is
`633e232ac261dc80a584442873c8a9b738528d2453f00ec65794e37e05c8aa09`.
Final working-tree and staged whitespace checks passed. All 56 relative file links
in the four changed documents resolve, and all 33 prior images are byte-identical
to HEAD. Implementation review checked the diff against all seven pre-merge acceptance
checkboxes; downstream Coordinator review of the final artifact remains required
before merge. This node does not push, open PRs, merge or comment.

Optional baseline production-browser discrimination, additional engines, physical
console comparison, human gameplay and commercial software were not performed.
There are no independently inspected per-model silicon logs and no measured
commercial-title impact. Disputed exceptional conversion flags and exact timing
remain separate work. No usage resets, allowance purchases or model/provider
switches occurred. Required post-merge verification: none.
