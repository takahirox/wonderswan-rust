# Issue #67 validation

Executed 2026-10-24 in the assigned GitWeave artifact worktree, starting at
`dba44fc9ec97c965bd1997910ec59678407929d6`; production matched evaluation
baseline `6cabc27cafb174fc0d08b7d0cb750d0249f2de89`. Read live GitHub Issue #67
using `gh issue view` and the repository development/review guidance. No push,
PR, merge, remote Issue edit or comment was performed. The node's Result reports
the final local commit. Required post-merge verification: none.

## Evidence and decisions before implementation

[Source reconciliation](single-step.md) was written before modifying production
REP/HALT handling or assigning trap-entry clocks. It records independent
IP/CS/FLAGS/SP/register/order traces, direct V30MZ requirements, older-core
applicability limits, ares corroboration and differences, and explicit REP/HALT
inferences. The primary manual establishes vector 1 and entry/priority rules;
§5.4 alone does not establish REP TF cadence. The related manual and ares support
per-element polling; HALT completion polling follows §5.2 and ares, separately
from release of sustained standby. Exact console phases remain unmeasured.

Inspected the primary PDF chapter 5 and chapter 6, the related instruction PDF,
and the pinned independent assembly/comparison harness and ares dispatch,
string, miscellaneous, execution and move sources. Web raw-source retrieval
returned cache misses; direct HTTPS retrieval succeeded. The PDF screenshot
also returned a cache miss; local PDF text and positional extraction confirmed
Table 5-1's shared software-entry 25-clock cell and vector-1 priority 4.
`pdftotext` was unavailable; a temporary-directory pypdf install provided
extraction without changing repository dependencies. No source code was copied.
The 25-clock baseline is integrated as 20 execution plus five aligned internal
word charges, with existing additional odd/wait charges. This accounting is an
integration policy tied to the primary table, not a measured silicon phase trace.
Existing synchronous/IRQ costs remain unchanged.

Correction: previous tests allowed FFD7 restored by POPF to remain live while
application instructions continued indefinitely. Single-step now delivers at the
eligible boundary, saving normalized FLAGS before clearing live IF/TF. Literal
arithmetic/control-flag expectations are retained either before entry in isolated
opcode checks or in real saved frames followed by IRET. The AAM-zero fault's
handler-advanced HLT continuation similarly traps at completion when IRET
restores TF; the prior sustained-HALT assertion was invalid under the chosen
completion rule. Existing division-fault restart/results remain unchanged.

## Actual pre-fix failures and fixes

The new tests and original fixture ran before changing production execution:

| Command | Actual result |
| --- | --- |
| `cargo test -p ws-core --lib --locked cpu::trap_tests::issue_67_popf_nop_reproduction -- --exact` | Exit 101: POPF activated F102, but NOP returned 2 clocks rather than 27 and did not enter vector 1. |
| `cargo test -p ws-core --test trap --locked` | Exit 101: Mono boot 1 RAM word 0 was 0BAD rather than 600D; the CPU-written handler count was absent despite a supplied stale success save. |

Both corrected commands passed. These baseline failures stop at the first
assertion; they do not claim failures observed on all models or in browsers.
Before the corrected fixture pass, review relocated scratch data/control from
0800/0900/0A00 to 1800/1900/1A00 to avoid the seven-phase observation block.
This changes no architectural expectation or production behavior.

The first full core regression run exposed 31 tests with inert-TF assumptions.
Literal opcode matrices now call the same decoded instruction implementation
through a test-only execution helper, before boundary delivery; no arithmetic,
full-FLAGS, branch, alias, state-preservation or instruction-cost assertions were
dropped. Scheduler division/PUSH SP checks instead observe the real frame and
execute the real handler IRET, scheduling its cost separately. The multiplication
history producer/persistence matrices retain all six producer forms and every
prior product/FLAGS/branch assertion. Final review strengthened that existing
six-producer sequence to use real vector-1 saved frames and IRET after POPF+NOP
and the intervening NOP/word DIV/byte IDIV, including across clones. Its focused
check passed, followed by another full workspace and Clippy pass. Additional
ordinary-step tests demonstrate history surviving real handlers before byte DIV.

