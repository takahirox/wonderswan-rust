# Issue #59 validation

Implemented on 2026-10-24 in the assigned GitWeave worktree, against evaluation
baseline `da4b5ecb2f2c72c779b033e08b4946c628a28b30` and upstream checkpoint
`127ed10366b4a5397d3aba74be1b16199c119dd8`. Read the live Issue using
`gh issue view 59 --repo takahirox/wonderswan-rust --json number,title,body,comments`.
The upstream input contained the Issue snapshot, without a separate technical
diagnosis. [Source reconciliation](decimal-adjust.md) records the original
assumptions, corrected requirements, pinned experiment calculators and primary
manual timing reference. The raw pinned report and assembly and relevant manual
sections were inspected. The Issue already specifies these corrected rules;
no remote specification edit was needed.

## Baseline discrimination and corrected behavior

The new CPU tests and extended source-included CC0 cartridge were executed before
changing the production handlers. After the initial tests stopped at timing,
the focused literal vectors were reordered to expose the flag discrepancies
before clock assertions; the unchanged handlers were tested again.

| Actual command | Observed old-handler result |
| --- | --- |
| `cargo test -p ws-core --lib --locked decimal_adjust -- --nocapture` | Five tests failed. DAA 007A/F002 returned F092 versus expected F892. DAS 0000/F012 returned F097 versus F096. The exhaustive test returned 2 versus 11 clocks; prefixed execution returned 4 versus 13. Normal scheduling executed additional instructions and reached cycle 40705 versus expected 40704. |
| `cargo test -p ws-core --test conversion --locked -- --nocapture` | All three models failed at the first DAA vector on boot 1, mode 00: AX=0080 / FLAGS=F092 / branch mask=0008, versus expected 0080/F892/000C. Every model reported 0BAD, zero completed phases. Later vectors, runtime transitions and restart were not reached on baseline. |

After correction all six final CPU tests passed. The exhaustive independent
literal table covers 73,728 decoded instructions, all AL values, four incoming
AF/CF combinations, opposing OF/SZP/control profiles and AH 00/AB in all nine
static model/mode configurations. The separate 37-row literal suite covers all
three exact Issue reproductions and specified boundaries, with 3,996 executions
across flag profiles and RAM/ROM with waits disabled/enabled. Normal scheduling
checks 555 vector executions across Mono and both seven-phase runtime round trips,
observing one timer decrement, frame/raster wrap and one stereo PCM frame.
Separate tests cover GDMA charging/gating and prefix/INT3/IRET continuation.

The cartridge passed on both boots of every model: Mono completed one phase,
Color/Crystal seven phases (00/80/00/C0/00/E0/00), reporting 600D each time.
First AX/FLAGS/branches were 0080/F892/000C. Every phase executes 36 DAA/DAS,
16 AAA/AAS and 27 AAM/AAD vectors under contrasting status (158 conversions).
Native and production browser checks compare all 3,329 CPU-written RAM/SRAM
observation words or persisted SRAM words respectively to host literals, including
zero unused Mono phases. Every boot clears volatile observations and invalidates
retained success; both hosts poison retained observations before boot 1. A fresh
restart must produce boot count 2 and all fresh observations.

## Validation failures resolved

The first corrected cartridge run used the previous four-frame native budget:
Mono passed, while Color/Crystal were still running with six phases complete and
no success marker. Increasing the bounded budget resolved completion. An attempted
early exit on HALT then checked pass pixels before the next full raster had drawn
them (white pixels on all three models); the final harness runs eight full frames
before all original pixel/state assertions. No assertion was removed.

The added GDMA test initially assumed DMA was active in mode 00 and expected
18 clocks there. Existing GDMA gating disables it in that mode, so extra CPU
instructions crossed the frame boundary at cycle 40707 instead of 40704. The
final test preserves that policy: 11 clocks and no transfer in mode 00; otherwise
7 separate GDMA clocks plus 11 instruction clocks, one transferred word and a
zero remaining length. Production DMA code is unchanged.

The first `npm run build:wasm` failed because PATH selected Homebrew Rust without
a Wasm target. Selecting the already installed Rustup stable toolchain and target
resolved it; no toolchain or target installation was needed. A GitHub HTML source
fetch returned a cache miss; the pinned raw report and assembly were retrieved
and inspected successfully.

## Required checks and final functional revision

| Command | Actual final result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 34 images reproducible. All 33 images other than conversion.ws are byte-identical to evaluation baseline. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 164 core unit tests, 30 core integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass; repeated after final documentation edits. |
| `npm run build:wasm` in web | Pass after selecting installed Rustup stable as above. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including DAA/DAS observations on every model, required runtime phase and both boots. |

