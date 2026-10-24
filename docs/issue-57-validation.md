# Issue #57 validation

Implemented on 2026-10-24 in the assigned GitWeave worktree, against evaluation
baseline `31862d19b85da3204b2afa90bee04b7d070625cc` and upstream checkpoint
`0176479d01c2c39f8a57f6b8ec7ad88d30b2ed56`. Issue #57 was read live using
`gh issue view 57 --repo takahirox/wonderswan-rust --json number,title,body,comments`.
No upstream technical diagnosis was supplied. The [source reconciliation](ascii-adjust.md) records the
original assumptions, corrected requirements, pinned experiment sources, primary
manual timing reference and rationale. The pinned raw report and assembly and
relevant manufacturer manual sections were inspected. The Issue already specifies
the corrected behavior; no technical specification correction or remote mutation
was needed.

## Baseline failures and resolution

New CPU tests and the extended original CC0 conversion cartridge were run before
changing either production handler.

| Actual command | Observed result against old handlers |
| --- | --- |
| `cargo test -p ws-core --lib --locked ascii_adjust -- --nocapture` | All five added tests failed. AAA with AX=00FA / FLAGS=F002 returned AX=0200 instead of 0100. AAS with AX=0100 / FLAGS=F012 returned AX=FF0A instead of 000A. The exhaustive and prefix tests returned 2 versus 10, and 4 versus 12 clocks. The scheduler overran the expected frame boundary by two clocks after executing extra instructions. |
| `cargo test -p ws-core --test conversion --locked -- --nocapture` | All three models failed at the first AAA case in graphics mode 00 on boot 1: actual AX=0200 / FLAGS=F013 / branch mask=0002, expected 0100 / F057 / 0013. Each wrote 0BAD and zero completed phases. Later vectors, modes and restart were not reached on baseline. |

After the handler correction, the five focused CPU tests passed. The exhaustive
literal-table test covered 73,728 decoded AAA/AAS instructions. The literal tests
covered another 1,728 executions across nine static configurations, three incoming
flag profiles and RAM/ROM with wait controls disabled/enabled. Normal Machine
scheduling covered 240 instructions across Mono and both seven-phase color runtime
round trips, observing returned clocks, raster/frame wrap, one timer decrement and
one stereo PCM frame. Prefix and INT3/IRET tests passed in all nine configurations.

The corrected cartridge passed on both boots of every model: one Mono phase and
seven Color/Crystal phases per boot, reporting 600D. All 1,817 RAM words and all
corresponding SRAM words were compared with host literals, including zero unused
Mono phases. First observed AX/FLAGS/branches were 0100/F057/0013. Each phase
contains 16 new AAA/AAS and all 27 retained AAM/AAD vectors, each under contrasting
incoming flags. Every boot clears volatile observations and invalidates retained
success. Host tests poison SRAM with an old success marker before first boot.

The first required Clippy run rejected an indexed AH-table test loop with
`clippy::needless_range_loop`. Iterating the paired original/expected AH rows
fixed it without changing expectations or production behavior. Clippy was rerun
and passed; the complete workspace suite was rerun against the final test source.
A documentation-link edit initially used a path relative to `web` and failed
before any write; rerunning it from the repository root fixed the link.

## Required commands and final revision

| Command | Actual final result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 34 images reproducible. The 33 images other than conversion.ws are byte-identical to upstream HEAD. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 158 core unit tests, 30 core integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass after the test-loop fix above. |
| `git diff --check` | Pass; repeated after final documentation edits. |
| `npm run build:wasm` in web | Pass using existing Rustup stable and installed Wasm target. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including the final AAA/AAS fixture and assertions on all three models, mode transitions and both boots. |

