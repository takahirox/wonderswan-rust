# Issue #47 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`b6bd482ced8395226dfcb260a2fca042d570ff8e`; production source matched evaluation
baseline `2669967f37dbbb65410ce8dcc496c1d1689a1693`. Read live Issue #47 (open,
zero comments) with `gh issue view`, and the repository development/review
instructions. No remote Issue, comment, PR, push or merge was performed. Changes
are committed locally in the assigned artifact worktree. Required post-merge
verification: none.

## Source reconciliation and scope

[The correction record](word-division-flags.md) identifies the erroneous retention
assumption, corrected successful-word rules, retained OF requirement, model/mode
applicability and experimental limits. The NEC manual §3.1.5 was read through
its PDF. Web requests for the pinned WSCPUTest report and assembly failed with
cache misses; `gh api` read the pinned README and calculator excerpts successfully.
One initial unquoted README API query failed with zsh glob expansion; quoting
the path resolved it. No source or cartridge bytes were copied.

A closer inspection of the signed calculator's pre-loop bound showed that
quotient magnitude 8000 belongs to its exceptional path. An initially drafted
−32768 / 1 success vector was therefore replaced by −32766 / 1; −32767 / 1
also covers the negative successful limit. The existing emulator's exceptional
classification was not changed. The final new vectors avoid establishing signed
exceptional results, consistent with the Issue's scope. This corrects our draft
coverage, not the Issue's expected successful-word flag rule.

## Actual baseline discrimination

These commands were executed before changing the production success path:

| Command | Actual failure (exit 101) |
| --- | --- |
| `cargo test -p ws-core --lib --locked cpu::divide_tests::issue_47_reproduction -- --exact` | F7 F3, DX:AX=0000:0006, BX=0002, incoming F002: quotient 3/remainder 0 passed, FLAGS F002 differed from F042. |
| `cargo test -p ws-core --lib --locked cpu::divide_tests::word_division_full_flags_results_mapped_sources_and_branches -- --exact` | Mono zero dividend under FFD7: actual F7D7 differed from F702. |
| `cargo test -p ws-core --test divide --locked` | Original CC0 probe on Mono boot 1 wrote RAM 0400=0BAD instead of 600D, despite seeded stale-success SRAM. |

Baseline tests stop at the first failed assertion; they do not establish a
baseline failure on every emulator model. Optional baseline production-Chromium
discrimination was not run. All three corresponding corrected checks passed.
The boundary-vector refinement above was made after initial corrected native
passes; it does not affect the baseline failures, which occur earlier.

## Implementation and coverage

The only production change adds arithmetic flag handling inside the existing
successful **word** branch: clear AF/CF/PF/SF/ZF, then set ZF for remainder zero
and odd quotient. The common successful OF clear remains. Neither byte division
nor the fault branch changes. The decoder, operand helpers, quotient/remainder
calculation, execution charge and shared scheduler are unchanged.

Four new CPU tests provide:

- The exact Issue reproduction.
- 26 independent literal arithmetic vectors, opposing incoming F002/FFD7 flags,
  five static model/mode combinations, and seven divisor locations: BX and
  aligned/odd RAM, ROM and SRAM with ES overriding distinct DS. This makes
  1,820 decoded division executions. Full literal FLAGS expectations include
  preserved TF/IF/DF, and never call the production flag helper. Zero, odd/even,
  exact/inexact, high-word unsigned, signed sign combinations, negative odd exact
  quotients and unambiguous successful boundaries are included.
- Complete registers, segments, RAM, ROM, saves, ports, HALT/shadow state, exact
  IP and following MOV/JZ/JC/JO/JS/JP/JL decoding/decisions. Each CPU step retains
  the existing 32-clock execution component and exact fetch/read charges and
  does not itself advance devices. Explicit DIV AX, IDIV AX and IDIV DX cases
  verify sampling before alias writeback. Successful unsigned DIV DX is
  impossible because its dividend high word equals its divisor.
- Live mono/color/mono/packed-color/mono transitions on all three emulated
  models, for both forms. A real `Machine::run_frame` call with one division
  remaining before frame completion advances shared devices exactly 34 clocks,
  completes exactly one frame, reaches raster line zero and continues at the
  next instruction. Mono does not receive color-only port writes.

