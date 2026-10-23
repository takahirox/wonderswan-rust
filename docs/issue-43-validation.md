# Issue #43 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`02aa4dd2054bff73f99c136b44e067a02fcb0b6c`, with production source matching the
Issue's evaluation baseline `d21e16af634bb3e6c73ab08b111a2bcc51fae465`.
The implementation coordinator read live Issue #43 (zero comments) and the
repository development/review guidance. No remote Issue, PR, branch or comment
was modified. This node commits locally without pushing or merging. Required
post-merge verification: none.

## Source reconciliation

[The correction record](multiplication-flags.md) was written before changing
production multiplication. The NEC V30MZ manual §3.1.5 was inspected through its
PDF; the pinned WSCPUTest README and assembly excerpts were read through the
GitHub connector. The manual leaves AF/PF/SF/ZF undefined; preservation was an
unsupported implementation assumption. The independent experiment supplies
console-specific deterministic flags. Its A0 physical-model selection establishes
unsigned ZF independently of graphics mode. Its immediate-word and sign-extended
byte tests resolve the earlier assessment's outstanding immediate prerequisite.

The corrected requirement covers F6/F7 /4 and /5 plus 69/6B, clearing AF/PF/SF;
unsigned ZF is clear on WonderSwan and set on Color/Crystal; signed ZF is set on
all three models. Products, destinations, CF/OF fit rules and control flags stay
unchanged. This evidence is an independent experiment reference, not an inspected
physical execution or per-model measurement log. No external implementation code
was copied. Division quirks, other arithmetic flags, single-step delivery and
absolute timing remain separate gaps. Prior acceptance is not weakened.

## Baseline discrimination actually executed

After correcting initial test-harness numeric inference and gating port 60 writes
on physical Mono, ran the following against unchanged production CPU code:

| Test command after `cargo test -p ws-core --lib --locked` | Actual failure (exit 101) |
| --- | --- |
| `cpu::multiply_tests::mul_full_flags_products_and_mapped_sources -- --exact` | Mono F6 E3, incoming FFD7: F7D6, expected F702. |
| `cpu::multiply_tests::imul_full_flags_products_and_mapped_sources -- --exact` | Mono F6 EB, incoming F002: F002, expected F042. |
| `immediate_imul_full_flags` | Mono 69 CB FF FF: F002, expected F042. |

`cargo test -p ws-core --test multiply --locked` also failed before the fix:
the original cartridge wrote 0BAD instead of 600D at RAM 0400 on Mono boot 1.
The test had deliberately supplied stale pass/garbage SRAM. Baseline tests stop
at their first failing assertion; this is not a claim of baseline execution on
every model, both immediate widths or in Chromium. A preliminary mistyped
`--exact` filter selected zero tests and establishes no coverage. The corrected
commands above selected and failed the intended tests. No baseline source
substitution or reset was needed.

## Implementation and independent coverage

Only the two existing multiplication decoder paths change production behavior.
Each clears AF/PF/SF and sets ZF as above, using `bus.model` for unsigned MUL.
The existing calculations, CF/OF writes, operand helpers, instruction lengths,
approximate execution components (byte/immediate 12, word group 20), scheduler
and public/browser APIs remain unchanged. No ROM-specific behavior, new fallback,
prefetch implementation or timing redesign is introduced.

Four CPU tests add 3,720 arithmetic vectors plus runtime transition/accounting
checks:

- Nine unsigned and fifteen signed literal product/full-status vectors, each on
  five model/mode combinations, two opposing incoming full-FLAGS patterns and
  seven source locations (1,680 executions). Locations are registers, aligned/odd
  RAM, aligned/odd byte-wide mapped SRAM and aligned/odd mapped ROM with live
  A0 set to a word bus. ES overrides a distinct DS. Cases cover zero/nonzero,
  negative operands, positive/negative fit boundaries, high halves and overflow.
- Seventeen immediate vectors × five combinations × two incoming flag patterns
  × four destination/register-alias configurations × three register/aligned/odd
  RAM sources (2,040 executions). Both immediate widths, negative immediates,
  6B sign extension (80 and FF), AX/DX/BX aliases, untouched registers, segments,
  memory, IP and separate bus/execution costs are checked.
- Both incoming patterns contrast every arithmetic flag and TF/IF/DF, with full
  expected control/status words. Group tests check all registers, segments,
  ports, RAM and SRAM, unchanged HALT, exact continuation and no device advance
  inside `Cpu::step`. Expectations are authored literals, not core-derived flags.
