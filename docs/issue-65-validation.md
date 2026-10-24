# Issue #65 validation

Executed 2026-10-24 in the assigned GitWeave artifact worktree, starting at
`6591426fc3c09f498d89091735419e6f0ae64d0f`; production matched evaluation
baseline `144eaef001109cf79d42f5b8426156fe2a6145ec`. Read live GitHub Issue #65
with the GitHub connector and repository development/review instructions.
No push, PR, merge, remote Issue edit or comment was performed. Required
post-merge verification: none. The final checkpoint commit is reported in this
node's Result; the exact production and fixture identities are below.

## Correction and source evidence

[Source reconciliation](byte-div-flags.md) records the old preservation and
universal OF-clear assumptions, corrected byte-DIV requirements, retained valid
word DIV/IDIV and byte-IDIV rules, direct evidence, applicability inferences
and remaining gaps. Inspected the NEC preliminary manual §3.1.5 through its PDF.
Web retrieval of pinned WSCPUTest raw sources returned cache misses; the GitHub
connector successfully read README lines 155–200 and assembly lines 5149–5285
at `245b12c06dede4a2bb9c04c942ebca2044ba4930`. The calculator seeds byte IMUL
history with AL=00/10 before expected-result arithmetic; the register experiment
loads opposing FLAGS independently before DIV. Extending that producer to the
other explicit multiplication instructions is an instruction-level inference,
not additional measured evidence. No per-model physical logs were inspected.

Successful unsigned `DIV AH` is mathematically impossible (AX/AH >= 256 when
AH is nonzero; AH=0 divides by zero). The Issue's successful-alias requirement
is reconciled as unsigned AL plus signed AL/AH, with unsigned AH fault coverage.
This corrects an impossible test case, without relaxing any valid acceptance.
The synthetic cleared-history startup value is documented as policy rather
than measured firmware behavior. No external implementation or assembly was copied.

## Actual pre-fix failures and resolutions

The following ran with the new tests/extended original fixture and the unchanged
production CPU success path; each exited 101:

| Command | Actual result |
| --- | --- |
| `cargo test -p ws-core --lib --locked cpu::divide_tests::issue_65_reproduction -- --exact` | AX=0003 passed; FLAGS F002 differed from F843 after overflowing byte IMUL followed by independent FLAGS replacement. |
| `cargo test -p ws-core --test divide --locked` | Mono boot 1 RAM word 0 was 0BAD instead of 600D, despite a poisoned stale-success save. |

Both corresponding corrected checks passed. These failures stop at their first
assertion; they do not establish pre-fix failures on every model. Optional
pre-fix production-browser discrimination was not run.

An initial focused-test build failed because a test-helper edit also inserted
an out-of-scope seed into the word matrix; that accidental insertion was removed
before the actual reproduction above. An early new unsigned-AH success vector
instead faulted (31 clocks versus the expected 26); the mathematical impossibility
is reconciled above and unsigned AH now has explicit fault/history coverage.
The first Wasm build selected Homebrew Rust without a Wasm target. The installed
Rustup stable toolchain already had `wasm32-unknown-unknown`; putting its bin
directory first in PATH resolved the build without installing a toolchain.

## Implementation and coverage

One private CPU boolean retains the equal multiplication CF/OF value, separately
from architectural FLAGS and automatically across clones. Both group multiplication
and immediate IMUL update it. Successful byte DIV clears AF/PF/SF, derives exact-odd
ZF and restores CF/OF. The default initialization clears it as synthetic policy;
existing lifecycle constructors provide reset/restart/load/model isolation.
No save format, public API, quotient/remainder computation, fault path, timing,
multiplication product/flag rule or word/signed division rule was changed.

The shared byte matrix preserves all 15 byte-IDIV vectors and adds 16 literal
unsigned-byte vectors (eight inputs under each history), incoming F002/FFD7,
five static model/mode configurations and seven operand locations: BL plus
even/odd ES-overridden RAM, ROM and SRAM. The new unsigned matrix executes
1,120 divisions. It checks all registers, segments, RAM/ROM/SRAM, ports, HALT,
interrupt shadow, full FLAGS, exact IP, following MOV and JZ/JC/JO/JS/JP/JL.
Expectations do not call production flag helpers. Additional successful unsigned
AL and signed AL/AH aliases verify sampling before writeback. Unsigned AH tests
preserve fault behavior and demonstrate history surviving a fault.

Producer vectors independently check fit/overflow products and full multiplication
FLAGS for all six forms: byte/word MUL, byte/word IMUL and immediate 69/6B IMUL.
They include AL/AX and immediate source/destination aliases and negative immediate
sign extension. A producer replaces previously opposite history. ADD, STC/CLC,
NOP and an independently loaded POPF intervene; cloned CPUs and repeated DIV
operations retain history, including across intervening word DIV and byte IDIV.
Lifecycle tests cover new/unloaded CPUs, loaded/unloaded restart, reset, replacement,
same/different compatible models, malformed loads, Color-on-Mono loads and rejected
model changes, preserving running state on failure.

Runtime 00/80/00/C0/00/E0/00 mode transitions cover all supported graphics modes
on Color/Crystal, with Mono kept in its supported mode. Both histories survive.
A cloned CPU provides its actual returned cost, compared with normal machine
execution one instruction before frame completion: exactly 26 clocks (24 execution
plus two fetches), one frame/raster wrap and one stereo audio sample, stopping at
the next opcode. Mapped byte operands retain five fetch plus one operand charge
at even and odd addresses. Absolute timing and TF delivery are not established.

