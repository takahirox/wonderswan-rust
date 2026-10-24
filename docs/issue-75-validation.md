# Issue #75 validation: Jcc and JCXZ taken-target timing

## Source and correction

[Issue #75](https://github.com/takahirox/wonderswan-rust/issues/75) was read
from GitHub during this run. Evaluation baseline:
`39e0c125cd058c13dca088d605f40b5cf99be68f`; assigned upstream checkpoint:
`6b12680f3a78f7c0bc446b45417e682af9fa4ae5`. Work and validation took place
in the assigned worktree on 2026-10-24. No push, PR, merge, comment or remote
Issue edit was performed. No allowance reset, purchase or model/provider switch
occurred. No further specification correction requiring a remote edit was found.

The [NEC V30MZ primary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, was read at Appendix A conditions/Note 5 and Table A-1,
printed pp. 73–76. The old AI assumption preserved six internal clocks for
taken odd Jcc/JCXZ destinations. The corrected requirement is seven, retaining
six for taken-even and three for fallthrough under two explicit internal byte
fetches. Four/one decoded taken/fallthrough clocks and the odd-destination
adjustment establish the correction; opcode parity alone adds nothing.
This follows the primary specification and does not weaken acceptance.

[Independent timing/state vectors and cartridge sums](jcc-timing.md) were
written before changing execution costs. Production changes add the taken
odd-destination clock after the existing IP update in 70–7F and E3 only.
Branch decisions, CX, full FLAGS, actual byte/prefix fetches, waits, word
consolidation, IRQ/TF entry, device scheduling and DMA are unchanged. There is
no destination pre-read, opcode-pattern workaround, full prefetch or new bus
policy. CALL, indirect JMP and other instruction costs remain separate.

## Native coverage

Six new CPU test groups cover:

- All 16 Jcc conditions using literal 32-bit truth sets over every CF/PF/ZF/SF/OF
  combination. All other FLAGS bits are tested clear and set, including reserved
  bits. Instruction-only vectors preserve incoming TF without taking an entry;
  separate step-level arbitration tests verify its delivery semantics.
- JCXZ with CX 0000/0001/0002/FFFF and contrasting condition/control flags.
  Every vector checks unchanged CX and other registers, full FLAGS, exact CS/IP,
  other segments, stack and RAM. Ports, fetch counts, HALT/shadow state and the
  absence of device advancement during instruction-only execution are checked.
- Both opcode and destination parities; zero, forward and backward displacements;
  signed extremes; IP FFFE/FFFF/displacement-fetch wrapping; physical FFFFF→00000
  fetch wrapping; and explicitly listed mapped-bank crossings. Expectations do
  not call production condition, address or timing helpers.
- Accessible internal RAM and ROM banks at live A0 00/04/08/0C, literal byte
  lane costs, unrelated mode/SRAM/I/O wait bits, zero through seven mixed
  segment/BUSLOCK prefixes, and rejection of both REP prefixes before operands.
  Existing JMP/LOOP preservation vectors now use seven for odd Jcc/JCXZ while
  retaining their CALL/indirect/LOOP/state/error acceptance.
- All nine model/mode configurations and live 00→80→00→C0→00→E0→00 transitions.
  Normal `Machine::run_frame` executes representative JZ/JCXZ even/odd/fallthrough
  paths at the frame boundary. Exact clocks, next frame/raster line, one timer
  tick and one stereo sample are asserted. Unchanged registers and poison INC BX
  bytes at continuations prove no destination instruction executes in that frame.
- IRQ before the branch, one-instruction shadows, IRQ priority over TF, TF-only
  entry, even/odd stacks, exact saved continuation IP/CS/FLAGS and actual IRET.
  External entry remains 32/35 internal clocks and TF entry 25/28. GDMA and SDMA
  retain separate seven-clock reservations with no CPU fetch, capability errors,
  GDMA color-mode gating and SDMA availability on Color/Crystal in mode 00.

The broader workspace checks preserve REP/HALT, interrupt shadows, firmware-free
three-model startup, LCD/raster/input/frame continuity, PCM/audio/Hyper Voice,
GDMA/SDMA, cartridge SRAM/EEPROM, internal EEPROM and RTC retention.

## Original cartridge and actual negative controls

The original CC0 source and reproducible 128 KiB cartridge measure JZ and JCXZ
at taken-even, taken-odd and fallthrough paths. The independent instruction
sums, every measured instruction, harness overhead, physical RAM layout and
256-clock timer quantization are in [the timing document](jcc-timing.md).
Native and independent production Chromium consumers require absolute budgets,
CPU-written BX/BP execution/destination counts 4096/4096, unchanged CX 0/0/1
and continuation 51A7. Every model/mode pass and both fresh boots are checked.
Native checks compare all 512 result RAM/SRAM bytes; browser checks use the real
player pause/save path and require no cartridge/save network requests. Both
poison retained success and verify its invalidation after one frame, before
measurement completes. Cleared unused observations are also required.

Before changing execution costs, the new native consumer actually failed at
**Mono boot 1 pass 0 sample 1 = 416 ticks**, outside **432–433**. The independent
browser consumer against old production Wasm failed at the same **416 ticks**.
Both used the final cartridge SHA-256 below. Old assets were
`ws_web_bg-BGU9c2Yb.wasm` (SHA-256
`89a01126854f416dd82d8cdeff1ffa47651682bbf9ac2088afab10f71fc5f2f8`)
and `index-BpN6Pl6l.js`. The even sample passed its absolute bound before the
odd sample failed; no relative-difference criterion was used.

All six new CPU groups failed against old branch costs. The first truth-set,
ROM/prefix, arbitration and DMA-following odd-target cases returned **6 rather
than 7**. The normal-frame case ended at **81409 rather than 81408** clocks,
exposing an extra destination instruction when the missing clock left frame time.

With corrected branches, temporarily restoring external IRQ execution 12 in
place of 27 made the revised IRQ native consumer fail at **184 ticks** against
**244–245**. A Python `finally` restored CPU source byte-for-byte before the
complete workspace check. No IRQ production change is retained.

Development/tooling notes: ROM test setup initially reparsed/checksummed the
same synthetic image for each vector. It now inserts once per bus and rewrites
only the test bytes, applying live A0 after insertion. No oracle or acceptance
changed. An obsolete slower focused run was terminated after the final complete
workspace suite passed all six new groups. One browser command was invoked from
the repository root without a package.json and ran no checks; it was rerun in
`web`, where the complete smoke suite passed. These are not reported as passed
application runs. The primary PDF screenshot request failed; the primary text
conditions and table were available and read.

## Corrected observations and existing probes

Corrected native and focused production Chromium observations, every pass and
both boots of all three models:

| Sample | JZ even | JZ odd | JZ fall | JCXZ even | JCXZ odd | JCXZ fall |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| HBlank ticks | 416 | 432 | 368 | 416 | 432 | 368 |
| BX / BP | 4096/4096 | 4096/4096 | 4096/4096 | 4096/4096 | 4096/4096 | 4096/4096 |
| CX | 0 | 0 | 1 | 0 | 0 | 1 |

All continuation markers were 51A7. Corrected native and focused production
Chromium IRQ observations were **244/268** on every pass and both boots, with
handler/application/countdown **1024/0/0**. The complete production compatibility
suite observed the same Jcc/JCXZ and IRQ tables on every pass and both boots;
its JMP and LOOP observations also matched the native results below.

Existing assembly sources, NASM layouts and generated identities were rechecked:

- IRQ handler at 200F has bytes **43 4E 75 04 B0 80 E6 B6 CF**. JNZ at 2011
  targets odd 2017 on 1023 iterations. Add **1023** to 61459/67603:
  **62482/68626 clocks**, **244–245/268–269 ticks**. Equivalently,
  1024×(32+2+2+7+10+8)+15−4−4+11 = 62482; odd stack adds 1024×6.
  Former IRQ entry subtracts 1024×15, giving 47122/53266 clocks and
  184–185/208–209 ticks. Fixture bytes, 32/35 entry and result/state checks
  remain unchanged. Historical observations in #69/#71/#73 are not rewritten.
- JMP measured JNZ returns to even 2004. Its unchanged sums remain
  11+4096×22−3 = **90120**, 11+4096×23−3 = **94216**,
  11+4096×26−3 = **106504**, 11+4096×27−3 = **110600**.
  Native samples remain **352/368/352/368/416/432** on all passes/boots;
  counts 4096/4096, CX=0 and continuation 51A7 pass unchanged bounds.
- LOOP taken E2 sums remain **45064/49161** (11 or 13 initial clocks,
  4096×4 for LEA, 4095×7 or 8, final 4); E1/E0 **49160/53257**.
  Fallthrough harness JNZ also targets even 2004: **94216/98312/98312**.
  Native samples remain **176/192/192/208/192/208/368/384/384**, except
  E1-even on Color/Crystal passes 2/5 is **193** within unchanged 192–193.
  Counts 4096, CX 0/1 and continuation 51A7 remain asserted.
- Ordinary-I/O has no measured conditional branch: **11+2048×8/9/10**,
  hence **64–65/72–73/80–81** ticks. Relative-word probes use LOOP, with
  unchanged odd-minus-even 4096 clocks = **16 ticks**, ±2 only for subtracting
  separately quantized samples. No bounds, source or layout changed.

Only the IRQ native/browser timing bounds change among existing consumers.
No tolerance was widened and no fixture was relaid out to avoid a failure.

## Commands and results

Commands ran in the assigned worktree, with npm commands in `web`.
Production preview: **http://127.0.0.1:4185/wonderswan-rust/**, explicitly supplied
as `WS_PREVIEW_URL`. Production smoke completed before compatibility started.
Wasm builds prepend the installed
`$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin` to PATH.

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Passed; new image reproduces and all existing images are unchanged. |
| `python3 demo/build.py --check` | Passed; demo unchanged. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Passed: 217 core unit + 37 integration + 1 web unit = **255**; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `git diff --check` | Passed; repeated after final documentation. |
| `npm ci` | Passed; lockfile unchanged. |
| `npm run build:wasm` | Passed for baseline negative control and corrected production Wasm. |
| `npm run build` | Passed for baseline and corrected production TypeScript/Vite assets. |
| `npm run test:browser` | Passed completely, including all models, desktop playthrough, local-file/lifecycle, fullscreen, input and mobile touch. |
| `npm run test:compat` | Passed completely after smoke: new Jcc/JCXZ, revised IRQ, unchanged JMP/LOOP, all original probes, PCM, input, raw-ROM identity, cartridge/console retention and storage-failure coverage. |

## Tested identities

SHA-256 of tested source/artifacts (validation prose excluded):

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `d852194f7e964cd0c26bfa75dbdf1884f138d9803763a0aae093b27b728ec6db` |
| `crates/ws-core/src/cpu/jcc_tests.rs` | `6e0c1c391a227d60851b6878fae023c3d0ee4d32dc2ac722d653d14d7d0992d7` |
| `crates/ws-core/src/cpu/jmp_tests.rs` | `942bd8df60b07d66c18ca7638a924dcb34e35ffa90b260284d6bc6e05613368c` |
| `crates/ws-core/src/cpu/loop_tests.rs` | `38430b9b8a88ae9060229fb88644ce00c0b85c7c57aa2b220837a20dcd6db65a` |
| `crates/ws-core/tests/jcc_timing.rs` | `d147569403640b182aebd0efaa85a671a137ed137551fe83be792c83e3724219` |
| `crates/ws-core/tests/irq_timing.rs` | `9ccd1550598bc3a20b0bb01f1fad62d129013b7f61464c33e651fc8a4de24ec8` |
| `fixtures/jcc-timing.asm` | `8a4440dd669dddcf8ff4f720a03d15c03a8dc28cdc9b65bf23d8136c19f8d2cd` |
| `fixtures/generated/jcc-timing.ws` | `9dbe44ea9745b91f4a8a54b450215d5ea27ed907efc7eb8d8e4eebac9713e2d7` |
| `fixtures/build.py` | `e6fb542ad45f82b412fcb10b1b20e1c98e6c354e0823f5e30b6d5d2a2ccd91ad` |
| `fixtures/generated/jmp-timing.ws` | `a609ec18fda97aa419338cf98905eaf8d2ab04097d305b38d6cec7172d192123` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `web/scripts/jcc-timing.mjs` | `1df60d06fe61ee120b136dd39127c56fa40d40a2e7af7d17f7e0da441dd0b7ab` |
| `web/scripts/irq-timing.mjs` | `65ec346fc49a6d8f11d13f7aa37c44f9f8210ac1144f42604beac0fd7b06fe6a` |
| `web/scripts/compat.mjs` | `ec6f5e01f3c270e7c65486be63b2c52331125771e4147408fd0c3fa89cf9f1b8` |
| `web/pkg/ws_web_bg.wasm` | `1b3a1319dcdad450e05de4f8a8f360e536e8932de2a9f825bf3954f53fc752fc` |
| `web/dist/assets/index-CDe2yE9Q.js` | `7cc0894316fb1d89958e6e9f30bda3d3fb86a49efc9b30b4004d0eb17a5a62bb` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/ws_web_bg-3vzSb5s_.wasm` | `1b3a1319dcdad450e05de4f8a8f360e536e8932de2a9f825bf3954f53fc752fc` |

Wasm/pkg/dist are ignored build products. Assembly, generated cartridge,
independent consumers and evidence are included in the checkpoint.

## Criterion review and limits

The local implementation/Coordinator review compared the source Issue, full
final diff and evidence against each mandatory criterion:

| Criterion | Review evidence |
| --- | --- |
| Independent oracle before cost edit; exact scope | Literal vectors and cartridge sums recorded first; only taken odd Jcc/JCXZ execution gains one clock. Fetches/waits and architectural paths unchanged. |
| Native truth/state, geometry, mappings, prefixes | Six passing groups cover all 16 conditions, all 32 flag combinations, CX boundary values, both parities, displacement limits/directions/zero, wraps/banks, all A0 controls, full state, prefixes through seven and REP errors. Obsolete JMP/LOOP literals corrected without removing their checks. |
| Shared devices and model/mode behavior | Nine configurations/live round trips, normal-frame exact raster/frame/timer/audio, no extra destination instruction, IRQ/TF/shadow/IRET, separate DMA clocks and gates; broader REP/HALT and lifecycle suites pass. |
| Original absolute cartridge/native/browser evidence | CC0 source/reproducible ROM; six absolute paths with CPU counts/CX/continuation, stale-save invalidation and restart; actual old native and production-browser failures and corrected focused passes. Complete compatibility also passes. |
| Existing oracles | Unchanged IRQ bytes gain 1023; revised absolute bounds and actual former-entry rejection, 32/35 entry and 1024/0/0 maintained. JMP/LOOP bytes, budgets/counts/CX/continuation unchanged. |
| Mandatory tooling and broad regressions | All 255 native tests, fixture/demo/fmt/clippy checks and complete production smoke pass. Complete compatibility also passes. |
| Documentation/provenance | Primary-source correction, independent sums, actual observations/failures, unchanged historical evidence, artifact identities and explicit unperformed checks recorded. |

Final diff and criterion review found no blocking issue within this scope.
All mandatory checks completed and passed. Recorded SHA-256 identities and
relative documentation links were checked against the final files; final staged
whitespace checks passed. No mandatory criterion is waived and no human or
console prerequisite is added.
Physical-console measurements, human gameplay, other browser engines,
additional software experiments and commercial-title compatibility were **not
performed**. Full pipeline/prefetch and external-bus policy remain unresolved;
CALL and indirect JMP timing are separate work. Required post-merge verification:
None.