The first new stack-wait test attempted Mono port 60, correctly rejected by the
existing model gate. It now leaves Mono's fixed SRAM wait policy intact and
programs the Color/Crystal wait only on those models. The first PUSH SP scheduler
adaptation assumed CS=0 when its existing program uses CS=0100. The shared frame
check now accepts the independently expected CS and verifies 0100 there. All
focused and full checks passed after these test corrections.

## Final implementation and coverage

The production change samples TF before decoded instruction execution and polls
after an eligible boundary. SS loads and synchronous entries suppress their own
trap boundary. Eligible external INT has priority over TF; entry consumes the
application interrupt shadow and clears IF/TF. There is no new persistent trap
latch or model/mode gate. Existing CPU flags and external-INT shadow clone/reset
through the established lifecycle constructors. REP retains one element per
bounded step, its first-prefix restart IP and count/index progress; HALT entry
is canceled by a completion trap. DMA-only and sustained-HALT calls execute no
instruction and cannot produce a new trap.

Eleven new core tests cover the literal reproduction, normalized frames and push
order, IF clear/set, POPF/IRET activation, repeated stepping, handler-cleared TF,
register/memory MOV SS and POP SS, other segment operations, segment/BUSLOCK and
mixed repeat prefixes, independent POPF+NOP and POPF+STI saved-IP expectations,
IRQ selection before execution and at a protected instruction's completion,
INT/INT3/INTO/BOUND/byte and word division/AAM faults without nested traps,
REP element/final/zero-count/early-comparison exit and IRQ prefix restart,
HLT completion/idle, internal/odd/SRAM-wait stack costs, arithmetic and explicit
multiplication history, cloning and lifecycle success/rejection.

All three models and all nine static supported model/mode configurations are
covered, with live 00→80→00→C0→00→E0→00 transitions on Color/Crystal. Lifecycle
coverage includes loaded/unloaded restart, reset, same/different compatible model
changes, successful replacement, malformed/Color-on-Mono load rejection and
rejected model changes, preserving running state on rejection. Normal Machine
scheduling checks exact instruction+trap, sustained-idle, GDMA-only and real
SDMA-stall costs at frame completion, one timer decrement, one raster/frame wrap
and one stereo sample, with no extra application instruction. Existing model,
interrupt, DMA and arithmetic behavior remains covered by the full suite.

