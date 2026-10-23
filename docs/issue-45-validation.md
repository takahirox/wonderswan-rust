# Issue #45 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`fd7783e54dfa32f82ef14157af7f5d0d38c78a96`; production source matched the
Issue's evaluation baseline `90ef271b149b95e3143d9459324d4820b8403d77`.
The implementation coordinator read live Issue #45 (open, zero comments) and
the repository development/review guidance. No remote Issue, comment, PR, branch
or merge was changed. This node commits locally without pushing. Required
post-merge verification: none.

## Source reconciliation

[The correction record](shift-rotate-flags.md) was written before production
behavior changed. The NEC V30MZ manual §3.1.5 was inspected through its PDF;
the pinned WSCPUTest report and assembly excerpts were read through the GitHub
connector after raw web requests failed. The manual leaves AF and multi-bit OF
unspecified; retention was an unsupported implementation assumption. Independent
console experiments supply deterministic rules, including masked-zero counts.
Direct inspected experiment coverage and application to other documented widths
and count encodings are distinguished in the record. No physical execution or
per-model measurement log was independently inspected. No external code was
copied, and no game-specific dispatch or fallback is introduced.

## Baseline failures actually executed

Before changing `Cpu::shift`, ran each command below against the original
production behavior. Every command selected one test and exited 101.

| Command after `cargo test -p ws-core --lib --locked` | Actual FLAGS / expected FLAGS |
| --- | --- |
| `cpu::shift_tests::zero_count_overflow_regression -- --exact` | SHR AL,0 (`C0 E8 00`), AL=40, incoming F002: F002 / F802. |
| `cpu::shift_tests::shift_auxiliary_carry_regression -- --exact` | SHL AL,1 (`D0 E0`), AL=01, incoming F012: F012 / F002. |
| `cpu::shift_tests::multi_bit_overflow_regression -- --exact` | SHL AL,2 (`C0 E0 02`), AL=20, incoming F002: F082 / F882. |

Results were correct; full-FLAGS assertions failed. The original CC0 cartridge
also failed natively before the correction:
`cargo test -p ws-core --test shift_rotate --locked` returned exit 101,
Mono boot 1 RAM word 0 = 0BAD instead of 600D, despite seeding stale pass/garbage
SRAM. This is actual baseline discrimination, not static reasoning. Baseline
commands stop at their first assertion; they do not establish baseline execution
on every model or in Chromium. Optional baseline Chromium comparison was not run.

## Implementation and independent coverage

The production diff changes only flag handling in the existing shared shift
helper. The unchanged per-bit loop still masks counts to five bits and computes
operands/last-bit CF. Its zero-iteration path retains CF. The common exit now
clears shift AF and calculates shift SZP even at zero; rotates retain AF/SZP.
Every count computes left OF from final CF XOR result MSB or right OF from the
two highest result bits. The old early return and count-one OF restriction are
removed. The ordinary D0–D3/C0–C1 decoder, operand helpers, access charges,
`2 + masked count` execution component and shared scheduler remain unchanged.
Undocumented /6 rejection, division and single-step delivery remain separate.

Six CPU tests add:

- Three separately executable baseline regressions, now passing.
- 224 independent literal input/count/result/full-FLAGS tuples with opposing
  F002/FFD7 incoming flags. These contrast every arithmetic flag and TF/IF/DF.
  Expectations were authored with whole integer and carry-ring arithmetic,
  independently of the CPU's per-bit loop; no expectation calls the core helper.
  The decoder matrix makes 23,100 executions across physical Mono, Color/Crystal
  in mono and color modes, all three count sources and five locations: register,
  aligned/odd RAM, aligned/odd byte-wide mapped SRAM. ES overrides a distinct DS.
  Byte results preserve AH. All registers, segments, RAM, SRAM, ports, HALT,
  interrupt shadow, exact IP, fetch/read/write charges, execution cost and the
  following MOV's decode/flags are checked; `Cpu::step` must not advance devices.
- Zero/nonzero operands, positive/negative sign boundaries, raw 0/32/64/128/224,
  1/2, width minus one, width, carry-ring boundaries, 31/33/63/255, and nonzero
  full rotations distinguished from masked-zero counts. Existing zero-count
  shift SZP cases remain, with corrected AF/OF expectations. Existing one-bit
  result/CF/OF vectors remain; old zero-rotate retention assertions now express
  the corrected left/right OF rules.
- Four explicit CL/CX destination alias and CH/AH decoding cases. CL is sampled
  before writing its destination; unrelated byte lanes retain their values.
- Repeated live mono/color/mono/packed-color transitions on all three physical
  model configurations. Exact costs, IP, access charges, results and full FLAGS
  are checked; each returned cost advances shared time exactly once, crossing
  scanlines. Physical Mono does not receive color-only port writes.

These are emulator regression assertions, not physical timing measurements.
The complete suite retains prefix, interrupt-shadow, HALT, bounded REP, BUSLOCK,
FPO1/POLL, multiplication and shared GDMA/SDMA scheduling regressions.

## Original cartridge and production browser path

