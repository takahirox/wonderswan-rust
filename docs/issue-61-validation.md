# Issue #61 validation

Implemented on 2026-10-24 in the assigned GitWeave worktree, against evaluation
baseline `7f4e1871edb1153bc4df4b4af8cc377c5ff36ddc` and upstream checkpoint
`87be5d6163f0db6fccbfac3823cb87c2692bc975`. Read live GitHub Issue #61 with the
connected GitHub issue-fetch tool; it is open and matches the supplied snapshot.
No separate upstream technical diagnosis was provided. Repository development
flow and review guidance were read. No push, PR, merge or remote comment occurred.

[The timing table](ordinary-io-timing.md) was written before implementation.
It records the old AI assumption, corrected requirement, primary manual reference
and integration rationale. The live Issue already contains that correction, so
no remote specification change is necessary (and this node is not authorized to
modify GitHub). The manual's Appendix A and Chapter 4 cached text was inspected;
PDF screenshot retrieval returned cache misses. No new hardware timing requirement
was inferred from test output. The only production behavior change is the ordinary
IN/OUT decoder's execution component from one to five clocks. The bus already
charges the included normal transfer: total = actual fetch + 6 + (I/O charge - 1).

## Baseline discrimination

The new focused unit test and original source-included CC0 cartridge were run
before changing the production decoder:

| Command | Actual result with old decoder |
| --- | --- |
| `cargo test -p ws-core --lib ordinary_io_literal --locked -- --nocapture` | Failed: Mono, mode 00, E4 port 20, RAM fetch, A0=00 returned 4 clocks; literal expected 8. Only the first failing vector was reached. |
| `cargo test -p ws-core --test ordinary_io --locked -- --nocapture` | Failed: Mono boot 1 reported 0BAD, first CPU-written HBlank observation 32 ticks versus independent bound 64–65. Remaining samples, other models/modes and restart were not reached on baseline. |

The baseline cartridge failure is an elapsed-time observation made by emulated
CPU instructions, not a host sum of instruction return values. Corrected focused
native cartridge execution passed both boots on all three models. First-phase
observations were `64,64,72,64,64,72,72,72,80,72,72,80` on Mono; Color/Crystal had
73 for the sixth sample and the same other values. All seven required runtime
phases passed independently on both Color models, including the final return to 00.
These observations satisfy the prior calculated bounds; they do not set them.

## Coverage and resolved failures

Five new CPU unit tests cover:

- 4,032 literal timing/data cases: all eight encodings, nine static configurations,
  seven internal port vectors, RAM/mapped-ROM fetches and all four live A0.2/A0.3
  combinations. Byte/even/split RAM totals are literal 7/7/8 for DX and 8/8/9 for
  immediate forms. Every instruction checks length, MOV continuation, AL/AH/AX,
  full unchanged FLAGS, all unrelated registers/segments/RAM/SRAM/ports, no device
  clock advance during decode, raw bus charges, palette/frequency masks and timer
  reload side effects. Valid odd ports include 21, 27 and A5.
- 5,760 cartridge-I/O cases on one live bus per model, including Color/Crystal
  00→80→00→C0→00→E0→00, external C0/C1 and BF/C0 crossings, RAM/ROM execution,
  live ROM controls and per-lane 60.3 waits. Mono's internal wait policy is also
  checked without pretending its unavailable 60 register can be programmed.
- All eight encodings under segment/BUSLOCK prefixes; unchanged REP rejection,
  full INT3 saved FLAGS/IRET return, masked-IRQ HALT release, invalid FFFF ports
  without wrapped port-zero access and high-lane failure after low-lane effects.
- Fifteen normal Machine-scheduled forty-IN bursts, one per runtime phase across
  all models. Each 280-clock burst begins 277 clocks before frame wrap, crosses
  two line ends, consumes two HBlank timer ticks, latches the timer IRQ, wraps
  raster/frame once and produces six interleaved PCM samples at offsets 21, 149
  and 277. Draining twice returns no duplicate samples. Enabled IRQ entry after
  HALT, IRET and INC continuation also pass without replaying the burst.
