# Issue #81 validation: RET, RETF and IRET

Implemented in the assigned GitWeave checkpoint worktree, from upstream
`17891ee6063ea538787ba34e5c7d8e0dbaa6bad9` (application baseline
`6f939d2f4b275d6989a7740cdd82912cac158ea7`). The live Issue #81 was read
through GitHub before implementation. No push, PR, merge or comment was made.

## Specification and independent oracles

The [primary NEC manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)
A13761EJ1V1UM00, Appendix A printed pp. 73/81, supplies decoded return costs
6/6/8/9/10. Actual aligned stack transfers are already charged, giving inferred
residuals 5/5/6/7/7 for C3/C2/CB/CA/CF. One clock for an odd restored IP applies
the general branch condition by inference, supported by Chapter 4's odd-target
fetch discussion. Replace the AI preservation assumption 4/4/4/4/6 with no
alignment charge. This correction preserves semantics, actual transfers and
wait controls. Full prefetch, effective-address hazards and unrelated bus policies
remain gaps; these serialized expectations are not physical-console measurements.

[Return timing](return-timing.md) records literal opcode/stack/destination parity
vectors, full state, the cartridge harness and independent instruction sums.
The document and first six CPU test groups were written before changing charges.
All five separate form tests rejected the old implementation: even opcode,
aligned stack/even destination returned 6/8/7/9/10 against 7/9/9/12/11.
The mapped-bus group rejected it too. No expected value calls a production timing
helper. Independent helpers only initialize state, bytes and address snapshots.

Production changes are limited to the RET/RETF residual selection and IRET
residual, each adding loaded IP parity after all pops. Pop order, FLAGS
normalization, immediate adjustment, IF shadows and TF policy are unchanged.

## CPU and shared-device acceptance

Twelve new test groups cover all five forms, both opcode/stack/destination
parities, nonzero CS/SS, complete registers/segments/FLAGS, frame and unrelated
RAM/SRAM snapshots, no destination pre-read or duplicate transfer, zero/odd/FFFF
and wrapping immediate adjustments, normalized IRET flags and SS despite mixed
segment/BUSLOCK prefixes through seven. REP retains its exact diagnostic.

Internal, mapped ROM and SRAM fetches/stacks use live A0 00/04/08/0C and existing
SRAM waits. Tests cover high RAM availability, instruction/stack offset wrap,
20-bit and bank-window crossings, both physical lanes and ignored unavailable
RAM writes. All nine model/mode configurations run, with live returns to 00.

Normal Machine::run_frame places every form and destination/stack parity at a
frame boundary. Exact elapsed clocks, raster/frame, one HBlank decrement, one
stereo pair, complete CPU state and unchanged RAM detect extra destination
execution. Separate gated GDMA/SDMA seven-clock reservations have zero CPU
fetches, with Mono capability errors, GDMA color-mode gating and sound DMA in
Color/Crystal mode 00 preserved. Existing REP/HALT coverage remains enabled.

All five forms exercise TF and IRQ priority after completed returns; subsequent
entry costs and frame bytes are accounted separately. Real handler IRET restores
the continuation. Dedicated IRET transitions cover old/new IF, normalized TF
activation, protected continuation and boundary IRQ priority. Existing real direct
and indirect CALL→RET/RETF and interrupt→IRET checks remain, with updated exact
return expectations, including nonzero mapped caller CS.

The first corrected unit run found one obsolete RETF expectation (7→9) and a
new frame test missing its test cartridge. These were fixed; no runtime workaround
or tolerance adjustment was introduced. The strengthened direct CALL regression
also checks actual near/far return charges to odd saved continuations.

## Original cartridge and independent consumers

Original source `fixtures/return-timing.asm`, cartridge, native and browser
consumers are CC0-1.0. Existing fixture tooling reproduces it; no firmware,
third-party ROM/assets or proprietary data is included. Ten measured samples
cover five forms and even/odd restored destinations. Literal absolute intervals
and 256-clock floor/ceil timer quantization are documented in return-timing.md.

The CPU writes execution/continuation counts 4096/4096, final CX=0 and full
FLAGS F046. Marker 51A7 requires restored SP, popped IP/CS/FLAGS, discarded
padding and preserved sentinels. Completion, capability, pass count, all unused
zero records and the full 1024-byte RAM/SRAM mirror are checked. Both independent
consumers poison retained success, inspect invalidation after one frame, and
require two fresh boots across Mono 00 and Color/Crystal 00/80/00/C0/00/E0/00.
Browser loads/pause/saves use the real production player and assert no ROM/save
network requests. No host injects results or completion markers.

