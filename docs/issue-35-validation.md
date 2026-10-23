# Issue #35 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`b6f5004069e7e54ab894d8481fb39a39d8f59739` (workflow checkpoints on evaluation
baseline `e14d8cb089b0f13a8bcd4e1eb0dcece0240cf71a`). The implementation coordinator
read open Issue #35 through the GitHub connector and followed the repository's
development/review guidance. No remote Issue, PR, branch or comment was modified.

## Correction and source limits

The old AI-authored RMW-only assumption and its unconditional ModR/M lookahead
rejected supported memory reads/writes and I/O, and added an artificial bus access
even to previously accepted instructions. The correction and primary references
are recorded in [CPU support and source reconciliation](hardware-subset.md#buslock-source-reconciliation-issue-35).
The coordinator inspected extracted text from the NEC preliminary manual,
§1.3 (printed pp. 14–16), §3.6.1, §5.3 and Appendix A's FPO1 entry. PDF screenshot
attempts for the BUSLOCK/prefix/interrupt sections failed with cache-miss errors;
visual PDF inspection is not claimed. The Issue's ares observation is reported
context, not code copied or newly executed here. FPO1 remains unresolved and
unchanged. Excluded older NEC extensions/REPC/REPNC are no longer listed as
required V30MZ gaps; supported carry/direction instructions remain covered.

No exact BUSLOCKB/prefetch timing, console DMA wiring/arbitration, POLL wiring,
single-step delivery or physical-console result is claimed. Treating BUSLOCK as
ineffective on supported instructions without memory/I/O is an explicit inference.
The pre-existing eight-prefix acceptance/ninth-prefix diagnostic is retained as
defensive emulator policy beyond the guaranteed seven, without hardware claims.
Commercial-title impact is unmeasured. No acceptance criterion was weakened.

## Implementation and independent evidence

F0 is consumed in the existing prefix loop. The RMW whitelist, ModR/M lookahead
and stale "DMA absent" comment are removed. Ordinary instruction decoding, effects,
bus faults, unsupported opcodes/extensions/ports and REP diagnostics retain their
existing paths. CPU effects still complete before the next scheduling boundary;
GDMA/SDMA and IRQ ordering were not changed. The old rejection assertions are
replaced; their PUSHA/POPA/ENTER/LEAVE regression remains.

Five new unit tests run on Mono, Color and Crystal. The 26 literal-byte vectors
cover byte/word memory reads/writes, accumulator and ModR/M forms, disp8/disp16,
immediates, read-only CMP, ADD/XCHG RMW, implicit PUSH/POP, all eight byte/word
immediate/DX IN/OUT forms, register arithmetic, immediate MOV and NOP. They assert
full registers, segments, flags, exact IP, unchanged/expected RAM and ports, and
literal bus-access/cycle counts before observations. A following MOV CX,ABCD
detects overconsumption. Other vectors cover repeated/mixed prefixes through
seven (and the defensive eight/nine boundary), last-segment selection, an IRQ
pending under STI's shadow, bounded seven-prefix REP with IRQ/IRET restart at
the first prefix, unsupported diagnostics and carry/direction operations.

`fixtures/buslock.asm` and `generated/buslock.ws` are original CC0-1.0 source/data.
The existing builder creates a 128 KiB cartridge with ordinary footer, header,
checksum, final-bank execution and 32 KiB SRAM. Explicit F0 bytes avoid NASM's
x86 LOCK restrictions. The CPU checks memory, stack, flags, I/O, seven-prefix
segment/REP combinations and register/NOP effects. Independent host literals
check twelve result words plus a retained boot count; the CPU copies raw RAM
observations into SRAM, rather than the host injecting results. The actual CS
marker is F000. The native integration test runs and restarts it on every model,
also checking RAM, SP, HALT, framebuffer and frame continuity.

The production compatibility module uses the real file chooser, Wasm CPU,
model selector, Restart control and player SRAM flushes. A fresh retained count
prevents old save data satisfying a restart assertion. Each physical model has
an isolated context; all result words are compared with independent JavaScript
literals. Request observations verify ROM/save bytes remain local. Existing
tooling is extended without another framework, dependency or core/browser API.

New cartridge SHA-256:
`ef777392e28689db8549f7dd393372f08712012a11313defa878a7d681999531`.
All 24 existing cartridges and the demo remain byte-identical. No proprietary
asset, downloaded cartridge/firmware, copied emulator implementation or
game-specific dispatch was introduced. Fixture provenance is updated.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 25 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 104 core unit tests, 15 integration tests, one adapter unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass after factoring the new test vector tuple into a type alias to resolve the initial type-complexity warning. |
| `git diff --check` | Pass, including the final record and staged diff before commit. |
| `npm run build:wasm` in `web` | Pass, using the README Rustup PATH selection. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium suite, including the new BUSLOCK CPU markers and restart on all three models. |

`npm ci` restored locked dependencies without lockfile changes. The production
preview is `http://127.0.0.1:4295/wonderswan-rust/`, supplied through
`WS_PREVIEW_URL`. The two complete browser suites execute sequentially after
both production builds; both exited successfully. JavaScript syntax and 25 relative
Markdown file targets/anchors in changed documents passed checks. Focused CPU and
native-cartridge tests passed before full checks. No runtime or browser test failed.

## Coordinator acceptance review

The implementation coordinator reviewed the final code, literal vectors, fixture
assembly/generated image, browser module, documentation and actual check outputs
against every source-Issue acceptance item:

- The RMW-only validator and unconditional operand read are gone. Supported
  memory/I/O, implicit accesses and ineffective register/NOP prefixes use ordinary
  decoding. Underlying opcode/extension/port and REP diagnostics remain covered.
- Independent byte vectors cover all three physical models, the required memory,
  displacement/immediate/comparison/RMW/stack and I/O cases, exact architectural
  state, following-instruction consumption and bus accounting.
- Redundant/mixed prefixes through seven stay with their instruction. Tests
  verify pending IRQ/shadow behavior and bounded REP restart through IRQ/IRET.
  Existing interrupt-shadow, HALT, GDMA and SDMA scheduling regressions pass;
  the policy beyond the hardware-guaranteed prefix range is documented.
- The source-included original CC0 cartridge follows the normal footer/banking
  paths. Native and production Wasm/browser execution compare CPU-written values
  with independent literals on every model, with fresh restart counters.
- Complete native and production browser suites preserve model gates, audio/PCM,
  GDMA/SDMA/Hyper Voice, SRAM/cartridge and console EEPROM, startup, input,
  frame continuity, browser persistence and local ownership/provenance. No related
  subsystem, persistence implementation or product flow was changed.
- Every required fixture/demo, format, native test, lint, whitespace, Wasm/build
  and sequential production browser command passed. No mandatory check was skipped.
- CPU support, stale comments and fixture provenance record the old assumption,
  corrected requirement, primary source, inference and timing limits. Older NEC
  exclusions are reconciled without removing carry/direction support or silently
  resolving FPO1. No unrelated features, assets or framework were introduced.

No blocking findings remain. No remote merge action was requested or performed.

Optional another-browser-engine, physical-device and commercial-ROM checks were
not performed. Human gameplay is unnecessary for acceptance. Required post-merge
verification: none. This node does not push, open a PR, merge or comment.
