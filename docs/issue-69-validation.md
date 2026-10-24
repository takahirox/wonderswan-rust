# Issue #69 validation: external IRQ entry timing

## Scope and evidence

Source Issue: [#69](https://github.com/takahirox/wonderswan-rust/issues/69), read
from GitHub during this run. Evaluation baseline:
`793eb58c688821ef6a601cad2472a07ebc19bc83`; assigned upstream checkpoint:
`847d354fa3471fc2e4c21384f6053aa73b66d887`. Work and checks below were performed
in the assigned worktree on 2026-10-24. No push, PR, merge or remote comment was
performed. No usage reset, allowance purchase or model/provider switch occurred.

[External IRQ timing](external-irq-timing.md) records the pre-implementation
accounting, independent vectors, primary reference, correction rationale and
cartridge budget. The NEC V30MZ manual's Table 5-1 specifies 32 clocks excluding
waits/holds. The old AI assumption of 17/20 complete clocks is replaced by
27 execution clocks plus actual entry transfers (32/35 internally). The same
constant is used before instruction execution and at an eligible TF boundary;
only the latter adds the completed instruction's own cost. No delivery logic or
bus policy changed. Single-step-only entry still uses 20 plus transfers.

## Native and cartridge coverage

- Four new CPU tests cover all nine configurations: Mono/00, Color and Crystal
  each in 00/80/C0/E0, including live 00→80→00→C0→00→E0→00 transitions.
- Literal entry totals are 32/35 internally and boundary-selected RAM NOP totals
  are 34/37. SRAM stacks at either alignment cost 35 with waits clear and 41
  with waits set on Color/Crystal; boundary totals are 37/43. Mono retains its
  current no-added-wait policy because its mode/wait register is unavailable.
- Frames check normalized FFD7 from incoming FFFF, saved CS=0010, current or
  completed IP, FLAGS/CS/IP stack order, SP−6, live IF/TF clearing, vector-7
  priority over vector 4, explicit acknowledgement, remaining lower-priority
  latch and IRET restoration/continuation. Expected bytes and costs use literals
  and test-side address arithmetic, never production accounting helpers.
- Selected boundary frames use FFFF:0017 (20-bit address wrap into RAM 1–6) and
  0100:0005 (SP offset wrap, IP low byte at SRAM 10FFF, high byte at RAM 1000).
  Both cost 35 clocks with waits clear and return through the real IRET.
- Normal `Machine::run_frame` tests put entry exactly at the frame boundary,
  proving only 32/35 or 34/37 clocks advance the raster/frame, one HBlank timer
  tick and one stereo audio sample, with no handler or extra application
  instruction. Separate DMA tests retain a seven-clock GDMA/SDMA reservation
  ahead of the 32-clock entry, including Color/Crystal SDMA in mono mode, the
  GDMA color-enable gate and Mono's unavailable DMA ports.
- Existing IRQ-shadow, simultaneous IRQ/TF, synchronous entry, REP
  restart/progress, HALT wake/idle, single-step costs and DMA scheduling
  regressions remain enabled. The existing word-transfer and ordinary-I/O HALT
  wake tests now expect 32/35 and 32 respectively instead of the old totals.

The original CC0 `irq-timing.asm` runs through normal cartridge startup and the
normal frame scheduler. A timer latches an IRQ under CLI, then the CPU measures
1024 repeated deliveries before the final acknowledgement. Independent native
and production-browser consumers check absolute timing, handler count 1024,
application LOOP completion 0, handler countdown 0, capability, pass count and
fresh boot count on every model and live mode pass. Native checks also compare
all 256 RAM result bytes to SRAM. Retained saves are poisoned with old success;
the CPU invalidates completion before sampling, and the native test checks that
invalidation after the first frame. Both consumers repeat after fresh restart.

Independent budgets are 59412 clocks aligned and 65556 odd; counter quantization
allows 232–233 and 256–257 HBlank ticks. Corrected native and production Chromium observations on both
boots were **232/256 on every pass of all three models**, with counts/continuation
**1024/0/0**. This is an absolute test, not an aligned/odd delta test.

## Failures and corrections actually observed

Before changing production timing, three new CPU tests failed against the old
paths: aligned entry returned **17 instead of 32**, wrapped odd entry returned
**20 instead of 35**, and the frame-boundary run advanced to **40710 instead of
40704**, executing handler instructions during the missing time.

After fixing the probe harness, the native cartridge test failed with
**Mono boot 1 pass 0 aligned = 172 ticks**. The production Chromium probe built
with the original 12-clock charge independently failed at the same **172 ticks**.
The ROM bytes for those failures are the same final ROM identified below.
After finalizing the native consumer, temporarily restoring only the 12-clock
constant reproduced its 172-tick failure against the final 232–233 oracle; restoring
27 then passed again. The final source was restored byte-for-byte.
The baseline Vite Wasm asset was `ws_web_bg-CIBKpoqM.wasm` and its JS asset was
`index-DrZUVvIO.js`; these ignored build files were replaced by the corrected
production build. The baseline retained the old runtime paths; test modules and
the new fixture/consumer were already present to detect the failure.

Implementation/test-development corrections:

- The first cartridge harness overlapped the caller's return address and did
  not complete. Moving sample stacks to 2800/2801 fixed it before the meaningful
  baseline failure above. An initial hand calculation treated taken JNZ as
  three clocks; inspecting the existing decoded path established six, producing
  the documented independent budget before changing production timing.
- The new DMA regression initially expected GDMA to run in Color mono mode.
  Existing bus code requires color enable; the test now explicitly retains
  that gate and still verifies SDMA there. No capability behavior was changed.
- The first full workspace run had 196 core tests pass and one old ordinary-I/O
  HALT-wake assertion fail (32 actual versus 17 expected). Correcting that stale
  IRQ literal retained all of its surrounding device/continuation checks.
- An initial Wasm attempt using default Homebrew Rust failed because that
  compiler has no Wasm target. Builds then used the already installed Rustup
  stable toolchain and its existing target. An npm invocation from the repository
  root also failed before being rerun from `web`.
- The first smoke command omitted `WS_PREVIEW_URL` and reached the unrelated
  default-port page, failing because its model element was absent. Rerunning
  against this worktree's explicit production preview resolved that setup error.

## Commands and final results

Production preview: `http://127.0.0.1:4187/wonderswan-rust/`, supplied through
`WS_PREVIEW_URL`. Browser suites run sequentially, smoke before compatibility.
The Wasm build prepends `$HOME/.rustup/toolchains/stable-aarch64-apple-darwin/bin`
to PATH; this is build-tool selection, unrelated to any usage allowance.

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Passed; all generated artifacts reproduce, existing bytes unchanged. |
| `python3 demo/build.py --check` | Passed; original demo unchanged. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Passed: 197 core unit + 34 integration + 1 web unit = 232 tests; doc tests empty. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed, including the final stale test-literal correction. |
| `git diff --check` | Passed, including the staged final diff. |
| `npm ci` in `web` | Passed; lockfile unchanged. |
| `npm run build:wasm` in `web` | Passed, corrected production Wasm. |
| `npm run build` in `web` | Passed, production TypeScript/Vite assets. |
| `npm run test:browser` in `web` | Passed against the explicit preview, complete Chromium smoke suite. |
| `npm run test:compat` in `web` | Passed, complete Chromium suite including the new absolute IRQ probe, all model/mode passes and fresh restart. |

## Tested identities

SHA-256 of the corrected sources and artifacts (validation prose excluded):

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `0c3daf86e75b739ef205b2cfa491ead3e5338f2d1eb39d7856ac1fbc5336c306` |
| `crates/ws-core/src/cpu/irq_timing_tests.rs` | `cc15d7e6408ac942acd7fbbe9087a60538746a0c86dbbb8910461a2c3d7bf002` |
| `crates/ws-core/tests/irq_timing.rs` | `505597598e45827ae1130aaf15ecb2101841399d6b1f19e17715cca4fb71475c` |
| `fixtures/irq-timing.asm` | `4349b70d04cbc832ce0610f970ef9de04522c8b640a2de0361ac9e7e3f32c530` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `web/scripts/irq-timing.mjs` | `7b7500cbb047aaea370ff83efadc323fcc6c9233a5856b10134fb455d78d40ac` |
| `web/pkg/ws_web_bg.wasm` and `web/dist/assets/ws_web_bg-q1NxpVjv.wasm` | `38be8b744af8de026d7643c292160c297959fb697cdd8f411b658d6abf48fb3b` |
| `web/dist/assets/index-DQwwLOcp.js` | `679f8451d3567ada3bc7fcfa97f20eb02b6b9e097cc0a7ed1776834d6396a472` |

Corrected production assets: `ws_web_bg-q1NxpVjv.wasm`, `index-DQwwLOcp.js`,
`index-CFIEpLwK.css`. Wasm/dist remain ignored build artifacts; the original
fixture source and generated cartridge are committed.

## Limits and acceptance review

The correction preserves Issue #39 raw transfers and Issue #67 delivery behavior.
The complete suites retain firmware-free startup, model capabilities, LCD/input/
frame continuity, PCM/audio, GDMA/SDMA/Hyper Voice, SRAM and cartridge/internal
EEPROM, RTC retention, save integrity, browser-local handling and provenance.
All new fixture source/artifacts are original CC0; no proprietary ROM or firmware
is necessary. No primary-specification correction weakened acceptance.

Physical-console comparisons, human gameplay, commercial-title impact, additional
browser engines and optional software experiments were **not performed**. Exact
console phases, Mono external wait behavior, prefetch and general instruction
timing remain unresolved as documented. Required post-merge verification: None.
The final staged diff was reviewed against every pre-merge criterion:

| Acceptance area | Final assessment |
| --- | --- |
| Accounting and independent vectors recorded first | Complete; primary manual correction, instruction/entry/handler/DMA distinctions and unresolved Mono waits documented. |
| Both external delivery paths and literal negative tests | Complete; 32/35 and 34/37 literals, entry-only transfer deltas, actual old-charge failures recorded. |
| SRAM waits, alignments and boundaries | Complete; 35/41 SRAM entry, both alignments, independent frame bytes and selected offset/20-bit wraps. |
| Nine configurations and architectural regressions | Complete; native matrices and original cartridge mode round trips; existing shadows, IRQ/TF, synchronous, REP, HALT, IRET and single-step tests pass. |
| Normal scheduler and DMA accounting | Complete; frame/raster/timer/audio advance once, no extra instruction, separate DMA reservation and capability gates pass. |
| Absolute original cartridge and independent consumers | Complete; native and production Chromium reject 172 baseline ticks and pass 232/256 corrected observations, counts/continuation and fresh restart with stale-save invalidation. |
| Broader preservation, provenance, documentation and checks | Complete; all mandatory commands pass, both full browser suites pass sequentially, tested identities and limitations recorded. |

No blocking findings remain within Issue #69's scope. Optional physical-console
and human-play checks remain unperformed; no mandatory check was skipped.