`npm ci` restored locked dependencies without changing lockfiles. Wasm used
`PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`;
no toolchain or target installation was needed. `node --check
web/scripts/conversion.mjs` passed. The production preview used
`npm run preview -- --port 4199 --strictPort` with
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/`. The complete smoke suite
exited zero before the compatibility suite began; both complete suites exited
zero. Builds used the final handler;
no production, fixture or browser-test edits followed those builds.

The following Git blob IDs identify the exact final functional revision covered
by these checks, independently of subsequent validation-document edits:

| File | Git blob ID |
| --- | --- |
| crates/ws-core/src/cpu.rs | af5745c808d076e0c678ee6b4059c103123306d7 |
| crates/ws-core/src/cpu/ascii_adjust_tests.rs | ebe1595c5bae80e6d76cf463077fcdfbb002a29b |
| fixtures/conversion.asm | 4bd751a4d56739163bc3b8391c9aacc682de320f |
| fixtures/generated/conversion.ws | f28ee61e9be59219027615fef7cf2f7ba06e37ef |
| crates/ws-core/tests/conversion.rs | 34f7e2d97ed6ddb532cac32b91f6492ebe3c5746 |
| web/scripts/conversion.mjs | b0379805a056c7daf6f85bef864fd375f6ed57f9 |

The cartridge SHA-256 is
`19f0cb8de32e02179dbbba86043275352780529761402820a102fc299de5ad9f`.
Focused baseline and corrected runs used the same final fixture/browser literals.
Only the native exhaustive loop's iteration form changed afterward for Clippy;
the final workspace run covers that revision.

## Acceptance review and limits

| Issue checkbox | Implementation and executed evidence |
| --- | --- |
| Ordinary decoding and state | Production changes are confined to 37/3F. Literal tests check independent byte wrapping, complete FLAGS, one-byte consumption, MOV continuation and unchanged other registers, segments, RAM, ports and SRAM. Prefix policy and INT3 stack/IRET continuation are checked explicitly. |
| Independent vectors and branches | All Issue reproductions and boundary cases have literal AX/full-FLAGS expectations. Documented literal nibble/AH tables supply exhaustive AL/AF/AH coverage with opposing arithmetic and control flags, without production helpers. Actual cartridge Jcc observations check ZF/CF/OF/SF/PF, including constant PF/cleared OF and ZF/SF unrelated to result zero/sign. |
| Models, modes and timing | Nine static configurations; runtime 00/80/00/C0/00/E0/00 on Color and Crystal; RAM/ROM with waits enabled/disabled; exact nine-clock execution plus actual fetch charges (10 or 11 total); normal Machine timer/raster/PCM advancement once. Existing DMA timing tests remain unchanged. |
| Original cartridge and browser | Source-included CC0 probe extended through existing build.py/NASM pipeline. Native and production Wasm check CPU-written complete observations on all models, modes and both boots with poisoned retained success. ROM identity, browser-local ownership and persistence policy are unchanged. Baseline discrimination and corrected configurations reached are recorded above. |
| Required checks | The required native/build checks and complete sequential production Chromium suites are recorded above against the final functional file revisions. |
| Retained regressions | Complete native and production browser suites retain AAM/AAD, multiplication, division, shift/rotate, PUSH SP, BUSLOCK, interrupts, three-model startup, video/input/frame, audio/PCM, GDMA/SDMA/Hyper Voice, RTC, SRAM/EEPROM and lifecycle/save-integrity coverage. |
| Documentation and provenance | Hardware subset, source reconciliation, independent oracle, fixture hash/layout/provenance, link to prior conversion evidence and this execution record updated. |

Implementation self-review compared the final diff against all seven pre-merge
checkboxes. Tests present and checks
actually executed are distinguished above. Downstream Coordinator review of the
final checkpoint remains required before merge. All 61 relative file links in
the five changed documents resolve; final unstaged/staged whitespace checks
passed. All 54 prior AAM/AAD probe inputs and native/browser expected triples
were compared to upstream HEAD and are unchanged. This node does not push, open a
PR, merge or comment.

Optional baseline production-browser discrimination, additional browser engines,
physical-console measurements, human gameplay and commercial-software testing
were not performed. No independent per-model silicon logs were inspected and no
commercial-title impact was measured. Exact pipeline timing, DAA/DAS and broader
exceptional arithmetic remain separate work. No proprietary ROM or firmware was
required. No usage-limit resets, allowance purchases or model/provider switches
occurred. Required post-merge verification: none.