The original source-included CC0-1.0 `fixtures/shift-rotate.asm` builds a 128 KiB
cartridge with 32 KiB SRAM, ordinary reset footer, bank entry and checksum. Each
mode pass runs 224 cases spanning all seven operations, both widths and all
three count sources, opposing incoming flags, zero/32, one/multiple bits,
width/carry-ring boundaries, 31 and 255. It captures actual whole AX, full FLAGS
and following ZF/CF/OF/SF/PF/signed-LT branch decisions. Branch collection restores
the captured flags before each Jcc. Each observation is compared inside the
cartridge and independently against literal native/browser expectations.

Mono runs one mode pass; Color/Crystal run mono, color, mono. Both native and
browser tests load poisoned retained observations containing a stale pass marker.
Every boot clears all volatile records, increments a retained count and
invalidates saved completion before probing. All 2,020 words, including unused
zero records on Mono, must match after load and fresh restart. Native tests
inspect RAM and CPU-written SRAM, stack/CS/HALT and framebuffer. Browser checks
use the production file chooser, Wasm core, rendering and actual SRAM flush;
ROM/save handling makes no subsequent network requests. Fresh counts and pixels
prevent retained observations satisfying restart assertions.

SHA-256: `038eb91f633183c0d6b0596bbc246cf59e743fb080f0b18519b85a90846bce59`.
All 29 earlier cartridges and the demo remain byte-identical. No earlier
fixture or unrelated acceptance assertion was removed or relaxed.

## Required checks actually executed

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 30 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 131 core tests, 20 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass after replacing a complex test-vector type with an alias. |
| `git diff --check` | Pass; repeated before commit. |
| `npm run build:wasm` in `web` | Pass using the installed Rustup stable toolchain PATH. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium compatibility suite, including the new probe on all three models and fresh restart. |

`npm ci` restored locked dependencies without lockfile changes. The first Wasm
build failed because Homebrew Rust lacked the Wasm target; selecting the already
installed Rustup stable toolchain resolved it without installing a compiler.
The first corrected focused CPU run found one obsolete zero-count rotate
retention expectation, now corrected to distinguish left/right OF. Clippy first
reported test-vector type complexity; the alias resolved it without suppressions.
Both production builds completed before the complete smoke suite; the complete
compatibility suite started after smoke passed, using this worktree's preview
at `http://127.0.0.1:4197/wonderswan-rust/` through `WS_PREVIEW_URL`.
JavaScript syntax checks passed for the probe and suite integration. No usage
reset, allowance purchase or model/provider switch occurred.

## Coordinator acceptance review

The implementation coordinator reviewed the production helper/decoder diff,
independent literals, original source/generated cartridge, native/browser
assertions, hardware documentation and provenance against every Issue checkbox.
No unrelated assertion or fixture/demo byte was changed. Both complete
production Chromium suites passed, including the new probe. Every mandatory
pre-merge criterion has supporting evidence; no blocking findings remain.

| Issue acceptance | Reviewed evidence |
| --- | --- |
| Source reconciliation and all forms | Old AF/OF assumptions, corrected zero/multi-bit semantics, pinned sources, direct/inferred applicability and evidence limits recorded before production edits; all seven operations and six count/width encodings use the same helper. |
| Independent result/full FLAGS vectors | 224 literal tuples, 23,100 decoded executions, opposing arithmetic/control flags, masked-zero and nonzero full rotations, sign/width/carry-ring boundaries, untouched state and explicit CL/CX/CH/AH cases. |
| Models, mode transitions and baseline discrimination | Three actual pre-fix flag failures and original native probe failure; corrected tests cover all three physical model configurations, five static combinations and repeated runtime video-mode transitions. These are emulated configurations, not physical-console runs. |
| Destinations, accounting and scheduling | Register and aligned/odd writable RAM/SRAM, ES overrides, exact continuation, fetch/read/write charges, unchanged execution components and exactly one shared-device advance; prior prefix/shadow/HALT/REP/BUSLOCK/FPO1/POLL/multiplication tests retained. |
| Original CC0 probe and fresh execution | All operations/widths/count sources, actual destination/full FLAGS/Jcc records, poisoned stale saves, fresh boot/completion, all 2,020 observations and restart on every model; native RAM/SRAM and actual production-browser SRAM comparisons. |
| Prior hardware/browser acceptance | Complete workspace and both complete production Chromium suites pass; compatibility checks retain firmware-free startup, physical-model gates, LCD, PCM/audio, GDMA/SDMA/Hyper Voice, SRAM, both EEPROM controllers, save integrity and input/frame continuity. All 29 earlier cartridges and demo files verified byte-identical. |
| Required commands | Both Python reproducibility checks, format, full workspace tests, Clippy, whitespace and both production builds pass; complete smoke passed before complete compatibility began, and complete compatibility passed. |
| Documentation, provenance and review | Hardware support, source reconciliation, original CC0 ownership/hash and actual validation/limits recorded; relative links and JavaScript syntax pass. No new fallback, proprietary dependency or unrelated production change. |

Physical-console comparison and commercial gameplay were not performed. The
source evidence remains an independent experiment report, not a measurement log
inspected here. The probe establishes bounded behavior, not commercial-catalog
compatibility. No proprietary ROM/firmware or additional browser engine was used.
Required post-merge verification: none.
