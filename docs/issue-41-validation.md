# Issue #41 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`4683059b737c8e792c1513832496e71c12c9849b`, on evaluation baseline
`9bcbf29727bd4b052991d8937964e25f51ac1a05`. The implementation coordinator
read live Issue #41, its comments (none), and the development/review guidance.
No remote Issue, PR, branch or comment was modified. This node does not push or
merge; no post-merge verification is required.

## Source reconciliation before implementation

[The source reconciliation](fpo1-poll.md) was written before decoder changes.
The NEC V30MZ manual §§1.3/3.6 calls FPO1 a NOP, while Appendix A depicts
memory/displacement fields. The pinned WSCPUTest report explicitly establishes
the selected console decode rule: D8–DF are two-byte NOPs; 9B completes without
waiting or exception. Its source tests all eight FPO1 opcodes with C0 and POLL
followed by ADC. The report, relevant source excerpts and pinned ares one-byte
fetch implementation were inspected through GitHub API after web fetching of
those pinned raw URLs failed; the manual was read through its PDF.

This corrects Issue #35's unresolved-prerequisite assessment without treating
the manual's table alone as proof of byte consumption. An effective POLLB-low
console profile is implemented on every model. The report author's probable
permanently-low electrical wiring remains a conjecture. Neither physical
execution nor per-model measurement logs were inspected. Wider second-byte
tests exercise the report's rule rather than claiming more hardware evidence.
No external implementation code was copied. Full prefetch, absolute instruction
timing, electrical pin behavior, single-step delivery and other unsupported
encodings remain outside this change. No commercial-title result is claimed.

## Implementation and independent regression coverage

The ordinary decoder now handles 9B with a nine-clock execution component and
D8–DF with the existing one-clock component and exactly one additional mapped
byte fetch. It does not call the operand decoder, inspect the swallowed byte,
access apparent operands, raise coprocessor exceptions or introduce a generic
NOP fallback. REP validation, prefix limits and unknown-opcode diagnostics are
unchanged. No device, DMA, cartridge identity, public API or browser player path
changed.

Five new CPU tests cover:

- Every D8–DF opcode × all 256 second bytes × two independent literal register/
  flag patterns × five model/mode combinations (20,480 vectors). Exact IP,
  registers, segments, flags, HALT state, RAM sentinels and ports are preserved;
  MOV CX,ABCD discriminates the next instruction. Two fetch clocks and one
  execution clock are asserted separately from the approximate total of three.
  Memory/displacement-looking, prefix-looking, HALT and unknown second bytes
  all obey the same rule. Access counts also prove no apparent operand reads.
- POLL with two literal flag/register patterns, separate one-fetch/nine-execution
  expectations and a following ADC checking carry and arithmetic flags.
- Every opcode through mapped ROM, bank-window boundaries, IP wrap, 20-bit wrap
  to RAM and all four live A0 width/wait combinations, plus an unmapped consumed
  byte. Independent byte fetches do not gain an aligned-word discount.
- Zero through seven segment/BUSLOCK prefixes under STI, MOV SS and POP SS
  shadows, with a pending IRQ delivered only after the whole decoded instruction.
  The return IP, flags and fetch count are checked. A subsequent unsupported
  opcode retains its exact diagnostic; REP/REPNE on these instructions still
  rejects before swallowing a byte. Swallowed versus actual HALT is explicit.
- Shared-clock advancement of the returned costs exactly once, with a separately
  charged seven-clock GDMA word before the CPU instruction in color mode.
  The existing bounded REP restart, interrupt, SDMA, HALT and BUSLOCK tests are
  retained and pass in the full workspace suite.

These are emulator regression vectors, not measured physical timing. The
execution/fetch decomposition follows the source record and existing additive
bus approximation. No prefetch or scheduling redesign is included.

## Baseline failures actually executed

Before changing production decoding, ran the new exhaustive FPO1 test and POLL
state test separately against unchanged CPU production code. Both failed with
exit 101: `Unsupported opcode D8 at 0000:1000.` and
`Unsupported opcode 9B at 0000:1000.` respectively.

Built the original cartridge and ran `cargo test -p ws-core --test fpo1_poll
--locked` before the fix. It failed with exit 101 at
`Unsupported opcode 9B at F000:004D.`. After the fix the same test passes on all
three models and restart. No baseline source substitution or reset was needed.
The baseline cartridge failure was measured natively; browser results below use
the corrected production Wasm.

During vector authoring, corrected the hand-counted ADC expected results and
flags: 71+71 sets OF, and 7F+71+carry is F1. These are checks of the following
ADC, not changes to POLL/FPO1 requirements. All final vectors pass.

## Original cartridge and production browser coverage

