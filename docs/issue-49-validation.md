# Issue #49 validation

Executed on 2026-10-24 in the assigned GitWeave artifact worktree, starting at
`4601497da07d63bcbd8a45090b5f78e541fc4522`; production source matched evaluation
baseline `f9464ad5f50dfc5f362fe6c1ea0cf3846394690b`. Read live Issue #49 and its
empty comment collection with `gh issue view`, plus repository development and
review guidance. No remote Issue edit, comment, push, PR or merge was performed.
Required post-merge verification: none.

## Correction and source limits

[The correction record](byte-idiv-flags.md) records the old retention assumption,
the successful byte-IDIV rule, preserved OF requirement, source rationale and
limits. Read the NEC V30MZ manual §3.1.5 through its PDF. Web retrieval of the
pinned WSCPUTest raw README and assembly returned cache misses; `gh api` read
the pinned report and register-test/calculator excerpts successfully. The
calculator uses AL to index its SZP table, preserves its own IF through F202,
and routes quotient magnitude 128 to its exceptional path. Test success
boundaries are ±127; no claim is made about the existing −128 classification.

Per-model physical logs were not independently inspected. Mapped operands and
graphics-mode applicability are instruction-level inferences. The fixture and
vectors are original CC0 work. No external assembly or cartridge bytes were
copied. This is the Issue's specified assumption correction, without a change
to its goal or acceptance requirements.

## Actual pre-fix discrimination

These commands ran with the new tests and extended original cartridge before
changing the production success path. Each exited 101:

| Command | Actual failure |
| --- | --- |
| `cargo test -p ws-core --lib --locked cpu::divide_tests::issue_49_reproduction -- --exact` | F6 FB, AX=0006, BL=02, incoming F002: resulting AX=0003 passed, FLAGS F002 differed from F006. |
| `cargo test -p ws-core --lib --locked cpu::divide_tests::byte_idiv_full_flags_results_mapped_sources_and_branches -- --exact` | First Mono zero-dividend case: resulting AX=0000 passed, FLAGS F002 differed from F046. |
| `cargo test -p ws-core --test divide --locked` | Mono boot 1 wrote RAM 0400=0BAD instead of 600D despite seeded stale-success SRAM. |

These baseline checks stop at their first assertion; they do not establish
baseline failures on every model. Optional baseline production-Chromium
discrimination was not run. All corresponding corrected checks passed.

## Implementation and native coverage

Six production lines inside the existing successful byte-signed branch clear
AF/CF and call the existing SZP helper with an explicitly truncated quotient
byte. The common OF clear remains. Byte DIV, word DIV/IDIV, faults, arithmetic
results, operand sampling, instruction continuation and accounting are unchanged.

Four added CPU tests cover the reproduction; 15 literal AX/full-FLAGS/branch
vectors with F002/FFD7 input; BL and even/odd ES-overridden RAM/ROM/SRAM operands;
and all five static model/mode combinations (1,050 matrix executions). They
check all registers, segments, RAM/ROM/SRAM, ports, HALT/shadow state, exact IP,
following MOV and Z/C/O/S/P/signed-LT branches. Sources are sampled before
writeback, including successful AL/AH aliases. A byte-DIV full-FLAGS vector
guards its retained behavior. The existing divide-fault regression gained
signed-byte zero and positive/negative overflow cases and unchanged-register
assertions; its fault-frame checks remain.

Runtime mono/planar-color/mono/packed-color/mono transitions run on Color and
Crystal; Mono remains in its supported mode. A normal `Machine::run_frame`
with one byte IDIV remaining advances exactly its returned 26 clocks (24
execution plus two fetches), wraps one frame, produces one stereo audio sample,
and stops at the next instruction. Mapped byte forms retain five fetch charges
plus one operand charge, at both even and odd operand addresses. Absolute
hardware timing and TF single-step delivery are outside this correction.

The extended `divide.ws` retains all 52 word operations and adds 30 byte-IDIV
operations per mode pass. The actual CPU writes AX, unchanged DX=5AA5, full
FLAGS and six branch observations. Mono runs one pass; Color/Crystal run
mono → color → mono. Native tests compare every one of the 988 RAM and SRAM
words, plus stack/CS/HALT and pass pixels, on every model and fresh restart.
The browser uses the real file chooser, production Wasm, rendering, player save
flush and localStorage, comparing every saved word. Each boot clears volatile
records, invalidates retained completion and requires a fresh boot count;
poisoned stale-success saves cannot satisfy the checks. No ROM/save upload occurs.
Final cartridge SHA-256:
`d581d36ffbb6cbf1a44b2c69068aedea29b1da9ac73f62d2006345c0a96611a0`.