Timing failures accumulate across every form, model/pass and both boots, so
negative controls do not conceal later samples. State checks still fail immediately.

All return controls below use cartridge SHA-256
`e8da3588a99979564e26811505a69fde1fd97b9135239cab48c26f5a82c092a8`.
Old residuals/no parity and corrected residuals/no parity were separate native
and production Chromium runs. Every model/pass and both boots completed before
reporting timing rejection; CPU state/frame/FLAGS/counts and save checks passed.

| Form | Required even/odd ticks | Old native and Chromium (Mono, both boots) | Parity-only native and Chromium (Mono, both boots) |
| --- | --- | --- | --- |
| C3 | 400–401 / 416–417 | 384 / 384 | 400 / 400 |
| C2 | 480–481 / 496–497 | 464 / 465 | 480 / 481 |
| CB | 480–481 / 496–497 | 448 / 448 | 480 / 480 |
| CA | 592–593 / 608–609 | 544 / 545 | 592 / 592 |
| CF | 576–577 / 592–593 | 560 / 560 | 576 / 576 |

Old-charge loops are 24/29/28/34/35 clocks for C3/C2/CB/CA/CF at either
loaded parity. Corrected-base/no-parity loops are 25/30/30/37/36. Thus the old
control rejects every form/parity and the parity-only control accepts all even
samples and rejects all odd samples. The one-tick C2/CA variations are timer
phase, not widened tolerances. Native and Chromium observations match across
all passes and boots in each control. No negative-control failure is reported
as a passing implementation check.

| Return control | Production Wasm SHA-256 | Production JavaScript SHA-256 |
| --- | --- | --- |
| Old charges | `7db7a47e12d5f37c8411da2f63ee662131f0a3d438fa9c4baabd14ae66ffde03` | `f58502ce64c7f914fad7ea32f23dcb69ee74f0288448160268beb4dc66676424` (index-D3v3-ISh.js) |
| Correct base, omit parity | `750190051396b68560a7fad16061e833530bae151d135594edb417f0d15453dd` | `a5f1facb6d200acf165d740332fe28a3ebcf18f985f4f7afef11773bfec2f719` (index-WvyAJ9j8.js) |

Corrected return observations on both native and production Chromium, both boots:

| Model / pass | C3 even/odd | C2 even/odd | CB even/odd | CA even/odd | CF even/odd |
| --- | --- | --- | --- | --- | --- |
| Mono 00 | 400/416 | 480/496 | 480/496 | 592/608 | 576/592 |
| Color/Crystal pass 0/1/2/3/5/6 | 400/416 | 480/496 | 480/496 | 592/608 | 576/592 |
| Color/Crystal pass 4 (00) | 400/416 | 480/496 | 480/496 | 593/608 | 576/592 |

Every count pair is 4096/4096, CX=0, full FLAGS F046 and frame/SP/continuation
marker 51A7. Native and browser return tables match on both boots.

## Existing interval audit and retained negative controls

