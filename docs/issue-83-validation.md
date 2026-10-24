# Issue #83 validation: shift and rotate timing

Work started in the assigned GitWeave worktree at
`48190ab6b45e5a32559e12df728dc63c35d2fb7f`, application baseline
`93e71358ef0f7352c4b076b1906981f629054267`. The live GitHub Issue #83 and
repository development/review guidance were read before implementation.
No push, PR, merge, Issue edit or comment was made.

## Correction and independent evidence

The old AI-authored assumption was residual `2 + (count & 31)` for every form.
The [primary NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A printed pp. 73/81–83, supplies fixed decoded
register costs 1/3/3 and memory costs 3/5/5 for implicit-one/CL/immediate.
Subtracting the two already charged aligned operand transfers gives inferred
residuals **1/3/3**. Actual fetches/transfers/waits remain additional and are
charged once. Section 1.3 requires writeback even at zero count. This replaces
an incorrect preservation assumption while retaining Issue #45 architecture.
No further specification reconciliation was necessary.

The table's decoded, aligned, no-wait and qualified-addressing conditions matter:
these tests establish serialized accounting, not full pipeline fidelity. The
manual's cached PDF text was inspected; web screenshot retrieval returned cache
misses, so no successful visual inspection of the PDF pages is claimed.

[Independent timing rationale](shift-rotate-timing.md) and the literal timing
regressions were written before changing production charges. Unprefixed internal
register totals are 3/5/6; `[BX]` byte/aligned-word totals 5/7/8, odd words 7/9/10.
Expectations never call production timing helpers. Existing Issue #45 independent
whole-integer/carry-ring result/full-FLAGS literals remain unchanged.

Actual baseline regressions, separately executable:

| Opcode / count | Required clocks | Old observed clocks |
| --- | ---: | ---: |
| D0 E0, implicit one | 3 | 5 |
| D2 E0, CL=31 | 5 | 35 |
| C0 E0 20, immediate=32 | 6 | 5 |

The first literal run returned 101 with exactly these three new failures
(42 other selected literal tests passed). The expanded pre-edit shift run also
rejected the old accounting in mapped fetch/data, boundary and shared-device
checks. No baseline failure is represented as a passing implementation check.

The runtime diff only chooses the residual by opcode. Operation decoding,
count sampling/masking, read→shift→write order, FLAGS and bus methods are unchanged.
Both documented widths and all seven operations receive the correction; /6
retains its existing rejection.

## CPU and shared-device acceptance

The retained Issue #45 matrix now checks fixed residuals across all nine model/mode
configurations: Mono 00; Color/Crystal 00/80/C0/E0. Its literal vectors cover all
seven operations, both widths, all applicable count sources, raw 0/1/2,
width/carry-ring boundaries, 31/32/33/255 and contrasting incoming FLAGS F002/FFD7.
Nonzero full rotations remain distinct from masked zero. Full registers, segments,
FLAGS, IP, untouched RAM/SRAM/ports and the actual next instruction are checked.
CL/CX/CH alias tests also assert exact count-independent timing.

New exact accounting tests cover writable RAM/SRAM, aligned/odd words, independent
DS/SS/ES targets, last overrides, 8/16-bit displacements, up to seven mixed segment
and BUSLOCK prefixes, internal/SRAM/ROM fetches, live A0 00/04/08/0C and existing
SRAM waits. Masked-zero reads and writes must both be present in access counts.
Whole RAM/SRAM snapshots detect unrelated writes and doubled nonzero operations.
REP retains its diagnostic, consumed bytes, state and two fetch charges.

Boundary tests cover operand offset wrapping, instruction offset wrapping,
RAM/SRAM bank crossings, mapped ROM windows, 20-bit wrapping, high RAM availability
and ignored unavailable writes. Timings follow the actual supported bus policies;
this change does not introduce transfer consolidation or unrelated bus corrections.

Normal `Machine::run_frame` crosses a frame/raster boundary with representative
implicit-one, CL=31, immediate=32 and odd-word immediate=255 shifts. Exact elapsed
clocks, frame number, raster line, one clock of overshoot, one HBlank decrement,
one stereo pair and CPU/memory state are asserted. A poison INC BX continuation
must not execute. Live mode returns to 00 are included on Color/Crystal.
TF delivery and priority IRQ after a protected shift retain separately charged
25/32-clock entries, exact saved frame/FLAGS and real IRET continuation. Separate
seven-clock GDMA/SDMA reservations have no CPU fetch or state change; Mono
capability rejection, GDMA mode gating and Color/Crystal SDMA in mode 00 remain.
The full suite retains REP/HALT, startup, branch/CALL/return/IRQ and device checks.

One initial corrected frame assertion wrongly expected AX=0002 for a byte shift
starting at AX=8081. Actual AX=8002 correctly preserves AH; the literal test was
fixed. Clippy identified two manual range checks in added tests, replaced with
`Range::contains`. Neither correction changed runtime behavior or tolerances.

## Original cartridge and independent consumers

The original CC0 source, deterministic cartridge and independently embedded native
and browser result/timing literals are included. There is no proprietary ROM,
firmware or third-party code/asset. [The timing document](shift-rotate-timing.md)
derives every measured instruction cost independently. Seventy samples cover all
seven operations, both widths and implicit-one/CL=0/CL=31/immediate=32/immediate=255.
Per-iteration operand and FLAGS initialization prevents large-count results from
collapsing into an uninformative repeated-shift steady state.

CPU-written records contain absolute timer ticks, whole AX, full captured FLAGS,
1024 executions, zero remaining iterations, unchanged raw CX and stack/continuation
marker 51A7. Fresh completion, capability and pass counts, every unused zero record
and the full native RAM/SRAM mirror are checked. Both consumers start from poisoned
retained success and observations, inspect invalidation after one frame, then
require two fresh boots. The final consumers re-poison before the second restart.
Chromium does this through actual local save restoration and Restart before RAF;
ROM/save handling remains local. Native uses `load_save` then `restart`.

Timing errors accumulate across all 2100 records (70 samples × 15 model/pass
configurations × 2 boots), so an early rejected timing cannot conceal later
samples. Architectural/state failures still stop immediately. Native and browser
run the same final cartridge bytes against old and corrected charges.

| Profile, every operation and width | Independent corrected ticks | Old native and Chromium ticks |
| --- | --- | --- |
| Implicit one | 128–129 | 136–137 |
| CL=0 | 136–137 | 132–133 |
| CL=31 | 136–137 | 256–257 |
| Immediate=32 | 140–141 | 136–137 |
| Immediate=255 | 140–141 | 260–261 |

Old-charge observations reject every profile. State/results/full FLAGS/counters,
completion and continuation pass. The original baseline and final strengthened
controls are separate runs; final control return codes and comparison summaries
are recorded below. Temporary old-charge source patches are restored with Python
`finally` before final builds. No runtime switch or test-only production option
is introduced.

## Existing acceptance and interval audit

No existing measured timing body contains an affected shift. Compatibility
startup ROL and keypad shifts have no absolute interval assertion; Issue #45's
cartridge checks architecture, not absolute timing. Explicit byte bodies and
source mnemonics were audited, including word/I/O, direct/indirect control, return,
IRQ, Jcc/JCXZ, JMP and LOOP loops. Therefore no existing interval or tolerance
needs changing. All prior cartridge/demo bytes, historical reports, independent
consumer budgets and unrelated assertions remain unchanged.

Complete production compatibility retains firmware-free startup/model gates,
LCD/input/frame continuity, sound/PCM/sweep, GDMA/SDMA/Hyper Voice, SRAM, both EEPROM
controllers, RTC, raw identities/provenance, local handling, browser save integrity
and denied/corrupt/full storage behavior. Its existing CALL/return/branch/IRQ
consumers remain enabled. This task does not claim new commercial-title coverage.

## Final old-charge controls and identities

Final strengthened native and browser consumers completed all six model/boot
records, including every mode pass, and reported **2100 timing failures each**.
Their entire observation tables match exactly. Native returned 101 and Chromium
returned 1 at the final accumulated timing assertion; no earlier architectural
failure concealed later samples. Actual profile ranges are exactly the old
column above, including both floor and ceiling values over the complete matrix.
The cartridge SHA-256 is
`85397cbd3d926fa7b2bce1d4a40a03821415bec7660e9b77fbda49855b4033ee`
for old and corrected controls. No bytes changed between them.

| Old-charge production artifact | SHA-256 |
| --- | --- |
| `web/pkg/ws_web_bg.wasm` | `af5b8e35d3a6a38a8fc80d614d53280f561bbcd5ffefdc626a59a65ffefeb52c` |
| `web/dist/assets/index-gkx9qQjn.js` | `ae25811e1486bfc45373921eed2cf6b33452b8b78f3304f96e00b8329f922cc0` |

Final tested source/artifact identities below exclude this validation prose.
Production pkg/dist and full raw logs in `target/issue-83-validation` are ignored
local build/evidence files; durable calculations, observations, failures and
identities are recorded here. No allowance reset, purchase or provider/model
switch occurred.

| Final file | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `f7ee3a9e323fd229db46d7cdc34289941489b06611be173d67cafcf9efe7a44a` |
| `crates/ws-core/src/cpu/shift_tests.rs` | `c8cf652e7553081af0c379c9efab107e51d66f4eb1f045e4698e41c8c587488e` |
| `crates/ws-core/tests/shift_timing.rs` | `1ecef07b860bd3b1b5017ed9098d79144354b120e231c3c1351b94baaebe016f` |
| `fixtures/shift-timing.asm` | `e8cf5f42ba852dbf475b2215a113d344130ee320283d93d8eedd1b4ca47c8282` |
| `fixtures/generated/shift-timing.ws` | `85397cbd3d926fa7b2bce1d4a40a03821415bec7660e9b77fbda49855b4033ee` |
| `fixtures/build.py` | `6ccded44906b9b8278f6c7cfe4ae669941dd76c7124128d095cf301c32ec1c63` |
| `web/scripts/shift-timing.mjs` | `74933bbb7f17f1768a28a72106f51d00a773a38d360a2a33b3d9ef05a99418cc` |
| `web/scripts/compat.mjs` | `8b4d170e145f66879355520422ab15baa0fbbe5a4cd7a4c8a7d8c96482ecd2dd` |
| `docs/hardware-subset.md` | `5167909a0e56bbef8951c76e9f10ad0522063180d727201ef8f50910b7c64772` |
| `docs/shift-rotate-timing.md` | `e92c37a2433cf3b8b109ea9100dd25e1c9bfd28a742d61432e8bdd18f31e3a0a` |
| `fixtures/README.md` | `2bd54161299fcc43fef0464eec79d4aaa5c80c7e4ab242138be2cce6b16ded20` |
| `web/pkg/ws_web_bg.wasm` | `ae816fc305a237d02a8cd5932bdf8baf610c53cd3182e8cf9e341d78bb4c7ead` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/index-DTpDalcZ.js` | `16746d6b738071f7840936603f5751df40705d4a411fa702bae3480a9027ac17` |
| `web/dist/assets/ws_web_bg-CgIgg5BB.wasm` | `ae816fc305a237d02a8cd5932bdf8baf610c53cd3182e8cf9e341d78bb4c7ead` |

## Final corrected observations and mandatory commands

Final native and complete production Chromium observations match exactly over
all **2100** records, all model/mode passes and both freshly poisoned boots.
Observed profile ranges are **128–129 / 136–137 / 136–137 / 140–141 / 140–141**.
Mono's two boots observe 128/136/136/140/140 for every operation/width; occasional
one-tick ceilings on Color/Crystal remain exactly within timer quantization.
Every result/full FLAGS/count/CX/continuation check passes. Canonical observation
table SHA-256 values below identify the complete matching native/browser tables
(JSON object keyed by lowercase `model:boot`, sorted keys, compact separators,
with arrays in pass/sample order):

| Charge implementation | Observations per consumer | Table SHA-256 |
| --- | ---: | --- |
| Old | 2100; every timing rejected | `0053e8ad337051e92d187bca935323eb8decf9ff956a6f2b223a53f1575a89a9` |
| Corrected | 2100; every timing/state accepted | `c338790db13408db3572f0c1d7c1974a48d2e2d39443e19e7ffa010c0bc7a0da` |

All commands below ran in the assigned worktree; npm commands ran in `web`.
The final production build was served explicitly at
`http://127.0.0.1:4187/wonderswan-rust/` using
`npx vite preview --host 127.0.0.1 --port 4187 --strictPort`.
Complete smoke then complete compatibility ran sequentially against that build,
with `WS_PREVIEW_URL` set to the explicit preview URL. `npm ci` passed without
manifest or lockfile changes. The first build:wasm attempt found Homebrew Rust
without its Wasm target. Prepending the already installed
`/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin` to PATH resolved
it; no toolchain/package/configuration change was necessary. Both final builds
and suites were rerun after the old-charge control and consumer strengthening.

| Mandatory command | Actual final result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass; all 45 cartridges reproduce, including the new probe. Existing bytes unchanged. |
| `python3 demo/build.py --check` | Pass; unchanged SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 257 core unit + 41 integration + 1 web unit = **299**; empty doc tests. |
| Focused final `shift_timing` native consumer with `--nocapture` | Pass; all 2100 observations captured and compared exactly with Chromium. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm run build:wasm` | Pass for final old-charge control and corrected production build. |
| `npm run build` | Pass for final old-charge control and corrected production build. |
| `npm run test:browser` | Complete production Chromium pass: all models, demo playthrough, lifecycle/loading/races, dialogs/fullscreen, input and touch. |
| `npm run test:compat` | Complete production Chromium pass: final new shift consumer, all existing timing, CPU and device consumers, ownership/provenance, save/local handling and storage failure checks. |

## Final criterion review

Coordinator review compared every Issue checkbox with the final diff, independent
oracles, source assembly, raw observations and actual checks:

| Issue acceptance | Final evidence / disposition |
| --- | --- |
| Independent pre-edit literal timing/state vectors | Fixed 1/3/3 residuals, literal register 3/5/6 and RAM 5/7/8 plus odd-word transfers; actual pre-edit failures and corrected passes. No production timing helper in expectations. |
| All operations, widths, counts and architectural acceptance | Retained independent #45 vectors and opposing full FLAGS, count/ring boundaries, CL/CX/CH aliasing, complete state/unrelated memory/IP and next decode pass. |
| Exact bus accounting | RAM/SRAM aligned/odd, DS/SS/overrides/displacements, wrapping/high RAM, mapped ROM/SRAM fetch, live A0 and SRAM waits, seven mixed prefixes, zero-count read/write and REP diagnostics pass. |
| Models and shared devices | All nine unique model/mode configurations and live returns to 00; normal run_frame exact elapsed clocks, frame/raster/timer/stereo and poison continuation pass. TF/IRQ/shadows/separate entry, GDMA/SDMA gating and retained REP/HALT pass. |
| Original cartridge and independent consumers | CC0 source/artifact and literal native/Chromium consumers pass all seven operations/both widths/all sources. Absolute sums, results/full FLAGS/counters/continuation, completion invalidation, re-poisoned restart and all mode passes/two boots pass. Final old charges reject all 2100 timings per consumer without hiding later samples; complete tables match. |
| Existing intervals and broad acceptance | No affected shift in measured existing loops, so no budgets or tolerances changed. Existing fixture/demo bytes, reports, branch/CALL/return/IRQ and complete startup/device/save/provenance acceptance pass. |
| Mandatory commands | Every command above passed on final source/artifacts; smoke and compatibility completed sequentially against the production build. |
| Documentation/evidence/review | Hardware support, accounting rationale, provenance, independent sums, actual failures/corrections/observations, source/artifact hashes and limitations recorded. Final diff and relative links reviewed; no blocking finding remains. |

No mandatory check was skipped or waived. Physical-console measurements, human
playtesting, other browser engines and additional lawful software workloads were
not performed. Commercial-title compatibility remains unestablished. Full
pipeline/prefetch, effective-address hazards, undocumented /6 and unrelated bus
corrections remain separate gaps. Required post-merge verification: **None**.