- Separate normal-scheduler GDMA/SDMA cases on all eight Color/Crystal static
  configurations: one seven-clock GDMA RAM word plus one seven-clock IN (GDMA
  gated off in mode 00); or IN 7 + SDMA stall 7 + IN 7, preserving source/length,
  delivery, CPU progress, raster/frame and PCM. Existing raw bus, DMA, interrupt,
  INS/OUTS, prefix and relative word-timing tests remain present and pass.

The first focused scheduler test incorrectly expected four PCM samples. Its
window begins 277 clocks before wrap, so the existing 128-clock PCM cadence
crosses three sample boundaries, not two. The assertion now documents and checks
the independently calculated six interleaved samples. Production audio code was
unchanged. The first full native run found two obsolete ordinary-I/O assertions:
RTC immediate totals 4/5 and GDMA-enabling OUT total 4. They were updated to 8/9
and 8, preserving the exact prior raw transfer counts, stage timing and separate
DMA checks. Earlier ordinary word and BUSLOCK total assertions were similarly
updated without reducing width, lane, alignment or architectural assertions.

The new `ordinary-io.asm` fixture records twelve 2048-transfer unrolled bursts
per phase, covering immediate/DX byte/even/odd IN and OUT. Starting IN/MOV costs
11 clocks; immediate transfers cost 8 or 9 and DX transfers with explicit NOP
cost 9 or 10. The independently computed intervals 16395/18443/20491 clocks
quantize at 256 clocks per timer tick to 64–65/72–73/80–81 ticks. Cartridge,
native and browser checks each contain their own literals, not parsed expectations
from the cartridge. Each host verifies 360 observations over all model/phase/boot
combinations, plus boot/status/phase words and zero result tails. Native checks
also compare all 256 CPU RAM/SRAM result bytes. Both hosts poison retained success,
check invalidation after one frame and require boot count 2 plus fresh results
following Restart. Native tests verify cleared volatile status before each boot.

Chromium uses the actual production player and compiled Wasm, controlled host
RAF callbacks, real pause/resume/save flushing and the normal ROM file input.
There are no execution/result mocks. ROM SHA-256 keys and browser-local handling
are preserved; zero network requests occur after the local ROM selection.
The cartridge SHA-256 is
`2876e7c63b4609285f805f3b24ba898cc7930138244f66282e0274f3a473fac1`.
All 34 existing generated cartridges were compared with baseline and are byte
identical, including the preserved relative word probe.

The first Wasm build failed because PATH chose Homebrew Rust without the Wasm
target. The already installed Rustup stable toolchain/target resolved it with
`PATH="/Users/takahiro/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH"`;
no installation was needed. `npm ci` restored locked dependencies without changing
lockfiles. No usage-limit resets, allowance purchases or model/provider switches
were attempted.

## Required checks and revision

| Command | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 35 images reproducible; all 34 prior images unchanged. |
| `python3 demo/build.py --check` | Pass: unchanged SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 169 core unit tests, 31 core integration tests, one adapter test, doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass; final documentation check recorded below. |
| `npm run build:wasm` in web | Pass using installed Rustup stable target as above. |
| `npm run build` in web | Pass: TypeScript and production Vite build. |
| Focused production Chromium ordinary-I/O probe | Pass: all models, runtime phases, both boots and first-frame success invalidation. |
| `npm run test:browser` in web | Pass: complete production Chromium smoke suite, exited zero before compatibility suite began. |
| `npm run test:compat` in web | Pass: complete production Chromium compatibility suite, exited zero, including the new absolute and unchanged relative timing probes. |

Preview: `npx vite preview --host 127.0.0.1 --port 4211 --strictPort`.
Both complete browser commands use
`WS_PREVIEW_URL=http://127.0.0.1:4211/wonderswan-rust/`. `node --check` passed for
the new browser script. No functional changes followed the final native/Wasm/web
builds and checks. The exact final functional revision is identified by these
Git blob IDs, independent of later documentation-only edits:

