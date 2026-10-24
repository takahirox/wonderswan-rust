# Issue #73 validation: direct JMP timing and destination alignment

## Source, scope and correction

Source [Issue #73](https://github.com/takahirox/wonderswan-rust/issues/73) was
read from GitHub during this run. Evaluation baseline:
`264461a4077d7b99f4b58478365e4f2eb8195ed1`; assigned upstream checkpoint:
`b6637d4be79e99ea4d020f9de3fb18d3a4b7d3b0`. Work and validation took place
in the assigned worktree on 2026-10-24. No push, PR, merge, comment or remote
Issue edit was performed. No allowance reset, purchase or model/provider switch
occurred. No additional specification correction requiring a remote Issue edit
was discovered.

The [NEC V30MZ primary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)
was inspected at Appendix A conditions and Note 5 (printed p. 73) and Table A-1
(printed p. 76), document A13761EJ1V1UM00. The old AI-authored assumption was
that default one-clock execution suffices for direct JMP. The replacement is
four decoded execution clocks for E9/EB and seven for EA, plus one for an odd
destination. These forms have no operand-memory transfers to subtract. Actual
opcode, prefix and immediate fetch charges remain separate and occur once.
This follows the primary table and preserves the Issue's accepted hardware goal.

[Direct JMP timing](jmp-timing.md) records independent literal timing/state
vectors and cartridge sums before the production execution-cost edit. The only
production change selects execution costs after the existing destination update;
E8 CALL keeps its shared arm and current cost. `Cpu::word`, `Bus::word_transfer`,
interrupt entry, DMA, architectural destination calculation and device scheduling
are unchanged. No opcode-pattern workaround, destination pre-read, prefetch queue
or external-bus policy change was introduced.

## Native coverage

Eight new CPU tests cover:

- EB/E9/EA, both opcode and target parities, forward/backward/zero relative
  displacement, signed displacement extremes, full incoming FLAGS including
  reserved bits, exact CS/IP and unchanged registers, stack, RAM and ports.
  EA changes CS, including FFFF:0010/0011 physical wrap destinations.
- All nine configurations: Mono/00; Color and Crystal each at 00/80/C0/E0,
  with live 00→80→00→C0→00→E0→00 transitions and unrelated SRAM/I/O wait bits.
- Live mapped ROM A0 00/04/08/0C width/waits, individually specified bank
  addresses, opcode/immediate bank crossings, immediate-word fetch splits,
  IP FFFE/FFFF wrapping within CS, and physical FFFFF→00000 fetch wrapping.
  Independent literal lane costs and internal prefix arrays are used; tests
  do not call production physical-address or timing helpers for expectations.
- Zero through seven mixed segment/BUSLOCK prefixes, actual immediate alignment,
  unchanged segment semantics, and both REP prefixes rejected before operands.
  Separate preservation vectors assert existing E8 CALL costs/return stack,
  register/memory near indirect JMP, far indirect JMP, taken/fallthrough Jcc
  and JCXZ. Odd targets discriminate against applying the new parity rule to
  those out-of-scope instructions.
- Normal `Machine::run_frame` at every form, opcode parity and destination
  parity in live modes: exactly one instruction's clocks advance the frame/raster
  boundary, one HBlank timer tick and one stereo audio sample. Exact destination,
  FLAGS and registers prove no destination INC BX executes in the same frame.
- Pending IRQ before JMP, one-instruction shadows, IRQ priority over simultaneous
  TF, TF-only entry, even/odd stacks, exact saved continuation CS/IP/FLAGS and
  actual IRET restoration. Independent 32/35 external and 25/28 trap clocks
  are added once, with exact transfer counts.
- GDMA/SDMA each retain their separate seven-clock reservation before JMP,
  zero CPU fetches during DMA, model capability errors, GDMA color-enable
  gating and SDMA availability on Color/Crystal in mono mode. The broader
  REP/HALT, shadow, trap, DMA and lifecycle suites remain enabled.

The original CC0 cartridge uses six copied RAM bodies. Its absolute interval,
fetch alignment, harness overhead and 256-clock timer quantization are derived
in [the timing document](jmp-timing.md). Native and independently written
production Chromium consumers require absolute budgets, CPU-written execution
and destination counts 4096/4096, final CX=0 and continuation marker 51A7.
They cover every model/mode pass and both fresh boots from poisoned retained
success. Native tests inspect startup invalidation after one frame and all 512
RAM/SRAM bytes. The browser controls only host RAF delivery, inspects invalidation
through the player's real pause/save path after one frame, then runs a bounded
160 frames and checks every saved result plus cleared unused observations.
Cartridge/save handling issues no network requests.

## Actual observations and negative controls

With the old direct-JMP costs, the new absolute native consumer failed at
**Mono boot 1 pass 0 sample 0 = 304 ticks**, outside **352–353**. The independent
browser consumer against the old production Wasm also failed at **304 ticks**.
Both used the final cartridge SHA-256 below. Old production asset names were
`ws_web_bg-BlZhds2A.wasm` and `index-D82v_ZVW.js`. The browser consumer's later
startup-invalidation check adds evidence without changing its absolute ranges.

Before changing CPU costs, seven timing-specific CPU groups failed: first
internal/prefix/mapped cases returned **3 instead of 6**; wrapped EB returned
**3 instead of 7**; DMA-following odd EB returned **3 instead of 7**; the
shadow/arbitration case returned **3 instead of 6**; normal frame scheduling
ended at **40705 instead of 40704** clocks. The preservation group initially
had an incorrect indirect-JMP expectation, described below; after correction
it passed against the old CPU before changing production execution costs.

Corrected native observations on every mode pass and both boots of all models:

| Sample | EB even | EB odd | E9 even | E9 odd | EA even | EA odd |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| HBlank ticks | 352 | 368 | 352 | 368 | 416 | 432 |

All execution/destination counts, CX and continuation markers passed.
The focused production Chromium consumer observed the same table on every
pass and both boots, including explicit invalidation of stale saved completion.
The complete production compatibility suite also observed the same table on
every pass and both boots. Its LOOP and IRQ observations matched the current
native observations described below, including the quantized LOOP phase shifts.

The unchanged IRQ consumer still rejects the former external-entry constant:
temporarily replacing execution 27 with 12 while keeping corrected JMP made it
fail at **180 ticks** against **240–241**. A `finally` block restored the CPU
source byte-for-byte before full native and production builds/checks.

Development test-oracle/setup corrections (acceptance was not weakened):

- Initial preservation literals incorrectly expected FF register/memory/far JMP
  totals 3/5/6. Inspection of the unchanged dispatch gives execution 4/4/6 plus
  actual fetch/operand charges: **6/8/11**. Corrected vectors passed against
  the old CPU, including E8 and Jcc/JCXZ preservation.
- Mapped tests initially set live A0 before inserting a synthetic cartridge;
  insertion reapplies its header's bus settings. A0 is now written after
  insertion. The first corrected-code failures (8 versus 7 at prefixed E9 and
  7 versus 6 at wrapped E9) exposed this harness error. No expected timing
  range was changed to accommodate it.
- The independently calculated far fetch crossing physical FFFFF→00000 with
  opcode at FFFFE includes an additional split word at RAM 00001/00002.
  The test formula was corrected before its meaningful corrected-code run;
  the implementation's word consolidation remains unchanged.

## Existing probes independently rechecked

NASM listings and measured instruction bytes were inspected for JMP, LOOP,
IRQ, ordinary-I/O and relative-word probes. Existing sources/cartridges and
native/browser consumers remain byte-for-byte unchanged.

LOOP intervals contain no direct JMP. Taken E2 even/odd still totals
11 + 4096×4 + 4095×7 + 4 = **45064** and
13 + 4096×4 + 4095×8 + 4 = **49161**. E1/E0 give **49160/53257**;
repeated fallthrough E2/E1/E0 give **94216/98312/98312**. Counts 4096,
final CX 0/1 and continuation 51A7 remain independently asserted. Corrected
JMP changes unmeasured setup phase: current native Color/Crystal passes 2/5
observe **193** for E1-even (range 192–193), whereas Issue #71 recorded 192;
E2 fallthrough there is now **368** (range 368–369), whereas #71 recorded 369.
Other samples remain 176/192/192/208/192/208/368/384/384. Every bound is unchanged;
unknown initial phase explains these single-tick differences. Historical evidence
was not rewritten as a new pass.

IRQ's measured body/handler contains IN/MOV/STI/LOOP/CLI and
INC/DEC/JNZ/MOV/OUT/IRET, with no direct JMP. Independently:
1024×(32+2+2+6+10+8)+15−3−4+11 = **61459**;
odd stack adds 1024×6 = **67603**. Expectations remain **240–241/264–265**
ticks and handler/application/countdown **1024/0/0**. Current native observations
are **240/264** on every pass and both boots. External entry remains **32/35**.

The ordinary-I/O interval has no JMP: **11 + 2048×8/9/10** clocks still yields
64–65/72–73/80–81 ticks. Relative-word transfer loops also contain no JMP:
4096 odd transfers add 4096 clocks = **16 ticks**, with the existing ±2
bound for subtracting separately quantized samples. No source, layout, literal
budget, continuation assertion or tolerance was altered in these consumers.

## Commands and results

All commands run in the assigned worktree; web commands run in `web`.
Production preview: **http://127.0.0.1:4183/wonderswan-rust/**, explicitly passed
as `WS_PREVIEW_URL`. Complete browser smoke precedes complete compatibility;
native checks finish before these suites. Wasm builds prepend the installed
`$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin` to PATH.

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Passed; new cartridge reproduces and existing cartridges are unchanged. |
| `python3 demo/build.py --check` | Passed; demo unchanged. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Passed: 211 core unit + 36 integration + 1 web unit = **248**; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `git diff --check` | Passed; final whitespace check repeated after documentation completion. |
| `npm ci` | Passed; lockfile unchanged. |
| `npm run build:wasm` | Passed; corrected production Wasm rebuilt after negative control restoration. |
| `npm run build` | Passed; production TypeScript/Vite assets. |
| `npm run test:browser` | Passed completely; desktop playthrough, local file/lifecycle, fullscreen, input and mobile touch coverage. |
| `npm run test:compat` | Passed completely after smoke; all original probes plus absolute JMP, unchanged LOOP/IRQ, input, raw ROM, retention and storage-failure coverage. |

## Tested identities

SHA-256 of tested sources/artifacts (validation prose excluded):

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `9cf3ff959de68204fce8aa61cb97fd6dd91b44dd6d2e4dc24afd7e1c1a9c6902` |
| `crates/ws-core/src/cpu/jmp_tests.rs` | `77e207b3b889b137aef8ff8fc96dad82630146cb7b016788365dd431e5af7ea2` |
| `crates/ws-core/tests/jmp_timing.rs` | `afea8be0f85705fd870875ac8e12db6503eb7be06bc14777d42dc0d1460566f4` |
| `fixtures/jmp-timing.asm` | `a6a3b63cfe7cbc20dceb2296efc03ff9ef9ab076d6707f728ec56a60f8557877` |
| `fixtures/generated/jmp-timing.ws` | `a609ec18fda97aa419338cf98905eaf8d2ab04097d305b38d6cec7172d192123` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `fixtures/generated/word-timing.ws` | `002bedf305b3fd2c4bde2db74c365d0970b4c8deb5ec8ce5e4c82dec9c64f73c` |
| `fixtures/generated/ordinary-io.ws` | `2876e7c63b4609285f805f3b24ba898cc7930138244f66282e0274f3a473fac1` |
| `fixtures/build.py` | `d9465d557d2e22e49863f8625cd6eebc815baf8653da2fe211e33b1ac17c25be` |
| `fixtures/loop-timing.asm` | `b042903d4efaef287aca66c8a550b53cfc3412e6da60c3c18b7561971ee4e40f` |
| `fixtures/irq-timing.asm` | `4349b70d04cbc832ce0610f970ef9de04522c8b640a2de0361ac9e7e3f32c530` |
| `crates/ws-core/tests/loop_timing.rs` | `9024e1dd653db8a11700eb6ded5a5253aacd8ace4ac86d6648bc734fceceaada` |
| `crates/ws-core/tests/irq_timing.rs` | `0c023c2da7fadb41a040cb90c61431f60734f20ef9eed0c7a2b3ab22bc558825` |
| `web/scripts/loop-timing.mjs` | `b620a5deb6635b103518e21d40ae4f22cea1ba96db03c235123ee4e09bd3bd33` |
| `web/scripts/irq-timing.mjs` | `1eabd0bd0af64bb628856aba91111ee54a6c33e8e5586661330e56c494d3ed4c` |
| `web/scripts/jmp-timing.mjs` | `8dc3734995d1470dc1978ef02271a6deac70e23e89e48f1baeb56f4f89cc858c` |
| `web/scripts/compat.mjs` | `30321a47dd710706d6babe339ff4e64dc9a4fe95fa17e2481e9009eab04b82e2` |
| `web/pkg/ws_web_bg.wasm` | `89a01126854f416dd82d8cdeff1ffa47651682bbf9ac2088afab10f71fc5f2f8` |
| `web/dist/assets/index-BpN6Pl6l.js` | `af970fb602b43a5644a685d5d2d089f57632e61da06400e6b08ccf1bdaf94134` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/ws_web_bg-BGU9c2Yb.wasm` | `89a01126854f416dd82d8cdeff1ffa47651682bbf9ac2088afab10f71fc5f2f8` |

Wasm/pkg/dist remain ignored build products. Original assembly/cartridge,
independent consumers and validation evidence are included in the checkpoint.

## Criterion review and limits

The local implementation/Coordinator review compared the final diff, source
Issue and evidence against each mandatory criterion. It also checked the generated
NASM layouts, independent sums, unchanged historical evidence and scope exclusions.

| Mandatory criterion | Evidence and assessment |
| --- | --- |
| Record oracle before changing execution costs | Independent state/timing tables and cartridge sums were recorded first in `jmp-timing.md`; the CPU edit changes only E9/EA/EB execution and target parity. Actual fetches/waits, interrupt entry and DMA remain separate. |
| Native timing/state coverage and preservation | Eight CPU groups pass, covering every form, both parities, relative limits/directions/zero, IP/physical/bank wraps, CS changes, full FLAGS/registers/stack/RAM/ports, A0 controls, internal RAM, prefixes through seven and E8/indirect/Jcc/JCXZ/REP preservation. |
| Nine configurations and shared device advancement | Live mode round trips and exact normal-frame tests pass for all forms and parities, with one instruction, raster/frame/timer/audio boundaries, no destination execution, IRQ/TF/shadow/IRET and separate DMA charges/gates. |
| Original absolute cartridge and independent consumers | CC0 source and reproducible ROM, all three forms and both target parities, CPU execution/destination counts, CX and continuation, stale completion invalidation and restart. Old native/browser CPU actually fails at 304 ticks; corrected native, focused browser and complete production compatibility consumers pass. |
| Existing timing consumers | Source, generated bytes and native/browser literals remain unchanged. LOOP sums/counts/CX/continuation and IRQ 61459/67603 clocks, 240–241/264–265 ticks, 32/35 entry and 1024/0/0 results are preserved. The former IRQ cost still fails at 180 ticks. |
| Broader supported behavior and mandatory commands | All 248 native tests, complete production smoke and complete production compatibility pass, including firmware-free startup, LCD/input/frame, audio/PCM/Hyper Voice, GDMA/SDMA, SRAM/EEPROM/RTC, save integrity, local handling and lifecycle checks. |
| Documentation, provenance and evidence | Primary-source correction, independent calculations, current observations, actual failures, unchanged historical evidence, artifact identities and unperformed optional checks are recorded. No proprietary data or new approval gate. |

Final diff and criterion review found no blocking issue within this scope.
All mandatory checks were performed and passed; no criterion was waived or
tolerance widened. Documentation links and recorded SHA-256 identities were
checked against the final files, and staged whitespace checks passed.

Physical-console measurements, human gameplay, additional browser engines,
optional software experiments and commercial-title compatibility were **not
performed**. Full pipeline/prefetch and external-bus policy remain unresolved;
CALL, indirect JMP and Jcc/JCXZ timing remain separate work. No human or console
prerequisite was introduced. Required post-merge verification: None.