The original CC0 fixture retains all 52 word and 30 byte-IDIV operations and adds
16 byte-DIV operations per pass. A separate initial DIV checks cleared synthetic
startup history before any producer; the fixture leaves set history before HALT.
Mono runs one pass; Color/Crystal run seven passes across all graphics modes.
Native and production Chromium/Wasm compare every one of 2,752 RAM/SRAM or saved
words, including actual AX, preserved DX, full FLAGS and six branch observations,
on every model and fresh restart. Each boot clears observations, invalidates saved
completion and increments its retained counter. Tests poison old observations
with a stale success marker; old output cannot satisfy fresh execution.

## Commands actually executed

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py` | Passed; rebuilt original fixtures, only divide.ws changed. |
| `python3 fixtures/build.py --check` | Passed, all fixture bytes reproducible. |
| `python3 demo/build.py --check` | Passed, demo unchanged and reproducible. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test -p ws-core --lib --locked divide` | Passed, 16 selected tests. |
| `cargo test -p ws-core --test divide --locked` | Passed, all three models and restart, complete records. |
| `cargo test --workspace --locked` | Passed: 182 core unit + 32 integration + 1 web unit = 215 tests; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `git diff --check` | Passed. |
| `npm ci` in web | Passed; lockfile unchanged. |
| `npm run build:wasm` in web | Initial Homebrew target failure above; passed with installed Rustup stable bin first in PATH. |
| `npm run build` in web | Passed, production TypeScript/Vite assets. |
| `npm run test:browser` in web | Passed, complete production Chromium smoke suite. |
| `npm run test:compat` in web | Passed, complete production Chromium compatibility suite, including new byte-DIV history observations on all models and fresh restart. |

After the full native pass, test-only cleanup replaced a tuple lint suppression
with a type alias and clarified comments.
`cargo test -p ws-core --lib --locked cpu::divide_tests` passed all 14 division
tests and Clippy passed again. Production source and fixture bytes were unchanged
by that cleanup.
Production preview runs this worktree's built dist at
`http://127.0.0.1:4183/wonderswan-rust/`; port 4173 was occupied by an unrelated
listener and was left alone. Both browser commands use that address through
`WS_PREVIEW_URL`, sequentially with smoke first. The Wasm command uses
`PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`.
No usage reset, additional allowance purchase or model/provider switch occurred.

The complete compatibility run also preserved SRAM/EEPROM/RTC retention, browser
save integrity, model gates, firmware-free startup, video/input/frame continuity,
audio/PCM, GDMA/SDMA/Hyper Voice and ownership/provenance checks. No browser
exceptions were reported. Final formatting, whitespace and changed Markdown
local-file-link checks passed.

## Artifact identities

- Production `crates/ws-core/src/cpu.rs` Git blob:
  `597bdceb265803aa1f067ee82bfe9b565a56292a`.
- Original `fixtures/divide.asm` SHA-256:
  `3df1f7cdc3d9e6aaee0ff27aea44e387ffb0c351676034753f59a2fd03ee8b2d`.
- `fixtures/generated/divide.ws`, 131,072 bytes, SHA-256:
  `a2da8eed0deac022b84fce73cb3b2f9b6f02bfca4f1783b00892316c91ca01f8`.
- Production Wasm `ws_web_bg-t8eWOnTZ.wasm` SHA-256:
  `e86968aebe63cf72f4285f3ace093b4c9252952178286cd6a2bce32602819eec`.
- Production JavaScript `index-Cxgv3XZF.js` SHA-256:
  `2f7b39e9215c53862a508dd360481c5bf458450dd4a66d5dd6ae594cfadbb234`.

Generated browser build files remain ignored; committed assembly, fixture bytes,
locked sources and commands reproduce the tested production path.

## Final acceptance review

| Issue checkbox | Diff and evidence review |
| --- | --- |
| Ordinary F6 /6 and explicit history producers | Private state, six producer forms and bounded success-path correction; literal products/flags retained. Direct evidence and inferences documented. |
| Literal AX/full-FLAGS expectations | Both histories and F002/FFD7 include all four required results, zero/exact/inexact/odd/even/FF. No production expectation helper. |
| Producers, persistence and lifecycle | Fit/overflow for all forms, sign extension and aliases, opposing-history replacement, POPF/arithmetic, clones, repeated DIV, successful lifecycle clearing and rejected-change preservation tested. |
| Operands, branches and scheduling | Register/AL and mapped even/odd RAM/ROM/SRAM with ES overrides, unrelated state, exact IP, six branches and exact existing costs; normal scheduler advances once. Signed AH success and unsigned AH impossibility/faults reconciled. |
| Models/modes and regressions | All three models, every supported graphics mode and live transitions; actual pre-fix focused/probe failures and corrected passes. Full workspace arithmetic/prefix/interrupt/HALT/DMA suite passed. Contradictory assertions corrected with evidence. |
| Original native/Wasm/browser fixture | Source-included CC0 extension retains word/signed observations; actual CPU AX/FLAGS/branches, poisoned observations, invalidated completion and fresh counters on all models/restarts. Native and complete production-browser checks passed. |
| Mandatory validation commands | All required native/build commands and both complete production Chromium suites passed sequentially. Existing gates and ownership/provenance are preserved. |
| Documentation and final diff | Hardware support, source reconciliation and fixture provenance updated; identities, actual failures/resolutions/checks and gaps recorded. Final diff reviewed against every Issue checkbox; no blocking findings remain. |

Optional baseline browser discrimination, additional browser engines, additional
software, physical-console comparisons and human gameplay were not performed.
Exceptional division flags/results, conversion-history effects, TF single-step
delivery and absolute timing remain separate visible gaps. Passing original
emulator probes does not establish commercial-game readiness or measured
commercial-title impact. No proprietary ROM or firmware was required.