| File | Git blob ID |
| --- | --- |
| crates/ws-core/src/cpu.rs | 02a4708ba3f73f6ee5e1c26d53601379927b5636 |
| crates/ws-core/src/cpu/ordinary_io_tests.rs | 4d6cdd564b6352e4fe8d9a9512cdc99e849510b3 |
| crates/ws-core/src/cpu/word_transfer_tests.rs | 2d30835f0c01c231192f84469230c076bc6121d7 |
| crates/ws-core/src/cpu/buslock_tests.rs | 86ab36ffd59b1bf3dff5a8dc036a339b7e1a7b64 |
| crates/ws-core/src/bus.rs | 24a657d87a10c928b623432ffec58262ee2fb468 |
| crates/ws-core/src/gdma_tests.rs | 2565a045f12dc4eb79945c19334f21fd0471c137 |
| crates/ws-core/tests/ordinary_io.rs | 4e6ac71bdfc2b825f487a8b36a29be7b37c3d6fb |
| fixtures/ordinary-io.asm | b58d8c1c92ece5417d47f9ac92eb3ef2b9585c0b |
| fixtures/generated/ordinary-io.ws | ecf56f1f15e47177aa7feb9a7f67f41ee0a0521a |
| fixtures/build.py | 31113f0b81c29840931e02936da71f85fa83216f |
| web/scripts/ordinary-io.mjs | aa123e3f1d502ae43bb638de161d6d810e4f84bd |
| web/scripts/compat.mjs | c0bafb46ae4f1fdbd96895611f10a0941a29d72c |

## Acceptance review and limits

| Issue criterion | Implementation and evidence |
| --- | --- |
| Reconciled table before implementation | Eight encodings, internal byte/even/split classes, external byte-wide I/O, waits and BF/C0 boundary documented before the decoder change. Literal 7/8 and 8/9 totals checked; actual ROM fetches and extra I/O waits remain in the bus. |
| Decode, state and prior transfer acceptance | Only the ordinary handler's execution allowance changes. Literal state/length/mask/reload vectors, prefix/BUSLOCK, REP rejection, port errors, INT3/IRET/HALT and existing INS/OUTS/raw bus tests pass. Prior alignment and relative probe assertions are unchanged. |
| Models and modes | Nine static configurations and both seven-phase runtime round trips covered. RAM/mapped ROM, live A0.2/A0.3, valid odd internal lanes, cartridge I/O waits and cross-region words checked. |
| Normal scheduling | Runtime bursts cross line/frame boundaries with exact elapsed clocks, timers, IRQ, raster/frame, PCM and no duplicate drain. Separate GDMA and SDMA accounting checked with normal Machine scheduling. |
| Original elapsed-time cartridge | Source-included CC0 NASM fixture, independently calculated timer bounds, actual CPU-written results, three models, runtime modes, both boots, cleared volatile results and invalidated saved success. Baseline failures distinguished from corrected passes; relative word probe preserved. |
| Required checks | Full native/build/lint results above; complete production Chromium smoke and compatibility suites both passed sequentially. Blob IDs identify final functional files. |
| Prior behavior and provenance | Complete workspace and browser regression suites retain CPU/interrupt, startup/firmware, video/input/frame, PCM/DMA/Hyper Voice, RTC, SRAM/EEPROM and lifecycle/save-integrity coverage. Raw ROM identities, ownership/provenance and browser-local transport paths unchanged. |

Implementation self-review compared every pre-merge criterion with the final diff
and executed evidence. The GitWeave Coordinator must review this checkpoint and
resolve any findings before merge; this implementation node does not approve or
merge a PR. There is no human-approval or independent technical-correction gate.
Final documentation links and whitespace passed after recording browser
completion. All 67 checked relative file links resolve.

Optional baseline production-browser discrimination, another browser engine,
physical-console measurements, commercial-title experiments and human gameplay
were not performed. No physical-device result or identity is claimed. Exact
prefetch/pipeline timing and intra-instruction device ordering remain outside
this bounded policy. Required post-merge verification: none.