Two native test-harness issues were resolved during corrected validation. The
new literal records were initially inserted into the expected header instead
of the per-mode results, causing a word-5 mismatch; moving them fixed that
layout. The longer three-mode probe can finish during frame two, leaving the
top pixel from the earlier mode at the old two-frame check. Running three
frames permits a complete frame after the final palette write. All literal
CPU observations, markers and pixel requirements remain intact, and the
corrected native probe passes on all models and both boots.

## Required commands

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 31 images; only divide.ws changed, all 30 others byte-identical. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 139 core tests, 21 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass, including final documentation. |
| `npm run build:wasm` in web | Pass with installed Rustup stable toolchain on PATH; initial Homebrew-Rust attempt failed for missing Wasm target. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, including word DIV/IDIV and byte IDIV on all three models and fresh restart. |

`cargo test -p ws-core --lib --locked cpu::divide_tests` passed all eight
focused word/byte tests. `cargo test -p ws-core --test divide --locked` passed
the corrected cartridge probe. JavaScript syntax checks passed for both changed
scripts. `npm ci` restored locked dependencies without lockfile changes. The
Wasm build used the already installed toolchain and target via
`PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`.
The default `/opt/homebrew/bin/rustc` lacks the Wasm target; selecting installed
Rustup resolved the build failure. No toolchain installation was needed.

Both production builds completed before browser acceptance. The complete smoke
suite exited zero before starting the complete compatibility suite, using
`WS_PREVIEW_URL=http://127.0.0.1:4199/wonderswan-rust/` against this worktree's
production preview. No usage reset, allowance purchase or model/provider switch
occurred. Relative documentation links resolve. A literal comparison confirmed
all 52 original word assembly cases and all 208 original observation literals
in each native/browser expectation table remain intact; all 30 other generated
fixtures are byte-identical.

## Acceptance review

The implementation coordinator reviewed the production diff, native literal
vectors, original cartridge source/generated bytes, browser observations,
provenance and documentation against all seven Issue checkboxes. All seven
criteria are satisfied by the recorded checks. The complete compatibility suite
exited zero, including the new byte-IDIV probe on all three models and both
boots; existing three-model/startup/video/input/frame/audio/PCM/GDMA/SDMA/Hyper
Voice/SRAM/EEPROM/browser-save assertions remain in place and passed.

| Issue acceptance | Reviewed evidence |
| --- | --- |
| Successful F6 /7 rule, sources and scope | Common decoding changes only successful byte IDIV; AF/CF clear, quotient-byte SZP, existing OF clear. The correction record distinguishes source evidence and inference. Word flags, byte DIV and faults retain their paths. |
| Literal AX/full FLAGS vectors | 15 literal vectors under opposite arithmetic and TF/IF/DF patterns; zero/nonzero remainder with zero AL, all sign combinations, exact/inexact results, parity differences and ±127. No expectation calls production flag helpers. |
| Operands, state, branches and accounting | BL/AL/AH, even/odd RAM/ROM/SRAM, ES overrides, all unrelated state/source preservation, exact continuation, Z/C/O/S/P/LT branches, 24-clock execution plus access charges, and one normal-machine shared-device advance using returned cost. |
| Models, modes, discrimination and regressions | Three emulated models, five static model/mode combinations and live mode changes. Actual baseline reproduction/matrix/cartridge failures followed by native passes. Full workspace retains word flags, byte DIV, fault, multiplication, shift, prefix, interrupt, HALT and DMA checks. |
| Original fixture and fresh execution | Original source-included CC0 fixture retains all word cases and adds actual AX/full FLAGS/branch records. All 988 words and fresh counts are checked for each model and restart, with poisoned stale-success saves. Native and production Chromium/Wasm checks passed. |
| Mandatory commands and prior acceptance | All required native/build commands and both complete production Chromium suites passed, sequentially with smoke first. No prior assertions or three-model gates were removed. |
| Documentation and final review | Hardware subset, correction/source limits, fixture provenance/hash and actual failures/resolutions/results recorded. Optional physical and commercial evidence explicitly unperformed. |

Physical-console measurements, per-model physical logs, human or commercial
gameplay, additional browser engines, optional baseline Chromium discrimination
and additional proprietary software were not performed. Automated evidence is
bounded emulator acceptance, without a claim of commercial-catalog compatibility
or complete CPU readiness. Byte DIV multiplication history, signed exceptional
results, fault refinements, decimal adjustment and absolute timing remain
unresolved separate work. Required post-merge verification: none.