The original source-included CC0 `divide.ws` probe uses 26 vectors with two
incoming FLAGS patterns, making 52 operations per mode pass. It captures actual
CPU AX, DX, full FLAGS and Z/C/O/S/P/signed-LT branches. It runs once on Mono and
mono → color → mono on Color/Crystal. Native tests compare all 628 RAM and SRAM
words against independent literals, stack/CS/HALT and pass pixels, on every model
and restart. Browser tests use actual file selection, compiled production Wasm,
rendering, save flush and localStorage, checking every saved word. Poisoned
retained observations include an old success marker; boot clearing, completion
invalidation and fresh counters ensure only new execution satisfies acceptance.
ROM/save handling stays local. Final probe SHA-256:
`49ff7cbcf107360d71abf5d330a2f251da458b4b73b546c55dc879a309f1c290`.

## Required commands

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 31 cartridges; all 30 earlier images byte-identical. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 135 core tests, 21 integration tests, one adapter test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` in `web` | Pass with the installed Rustup stable toolchain on PATH. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete production Chromium smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium compatibility suite, final word-division probe on all three models and restart. |

`npm ci` restored locked dependencies without lockfile changes. The Wasm build
used `PATH="$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`
to select the already installed toolchain with its Wasm target. Both production
builds completed before browser tests. After discovering the signed-boundary
coverage issue, the first compatibility run was deliberately interrupted with
SIGINT (exit 1), after the save/audio/DMA/startup checks had passed. It had not
reported a division assertion failure. The complete suite was restarted after
updating the original cartridge and its native/browser literal expectations.
The CPU production code and compiled Wasm were unchanged by this test-only
refinement; the complete native workspace and reproducibility checks were rerun.
The complete smoke suite passed before the complete compatibility suite started against this worktree's production
preview, `http://127.0.0.1:4197/wonderswan-rust/`, using `WS_PREVIEW_URL`.
JavaScript syntax and relative documentation links were checked. No usage reset,
allowance purchase or model/provider switch occurred.

## Coordinator acceptance review

The implementation coordinator reviewed the final production diff, literals,
original source/generated cartridge, native and browser assertions, provenance
and documentation against every Issue checkbox. All seven criteria are satisfied
by the recorded automated evidence. The final complete compatibility run exited
zero, including the final probe on all three models and restart. All earlier
fixture bytes and prior assertions remain intact.

| Issue acceptance | Reviewed evidence |
| --- | --- |
| Correction, sources, both forms and scope | Unsupported flag retention replaced by the experiment's exact successful-word rule; OF remains cleared. The common decoder handles F7 /6 and /7. Byte division and fault classification are unchanged. Model applicability, inference and physical-evidence limits are explicit. |
| Literal results and full FLAGS | 26 literal vectors under opposite incoming arithmetic/control flags cover zero, odd/even, exact/inexact, high unsigned words, signed combinations, negative odd exact quotients and unambiguous success boundaries. No expectation calls the production helper. |
| Operands, state, branches and accounting | BX/AX/DX aliases and aligned/odd RAM/ROM/SRAM with ES overrides, complete state/source preservation, exact IP and following branches, unchanged access/execution charges, and normal machine shared-device advancement are asserted. |
| Models, modes, baseline and regressions | Three emulated models, five static combinations and live mode transitions; actual pre-fix reproduction/matrix/probe failures followed by native passes. Full workspace tests retain divide faults, byte division, multiplication, shifts, prefixes, interrupts, HALT and DMA. |
| Original probe and fresh execution | Source-included CC0 probe; actual CPU quotient/remainder/full FLAGS/six branches; poisoned stale completion, fresh counts and all 628 words checked on all models and restart, natively and in production Chromium/Wasm. |
| Required commands and prior acceptance | All required native/build checks and both complete production Chromium suites passed, sequentially with smoke first. Existing three-model/startup/video/input/frame/audio/PCM/GDMA/SDMA/Hyper Voice/SRAM/EEPROM/browser-save acceptance remains intact. |
| Documentation, provenance and final review | Hardware support, pinned references, corrected assumption, generated cartridge ownership/hash, actual failures, resolution and passes are recorded. Optional physical-console and commercial evidence is explicitly unperformed. |

Physical-console measurements, per-model physical logs, commercial gameplay,
additional browser engines, optional baseline Chromium discrimination and
additional proprietary software were not performed. This evidence is bounded
emulator acceptance and does not establish commercial-catalog compatibility or
complete CPU readiness. Byte flags, signed exceptional results, fault refinements,
decimal adjustment and absolute timing remain unresolved work. Required
post-merge verification: none.