`fixtures/fpo1-poll.asm` and `fixtures/generated/fpo1-poll.ws` are original
CC0-1.0 source/image with no imported assets or firmware. The ordinary 128 KiB
cartridge has 32 KiB SRAM, footer reset far jump, banking and checksum. It
executes POLL and all eight FPO1 opcodes, including memory/displacement-looking
06/46/86/00, prefix-looking F0/F3/26 and C0. Seven mixed/redundant prefixes
precede the final FPO1. Flags and ADC results independently expose continuation,
byte alignment and carry preservation; registers, stack balance, untouched RAM
and actual CS are checked before completion.

Boot clears volatile results, increments a retained boot count and invalidates
saved completion before probing. Native tests check all 22 actual CPU-written
RAM and mirrored SRAM words, framebuffer, stack and sentinel on all three models,
then repeat after restart. The new production compatibility check selects the
local image through the actual file chooser/player/Wasm, reads real SRAM flushes
against separately authored literals, and requires the incremented boot count
and completion on restart. It also asserts zero subsequent network requests.
No saved pass markers are supplied by tests.

SHA-256: `1fe1db8b9badf9e1a6062efbf52502df3f76a6b637ac039115a5d93d460bbc3e`.
All 27 prior cartridges and the demo remain byte-identical.

## Required checks actually executed

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 28 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 121 core tests, 18 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass, zero warnings. |
| `git diff --check` | Pass; repeated before commit. |
| `npm run build:wasm` in `web` | Pass using the installed Rustup stable toolchain PATH. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium production smoke suite. |
| `npm run test:compat` in `web` | Pass: complete Chromium production compatibility suite, including the new probe on every model and fresh restart. |

`npm ci` restored locked dependencies without lockfile changes. The first Wasm
build attempt selected Homebrew Rust, whose sysroot lacks the Wasm target.
Selecting the already installed Rustup stable toolchain resolved it; no compiler
installation, model/provider switch, usage reset or allowance purchase occurred.
Both production builds completed before the smoke suite; the compatibility suite
started after the complete smoke suite passed. This worktree's production preview
is `http://127.0.0.1:4197/wonderswan-rust/`, selected through `WS_PREVIEW_URL`.
JavaScript syntax checks also passed for the new probe and its suite integration.

## Coordinator acceptance review

The implementation coordinator reviewed the final decoder diff, literal vectors,
assembly/generated image, native/browser assertions, support documentation and
actual results against every pre-merge checkbox. The ADC expectation arithmetic
was corrected during test authoring; no blocking findings remain. No existing
acceptance assertions were removed or relaxed.

| Issue acceptance | Reviewed implementation and executed evidence |
| --- | --- |
| Source reconciliation before implementation | Original unresolved-prerequisite assumption, corrected two-byte/nonblocking requirements, manual/table discrepancy, pinned report/source, effective-low profile versus wiring conjecture and experimental limits recorded before decoder edits. |
| Eight FPO1 opcodes and POLL | Two ordinary match arms; no operand interpretation or exception path. Exhaustive state/alignment tests and original cartridge pass. Unknown operations retain generic errors. |
| Mapped fetch and execution/device accounting | Existing byte helper, independent literal one/nine execution and two/one fetch costs, live A0 cases, mapped bank boundaries, IP/20-bit wrap, shared-clock and separate GDMA assertions pass. Device/DMA scheduling code unchanged. |
| Independent vectors, models and baseline failures | 20,480 exhaustive vectors plus POLL, sentinel, wrapping and boundary cases across Mono and Color/Crystal in mono/color modes. Baseline D8/9B unit and cartridge failures actually observed before the fix. |
| Prefixes and control-flow regression | Segment/BUSLOCK through seven prefixes, STI/MOV SS/POP SS shadows, no mid-instruction IRQ, swallowed prefixes/HALT, REP rejection and subsequent unsupported diagnostic checked. Existing bounded REP restart and HALT/interrupt tests retained and pass. |
| Original source-included CC0 cartridge | Normal footer, checksum and SRAM; all eight opcodes/POLL, memory-looking bytes, independent flags/ADC/sentinel/CS values and fresh completion/boot markers. Native RAM/SRAM and production Chromium/Wasm pass on every model and restart. |
| Existing hardware and browser acceptance | Complete workspace and both complete production Chromium suites pass, retaining firmware-free startup/model gates, LCD pipeline, exact PCM/audio, GDMA/SDMA/Hyper Voice, both EEPROM controllers, SRAM, save integrity, input/frame continuity and local ROM ownership checks. All previous cartridge/demo bytes unchanged. |
| Required command set | All listed Python, format, workspace test, Clippy, whitespace, Wasm, production build and sequential full Chromium commands passed. Browser suites ran against this worktree's production build. |
| Documentation, provenance and final review | Hardware support/gap list updated without removing unrelated limits. Source correction, CC0 provenance/hash, actual failures/passes, execution versus fetch costs and optional unperformed hardware checks recorded. Relative documentation targets and JavaScript syntax checks pass. |

Physical-console comparisons and commercial-title gameplay are optional and were
not performed. No proprietary ROM/firmware or additional browser engine was used.
No required post-merge verification.