Re-read unchanged direct CALL, indirect and IRQ fixture source/encodings.
[Return timing](return-timing.md#recalculated-existing-intervals-with-unchanged-cartridge-bytes)
records independent sums: direct loop costs 30/31/38/39, indirect CALL costs
31/32/34/35/42/43, IRQ 64530/70674 clocks (IRET to odd 2005 gains 2048).
Consumers use the Issue's exact bounds. Unaffected indirect/direct JMP,
Jcc/JCXZ, LOOP, ordinary I/O and word intervals and all tolerances are unchanged.
Historical reports remain byte-identical. No existing fixture bytes changed.

The following controls retain corrected return accounting. Separate former
near/far dispatch experiments ensure a first near failure cannot conceal the
far check. Native and production Chromium both rejected the listed sample;
prior samples in that consumer had passed. All temporary source patches were
restored with Python finally before final builds and checks.

| Temporary control | Native rejection | Chromium rejection | Required ticks |
| --- | --- | --- | --- |
| Direct E8 residual 1/no parity only | Sample 0: 432 | Sample 0: 432 | 480–481 |
| Direct 9A residual 6/no parity only | Sample 2: 576 | Sample 2: 576 | 608–609 |
| Indirect near residual 4/no parity only | Sample 1: 496 | Sample 1: 496 | 512–513 |
| Indirect far residual 6/no parity only | Sample 4: 640 | Sample 4: 640 | 672–673 |
| Former IRQ entry residual 12 only | Sample 0: 192 | Sample 0: 192 | 252–253 |

Independent control sums are 11+4096*27−3=110600 (direct near),
11+4096*36−3=147464 (direct far), 11+4096*31−3=126984 (indirect near odd),
11+4096*40−3=163848 (indirect far even), and 64530−15*1024=49170
(former even-stack entry). Former odd-stack entry is 55314 (216–217 ticks).
Final external IRQ entry remains exactly 32/35 clocks, with separate subsequent
IRET and return alignment costs. Each prior consumer still discriminates its
original timing defect under corrected return accounting.

| Prior control | Production Wasm SHA-256 | Production JavaScript SHA-256 |
| --- | --- | --- |
| former-direct-near | `4d5bc1e0ed3cb433cffe49adb20129dca9534592f8a050855c0b05c28f4e1545` | `e98c6d4c460027fb13b9abf6dcca0ff996c34732a9bbb8d328a6ba8b068df99e` (index-DKwHSaLq.js) |
| former-direct-far | `ea9a4f3919d7557c8fd54c10e9016f5a8cd676916158dd01a16226abaaa9b56e` | `3070576d93c090e1c5f4c36e3248fa5afb73ecd6a550b81aeb0c9c7cf05a9753` (index-BSaxBvEa.js) |
| former-indirect-near | `9bfcbf83a1dd283bd0522d64f21792e61850506b3ea39ece13656684ba35a131` | `83bfff2dbc7aa8820c8e9a0686dfa80793b8f399703022432919d6f25e293781` (index-CaBqHQ6x.js) |
| former-indirect-far | `afccc00d9815c6f6918683137a41374f23646745c763667fcb18feb246032d60` | `9e10bb1ca523f83156d03ea0b8c855fef46d96b04e4085d3c4c78d25f038e3ef` (index-DQ0hAfXN.js) |
| former-irq-entry | `069c2adba14d9734c390b1847339d6abd80bc28268e59f84988a9c7107522a49` | `398aad3ce216588f400b15c5f7c4f22f9c1dfe2e7d88a1d4178e5bb59f462955` (index-pjvPV3D_.js) |

Corrected native and production Chromium CALL, indirect, IRQ and new return
observations match across every model/pass on both boots (checked by parsing
the independent consumers' output). Direct CALL base ticks are 480/496/608/624;
indirect CALL 496/512/544/560/672/688; IRQ 252/276 with counts 1024/0/0.
Every observed phase variation remains within the exact one-tick bounds.
JMP/Jcc/LOOP/ordinary-I/O/word acceptance remains enabled and unchanged.

## Mandatory commands and actual results

Commands ran in the assigned worktree; npm commands in `web`. Production
preview explicitly targets `http://127.0.0.1:4187/wonderswan-rust/` via
`npx vite preview --host 127.0.0.1 --port 4187 --strictPort` and
`WS_PREVIEW_URL`. Complete smoke precedes complete compatibility in one
sequential command chain. The already installed stable rustup toolchain path
is prepended for build:wasm, as in Issue #79; no package/toolchain/configuration
change was needed. `npm ci` passed without manifest/lockfile changes.

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass; new return cartridge reproduces and existing cartridges remain byte-identical. |
| `python3 demo/build.py --check` | Pass; unchanged demo reproduces. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 248 core unit + 40 integration + 1 web unit = **289**; empty doc tests. |
| Targeted four timing consumers with `--nocapture` | Pass; captures the corrected native observations and exact CPU-written checks for comparison with Chromium. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass on the final corrected source and all added tests. |
| `git diff --check` | Pass. |
| `npm run build:wasm` | Pass for old, parity-only, five prior controls and final corrected builds. |
| `npm run build` | Pass for all control builds and final corrected production. |
| `WS_PREVIEW_URL=… npm run test:browser` | Complete pass: all physical-model emulations, demo playthrough, lifecycle/load/races, dialogs/fullscreen, input and touch. |
| `WS_PREVIEW_URL=… npm run test:compat` | Complete pass: new return and existing timing consumers; CPU/model/startup/firmware-free policies, LCD/input/frame continuity, audio/PCM, DMA/Hyper Voice, SRAM/cartridge/internal EEPROM/RTC, raw identities, local handling and browser storage failures. |

Full logs are local ignored evidence in `target/issue-81-validation`; durable
calculations, actual observations, corrections, control failures and identities
are recorded in this report. Native control return codes were 101 and browser
control return codes 1, with the stated timing assertion failures; corrected
runs return zero. There were no allowance resets, purchases or provider switches.

## Tested source and artifact identities

SHA-256 below identifies final tested files; validation prose is excluded.
Production pkg/dist remain ignored build products. Source, original cartridge,
consumers, timing documentation and this evidence belong to the checkpoint.

| File | SHA-256 |
| --- | --- |
| `crates/ws-core/src/cpu.rs` | `d531e55b997faef0993a87f0aa7c42cd8274dfca6ae0ac381b4779d5872f1d07` |
| `crates/ws-core/src/cpu/return_tests.rs` | `9e2e25a518fc45aefa6bce2e6989234c2205d5830f458dc89a68b71a7439c694` |
| `crates/ws-core/src/cpu/call_tests.rs` | `8c160bb9df819962aa8ef0a1550a0b8e69672a1ecb19a42f0cfb84c051471d07` |
| `crates/ws-core/src/cpu/indirect_tests.rs` | `e22abf538e0c9cd5bee5f48d0cb3b7beab9b85b28be3f3fc572834f650d4f1af` |
| `crates/ws-core/src/cpu/trap_tests.rs` | `a1050864e816f5dc074e56719499cdd379aa0f1bce94468bca55ec1ee2e7246a` |
| `crates/ws-core/src/cpu/word_transfer_tests.rs` | `2294cc7742153e40c16e4aa225563eb281302ee045d1eb3068bed5e48b64da06` |
| `crates/ws-core/tests/return_timing.rs` | `e5872e55c002aa480fc660eb7690d3a81c412cde6bafbe6cd89f16fee859366f` |
| `crates/ws-core/tests/call_timing.rs` | `bec27e52e909662bb87bc97e8e8e46b3a6d1fa8bca999a4812dd04f5bbf87190` |
| `crates/ws-core/tests/indirect_timing.rs` | `10f8089f907d7df9ce15ffa0e1cc52ea040ee5553c08ba629ca56b7bef7664ce` |
| `crates/ws-core/tests/irq_timing.rs` | `07425d667e131cd44babca83a5880cb6e9a0c4ca38bae5fe27ebd2c295315764` |
| `fixtures/return-timing.asm` | `5109953c54675e55aae44248b97d98cb47a16e3b518bbed7911a03343cef5dc9` |
| `fixtures/generated/return-timing.ws` | `e8da3588a99979564e26811505a69fde1fd97b9135239cab48c26f5a82c092a8` |
| `fixtures/build.py` | `2746076fd5c79b9f29b149f05f40f922247b64fc229cea9013a9ec0d620f2b6c` |
| `web/scripts/return-timing.mjs` | `24b32f4578441d23caddd8274541cd3d7c57759deb860e21cbf2e7bf504588c2` |
| `web/scripts/compat.mjs` | `28cd908b17c4764e730ea4428e4e2f7a912849a4709d13c2a5fa43b32cbdd23f` |
| `web/scripts/call-timing.mjs` | `5667af0d62724e4fc1479d3e7a44f935a8af42219c6206a1f6e3b959752bf644` |
| `web/scripts/indirect-timing.mjs` | `454a9d205a55cff849cdf06083a1f97e92be715ef0465f4a737a5c82f0eb75be` |
| `web/scripts/irq-timing.mjs` | `a74985e5ae2babf868d7f09cce3a811b1b2cc81e325fa2e99b282359235c0754` |
| `fixtures/generated/call-timing.ws` | `f2d8d25bab282202fcb89c8ce5c839f106195ed91a8ba9475aa9bcca2ed82651` |
| `fixtures/generated/indirect-timing.ws` | `e072516d32eb71541878f30670960bdcbdbb29325b76ffc585e62f63c4a2af83` |
| `fixtures/generated/irq-timing.ws` | `dd564c5971a8b04ae5f5fa4340dcae3155b50ed44e1a0290a1bdd6252191153a` |
| `fixtures/generated/jmp-timing.ws` | `a609ec18fda97aa419338cf98905eaf8d2ab04097d305b38d6cec7172d192123` |
| `fixtures/generated/jcc-timing.ws` | `9dbe44ea9745b91f4a8a54b450215d5ea27ed907efc7eb8d8e4eebac9713e2d7` |
| `fixtures/generated/loop-timing.ws` | `90796df1e14308df6387d27296d50d3217b2eef6a9f05e877dd99095d32d661d` |
| `fixtures/generated/word-timing.ws` | `002bedf305b3fd2c4bde2db74c365d0970b4c8deb5ec8ce5e4c82dec9c64f73c` |
| `fixtures/generated/ordinary-io.ws` | `2876e7c63b4609285f805f3b24ba898cc7930138244f66282e0274f3a473fac1` |
| `web/pkg/ws_web_bg.wasm` | `af5b8e35d3a6a38a8fc80d614d53280f561bbcd5ffefdc626a59a65ffefeb52c` |
| `web/dist/assets/index-CFIEpLwK.css` | `57a00016f53344ece4d21daefd4d4ae9d9c0c365611e5306ad90e68c5449f430` |
| `web/dist/assets/index-gkx9qQjn.js` | `ae25811e1486bfc45373921eed2cf6b33452b8b78f3304f96e00b8329f922cc0` |
| `web/dist/assets/ws_web_bg-DOqtxCxu.wasm` | `af5b8e35d3a6a38a8fc80d614d53280f561bbcd5ffefdc626a59a65ffefeb52c` |

## Final criterion review

Local review compared the live source Issue, primary specification, full diff,
literal oracles, original assembly and actual automated results:

| Mandatory acceptance | Final evidence and result |
| --- | --- |
| Five return charges and pre-edit literal vectors | Form-specific 5/5/6/7/7 residuals plus loaded IP parity; exact pre-edit CPU failures for every form, opcode/stack/destination parity table and separate old/parity native/browser controls. Actual fetches/reads/waits charged once; no destination pre-read. |
| Architectural behavior and continuation | Full raw/normalized FLAGS, all registers/segments, CS/IP/SP, pop order, SS selection, adjustment/wrapping, frame/unrelated memory snapshots, code/stack/bank/20-bit boundaries and actual CALL→RET/RETF and interrupt→IRET pass. IF shadows, TF activation/delivery and IRQ priority retain separate entry budgets and frames. |
| Buses, waits, prefixes and diagnostics | Internal/ROM/SRAM fetches and stacks, live A0 00/04/08/0C, existing SRAM waits/high RAM availability, exact access clocks, seven mixed segment/BUSLOCK prefixes and REP errors pass; transfer consolidation and other bus policies unchanged. |
| Models, modes and shared devices | Mono 00 and Color/Crystal 00/80/C0/E0 with live returns to 00; normal run_frame checks exact elapsed/frame/raster/timer/stereo and no extra destination instruction. Separate gated GDMA/SDMA, REP/HALT and broad device regression acceptance pass. |
| Original cartridge and independent consumers | Source-included CC0, deterministic artifact, all five forms/even-odd targets, absolute budgets, CPU counts/FLAGS/frame/SP markers, completion invalidation, all modes, unused zeros and two boots pass. Both consumers reject every old-charge form and parity-only odd form before corrected matching passes. |
| Existing intervals and prior controls | Direct CALL/indirect/IRQ recalculated exactly as requested; unchanged cartridge identities, fixed quantization, unaffected timing bounds, IRQ entry 32/35 and discriminating prior CALL/indirect/former-entry controls pass. Historical observations untouched. |
| Broad automated acceptance | Every required fixture/demo/fmt/workspace/clippy/whitespace command and final build:wasm/build pass; complete production Chromium smoke then compatibility pass sequentially against explicit preview. Ownership/provenance, local data, saves/storage, startup, firmware policies, LCD/input/audio/DMA/EEPROM/RTC coverage remains enabled. |
| Evidence and scope | Independent sums, tested source/artifact hashes, actual observations/control failures, test setup corrections and optional unperformed checks recorded. Only return charges change runtime behavior; other changes support acceptance and current documentation. No blocking finding remains. |

All mandatory acceptance is complete; none was waived. Final diff review found
no missing criterion or unrelated runtime change. Documentation links, hashes,
fixture identities and whitespace were checked against final files.

Physical-console measurements, human gameplay, another browser engine and
additional lawful software experiments were not performed. Commercial-title
compatibility remains unestablished. Full pipeline/prefetch, effective-address
hazards and unrelated bus policies remain separate gaps. Required post-merge
verification: **None**.