- Repeated live mono/color/mono/packed-color transitions use physical-model ZF.
  Literal byte/signed-word/immediate costs, fetch charges, IP, products and flags
  are checked. Each returned cost advances the machine exactly once, including
  crossing a scanline after six mode passes. Existing GDMA/SDMA shared-clock,
  prefix/interrupt-shadow, HALT, bounded REP, BUSLOCK and FPO1/POLL assertions
  remain unchanged and pass in the complete workspace suite.

These are emulator regression assertions, not measured physical timings.

## Original cartridge and production browser path

The original CC0-1.0 `fixtures/multiply.asm` and generated 128 KiB cartridge
execute all six multiplication forms through normal footer/reset banking.
They include positive/negative and overflowing results, both immediate widths,
6B -128 sign extension and aliasing, and aligned/odd ES memory sources.
CPU-written observations include actual destinations, DX, full FLAGS and
subsequent conditional-branch decisions for ZF/CF/OF. Immediate AX/source
preservation and stack balance are also checked inside the cartridge.

Mono runs one mode pass; Color/Crystal run mono, color, then mono again. Every
pass repeats all eight records. Native assertions independently inspect all 100
RAM and SRAM words, stack/CS/HALT and framebuffer on each model and restart.
The production compatibility test uses the actual file chooser, production Wasm,
player rendering and actual SRAM flushes. Both tests supply a stale pass marker
with poisoned observations. Boot clears volatile observations, increments the
retained count and invalidates saved completion before multiplication. Each boot
must produce a fresh counter, completion and every expected observation; unused
Mono records must be zero. Browser ROM/save handling produces no subsequent
network requests.

SHA-256: `e76afc9641cb24312f3ccddd3c0689d2c097c884729355711ea96166549d52e5`.
All 28 earlier cartridges and the demo remain byte-identical.

## Required checks actually executed

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 29 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 125 core tests, 19 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass, zero warnings. |
| `git diff --check` | Pass; repeated before commit. |
| `npm run build:wasm` in `web` | Pass using the installed Rustup stable toolchain PATH. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium compatibility suite, including the new probe on every model and fresh restart. |

`npm ci` restored locked dependencies without lockfile changes. Both production
builds completed before the smoke suite, and the full compatibility suite started
after the full smoke suite passed. The preview URL is this worktree's
`http://127.0.0.1:4197/wonderswan-rust/`, selected through `WS_PREVIEW_URL`.
JavaScript syntax checks passed for the new probe and suite integration. No
compiler installation, usage reset, allowance purchase or model/provider switch
occurred.

## Coordinator acceptance review

The implementation coordinator reviewed the production decoder diff, independent
literal vectors, source/generated probe, native/browser assertions, support docs
and provenance against every pre-merge checkbox. No existing acceptance
assertions were removed or relaxed. Both complete Chromium suites passed; no
blocking findings remain.

| Issue acceptance | Reviewed evidence |
| --- | --- |
| Source reconciliation and all forms | Both erroneous assumptions, pinned manual/report/source, physical-model applicability and experimental limits recorded before production edits. Both existing decoder paths updated, covering F6/F7 /4,/5 and 69/6B. |
| Independent products/full flags | Literal zero/nonzero, signed boundaries/overflow, byte/word and both immediate widths, control flags, untouched state and alias vectors pass across five combinations. |
| Physical models, mode transitions, baseline | All three models and five combinations, repeated runtime transitions, and actual pre-fix unsigned/signed/immediate failures recorded. |
| Memory/decoding/accounting/scheduling | Register, RAM, SRAM, ROM, aligned/odd and ES override sources; exact IP, separate access/execution costs and single shared-clock advance pass. Prefix/shadow/HALT/REP/BUSLOCK/FPO1/POLL assertions retained. |
| Original CC0 probe and fresh completion | All forms in applicable modes, actual products/full flags/conditional branches, poisoned stale saves and independent fresh boot/completion comparisons pass natively and in production Chromium on every model and restart. |
| Prior hardware/browser acceptance | Full workspace and both complete production Chromium suites pass, retaining firmware-free/model gates, LCD, exact PCM/audio, GDMA/SDMA/Hyper Voice, SRAM, both EEPROM controllers, browser save integrity and input/frame continuity. Existing fixture/demo bytes verified unchanged. |
| Required commands | All listed Python, format, workspace test, Clippy, whitespace, Wasm build, production build and sequential full Chromium commands passed. |
| Documentation/provenance/review | Support/gap list, source record, CC0 provenance/hash, baseline failures, actual validation and optional limits recorded. Relative file targets and JavaScript syntax checks pass. |

Physical-console comparison, optional expanded reference calculations and
commercial gameplay were not performed. No proprietary ROM/firmware or additional browser engine
was used. The probe establishes bounded behavior, not commercial-catalog
compatibility. No required post-merge verification.
