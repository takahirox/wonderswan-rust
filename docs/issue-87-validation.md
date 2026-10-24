# Issue #87 validation and criterion review

## Scope and source reconciliation

Implemented in the assigned GitWeave worktree starting at
`5739a252e705d05aa2e7a908a1ed40f6944a355b`, against evaluation baseline
`e08ee15715a9380d842623c72f6dfc00852d27f7`. The live
[Issue #87](https://github.com/takahirox/wonderswan-rust/issues/87) was read with
the GitHub connector. Checks below were executed on 2026-10-24. No push, Issue
write/comment, PR or merge was performed.

The former AI-authored requirement retained width-only 24/32 residual clocks
for successful division. The primary NEC V30MZ preliminary manual,
A13761EJ1V1UM00, Appendix A printed pp. 73/77–78, instead gives decoded register
costs 15/23 unsigned and 17/24 signed, with one normal aligned transfer added
for memory. Using the register costs as residuals subtracts the operand transfer
already charged by the bus. [Accounting and conditions](division-timing.md)
separate this scheduler inference from complete pipeline fidelity. The primary
manual and pinned WSTimingTest/WSCPUTest reports were inspected; the latter
reports retain their aggregate-loop and exceptional-division limitations.
No further evidence-based acceptance correction was necessary.

Production changes only successful charge selection and moves the existing fault
assignment into the fault branch. Quotient/remainder, full FLAGS, sampling,
multiplication history, faults, bus operations and continuation are unchanged.
The earlier Issue #47/#49/#65 architectural acceptance remains enabled and is
expanded to all nine unique model/mode configurations. Their historical reports
are preserved, with superseding timing notices in the active reference documents.

## Actual failures, corrected passes and resolutions

Four independent literal timing/full-state tests were added **before changing
production charges**. The baseline run exited 101 and failed all four after
registers, full FLAGS, segments, exact IP, memory/ports and fetch-count checks
passed:

| Internal-RAM bytes | Baseline observed | Literal required |
| --- | ---: | ---: |
| F6 F3, DIV BL | 26 | 17 |
| F6 FB, IDIV BL | 26 | 19 |
| F7 F3, DIV BX | 34 | 25 |
| F7 FB, IDIV BX | 34 | 26 |

The same tests include even/odd [BX] literal totals 18/18, 20/20, 26/27 and
27/28, respectively, and an actual following MOV. All pass with the correction.
The final focused selection `cargo test -p ws-core --lib divide --locked`
passes **28 tests**. Expectations never call production timing helpers.

One first-pass boundary test incorrectly assumed every source's absolute signed
magnitude exceeded seven. Unavailable RAM returns FF, which is signed −1,
so signed 7/FF produces −7, not quotient zero/remainder seven. The rejected test
was corrected to derive the actual mapped divisor from independent snapshots
and whole-integer arithmetic, including literal full-FLAGS outcomes. No runtime
or timing bound changed. An initial draft cartridge budget counted the loop
excluding division as 37 clocks; inspecting emitted bytes established 36 before
controls ran. Final bounds in both consumers use the documented 36-clock sum.

The first Wasm build attempt selected Homebrew Rust, whose sysroot lacks the
Wasm target, and failed before compilation. Prepending the already installed
stable Rust toolchain's bin directory to PATH selected its installed target;
old-control and final builds then passed. No toolchain or dependency configuration
was changed. Initial web reads of the two pinned GitHub reports missed cache;
the GitHub connector read both exact pinned files successfully.

## CPU, bus, scheduler and fault evidence

The retained arithmetic matrices cover contrasting F002/FFD7 FLAGS, zero/nonzero
remainders, odd/even quotients, unsigned high-word dividends, signed operand
combinations and unambiguous ±127/±32767 successful boundaries. They assert
quotient/remainder, complete registers/segments/FLAGS, exact IP, actual MOV and
conditional continuations, unchanged memory/ports and private multiplication
history behavior. Valid AL/AH/AX/DX aliases remain sampled before writeback.
Unsigned byte DIV AH remains a fault regression, since it cannot succeed.

Exact accounting tests cover registers, internal RAM, SRAM and ROM, even/odd
operand and fetch addresses, live A0 00/04/08/0C and existing SRAM waits. They
exercise DS/SS defaults with distinct segments, overrides, byte/word
displacements, last override selection and up to seven mixed segment/BUSLOCK
prefixes. Charged accesses equal fetches plus one source read; full RAM/SRAM/ROM
snapshots reject writes and duplicate source accesses. REP F2/F3 diagnostics,
consumed bytes and unchanged state/history remain checked.

Boundary cases cover offset wrapping, RAM availability edges, SRAM/window and
ROM bank boundaries, and physical 20-bit wrapping, for all four forms. Mapped
fetches are tested at both parities and across offset/window/physical boundaries.
Existing unavailable-memory and transfer-consolidation policies are retained.

Discriminating zero-divisor and unsigned/signed overflow cases preserve baseline
31/39-clock register fault totals, all registers except stack adjustment, exact
saved restart IP/CS/full FLAGS, handler FLAGS, history under both incoming values,
unchanged memory outside the saved frame and actual IRET restart. The same new
fault-policy test was executed under temporarily restored former success charges
and passed; it also passes in the corrected focused/full suites. Temporary source
edits were restored with Python `finally`. Fault residual selection is explicitly
separate from success. These are baseline regressions, not proof of hardware
fault classification, exceptional FLAGS, continuation or exact timing.

Normal `Machine::run_frame` tests run all four register forms and odd word memory
forms across raster/frame boundaries with one-clock overshoot. Exact elapsed
clocks, frame increment, raster line/phase, HBlank timer decrement, one stereo
sample pair, results/full FLAGS and unchanged source memory are checked. A poison
INC BX at the continuation must not execute. Mono stays in 00; Color/SwanCrystal
exercise 00/80/00/C0/00/E0/00, including live mode returns.

Separate tests preserve IF shadows, TF/priority IRQ delivery with 25/32 entry
clocks accounted separately, exact saved frames and actual 11-clock aligned
IRET continuation. Seven-clock GDMA/SDMA reservations fetch no instruction and
preserve CPU state; Mono rejection, color-mode GDMA gating and Color/Crystal
SDMA in 00 remain covered. Existing REP/HALT acceptance remains in the full suites.

## Original cartridge and native/production Chromium controls

The original source-included CC0 cartridge uses existing fixture tooling and
contains no external program/ROM/firmware/assets/device identity. Every one of
its twelve samples reloads valid dividend, high word, divisor/address and FLAGS
for each repetition. Independent native and browser consumers check CPU-written
quotient/remainder, full FLAGS, execution/remaining counts 1024/0, source,
stack/continuation, early stale-completion invalidation, cleared records and
fresh completion. They re-poison retained observations before each of two boots.
Native compares the entire RAM/SRAM mirror; production Chromium exercises real
player save restoration, Restart and persistence with local-only ROM/save handling.

[Emitted instruction sums](division-timing.md) establish
`22 + 1024*(36 + division clocks)`. Bounds are exact floor/ceiling quantization
on a 256-clock HBlank timer, without widening tolerance. Identical final cartridge
SHA-256 **ec828bb3833d826da054091fd31dadb23a9f675e68311013419967f8a65912f1**
was run under former and corrected charges in both consumers.

Former native exited 101 and former production Chromium exited 1 at their final
accumulated timing assertions. Each collected all **360 timing failures** across
all model/mode and both boot observations before rejection. Architectural and
freshness checks passed for every record. Their complete observation tables
match exactly. Corrected native passes all 360 records.

Corrected production Chromium also passes all 360 records. Its complete table
matches corrected native exactly (all six model/boot records, including every
live mode pass). This was observed during the complete compatibility suite,
which passed after the complete production smoke suite against the final build.

Across every pass and both boots, observed register/even/odd ranges are:

| Form | Former native and Chromium | Corrected native and Chromium |
| --- | --- | --- |
| Byte DIV | 248–249 / 252 / 252 | 212–213 / 216 / 216 |
| Word DIV | 280–281 / 284 / 288 | 244–245 / 248 / 252 |
| Byte IDIV | 248–249 / 252 / 252–253 | 220–221 / 224 / 224–225 |
| Word IDIV | 280–281 / 284 / 288–289 | 248–249 / 252 / 256–257 |

Canonical tables use compact JSON with sorted lowercase `model:boot` keys and
arrays in pass/sample order. SHA-256 identities are:

| Charges | Samples per consumer | Observation-table SHA-256 |
| --- | ---: | --- |
| Former | 360, all timing rejected | `fb8837aa2bb8f64ec98bc8aec8bca5af377c3fb5026e4ea7c19e441f27e9f1a0` |
| Corrected | 360, all timing/state accepted | `1880c1062b256a810e4a400f7bd8413f81c2302d86bfb6f9a7602c8167f6c2b1` |

Former production Wasm SHA-256:
`4be4ce09e632b3ce1c7c2ffbfadf82333b08c1d0bbabd43899e25ad49dfa50e8`;
its `index-Bk7nlDmM.js` SHA-256:
`1599e8778e85cdc990b9f85ba4b39241ef89f97356f21db635c46f195a38545b`.
Final corrected identities follow below. Raw logs, observation tables and pkg/dist
builds are ignored local artifacts in `target/issue-87-validation`; this report
preserves durable observations, independent sums, identities and dispositions.

## Existing acceptance audit and mandatory commands

All fixture/demo assembly was audited for executed division. Prior division
appears only in arithmetic/history `divide.asm`, without an absolute interval;
other measured timing bodies and demo contain none. No prior cartridge interval
or tolerance changed. All **46 prior fixture images** and demo bytes remain
unchanged; the new fixture is the only added image. CPU arithmetic/alias/scheduler
assertions were recalculated from independent fetch/source/residual sums. The
former 51-clock byte-division-plus-trap assertions become **42 (DIV)** and
**44 (IDIV)**, retaining all saved-frame/FLAGS/IRET assertions and five entry
accesses. Word scheduler budgets become 25/26. Historical reports are unchanged.

All commands ran in this worktree; npm commands ran in `web`. `npm ci` passed
without manifest/lock changes. Wasm builds used the installed stable toolchain's
Wasm target via PATH. No model/provider switch, usage reset or allowance purchase
was performed. Production artifacts were served with
`npx vite preview --host 127.0.0.1 --port 4187 --strictPort` at
`http://127.0.0.1:4187/wonderswan-rust/`. `WS_PREVIEW_URL` selected that explicit
production URL. Complete smoke and complete compatibility ran sequentially.

| Mandatory command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 47 images reproduce. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 281 core unit + 43 integration + 1 web unit = **325**; empty doc tests. |
| Focused `divide` core tests | Pass: 28 tests. |
| Native `divide_timing` with `--nocapture` | Pass: all 360 records. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` | Pass for former-control and final corrected artifacts after PATH correction. |
| `npm run build` | Pass for former-control and final corrected artifacts. |
| `npm run test:browser` | Complete production Chromium pass. |
| `npm run test:compat` | Complete production Chromium pass, including the new division consumer and all prior consumers. |

The suites retain prior CPU/timing acceptance, firmware-free startup and all
three-model gates, LCD/input/frame continuity, stereo audio/PCM/sweep,
GDMA/SDMA/Hyper Voice, SRAM, both EEPROM controllers, RTC, browser save integrity,
storage failure handling and ownership/provenance. Bounded original-probe success
does not establish commercial-catalog readiness.

## Final source and artifact identities

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `61eed814e7c5f1867e1f1dd3d5c823f90106cdffceaeb9705934034bc8297c51` |
| `crates/ws-core/src/cpu/divide_tests.rs` | `8092cb035f71c52738182677b80eb7548be9753e92e8e39e2011386934367442` |
| `crates/ws-core/src/cpu/divide_timing_tests.rs` | `a48f632918ae916539221f404379dd3d826d8a0f6a1b81cef39efac171846980` |
| `crates/ws-core/tests/divide_timing.rs` | `2a4888581823fcbb809c643a16fc2a295c634ac113a21c49e7da834d573d9632` |
| `fixtures/divide-timing.asm` | `7d12de8053f84e122fd8f856375d0e2354b03384b5c08885339940dfb35a0bb7` |
| `fixtures/generated/divide-timing.ws` | `ec828bb3833d826da054091fd31dadb23a9f675e68311013419967f8a65912f1` |
| `web/scripts/divide-timing.mjs` | `8218152884f8ba2c8aec28732cf307948cf5922628f7ed7bfd2cc2904f91fc10` |
| `web/pkg/ws_web_bg.wasm` and production Wasm | `75fd50f5fc56c4d86886d32e49649b49161fc731537bb6d4abf74348b1293758` |
| `web/dist/assets/index-B9zPAQH9.js` | `1923335f0d4d34c9d270fdb62731a3afb0fa316cad28cd7b7f4aa6a32dc0c54a` |

## Coordinator criterion review

The Coordinator reviewed all eight pre-merge checkboxes against the final diff
and executed evidence:

| Issue criterion | Final disposition |
| --- | --- |
| Pre-edit literal timing/full-state vectors for all four successful forms, register and even/odd [BX] | Met: four actual baseline failures, corrected passes, literal 17/19/25/26 register totals and 18/20/26/27 even memory totals; odd words add one. |
| Complete FLAGS/results, signed/unsigned cases, aliases, unrelated state, continuation and history | Met: retained independent vectors expanded to nine configurations, unambiguous signed boundaries, source snapshots, real following instructions, producer/history/lifecycle tests and unsigned-AH fault regression. |
| Exact fetch/source/mapped/wait/prefix/boundary accounting | Met: dedicated register/memory matrices, both parities, A0 00/04/08/0C, SRAM waits, distinct DS/SS/overrides/displacements, offset/window/20-bit boundaries, mixed BUSLOCK and retained REP diagnostics. |
| Separate fault policy and discriminating zero/overflow regressions | Met: successful and fault assignments are distinct; 31/39-clock fault tests preserve exact frame/registers/history and IRET under former and corrected charges. Unresolved hardware accuracy is explicit. |
| Model/mode and normal scheduler/device/interrupt/DMA behavior | Met: all nine configurations with live returns, exact frame/raster/timer/audio progression and poison continuation, independent TF/IRQ entry costs, saved frames/IRET, IF shadows, REP/HALT and capability-gated DMA reservations. |
| Original CC0 cartridge and independent native/production-browser old/new consumers | Met: twelve profiles, operand reloads, emitted-instruction bounds, full CPU-written state/counts/continuation, per-boot poisoning/freshness, all 360 observations collected per consumer before timing rejection; old and corrected table equality verified. |
| Existing interval/scheduler/trap audit and prior hardware/provenance acceptance | Met: 51 becomes 42/44 only where affected; 25/26 word totals, unrelated assertions/tolerances and all 46 prior fixture/demo images preserved; full native/browser suites retain all listed prior CPU/device/save/provenance coverage. |
| Mandatory commands, documentation, identities and final review | Met: all mandatory commands pass against final source/production artifacts; accounting, hardware/provenance references, actual failures/resolutions and tested hashes are recorded above. |

No blocking findings remain. No acceptance criterion was weakened, and no
human-approval gate or unperformed hardware claim was introduced.

Physical-console measurements, human gameplay, other browser engines and
commercial workloads were optional and unperformed. No device identities or
human sign-off were invented. Pipeline/effective-address fidelity and exceptional
division accuracy remain explicit gaps. Required post-merge verification: **None**.