The new source-included CC0 fixture runs ten programs/twelve handler frames per
phase. It records actual handler counts, continuation BX, IP/CS/saved FLAGS,
live handler FLAGS/SP and CX/SI/DI/BX. Native and production Chromium independently
compare every one of 900 words on every model and fresh restart; native also
compares the RAM mirror, copied bytes and final stack/HALT. Color/Crystal run all
seven mode phases; Mono runs its supported mode. Both hosts poison stale results
and supply an old pass marker; each boot clears observations, invalidates saved
completion and increments the retained execution count. No proprietary ROM or
firmware is needed. [Fixture provenance](../fixtures/README.md#vector-1-single-step-probe-issue-67)
describes the layout and ownership.

## Commands actually executed

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py` | Passed; only the new trap.ws was added, existing fixture bytes unchanged. |
| `python3 fixtures/build.py --check` | Passed; all fixture bytes reproducible. |
| `python3 demo/build.py --check` | Passed; demo unchanged and reproducible. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test -p ws-core --lib --locked cpu::trap_tests` | Passed, eleven new tests. |
| `cargo test -p ws-core --test trap --locked` | Passed, every model and fresh restart, all observations. |
| `cargo test -p ws-core --lib --locked cpu::push_sp_tests::push_sp_runtime_modes_and_normal_machine_scheduler_advance_once -- --exact` | Passed after correcting the expected saved CS. |
| `cargo test --workspace --locked` | Passed: 193 core unit + 33 integration + 1 web unit = 227 tests; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `git diff --check` | Passed. |
| `npm ci` in web | Passed, lockfile unchanged. |
| `npm run build:wasm` in web | Passed with the installed Rustup stable toolchain and Wasm target. |
| `npm run build` in web | Passed, production TypeScript/Vite assets. |
| `npm run test:browser` in web | Passed, complete production Chromium smoke suite. |
| `npm run test:compat` in web | Passed, complete production Chromium compatibility suite, including every new trap record on all models and fresh restart. |

Production preview serves this worktree's built dist at
`http://127.0.0.1:4187/wonderswan-rust/`, using that address through
`WS_PREVIEW_URL`. Browser commands run sequentially, smoke before compatibility.
The Wasm command uses
`PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`;
no toolchain/model/provider switch was used to evade a usage limit.
No usage reset or allowance purchase occurred.

Both complete production suites passed sequentially. Compatibility also retained
model gates, firmware-free startup, video/input/frame continuity, audio/PCM,
GDMA/SDMA/Hyper Voice, SRAM/EEPROM/RTC retention, browser save integrity,
console-record isolation and ownership/provenance. No browser exceptions were
reported. A final changed-document local-file-link check and whitespace check
passed. The final diff was reviewed against every acceptance row below.

## Artifact identities

- Production `crates/ws-core/src/cpu.rs` Git blob:
  `be53f03ab5c05a874b88c73ea87670eba37bf569`.
- Original `fixtures/trap.asm` SHA-256:
  `335d8301c5a887c9e4f9693a2862e95467f695345a3329224f6fb83d72d3eada`.
- `fixtures/generated/trap.ws`, 131,072 bytes, SHA-256:
  `2a0db0bc388457c00ea28eb1f5a0ff07c85e079cdf72c3f292f48626e881820f`.
- Production Wasm `ws_web_bg-DrBYWedh.wasm` SHA-256:
  `344501beddb922c3dcaadc25ecd973dd539bb0d8d1af53bb74c6e111743ffa6a`.
- Production JavaScript `index-CZnzg1ub.js` SHA-256:
  `a904288e7fe4413459ccb381c560d41f602604ca26ac1df23ec13a6027f8005c`.
- Retrieved V30MZ PDF SHA-256:
  `a58c86b1d5e9b7ffdcd7c7cd31dd31eff93308098c1c5c3ed0a41825b9a9039e`.
- Retrieved related instruction PDF SHA-256:
  `19a69547651aa65e32df33f1bf84dea095a00237f680a67b166213fcc6d96f6a`.

Browser build files remain ignored. Committed sources, fixture bytes and lockfiles
reproduce the tested production path; external source revisions are pinned in
the reconciliation. No runtime/core public API or save format was added.

## Final acceptance review

| Issue checkbox | Diff/evidence review |
| --- | --- |
| Source reconciliation before REP/HALT/accounting | Written before production edits; independent traces, primary priority/entry, older-core and ares limits, REP/HALT inferences and total-vs-transfer accounting recorded. |
| Vector 1, activation and continuation | Literal reproduction, IF clear/set, normalized frame/order/SP, repeated IRET and real stacked-TF clearing pass. |
| SS and prefix exclusions | Register/memory MOV SS, POP SS, negative segment controls, segment/BUSLOCK/REP and mixed prefixes pass; external-INT shadow stays distinct. Independent NOP/STI saved IPs reproduced. |
| Priority, REP/HALT and DMA boundaries | IRQ-before-execution/completion, synchronous entry/return, no fabricated nested frame, element/final/zero-count/early exit, full prefix restart, HLT completion/idle and both DMA-only stalls pass. |
| Arithmetic and multiplication history | Every prior valid literal/branch/state assertion retained via isolated opcode execution or saved frames; ordinary trapping history/DIV tests and unchanged division faults pass. |
| Models, modes, lifecycle and scheduling | Every model/mode, runtime transitions, clone and success/rejection lifecycle isolation, normal Machine timer/raster/frame/audio once and no extra application instruction pass. |
| Original native and Wasm fixture | Actual pre-fix focused/native fixture failures recorded, corrected native and complete production-browser passes. Independent actual frame/count/continuation records, poisoned stale output, completion invalidation, all models/restarts and CC0 provenance included. |
| Mandatory commands | All mandatory native/build checks and both complete production Chromium suites passed sequentially. |
| Documentation and final review | Hardware support, reconciliation, provenance, identities, failures/resolutions and unperformed checks recorded; final diff reviewed against all Issue checkboxes, with no blocking findings remaining. |

Optional baseline browser discrimination, additional browser engines/software,
physical-console comparisons and human gameplay were not performed. Exact
silicon REP/HALT phases, prefetch/general timing accuracy, new NMI sources,
exceptional division continuation refinements and REP on non-string instructions
remain separate. Original emulator probe passes do not establish commercial-game
readiness; commercial-title impact is unmeasured.