`npm ci` restored locked dependencies without changing lockfiles. Wasm used
`PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`.
`node --check web/scripts/conversion.mjs` passed. The preview command was
`npm run preview -- --port 4199 --strictPort`, with
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/` for browser checks.
The complete smoke suite exited zero before the compatibility suite began;
the complete compatibility suite also exited zero.
No production, fixture or browser-test edits followed those builds.

The following Git blob IDs identify the exact functional revision covered by
these results, independently of subsequent validation-document edits:

| File | Git blob ID |
| --- | --- |
| crates/ws-core/src/cpu.rs | 37177293cb3975265edb998db59284508d17d49a |
| crates/ws-core/src/cpu/decimal_adjust_tests.rs | 20bbd8b7bf42d64f0fafa1b69e100295aab0e334 |
| crates/ws-core/src/cpu/decimal_adjust_vectors.txt | 09324de2f101804577cf98692a536b01997d2a46 |
| fixtures/conversion.asm | a71b73bf827bc59f5181a7c23495385669d0585a |
| fixtures/generated/conversion.ws | ba7c6ac27902716dc0ce8a623abcd4379a7a198e |
| crates/ws-core/tests/conversion.rs | d137a2dd77c93fb7f473e01e9dfc9d2255590c12 |
| web/scripts/conversion.mjs | d444cd9cb3a7daddd570e9129a0ed0cb3ff2d630 |

The cartridge SHA-256 is
`4fd104a7698ce2d9fcfa697610aecea4c2c3476dfffc91bfa46a2b9dcdc8b1ad`.
The final workspace suite covers the strengthened prefix/DMA tests and eight-frame
native harness; baseline evidence predates those harness-only refinements.

## Acceptance self-review and limits

| Issue checkbox | Implementation and executed evidence |
| --- | --- |
| Ordinary decode and state | Production changes confined to 27/2F: CF equals high adjustment, OF derives from byte CMP final/original, ten execution clocks. Existing correct AL adjustment, AF and SZP behavior retained. Tests assert one-byte consumption, MOV continuation, AH preservation, full FLAGS and untouched unrelated registers/segments/RAM/ports/SRAM. Adjusted full FLAGS survive prefix decode, INT3 saved stack, IRET and HALT continuation. |
| Independent literals and branches | All reproductions and requested boundaries have literal AX/full FLAGS. The documented 2,048-row original table supplies exhaustive AL/AF/CF coverage with no production helpers. The original cartridge records real ZF/CF/OF/SF/PF branches, with independent native/browser literals. |
| Models, modes and clocks | Nine static configurations, seven-phase Color/Crystal runtime round trips, all four RAM/ROM wait configurations. Ten execution clocks plus actual fetch charges (11/12 total); normal timer/raster/frame/PCM scheduling once. Separate GDMA charge and mode gating tested; existing DMA regressions retained. |
| Cartridge and browser | Existing CC0 NASM/build.py pipeline, original fixture extended. All model/mode/boot observations checked, volatile block cleared and poisoned retained success invalidated. ROM SHA-256 identity, local ownership and transport policy retained. Baseline failures and configurations reached distinguished above. |
| Required checks | Native/build and complete sequential production Chromium results are recorded above against final functional blobs. |
| Prior regressions | Complete native and browser suites retain prior CPU/prefix/interrupt, AAA/AAS/AAM/AAD, three-model startup, firmware, video/input/frame, audio/PCM, GDMA/SDMA/Hyper Voice, RTC, SRAM/EEPROM and lifecycle/save-integrity assertions. All 86 prior cartridge conversion inputs/expected triples and all corresponding native/browser literal triples were compared with evaluation baseline and are unchanged. |
| Documentation and provenance | Hardware subset, source reconciliation, original literal-table derivation, fixture layout/hash/provenance and this executed evidence updated. |

Implementation self-review compared the final diff with all seven pre-merge
checkboxes. Downstream Coordinator review of the final checkpoint remains required
before merge. All 68 relative file links in the changed documents resolve, and
their local heading anchors were checked. Tests present are distinguished from
actual executions above.
This node does not push, open a PR, merge or comment.

Optional baseline production-browser discrimination, other browser engines,
physical-console measurements, human gameplay and commercial-software testing
were not performed. No independent per-model silicon logs were inspected and no
commercial-title impact was measured. Exact prefetch/pipeline accuracy and broader
exceptional arithmetic remain separate work. No proprietary ROM or firmware was
required. No usage-limit resets, allowance purchases or model/provider switches
occurred. Required post-merge verification: none.
