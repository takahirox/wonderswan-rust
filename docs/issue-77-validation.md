# Issue #77 validation: direct near and far CALL

## Scope and primary evidence

Read live GitHub [Issue #77](https://github.com/takahirox/wonderswan-rust/issues/77)
and repository development/review instructions. Evaluation baseline:
`9e0da4a88f7c0b160349041a65f49f4a116a0faf`; assigned upstream checkpoint:
`bc07a5e028a3be8e32ddfb41e4086ae945299af8`. Implementation and checks took
place in the assigned worktree on 2026-10-24. No push, PR, merge, comment or
remote Issue edit was performed. No usage reset, purchase or model/provider
switch occurred. No new specification correction beyond the Issue was needed.

Read the [NEC V30MZ primary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A conditions/Note 5 and Table A-1, printed pp. 73/76,
and Chapter 4 word/stack alignment rules, pp. 47–48. The old AI assumption
preserved five clocks for E8 02 00 at 0000:1000, aligned internal stack,
odd destination 1005. The replacement is nine. Far CALL's old six-clock
residual becomes eight plus target parity. Decoded 5/10 includes one/two
normal aligned stack writes, so residual execution is 4/8, plus one clock
for an odd destination. Actual fetches, stack writes and waits remain separate
and counted exactly once. These are serialized emulator inferences from the
primary hardware specification, not physical-console measurements.

[Independent timing/state vectors and interval sums](call-timing.md) were
recorded before editing production charges. Only E8 and 9A charges change.
E9 still uses the same four-clock residual and target parity in its shared arm.
No fetch, mapping, transfer consolidation, return timing, instruction semantics,
prefetch, device scheduler or DMA implementation changes are included.

## CPU and shared-device coverage

Nine new CPU groups pass, with expectations independent of production timing
and address helpers:

- Literal zero/forward/backward/signed-extreme near displacements, far literal
  segments/offsets, both opcode/destination/stack parities, contrasting full
  FLAGS including reserved/control bits, exact CS/IP/SP, registers/segments,
  return bytes/order, ports and unrelated RAM. Near saves post-immediate IP;
  far saves old CS then post-immediate IP. Instruction-only execution preserves
  incoming TF without servicing it; separate step tests verify TF delivery.
- Accessible internal RAM and mapped ROM; bank crossings; immediate/IP wrapping
  at FFFE/FFFF; 20-bit FFFFF→00000 fetches; live A0 00/04/08/0C; internal and
  mapped ROM fetch geometry through zero to seven mixed segment/BUSLOCK
  prefixes. Literal byte/word lane costs include actual waits. SS remains the
  stack segment despite overrides. Both REP prefixes reject before operands.
  Exact access-clock counts expose any destination pre-read.
- Stack SP 0000/0001/0002/0003/2800/2801 through accessible/inaccessible internal
  RAM and mapped SRAM, live SRAM waits, ignored high mono-RAM writes, offset
  wrapping and byte lanes. Physical stack writes FFFFF→00000 retain the existing
  ROM ignored-write/wait behavior. Actual RET/RETF restore continuations and SP;
  a mapped nonzero-CS caller returns from internal RAM with its original CS.
  RET/RETF's four-clock execution approximation is unchanged.
- Mono 00 and Color/Crystal 00/80/C0/E0, including live round trips to 00.
  Normal `Machine::run_frame` places near/far calls at the frame boundary with
  both target/stack parities. It asserts exact elapsed clocks, next frame and
  raster line zero, one HBlank timer decrement and one stereo sample. Poison
  INC BX at the destination exposes any extra instruction in that frame.
- Pending IRQ before CALL, one-instruction shadow, pending IRQ priority over
  TF after CALL, TF alone, aligned/odd stacks, exact nested saved continuation
  IP/CS/FLAGS, actual IRET, and subsequent actual return to the caller. External
  entry stays 32/35 internal clocks; TF entry stays 25/28. GDMA and SDMA retain
  separate seven-clock reservations with no CPU fetch, capability errors,
  GDMA color-mode gating and SDMA availability on Color/Crystal in mode 00.

Existing CALL assertions in JMP and word-transfer tests now require 9/8 for
odd/even targets, and 9/10 for the odd target with aligned/odd stack. Their
unrelated JMP, Jcc/JCXZ, return and IRQ assertions remain intact.

A test-authoring correction: the first wrapping-stack oracle incorrectly
charged near CALL with SP=0002 as nine clocks in mono mode. It writes at
0000, an accessible aligned word, so the correct sum is 3 fetch + 1 write +
4 execution = **8**. Chapter 4 and the existing internal-word transfer table
establish that correction. Far SP=0002 remains three write clocks in mono
mode (aligned 0000 plus inaccessible FFFE). This fixes address classification
without widening any tolerance or changing the Issue's hardware requirement.
Initial test compilation also needed explicit u16 annotations. Neither initial
compilation errors nor that incorrect test expectation are reported as passes.

## Original cartridge and negative controls

Original CC0 assembly and reproducible cartridge measure E8/9A to even/odd
RAM destinations. [Every measured instruction, harness cost and timer
quantization](call-timing.md) is documented, including retained approximate
RET/RETF costs. Literal absolute budgets are **118792/122888/147464/151560**
clocks, or **464–465/480–481/576–577/592–593** HBlank ticks. Both consumers
require CPU-written call/destination counts 4096/4096, final CX=0 and marker
51A7, published only after actual return, SP and saved-frame checks.

Mono executes one mode pass; Color/Crystal execute 00/80/00/C0/00/E0/00.
All 512 result bytes clear on startup and mirror to SRAM. Native checks compare
all RAM/SRAM bytes. Browser checks use the real player load/pause/save path
and require zero ROM/save network requests. Both consumers poison retained
success, check invalidation after one frame and verify every pass on two fresh
boots. Cleared unused records are checked too. No application/persistence code
was changed.

Actual negative controls, using the final cartridge identity below:

| Control | Native observation | Production Chromium observation |
| --- | --- | --- |
| Both old CALL charges, before editing production | Mono boot 1 pass 0 near-even = **416**, rejected against 464–465 | Same sample **416**, rejected against 464–465 |
| Corrected near CALL, temporarily restore only far residual six/no parity | Near samples pass; far-even = **544**, rejected against 576–577 | Near samples pass; far-even **544**, rejected against 576–577 |
| Corrected CALL, temporarily restore former IRQ residual 12 | IRQ = **184**, rejected against unchanged 244–245 | Not rerun; unchanged IRQ absolute consumer runs with corrected source in complete compatibility |

Old near sum is 11+4096*26−3=106504 clocks; old far-even sum is
11+4096*34−3=139272. These fail absolute budgets, not relative differences.
The first seven new CPU groups also actually failed against old charges:
5 versus 9 in the full-state vector, 5 versus 8 in prefix/wrap vectors, and
40705 versus 40704 clocks at the normal frame boundary. The two additional
physical-stack/nonzero-CS return groups passed in the final workspace suite.

Old production Wasm: `ws_web_bg-3vzSb5s_.wasm`, SHA-256
`1b3a1319dcdad450e05de4f8a8f360e536e8932de2a9f825bf3954f53fc752fc`;
JavaScript `index-CDe2yE9Q.js`. Far-only old-charge control Wasm SHA-256:
`0b0b2e8ea0bdfb272d242711d9b156036ee1ce0fac710ca31d92351d94f7fad2`;
JavaScript `index-V-stfCq2.js`, SHA-256
`86ac099ba13644fbc4ec5c3a10313ea3da7a572a361c968d2eefe21944643321`.
A Python `finally` restored CPU source byte-for-byte after temporary far/IRQ
controls. Corrected Wasm was rebuilt before complete production suites.

Corrected native observations on both boots:

| Model / pass | Near even | Near odd | Far even | Far odd |
| --- | ---: | ---: | ---: | ---: |
| Mono 00 | 464 | 480 | 576 | 592 |
| Color/Crystal pass 0/2/3/4/6 | 464 | 480 | 576 | 592 |
| Color/Crystal pass 1 (80) | 465 | 481 | 576 | 592 |
| Color/Crystal pass 5 (E0) | 464 | 480 | 576 | 593 |

Every count pair was 4096/4096, CX=0 and continuation/stack marker 51A7.
Production Chromium compatibility produced exactly the same CALL table on
every pass and both boots, including the three one-tick phase variations.
All counts, CX and stack/return/continuation markers passed.

## Existing interval audit

Re-read all existing timing assembly and sample boundaries. None of the IRQ,
JMP, LOOP, Jcc/JCXZ, ordinary-I/O or word probes executes a direct CALL between
its timer observations. Setup calls change initial timer phase only; existing
floor/ceil quantization already permits that phase. No existing fixture bytes,
layout, absolute bounds or tolerances change.

Independent recalculation from measured instructions:

- IRQ: 1024*(32+2+2+7+10+8)+15−4−4+11 = **62482**; odd stack adds 1024*6
  = **68626** (244–245/268–269 ticks). Former entry subtracts 1024*15 =
  47122/53266 (184–185/208–209). Handler JNZ at 2011 targets odd 2017 on
  1023 iterations. Counts remain 1024 handler / 0 application / 0 countdown.
- JMP: 11+4096*22/23/26/27−3 = **90120/94216/106504/110600**.
  All harness JNZ targets are even 2004; unchanged counts/CX/marker apply.
- Jcc/JCXZ: 11+4096*26/27/23−3 = **106504/110600/94216**, with taken
  even/odd/fallthrough and unchanged CX 0/0/1, counts and marker.
- LOOP taken E2: 11 or 13 + 4096*4 + 4095*7 or 8 + final 4 =
  **45064/49161**. E1/E0 final/taken overhead gives **49160/53257**.
  Fallthrough harnesses remain **94216/98312/98312**.
- Ordinary I/O: **11+2048*8/9/10**, hence 64–65/72–73/80–81 ticks.
  Relative-word odd-minus-even is **4096 clocks = 16 ticks**, retaining ±2
  for subtraction of two independently quantized intervals.

All unchanged native fixture consumers pass in the complete workspace suite.
Production Chromium observations on both boots of all models:

- IRQ remains 244/268 ticks, with counts 1024/0/0 on every pass.
- JMP base samples 352/368/352/368/416/432; Color/Crystal pass 5 sample 3
  is 369 within unchanged 368–369.
- Jcc/JCXZ base samples 416/432/368/416/432/368; Color/Crystal pass 4
  sample 4 is 433 within unchanged 432–433.
- LOOP base samples 176/192/192/208/192/208/368/384/384; sample 4 on pass 0
  is 193 on all models, within unchanged 192–193. All counts/CX/markers pass.
- Ordinary I/O and relative-word consumers pass their unchanged absolute and
  relative bounds. Initial phase differences follow the slower setup calls;
  measured instructions/layouts and tolerances are unchanged.

Historical evidence in Issues #39/#69/#71/#73/#75 is preserved.

## Commands and results

Run from the assigned worktree, npm commands in `web`. Explicit production
preview: `http://127.0.0.1:4187/wonderswan-rust/`. `npm run test:browser`
finished successfully before `npm run test:compat` started. Wasm builds use
the installed stable-aarch64-apple-darwin toolchain directory prepended to PATH.

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass; new cartridge reproduces, all existing cartridges unchanged. |
| `python3 demo/build.py --check` | Pass; demo unchanged. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 226 core unit + 38 integration + 1 web unit = **265**, no failures; empty doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass; repeat after final evidence. |
| `npm ci` | Pass; package/lockfile unchanged. |
| `npm run build:wasm` | Pass for both negative-control builds and corrected production. |
| `npm run build` | Pass for both negative-control builds and corrected production. |
| `WS_PREVIEW_URL=… npm run test:browser` | Complete pass: all models, desktop playthrough, local-file/lifecycle/race coverage, fullscreen, input and mobile touch. |
| `WS_PREVIEW_URL=… npm run test:compat` | Complete pass: new CALL absolute probe; unchanged IRQ/JMP/LOOP/Jcc/word/I/O probes; all CPU, model, LCD/input, PCM/audio/DMA/Hyper Voice, save, internal EEPROM, RTC and storage-failure checks. |

## Tested identities

SHA-256 of tested source/artifacts; validation prose excluded. Wasm/pkg/dist
are ignored build outputs; source, cartridge, consumers and this evidence are
committed in the assigned artifact worktree.

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `6c5384eeb6b28bef15c0870f466b0fe19760898567a7f293247e6cb39e308de7` |
| `crates/ws-core/src/cpu/call_tests.rs` | `ee922219f3658cfbcf3e9c9e79e2393a58fe99f819bbba4dfcdfd6037ef50e6b` |
| `crates/ws-core/src/cpu/jmp_tests.rs` | `2af382ad8d7fe73d48d4bcc406b72acded5e1faccbe6c081371e7d50d1ec40a8` |
| `crates/ws-core/src/cpu/word_transfer_tests.rs` | `0e3608e1651251a81ab39ea2a50b9df1e2f3a129a94e7210d25bd4899eba4bba` |
| `crates/ws-core/tests/call_timing.rs` | `1b4fbe79318444c04a6b3b3dc9d7098b1daf69300bf8e5c228d0ce0d9c67082e` |
| `fixtures/call-timing.asm` | `601980b6523868994601e992004dc65520ff928c0aee77c5ff05166b4925bec6` |
| `fixtures/generated/call-timing.ws` | `f2d8d25bab282202fcb89c8ce5c839f106195ed91a8ba9475aa9bcca2ed82651` |
| `fixtures/build.py` | `ce188c429f09fa171472c4f162650754d8f3f0a65feff336b1cb8884e10fa928` |
| `web/scripts/call-timing.mjs` | `0930ae528edf204f519536ba1a69aa41d6f8c39a5bc37f2b89de03e9d455c618` |
| `web/scripts/compat.mjs` | `489cab02806200f137559c8e5c97ffed235933bf9e10154bf741b127f33593ac` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `fixtures/generated/jcc-timing.ws` | `9dbe44ea9745b91f4a8a54b450215d5ea27ed907efc7eb8d8e4eebac9713e2d7` |
| `fixtures/generated/jmp-timing.ws` | `a609ec18fda97aa419338cf98905eaf8d2ab04097d305b38d6cec7172d192123` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/generated/word-timing.ws` | `002bedf305b3fd2c4bde2db74c365d0970b4c8deb5ec8ce5e4c82dec9c64f73c` |
| `web/pkg/ws_web_bg.wasm` | `9f7ee33c838e39c74590826795e0b4b60de72f29df80a7aeff31aa94018dd6ea` |
| `web/dist/assets/index-BoEjn7sk.js` | `88204a72a7d47b8b9020fc2972298e99526484377b4049920ee344ef908e63d9` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/ws_web_bg-BQGm9XWk.wasm` | `9f7ee33c838e39c74590826795e0b4b60de72f29df80a7aeff31aa94018dd6ea` |

## Final criterion review

The local implementation/Coordinator review compares the Issue, complete diff,
primary specification, literal oracles and actual native/browser evidence:

| Mandatory criterion | Evidence |
| --- | --- |
| Independent vectors before charge edit, smallest complete implementation | Timing document and cartridge sums recorded first; only E8/9A residual/parity charges change. E9 behavior, fetch/stack paths, consolidation, waits and destination-read policy are preserved. |
| State, geometry, return frames and architectural continuation | Nine CPU groups cover both opcode/target/stack parities, displacement boundaries, IP/physical/bank wraps, exact full state/return bytes, actual RET/RETF and nonzero caller CS. |
| Internal/mapped fetch and stack, live A0, prefixes/REP | Internal/ROM/SRAM vectors cover live waits and modes, seven segment/BUSLOCK prefixes, SS selection, REP rejection and exact access counts. |
| Shared-device/model integration and arbitration | Normal-frame exact raster/frame/timer/audio, no extra destination instruction, nine model/mode configurations and live returns, IRQ/TF/shadow/IRET, separate GDMA/SDMA and gates; broader REP/HALT regressions pass. |
| Original cartridge and independent absolute consumers | CC0 source/reproducible cartridge, four absolute intervals, CPU call/destination counts, return/stack integrity and continuation, poisoned saved success invalidation and fresh restart; actual near and far old-charge failures in native and production Chromium. |
| Existing timing consumers and former IRQ rejection | Measured bytes audited: no CALL inside old intervals. All bounds/fixtures unchanged; independent recalculation and actual former-IRQ failure retain 32/35 entry and architectural/count acceptance. |
| Complete mandatory tooling and broad regressions | Fixture/demo/fmt/clippy, all 265 workspace tests, complete production browser smoke pass. Complete production compatibility also passes. |
| Documentation, ownership and evidence | Current timing references updated, historical observations retained, CC0 provenance/source and artifact identities recorded, actual failures/corrections described, optional unperformed work stated. |

Final diff/evidence review found no blocking findings within the Issue scope.
All mandatory checks completed and passed; no acceptance criterion was waived.
Recorded source/artifact hashes and changed relative documentation links were
verified against final files. Final staged whitespace checks passed.

No physical-console measurement, human gameplay, other browser engine,
additional lawful software experiment or commercial-title compatibility test
was performed. Those remain optional, and commercial compatibility/full console
pipeline timing remain unestablished. Full prefetch, effective-address hazards,
indirect CALL/JMP, return timing and unrelated bus policies remain separate work.
Required post-merge verification: **None**.
