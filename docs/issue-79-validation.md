# Issue #79 validation: indirect CALL/JMP

## Scope and correction record

Read live GitHub [Issue #79](https://github.com/takahirox/wonderswan-rust/issues/79),
including its empty comments, and repository development/review instructions.
Evaluation baseline: `d068ada04ccb96552f865eee7a23e8948365fc88`;
assigned upstream checkpoint: `5be90a8e40f26deac01c0d6b10e05594b5c0e289`.
Implementation and validation took place in the assigned GitWeave worktree on
2026-10-24. No push, PR, merge, comment or remote Issue edit was performed.
No usage reset, purchase or model/provider switch occurred.

The old AI-authored assumption preserved eight/eleven clocks for near/far
memory JMP to odd 1041 at even opcode IP. The replacement is **nine/fourteen**.
Read the [NEC V30MZ primary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A conditions/Note 5 and Table A-1, printed pp. 73/76,
and Chapter 4's word/stack alignment discussion, pp. 47–48. Its decoded
register/near-memory/far-memory JMP costs 4/5/10 and CALL costs 5/6/12 include
normal aligned operand/stack transfers. Subtract those already charged transfers
to infer residuals **4 near, 8 far**, plus one clock for an odd loaded IP.
Actual fetches, pointer reads, stack writes, alignment and waits remain charged
once. This replaces the preservation assumption for the primary-specification
reason stated in the Issue. No further specification correction was necessary.
These are serialized emulator expectations, not physical-console measurements.

[Literal timing/state vectors and independent cartridge sums](indirect-control-timing.md)
were written before changing production charges. Only FF /2,/3,/4,/5 timing
changes. Near operand sampling, including SP, and far CALL's stack-before-pointer
ordering are unchanged. The parity expression uses the destination actually
loaded after any aliasing. RET/RETF, effective-address hazards, full prefetch,
DMA, device scheduling and other bus policies receive no implementation changes.

## CPU and device acceptance

Ten new CPU groups pass. Oracles use literal encodings, physical addresses,
independent snapshots and transfer sums; none calls production timing helpers.
The three shared helpers from the direct CALL tests only construct initial
state, map source bytes and write expected frame bytes into a snapshot.

- All eight register operands, including SP before its decrement; both opcode,
  destination and stack parities; contrasting full FLAGS 0000/FFFF/F002/FCD7;
  exact registers/segments/IP/SP, saved return bytes, unrelated RAM and ports.
- All eight ModR/M base forms, direct disp16, signed disp8 limits and wrapping
  disp16 examples, default DS/SS and ES/CS/SS/DS overrides. Memory CALL/JMP
  checks near/far forms, target/pointer/stack/opcode parity, exact full state,
  return frames and unrelated memory. High effective offsets use explicit
  segment aliases into accessible RAM without introducing EA-hazard costs.
- Near pointer/stack aliasing samples before pushing; far CALL reads after both
  pushes, with offset-only, segment-only, both-word and nonoverlap cases.
  Even source pointers can become odd loaded destinations through the saved
  continuation. Exact cycles follow that newly loaded parity.
- Internal and mapped ROM fetches/pointers, all live A0 00/04/08/0C controls,
  odd/aligned pointer words and zero through seven segment/BUSLOCK prefixes.
  Longer chains include all segment selectors with DS last; SS still selects
  the stack. Both REP prefixes reject all extensions before operand fetches;
  all far-register encodings retain their exact diagnostic. Exact access-clock
  assertions expose destination pre-reads or duplicate transfers.
- Instruction and pointer offset wrapping, ROM bank-window crossing and 20-bit
  FFFFF→00000/FFFFE→00000 boundaries, with explicit per-lane costs. SS=FFFF
  stack writes crossing into low RAM retain ignored ROM-write accounting.
- Mapped SRAM pointers and stacks at both parities and offsets FFFF/0000,
  live existing SRAM wait bit, stack SP 0000/0001/0002/0003/2800/2801 and
  accessible/inaccessible high internal RAM. Entire RAM and SRAM snapshots
  check ignored writes, mirroring, unrelated bytes and aliasing. Actual RET/RETF
  restore intact continuations, full FLAGS and SP at their retained approximate
  costs. A nonzero mapped caller CS is restored by actual RETF.
- Mono 00; Color/SwanCrystal 00/80/C0/E0, including live returns to 00.
  Normal `Machine::run_frame` exercises all six forms at both destination/stack
  parities at frame boundaries. Exact shared clocks, frame/raster progression,
  one HBlank decrement and one stereo sample pair are checked. Poison INC BX
  destinations detect an extra instruction; exact CS/IP/registers/FLAGS remain
  at the loaded continuation.
- All six forms also exercise pending IRQ before dispatch, IRQ/TF priority,
  one-instruction shadow, TF-only delivery, exact nested destination frames,
  actual IRET and subsequent CALL return integrity. External IRQ entry stays
  32/35 clocks, TF 25/28. Separate GDMA/SDMA seven-clock reservations perform
  no CPU fetch; Mono capability errors, GDMA color-enable gating and SDMA
  support in Color/Crystal mode 00 remain checked. Existing REP/HALT and
  instruction-generated shadow suites pass too.

The two obsolete JMP assertions now require nine/fourteen instead of
eight/eleven. Direct CALL/JMP, Jcc/JCXZ, LOOP and word-transfer assertions retain
their existing bounds and behavior. No application or persistence code changed.

## Original cartridge and independent consumers

Original CC0 source `fixtures/indirect-timing.asm` builds a reproducible 128 KiB
cartridge with 32 KiB SRAM through existing fixture tooling. Twelve samples
cover all six forms and both destination parities. The timing document records
every measured instruction, exact encoding/target offsets, harness costs,
absolute interval sums and 256-clock HBlank quantization. RET/RETF and the
other unchanged harness instruction costs remain emulator approximations.

Both independent consumers require literal absolute budgets, CPU-written
execution/destination counts 4096/4096, CX=0, actual return continuation and
marker 51A7 conditional on SP and saved frame/sentinel checks. Native compares
all 1024 RAM/SRAM result bytes. Production Chromium uses the real player
load/pause/save path and requires zero ROM/save network requests. Both poison
retained success, require completion invalidation after one frame, clear unused
observations, run all nine configurations with live round trips and verify two
fresh boots. No host injects timing results, execution counts or pass markers.

Actual separate negative controls, all using cartridge SHA-256
`e072516d32eb71541878f30670960bdcbdbb29325b76ffc585e62f63c4a2af83`:

| Control | Native rejection | Production Chromium rejection |
| --- | --- | --- |
| Original indirect near/far charges | Mono boot 1 pass 0 sample 1 (odd register CALL): **464** ticks versus required 480–481 | Same sample **464**, rejected against 480–481 |
| Correct near parity, retain only original far six/no-parity | First four near samples pass; sample 4 (even far CALL): **592** versus required 624–625 | Same sample **592**, rejected against 624–625 |
| Correct indirect charges, temporarily restore former IRQ residual 12 | Existing IRQ sample 0: **184** versus unchanged 244–245 | Not repeated; complete corrected compatibility runs the unchanged IRQ consumer |

Old near-odd sum is 11+4096*29−3=118792 clocks; old far-even sum is
11+4096*37−3=151560. Rejection uses absolute bounds rather than parity-only
comparisons. Seven new CPU groups were also run before the charge edits;
all seven rejected the old implementation. No failing negative control is
reported as a passing implementation run. The temporary IRQ experiment used
a Python `finally` to restore source bytes before final native/Wasm builds.

Original-charge production Wasm SHA-256:
`db09f769e669221a50237f2a7aa08d9527015adcf7bc921d9d394b98130845fd`;
JavaScript `index-Dkdkkerf.js` SHA-256
`d0fa79e73bf805a1e20d42e838ce0b772585a2adc727debd5f6abc83e5d83c43`.
Far-only original-charge control Wasm SHA-256:
`50c685934b9dd62f9fc98651afa82fdae963d1e2683bf21b2162f969eed394e9`;
JavaScript `index-DoksMdP-.js` SHA-256
`e442624ee9c3c505f64a35559c6145e105e45b966ffe15ce16069c7d3b99efc5`.

Corrected native observations on both boots:

| Model / pass | CALL reg even/odd | CALL near mem even/odd | CALL far mem even/odd | JMP reg even/odd | JMP near mem even/odd | JMP far mem even/odd |
| --- | --- | --- | --- | --- | --- | --- |
| Mono 00 | 464/480 | 512/528 | 624/640 | 352/368 | 400/416 | 480/496 |
| Color/Crystal pass 0/1/2/3/6 | 464/480 | 512/528 | 624/640 | 352/368 | 400/416 | 480/496 |
| Color/Crystal pass 4 (00) | 464/480 | 512/528 | 624/640 | 352/368 | 400/417 | 480/496 |
| Color/Crystal pass 5 (E0) | 464/480 | 512/528 | 625/640 | 352/368 | 400/416 | 480/496 |

Every count pair is 4096/4096, CX=0 and return/continuation marker 51A7.
Production Chromium produced exactly the same table on every pass and both
boots, including the one-tick variations. All absolute bounds, CPU-written
counts, CX and return/continuation markers passed.

## Audit of existing intervals

Read all existing timing assembly and sample boundaries. None contains an
indirect CALL/JMP between timer observations. Existing source/cartridge bytes,
layouts, absolute intervals and tolerances remain unchanged, including setup.
Independent recalculation, with the same retained harness approximations:

- Direct CALL: 11+4096*29/30/36/37−3 =
  **118792/122888/147464/151560** clocks, 464–465/480–481/576–577/592–593 ticks.
- Direct JMP: 11+4096*22/23/26/27−3 =
  **90120/94216/106504/110600**; harness JNZ targets even 2004.
- Jcc/JCXZ: 11+4096*26/27/23−3 = **106504/110600/94216**.
- LOOP taken E2: 11 or 13 + 4096*4 + 4095*7 or 8 + final 4 =
  **45064/49161**. E1/E0 final/taken overhead yields **49160/53257**;
  fallthrough harnesses remain **94216/98312/98312**.
- IRQ: 1024*(32+2+2+7+10+8)+15−4−4+11 = **62482**;
  odd stacks add 1024*6 = **68626** (244–245/268–269 ticks).
  Former entry subtracts 1024*15, giving 47122/53266 (184–185/208–209).
  Counts stay 1024/0/0. Actual former-charge rejection is recorded above.
- Ordinary I/O: **11+2048*8/9/10**, retaining 64–65/72–73/80–81 ticks.
  Word-transfer odd-minus-even remains **4096 clocks = 16 ticks**, retaining
  its existing ±2 for subtraction of independently quantized intervals.

All unchanged native consumers and complete production Chromium compatibility
pass. IRQ remains 244/268 ticks with counts 1024/0/0. Direct CALL remains
464/480/576/592, with Color/Crystal pass 1 near samples 465/481 and pass 5
far-odd 593. Direct JMP remains 352/368/352/368/416/432, with pass 5 sample 3
at 369. Jcc/JCXZ remains 416/432/368/416/432/368, with pass 4 sample 4 at
433. LOOP retains its existing 176/192/192/208/192/208/368/384/384 base
samples and floor/ceil variations. Ordinary I/O and relative-word consumers
pass their unchanged assertions. No historical observation or tolerance was
rewritten.
Historical observations in previous validation reports remain unchanged.

## Commands and actual results

Commands run in the assigned worktree; npm commands in `web`. Explicit local
production preview: `http://127.0.0.1:4187/wonderswan-rust/`. Browser smoke
finishes before compatibility starts. The initial Wasm build found Homebrew
Rust without the Wasm target; selecting the already installed stable rustup
toolchain directory in PATH fixed it. No package/lockfile/toolchain installation
or configuration change was needed. `npm ci` passed without changing manifests.

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass; new cartridge reproduces; existing artifacts unchanged. |
| `python3 demo/build.py --check` | Pass; unchanged demo reproduces. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 236 core unit + 39 integration + 1 web unit = **276**, empty doc tests. Final workspace run includes the strengthened mixed-prefix and nonzero-CS checks. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass, including the final mixed-prefix and nonzero-CS test refinements. |
| `git diff --check` | Pass, including final evidence and staged files. |
| `npm run build:wasm` | Pass for both original-charge controls and corrected production. |
| `npm run build` | Pass for both original-charge controls and corrected production. |
| `WS_PREVIEW_URL=… npm run test:browser` | Complete pass: all models, desktop playthrough, loading/lifecycle/races, fullscreen, input and mobile touch. |
| `WS_PREVIEW_URL=… npm run test:compat` | Complete pass: indirect CALL/JMP and unchanged timing consumers; CPU, model, LCD/input, audio/PCM, DMA/Hyper Voice, cartridge saves, EEPROM/RTC and storage-failure coverage. |

Final review strengthened mixed-prefix coverage and nonzero destination CS in
frame/IRQ/IRET/RETF checks. All ten CPU groups and clippy passed after these
refinements. These test-only changes do not alter production Wasm. Full output
logs are local ignored build evidence in `target/issue-79-validation`; the durable results,
calculations and artifact identities are included here.

## Tested source and artifact identities

SHA-256 of tested source/artifacts; validation prose excluded. Wasm/pkg/dist are
ignored build outputs. Source, cartridge, consumers and this report belong to
the assigned checkpoint worktree.

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `5170f8f30dac443d84d05dd4276cfd5504e13261648a80703f44deb70637a7f1` |
| `crates/ws-core/src/cpu/call_tests.rs` | `1bba123ee0a0428ece455d3729750576160e5f8b7517688015377b8d6422a1d1` |
| `crates/ws-core/src/cpu/jmp_tests.rs` | `34c1b63676279854ddc527dee3625d634ec19d55b2f1c5f8f14b091d5418bd07` |
| `crates/ws-core/src/cpu/indirect_tests.rs` | `71b5778832aa24b84b90643086ba6b9983d01d2cdc9bd43e328849f211629825` |
| `crates/ws-core/tests/indirect_timing.rs` | `61c1240ebea10d2b5d0e5a129b44cf247d9be0a759730499bc2031c873a5967c` |
| `fixtures/indirect-timing.asm` | `8f74090acba8d42272ec02b5d79349405cf079ffebd0d165c556bb4bb53bac0b` |
| `fixtures/generated/indirect-timing.ws` | `e072516d32eb71541878f30670960bdcbdbb29325b76ffc585e62f63c4a2af83` |
| `fixtures/build.py` | `15fa8db559fb982f4e939787b4c58f7046ac3b104c8983ea7340580524f30917` |
| `web/scripts/indirect-timing.mjs` | `954aab029d8a07b001cf449f485573ec96235e88c888a04f8284549cb4d18007` |
| `web/scripts/compat.mjs` | `b8c0062e6a022eb4774382c8d9de69a1dd45e78bccbfe0ddf07d64f36d729214` |
| `fixtures/generated/call-timing.ws` | `f2d8d25bab282202fcb89c8ce5c839f106195ed91a8ba9475aa9bcca2ed82651` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `fixtures/generated/jcc-timing.ws` | `9dbe44ea9745b91f4a8a54b450215d5ea27ed907efc7eb8d8e4eebac9713e2d7` |
| `fixtures/generated/jmp-timing.ws` | `a609ec18fda97aa419338cf98905eaf8d2ab04097d305b38d6cec7172d192123` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/generated/ordinary-io.ws` | `2876e7c63b4609285f805f3b24ba898cc7930138244f66282e0274f3a473fac1` |
| `fixtures/generated/word-timing.ws` | `002bedf305b3fd2c4bde2db74c365d0970b4c8deb5ec8ce5e4c82dec9c64f73c` |
| `web/pkg/ws_web_bg.wasm` | `66cc66581f330ffe886ce65fe9754444ed04e9b5fb7dba4986783be6ec0081cf` |
| `web/dist/assets/index-Bi4yU0TC.js` | `fff27a923b39837427901eb9e556705ad3a8c76e086f61d494aba4d685b12ae0` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/ws_web_bg-B5LnRSVZ.wasm` | `66cc66581f330ffe886ce65fe9754444ed04e9b5fb7dba4986783be6ec0081cf` |

## Final criterion review

Local review compared the source Issue, primary specification, complete diff,
independent oracles and actual native/production-browser evidence:

| Mandatory criterion | Evidence and result |
| --- | --- |
| Correct all six forms with an independent pre-edit oracle | Literal document and cartridge budgets preceded charge edits; near residual 4 plus loaded IP parity, far 8 plus loaded IP parity; actual transfers retained once. Only two production charge expressions change. |
| Architectural state, ordering, addressing and continuation | All registers including SP, full FLAGS, ModR/M/displacements/overrides, opcode/destination/pointer/stack parity, offset/bank/20-bit wrapping, snapshots, aliasing and actual RET/RETF pass. Far parity follows the post-stack pointer value. |
| Mapped buses, waits, prefixes, errors and counts | Internal/ROM/SRAM, live A0 and SRAM waits, mixed prefixes through seven, SS selection, REP and far-register diagnostics, exact access counts all pass. Existing consolidation and ignored-write policies remain in place. |
| Shared clocks and model/device integration | All six forms at normal frame boundaries check exact cycles, raster/frame, HBlank timer and stereo samples without an extra destination instruction; nine configurations/live mode round trips, IRQ/TF/shadows, nonzero-CS saved destination/IRET/return integrity and separate gated GDMA/SDMA reservations pass. Existing REP/HALT and device suites pass. |
| Original source-included cartridge and independent consumers | CC0 source/reproducible artifact, twelve absolute intervals, CPU-written counts/CX, stack/return/continuation markers, every model/mode, stale-save invalidation and two boots pass. Old near parity and old far charges separately fail native and production Chromium consumers using the same final fixture identity. |
| Existing timing acceptance | All existing timing bytes/bounds remain unchanged; independent sums and actual complete native/browser acceptance retain direct CALL/JMP, Jcc/JCXZ, LOOP, ordinary I/O, words, IRQ 32/35 and former-entry-charge rejection. Historical reports are preserved. |
| Broad regressions and mandatory tooling | Fixture/demo/fmt/workspace/clippy/whitespace and sequential complete production browser smoke/compatibility pass; startup, firmware-free policy, models, LCD/input/frame, PCM/audio, DMA/Hyper Voice, SRAM/cartridge/internal EEPROM, RTC, local storage and ownership paths retain their acceptance. |
| Durable evidence and bounded scope | Source/artifact hashes, calculations, controls, actual observations and tool-environment correction are recorded here. Current docs point to the corrected indirect policy. No unrelated runtime changes, new approval gates, weakened bounds or historical result edits. |

Final local review found no blocking issue within the requested scope. All
mandatory acceptance completed; none was waived. Changed documentation links,
recorded source/artifact hashes and final whitespace were checked against files.

Physical-console measurements, human gameplay, another browser engine,
additional lawful software experiments and commercial-title compatibility tests
were not performed. These remain optional. Full console pipeline/prefetch,
effective-address hazards, return timing and unrelated bus policies remain
separate gaps; commercial compatibility is unestablished. Required post-merge
verification: **None**.
