# Issue #85 validation and criterion review

## Scope, source and correction

Implemented against evaluation baseline
`07b1d1f17ff32dabc6295206dda1d9fe026bb479`, in the assigned GitWeave worktree
starting at `35ef291580235e72abf2c2eca1adc9620ccd272f`. The live
[Issue #85](https://github.com/takahirox/wonderswan-rust/issues/85) was read using
the GitHub connector. Validation below was actually executed on 2026-10-24;
this report is separate from historical Issue #43/#65/#83 reports.

The former AI-authored assumption retained 12/20 residual clocks for byte/word
MUL/IMUL and 12 for immediate IMUL. The NEC V30MZ preliminary manual,
A13761EJ1V1UM00, Appendix A printed pp. 73/79, instead gives decoded
register/memory costs 3/4. Subtracting the already charged aligned operand read
implies **three residual clocks for every form** in this serialized scheduler.
[The accounting document](multiplication-timing.md) links the primary manual,
records its conditions and separates that inference from full pipeline fidelity.
The pinned WSTimingTest report was also inspected; aggregate loop measurements
are not transplanted as isolated instruction costs. No further primary-evidence
change to the Issue's acceptance was needed.

The runtime change replaces two timing assignments. Product calculation,
complete FLAGS, operand sampling, bus methods, multiplication overflow history
and continuation remain unchanged. One existing CPU test's assumption that
register MUL must exceed a memory MOV is replaced with an independently derived
five-clock assertion. Its other assertions remain. No external write, push,
Issue comment, PR or merge was performed for this node.

## Actual pre-edit discrimination and resolved failures

Literal timing/state oracles were added before changing production charges.
With the former implementation, the first focused run returned 101 and failed
all four new tests **after products, full FLAGS and exact IP passed**:

| Bytes, at even internal address | Required clocks | Baseline observed |
| --- | ---: | ---: |
| F6 E3, MUL BL | 5 | 14 |
| F7 EB, IMUL BX | 5 | 22 |
| 69 DB FE FF, IMUL BX,BX,-2 | 6 | 15 |
| 6B DB FE, IMUL BX,BX,-2 | 6 | 15 |

The expanded pre-edit run returned 101 with all 15 selected multiplication tests
failing. It includes literal memory/alias vectors and mapped-bus/shared-device
checks. After correcting the two assignments and completing coverage, all **16**
final focused multiplication tests pass. Expectations use literals or independent
whole-integer/address arithmetic, not production timing helpers.

Two first-pass test literals were corrected: signed `8081 * 8081` produces
`DX:AX=3F7F:4101`, and successful immediate IMUL must set ZF (`F042`), including
its DMA continuation check. These fixes changed neither runtime nor tolerances.
The first full workspace run then rejected the older `memory MOV < register MUL`
assertion. From its actual bytes, `A1 00 08` and `F7 E0` each cost five clocks.
Only the affected MUL assertion was replaced; final workspace checks pass.

A log-comparison script initially selected same-format shift records as well as
multiplication records. Filtering by this probe's 18/126-record dimensions fixed
the parser; this did not change observations, consumers or acceptance bounds.

## CPU, bus and shared-device evidence

The retained independent products/full-FLAGS vectors now exercise all nine unique
model/mode configurations: Mono 00; Color and Crystal 00/80/C0/E0. They include
all six forms, zero, fit/overflow and signed boundaries, both immediate widths,
6B sign extension, contrasting F002/FFD7 incoming flags and physical-model unsigned
ZF. Every register, segments, IP, RAM, mapped SRAM and actual following instruction
are checked in the arithmetic matrices. AL/AH/AX/DX source aliases additionally
check complete registers/segments, ports, untouched RAM and real MOV continuation.
Immediate source/destination aliases include BX/BX, AX/AX and DX/DX.

New exact accounting checks cover register and mapped RAM/SRAM/ROM sources,
aligned/odd words, mapped instruction fetches at both parities, live A0
00/04/08/0C and existing SRAM waits. DS/SS defaults, independent ES/SS overrides,
last segment override, byte/word displacements and up to seven mixed segment and
BUSLOCK prefixes are exercised. Charged accesses prove exactly one source read
and no source writes; RAM/SRAM/ROM snapshots detect unrelated mutations. REP
retains its diagnostic, two consumed bytes, charged fetches and unchanged state.

Boundary coverage includes operand/instruction offset wrapping, bank/window and
physical 20-bit wrapping and RAM availability. All six forms cross literal
instruction-fetch boundaries; independent operand arithmetic covers every form
at RAM availability edges and RAM/SRAM/ROM boundaries. Existing bus policy,
including unavailable RAM reads and word consolidation eligibility, is retained.

Normal `Machine::run_frame` runs representative byte, word, both immediate widths
and odd memory multiplication across a frame/raster boundary, with exact returned
elapsed clocks and one-clock overshoot. Assertions cover frame number, raster,
HBlank timer decrement, one stereo pair, products/full FLAGS, unchanged source
RAM and a poison INC BX continuation that must not execute. Live returns to 00
are included. Separate tests retain IF shadows, TF/priority IRQ with separate
25/32-clock entry charges, exact saved FLAGS/IP and actual IRET continuation.
Seven-clock GDMA/SDMA reservations fetch no instruction and preserve CPU state;
Mono rejection, GDMA color-mode gating and Color/Crystal SDMA in 00 remain.

The existing Issue #65 tests remain enabled for all explicit history producers,
replacement, independence from live FLAGS, later byte DIV, traps and CPU/cartridge/
model/restart lifecycle. REP/HALT and all unrelated CPU/device acceptance remain
in the full suites.

## Original cartridge and independent native/Chromium controls

The source-included CC0 cartridge and independent consumers use eighteen samples:
all six forms, each through registers, even RAM and odd RAM. Each repetition
reloads operands and FLAGS. The CPU writes AX, DX, complete FLAGS, 1024 executions,
zero remaining count, preserved source, stack/continuation and completion.
Both consumers re-poison retained observations before each of two boots, inspect
early completion invalidation and require fresh completion. Native checks the
entire RAM/SRAM mirror; Chromium checks persisted data and local ROM/save handling
through the real production player. Neither consumer writes successful records.

[Independent emitted-instruction sums](multiplication-timing.md) establish
`22 + 1024*(33 + multiplication clocks)` per interval. Exact 256-clock timer
floor/ceiling bounds follow without added tolerance. The same final cartridge
SHA-256, **f808fb8ef8bdf3f1d39dfd601f9206c130bac157646d20f898f4da2bbfa1ad4e**,
is used with old and corrected charges in both consumers.

Temporary old-charge source edits were restored with Python `finally` before
final builds. No production timing switch or testing-only runtime option was
added. Old native returned 101 and old production Chromium returned 1 **at their
final accumulated timing assertion**. Each collected all **540 timing failures**
(18 samples * 15 model/pass configurations * 2 boots), with all architectural
checks and all six model/boot records completed. Their complete observations
match exactly. Corrected native and complete production Chromium pass all 540
records, with complete observation-table equality.

Actual ranges across all passes and boots, in register/even/odd sample order:

| Form | Old native and Chromium | Corrected native and Chromium |
| --- | --- | --- |
| Byte MUL | 188 / 192 / 192–193 | 152 / 156 / 156–157 |
| Word MUL | 220 / 224–225 / 228 | 152 / 156–157 / 160 |
| Byte IMUL | 188–189 / 192 / 192 | 152–153 / 156 / 156 |
| Word IMUL | 220 / 224 / 228–229 | 152 / 156 / 160–161 |
| 69 IMUL | 192 / 196 / 200–201 | 156 / 160 / 164–165 |
| 6B IMUL | 192 / 196 / 200–201 | 156 / 160 / 164–165 |

Complete tables are canonically serialized as compact JSON with sorted lowercase
`model:boot` keys, arrays in pass/sample order. Their SHA-256 identities are:

| Charges | Samples per consumer | Observation-table SHA-256 |
| --- | ---: | --- |
| Former | 540, all timings rejected | `ad52623decc63dd59d4a27f1a8056875135e8fcedf9024acea68dfced5994a9a` |
| Corrected | 540, all timings/state accepted | `76f6641a38ca7ff27bd4afaa9ad248393ecec8c6cd695c3c945b912d6e85631f` |

The old production Wasm SHA-256 is
`ae816fc305a237d02a8cd5932bdf8baf610c53cd3182e8cf9e341d78bb4c7ead`;
its `index-DTpDalcZ.js` is
`16746d6b738071f7840936603f5751df40705d4a411fa702bae3480a9027ac17`.
Corrected source/artifact identities are listed below. Full raw logs and
pkg/dist builds are ignored local artifacts in `target/issue-85-validation`;
this report preserves durable observations, sums, identities and dispositions.

## Existing interval audit and mandatory commands

Measured instruction bodies in existing cartridges contain no multiplication;
setup and multiplication/history-only uses have no absolute multiply interval.
Only the older CPU ordering assertion above needed recalculation. No prior
cartridge budget or tolerance was widened. All 45 prior fixture images and the
demo bytes are unchanged; historical reports and unrelated assertions remain.

All commands ran in the assigned worktree; npm commands ran in `web`. `npm ci`
passed without changing manifests/locks. Wasm builds used the already installed
stable Rust toolchain's Wasm target by prepending its bin directory to PATH.
No toolchain/package configuration, provider/model, usage reset or allowance
purchase was changed. The final production build was served with
`npx vite preview --host 127.0.0.1 --port 4187 --strictPort` at
`http://127.0.0.1:4187/wonderswan-rust/`. `WS_PREVIEW_URL` selected that explicit
production URL. Complete smoke then complete compatibility ran sequentially.

| Mandatory command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 46 images reproduce, including the new cartridge. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 269 core unit + 42 integration + 1 web unit = **312**; empty doc tests. |
| Focused `multiply_timing` native consumer with `--nocapture` | Pass: all 540 observations compared with production Chromium. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` | Pass for old-charge control and final corrected production artifacts. |
| `npm run build` | Pass for old-charge control and final corrected production artifacts. |
| `npm run test:browser` | Complete production Chromium pass: all models, demo playthrough, lifecycle/loading/races, dialogs/fullscreen, keyboard/touch. |
| `npm run test:compat` | Complete production Chromium pass, including final multiplication consumer and all existing CPU, timing, device and save consumers. |

Compatibility includes prior shift/control-flow/CALL/return/IRQ acceptance,
firmware-free startup and model gates, LCD/input/frame continuity, stereo audio/
PCM/sweep, GDMA/SDMA/Hyper Voice, SRAM, both EEPROM controllers, RTC, browser save
integrity, storage denial/corruption/full behavior and ownership/provenance. No
commercial-title coverage is inferred from these original probes.

## Final source and artifact identities

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `845d4e3ba485b758f87dc4d8d27b63d36ca0481491ecb631ce14723bf540e02d` |
| `crates/ws-core/src/cpu/multiply_tests.rs` | `be5866883e2f17dee2b6f140c637b7758d44de7e170f3ab0fcdf6fbf39d91500` |
| `crates/ws-core/tests/multiply_timing.rs` | `b817ef6759773d8f69cea0ba1cf7f4312709bd569e3ecb6cdcb28fd6757f5638` |
| `fixtures/multiply-timing.asm` | `b0d53e36fdaef2929d586c010d864cb5391982cd3c265804902fe7732df2cc0e` |
| `fixtures/generated/multiply-timing.ws` | `f808fb8ef8bdf3f1d39dfd601f9206c130bac157646d20f898f4da2bbfa1ad4e` |
| `fixtures/build.py` | `f5d92c8f7d573be856953d6d2024cb0db190ac96181491f08d4df8e5ec7178d1` |
| `web/scripts/multiply-timing.mjs` | `87870a1f996cdd2718f8ce7d8c8341d4d2e729d15f455c6d4570606489e30f50` |
| `web/scripts/compat.mjs` | `f83e3c894dd226bcd8ac96c6a2bf5250e48fc19d4157bbac4aa1e22950119862` |
| `docs/hardware-subset.md` | `46662017a60e263a3ba4f1236c8743e5fce3e2982845ab2e0595d5e8ca5384a3` |
| `docs/multiplication-flags.md` | `98f49b442488b0c05404139c3a4b58ed1bfe7b5334cdff3ed4dc65ec5f34a2a7` |
| `docs/multiplication-timing.md` | `d2184e911ef94a4086f8077b43c41f6f3a2c758eeabaebe4b0b3436aed6ae72c` |
| `fixtures/README.md` | `441742f57f03be7eaab758165ca52673ecb36eba05b134a75a82e254140b846f` |
| `web/pkg/ws_web_bg.wasm` | `b2c6739069777fbfe151c0ffccf6c1a8de01102c7274b9c1dd7e35922ca6a4ab` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/index-DFN4UzJ8.js` | `37916394ec1374036da598d39af34caaa3bf17df92a44408ae4fd74c0312f6c8` |
| `web/dist/assets/ws_web_bg-qOBj2spW.wasm` | `b2c6739069777fbfe151c0ffccf6c1a8de01102c7274b9c1dd7e35922ca6a4ab` |

## Coordinator criterion review

The implementing Coordinator reviewed every Issue checkbox against the final
diff, assembly, independent oracles, complete control observations and executed
checks. No separate agent or human approval was required by this task.

| Issue acceptance | Evidence and disposition |
| --- | --- |
| Pre-edit independent literals and actual failures/passes | Four literal baseline failures after state passes; expanded pre-edit 15 failures; final 16 focused CPU tests pass, including exact 5/6 register and 6/7 memory totals. |
| All six forms and architectural preservation | Retained independent products/full FLAGS, opposing incoming flags, fit/overflow/zero/signed boundaries, immediate widths/sign extension, model ZF, aliases, untouched state/memory/IP and actual continuation; existing history production/replacement/FLAGS independence/DIV/lifecycle tests pass. |
| Exact bus accounting | Register/mapped RAM/SRAM/ROM, both fetch parities, aligned/odd sources, DS/SS/overrides/displacements, seven mixed prefixes, all live A0 states/SRAM waits, offsets/banks/20-bit/RAM availability, one source read/no writes and retained REP diagnostics pass. |
| Model/mode and shared scheduler | All nine unique configurations plus live returns to 00; normal run_frame frame/raster/timer/stereo and poison continuation checks; IF/TF/IRQ separate entries, REP/HALT and capability-gated DMA reservations pass. |
| Original probe and independent consumers | CC0 source/artifact/provenance; all six forms with register/even/odd memory, independent absolute bounds, products/full FLAGS/counts/source/continuation/fresh completion; poisoned records before both boots. Identical final bytes produce 540 old timing failures in each consumer and 540 corrected passes, with matching complete tables. |
| Existing intervals and broad acceptance | Audited emitted instruction bodies; only affected CPU relative assertion recalculated from bytes. No cartridge interval/tolerance change, old fixture/demo/history files preserved. Complete production compatibility and workspace acceptance pass. |
| Mandatory commands | All required commands above run and pass; browser suites run sequentially against final production artifacts. |
| Documentation, identities, evidence and review | Hardware subset, multiplication rationale/correction, provenance and this evidence updated; actual failures/resolutions, complete ranges/table hashes, tested identities and limitations recorded. Final diff, links and checkbox mapping reviewed; no blocking finding remains. |

No mandatory check was skipped or waived. Physical-console measurements, human
playtesting, additional browser engines and commercial/lawful external workloads
were not performed. Commercial-title impact remains unmeasured. Full pipeline/
prefetch fidelity, effective-address hazards, division timing and unrelated bus
corrections remain separate gaps. Required post-merge verification: **None**.
