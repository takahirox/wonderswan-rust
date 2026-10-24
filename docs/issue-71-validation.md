# Issue #71 validation: LOOP timing and probe budgets

## Scope and source reconciliation

Source Issue: [#71](https://github.com/takahirox/wonderswan-rust/issues/71), read
from GitHub during this run. Evaluation baseline:
`995741d92168ee7156cb8d853ac86d244714bf19`; assigned upstream checkpoint:
`04bf844318352c764bd44307ede15ca6bb1406c4`. Work and checks were performed in
the assigned worktree on 2026-10-24. No push, PR, merge or remote comment was
performed. No usage reset, purchase or model/provider switch occurred.

[LOOP timing](loop-timing.md) records the instruction-specific accounting,
independent literal vectors and cartridge sums **before** the CPU timing edit.
The NEC V30MZ manual was inspected at its primary Renesas URL: Appendix A
conditions/Note 5 and Table A-1 (printed pp. 73, 77). The old AI-authored
4/1 decoded-clock assumption for all E0–E2 is replaced by E2 5/2 and E0/E1
6/3, with one extra clock only for a taken odd destination. These register-only
instructions have no operand transfer to subtract. Two actual byte fetches
remain charged separately, yielding internal totals E2 7/8/4 and E0/E1 8/9/5
(taken even/taken odd/fallthrough). CS/IP, CX decrement and FLAGS semantics
are unchanged. E3, JMP, other instruction timing, external bus policy and
prefetch are outside this change. No further primary-specification correction
or acceptance reduction was necessary.

## Native coverage and actual observations

Six new CPU tests cover:

- Literal E0/E1/E2 vectors with both ZF values, incoming CX 0/1/2/FFFF,
  decrement wrap, positive/negative displacements, both target parities,
  complete incoming FLAGS patterns (including reserved bits), all unchanged
  registers/segments, RAM and ports, and no device advancement inside CPU step.
- All nine configurations: Mono/00, Color and Crystal each in 00/80/C0/E0,
  with live 00→80→00→C0→00→E0→00 transitions. Internal fetches ignore SRAM
  and cartridge-I/O waits; mapped ROM fetches retain live A0 00/04/08/0C
  width/wait controls and two distinct byte charges. Mapper-bank boundaries,
  IP FFFF displacement-fetch wrap, positive/negative target wrap and physical
  FFFFF→00000 fetch wrap have independently addressed literal expectations.
- Zero through seven mixed segment/BUSLOCK prefixes add only their actual
  fetch costs. Existing REP rejection and E3 timing/unchanged CX are asserted.
- Normal `Machine::run_frame` boundaries for each opcode and even/odd taken
  and fallthrough paths. Exactly one instruction advances raster/frame state,
  one HBlank timer tick and one stereo audio sample, with no extra INC BX or
  application instruction. The old execution path fails this boundary check.
- IRQ before execution leaves CX untouched; a shadow permits exactly one LOOP;
  simultaneous IRQ/TF at the eligible boundary selects external entry. TF-only
  entry and external entry add their independent 25/28 and 32/35 clocks once,
  with exact entry-transfer counts, saved FLAGS/CS/IP and real IRET continuation.
- GDMA and SDMA retain separate seven-clock reservations before LOOP and make
  no CPU fetch charge. Mono capability errors, GDMA's color-enable gate and
  SDMA in Color/Crystal mono mode remain asserted. Existing REP/HALT, shadows,
  priority and other delivery tests remain enabled in the full workspace suite.

The new original CC0 cartridge runs through normal firmware-free startup and
`Machine::run_frame`. Its native and independent production Chromium consumers
check absolute HBlank observations for all three opcodes, even/odd taken paths,
and repeated fallthrough paths (conditional early stops with CX remaining 1).
Each sample has an independent CPU-written count of 4096, final CX and
continuation/destination marker 51A7. Every model, live mode pass and fresh
restart is checked. Startup invalidates stale SRAM success; native tests verify
invalidation after the first frame and compare all 512 result bytes with SRAM.
Browser consumers poison retained saves, require the new boot count plus full
records, and assert no requests occur while handling the user's ROM/save.

Actual native observations on both boots:

| Model / live pass | E2 even/odd | E1 even/odd | E0 even/odd | E2/E1/E0 fallthrough |
| --- | --- | --- | --- | --- |
| Mono, 00 | 176/192 | 192/208 | 192/208 | 368/384/384 |
| Color and Crystal, passes 0/1/3/4/6 | 176/192 | 192/208 | 192/208 | 368/384/384 |
| Color and Crystal, passes 2/5 (return to 00) | 176/192 | 192/208 | 192/208 | 369/384/384 |

All counts, final CX and continuation markers passed. Only the independently
derived 256-clock quantization permits the +1 tick; no tolerance was widened.
Production Chromium observations matched this native table exactly on both
boots of every model and pass, including the 369-tick E2 fallthrough samples
on Color/Crystal passes 2/5. All CPU counts, CX and continuation markers passed.

The existing IRQ fixture source and cartridge are byte-for-byte unchanged.
Its measured LOOP target is 2005, so 1023 taken iterations gain two clocks each
and final fallthrough gains one: 59412/65556 becomes **61459/67603** clocks.
The native and browser literals now require **240–241 / 264–265** ticks.
Native and production Chromium observations were **240/264 on every pass
and both boots**, with handler
count, application count and handler continuation **1024/0/0**. The entry cost
remains 32/35 internally. See [the recalculated sum](external-irq-timing.md).
Issue #69's prior 232/256 observations remain historical, not new passes.

## Negative controls and development failures

Before changing timing, all four initial CPU test groups failed on the old
implementation: internal E2 taken returned **6 instead of 7**, mapped ROM E2
fallthrough returned **3 instead of 4**, the prefix test returned **6 instead
of 7**, and the normal frame check reached **40705 instead of 40704** clocks.
The two added integration groups for IRQ/TF and DMA were added afterwards.

The new absolute cartridge test failed natively with **Mono boot 1 pass 0,
sample 0 = 160 ticks**, outside the independently derived 176–177 range.
The independent browser consumer against a production build of the old CPU
also failed at **160 ticks**. This used the final cartridge bytes, not a
relative-alignment-only comparison. Old production assets were
`ws_web_bg-BJARsFZz.wasm` and `index-CghJ2dQB.js`; they were rebuilt with the
corrected CPU before final browser acceptance.

With corrected LOOP timing, temporarily restoring **only** the former IRQ
execution constant 12 instead of 27 made the updated native IRQ consumer fail
at **180 ticks** against 240–241. The source was restored byte-for-byte in a
`finally` block before running the full workspace suite. Thus the recalculated
IRQ oracle still discriminates against the former IRQ charge.

Development/setup corrections, without changing acceptance:

- The first new native cartridge consumer did not compile because its pass
  counter inferred u16 while memory indexing required u32. The explicit
  conversion was fixed before the meaningful old-code cartridge failure.
- The prefix vector initially omitted the target parity change induced by an
  odd prefix count. Its literal expected cost was corrected before running the
  corrected CPU. The old-code prefix failure at zero prefixes is unaffected.
- Odd cartridge bodies deliberately insert one NOP before their target, so
  their sums include its two clocks once (49161/53257). Generated bytes,
  native/browser integer ranges and the final accounting agree.
- Three initial full smoke attempts exposed asynchronous input assertions: the
  first captured a release framebuffer before a later redraw (hash 936c… then
  8d31…); the second missed the first right-move tile at (4,8); the third missed
  a right-move during the local-cartridge playthrough at (18,5). The first two
  overlapped native validation; the third occurred after it, so native load
  alone does not explain them. These are failures, not passing runs.
  The test's fixed 45 ms tap can elapse without a renderer tick, and release
  snapshots can precede a pending redraw. `smoke.mjs` now waits for two renderer
  callbacks with input down and two after release, including release/cancel/blur
  snapshots. This gives the existing scheduler opportunities to observe input
  and render; its four-frame catch-up bound keeps the tap within the demo's
  eight-frame repeat interval. All exact tile, score, framebuffer-stability,
  repeat and input-clearing assertions remain. No production input/scheduler
  code, timing oracle or tolerance changed. Final smoke: **passed completely**, including all desktop playthroughs and
  real mobile touch hold/release/cancel/blur checks.

## Commands and final results

The explicit production preview is `http://127.0.0.1:4187/wonderswan-rust/`,
supplied through `WS_PREVIEW_URL`. The final complete browser suites execute
sequentially, smoke before compatibility. Wasm uses the already installed Rustup
stable toolchain/target by prepending
`$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin` to PATH.

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Passed; new original cartridge reproduces; all existing bytes unchanged. |
| `python3 demo/build.py --check` | Passed; demo unchanged. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Passed: 203 core unit + 35 integration + 1 web unit = **239** tests; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed. |
| `git diff --check` | Passed; final diff and documentation links reviewed. |
| `npm ci` in `web` | Passed; lockfile unchanged. |
| `npm run build:wasm` in `web` | Passed; corrected production Wasm. |
| `npm run build` in `web` | Passed; production TypeScript/Vite assets. |
| `npm run test:browser` in `web` | Passed completely against the explicit production preview after input/render synchronization. |
| `npm run test:compat` in `web` | Passed completely after the final smoke run; all original probes plus LOOP and recalculated IRQ observations. |

## Tested identities

SHA-256 of the tested source/artifacts (validation prose excluded):

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `a0f84349527b6f60c914c12c5fd9ad6604e30003d258d5d947d9ceabf8682950` |
| `crates/ws-core/src/cpu/loop_tests.rs` | `dbe100aa770cc64130056c169eeff0a886e8106a62f9b92fc5316731ea9bb63c` |
| `crates/ws-core/tests/loop_timing.rs` | `9024e1dd653db8a11700eb6ded5a5253aacd8ace4ac86d6648bc734fceceaada` |
| `crates/ws-core/tests/irq_timing.rs` | `0c023c2da7fadb41a040cb90c61431f60734f20ef9eed0c7a2b3ab22bc558825` |
| `fixtures/loop-timing.asm` | `b042903d4efaef287aca66c8a550b53cfc3412e6da60c3c18b7561971ee4e40f` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/irq-timing.asm` | `4349b70d04cbc832ce0610f970ef9de04522c8b640a2de0361ac9e7e3f32c530` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `web/scripts/loop-timing.mjs` | `b620a5deb6635b103518e21d40ae4f22cea1ba96db03c235123ee4e09bd3bd33` |
| `web/scripts/irq-timing.mjs` | `1eabd0bd0af64bb628856aba91111ee54a6c33e8e5586661330e56c494d3ed4c` |
| `web/scripts/smoke.mjs` | `c227f415a36f8bcf13765441f2993a96dac6b71ec4fa93e245862d9602a4afcf` |
| `web/scripts/compat.mjs` | `dcb558405b442726d6ff17eb545ffdede168710544bcf8305a6f0ab2c1b594b5` |
| `web/pkg/ws_web_bg.wasm` | `0f7274ba0fae302e98e4a71ecca3bdfa25ae42be6f1b7c2a9e398b3cd85899a5` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/index-D82v_ZVW.js` | `e363aac8e50da3ee67ae81417ed05205bbe9b720c761b8c84b149c5ae49aba97` |
| `web/dist/assets/ws_web_bg-BlZhds2A.wasm` | `0f7274ba0fae302e98e4a71ecca3bdfa25ae42be6f1b7c2a9e398b3cd85899a5` |

Wasm/pkg/dist remain ignored build products. Fixture assembly, generated
cartridge, independent consumers and validation evidence are committed.

## Acceptance review and limits

The final diff was checked against each mandatory criterion:

| Acceptance area | Evidence / assessment |
| --- | --- |
| Accounting recorded before timing edit | Instruction-specific costs, actual fetches and Note 5 parity are recorded in `loop-timing.md`; no prefetch or external-bus change. |
| Independent native vectors | Full flags/state matrix, decrement and address wraps, mapped ROM width/waits, prefixes and E3 are asserted with literal costs. |
| All model/mode configurations and device advancement | Live transitions plus normal frame boundaries assert exact cycles, raster/frame, timer/audio and no extra application instruction. |
| Delivery and DMA preservation | New LOOP-specific shadow/IRQ/TF/IRET and separate GDMA/SDMA tests pass; existing REP/HALT and all broader regressions pass. |
| Original absolute cartridge and independent consumers | CC0 source/reproducible bytes, all opcodes/paths, counts/CX/destinations, stale-save invalidation and restart; old native/production-browser CPU fails at 160 ticks. |
| Existing IRQ oracle | Unchanged fixture, independent 61459/67603 sum, 240/264 observations, 1024/0/0 continuation, 32/35 entry and former-charge 180-tick rejection retained. |
| Broader preservation and provenance | Complete native and production Chromium suites retain firmware-free startup, three models, LCD/input/frame, audio/PCM/Hyper Voice, both DMA engines, SRAM/cartridge/internal EEPROM/RTC retention, browser save integrity and local handling. No proprietary assets. |
| Documentation and validation | Correction rationale, historical observations, actual failures, source/artifact identities and optional unperformed checks are explicit; mandatory checks listed above. |

All mandatory checks passed, including the complete final smoke→compatibility
sequence against the explicit production preview. No blocking finding remains
within Issue #71. No mandatory check was skipped.

Physical-console measurements, human gameplay, additional browser engines,
optional software timing experiments and commercial-title impact were **not
performed**. Full console pipeline/prefetch accuracy, external bus phases and
Mono external-wait policy remain unresolved. No proprietary ROM or firmware is
required. Required post-merge verification: None.
