# Core performance investigation (Issue #99)

The following is the original measurement record. Further optimization in
[Issue #102's follow-up](performance-parity.md) reaches comparable native
throughput on these workloads, including a moving-input replay.

Measured on 2026-11-17, starting from `9fcf981`. The independently implemented
core is **2.22× faster on Beacon Trail and 7.37× faster on the audio/sprite
compatibility cartridge** after successive changes. It now matches or exceeds
the measured Oswan fork on the compatibility cartridge. Beetle remains about
twice as fast on the two mutually passing workloads. The native comparison and
separate Wasm adapter measurements below do not establish commercial-game
compatibility or performance on other hardware.

## Existing implementations and transferable techniques

Source inspection and experiments used these pinned revisions; no external core
code was copied, ported, vendored or added as a runtime dependency.

| Implementation | Primary sources | Observed technique | Decision here |
| --- | --- | --- | --- |
| Beetle WonderSwan / Mednafen / Cygne, `4b01295838ea89e3f1355bbe4cb5cf98aa6108cd` | [sound.c](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/mednafen/wswan/sound.c), [tcache.c](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/mednafen/wswan/tcache.c), [CPU](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/mednafen/wswan/v30mz.c) | Timestamp-based sound updates process oscillator transitions; decoded and flipped tiles are cached; CPU arithmetic flags use deferred values and a parity table. | Batch our existing sound model between its observable boundaries; decode tile rows and reuse consecutive identical map entries. Defer a persistent tile cache and lazy CPU flags until profiling justifies their invalidation/state complexity. |
| Gameblabla's Oswan, `efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33` | [README](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/README.md), [WSRender.c](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WSRender.c), [WS.c](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WS.c), [WSApu.c](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WSApu.c) | Renderer decodes/draws eight pixels together. CPU/device work is scheduled in 32-clock slices. Sound is generated at scanline cadence and repeated for output. | Render spans rather than decoding map entries per pixel. Advance a halted CPU directly to its next possible raster/DMA event. Keep our finer active-CPU and sound timing. |
| NitroSwan, `564e171fb93c2c11f98f38607493675727676c2c` | [README](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/README.md), [cpu.s](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/source/cpu.s), [Gfx.s](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/source/Gfx.s), [submodules](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/.gitmodules) | ARM assembly CPU execution in scanline slices, tile/map/sprite conversion and Nintendo DS DMA/display hardware, optional CPU speed hacks. | Event batching is portable. DS graphics registers and ARM32 assembly do not transfer to the Rust/Wasm core; speed hacks would change the behavior being preserved. No throughput measurement: a DS runtime is unavailable. |

These are candidates selected for concrete implementation techniques, not a
claim to have identified the fastest WonderSwan emulator on every machine.

## Measurement method and boundaries

- macOS 27.2.1, arm64; exact CPU model was unavailable in the sandbox.
- Native builds: Homebrew Rust 1.94.0 (`4a4ef493e`, 2026-03-02), release profile.
  The last two stages enable thin LTO and one codegen unit. No `target-cpu=native`.
- References: Apple Clang 21.0.0. Beetle uses its upstream macOS build, including
  `-O2`, `-funroll-loops` and `_LOW_ACCURACY_`. Oswan uses `-O2`, `SOUND_ON`,
  `SOUND_EMULATION`, `NOSDL_FB`; its emulation sources are unmodified.
- Identical committed CC0 ROM bytes, no held keys, 120 warmup frames, then
  3,000 requested frames per process, five repetitions. Runners alternate order.
  Use the median; raw ranges and all 115 samples are in
  [performance-results.json](performance-results.json), including an excluded
  reference workload. ROM SHA-256 values are recorded there.
- Time includes CPU, peripherals, video production and audio production/draining.
  It excludes file loading, warmup, final hashes, display presentation and audio
  playback. No frame skipping, throttle, GPU display effect or frontend UI runs.
- Rust uses the Crystal model. Beetle and Oswan use their firmware-free automatic
  hardware behavior. The two comparable cartridges run at 256 × 159 clocks/frame
  (75.471698 Hz); no 60 Hz conversion is enabled in Beetle. All these builds
  execute on the same host, but instruction timing, startup state, pixel format,
  audio mixing/filtering and feature coverage differ between cores.
- Rust produces RGBA and 24 kHz floating-point stereo; Beetle produces RGB565
  and 24 kHz integer stereo. Oswan computes audio at 12 kHz scanline cadence and
  alternates repeating samples three/four times. That is not equivalent audio
  accuracy or work. Oswan's `WsRun` ends after a fixed clock slice, so its host
  divides by actual presented frames (3,001 here), rather than assuming one call
  equals one frame. Its partial-frame residue adds a bounded timing difference.
- The hosts leave video/audio generation enabled and consume output without
  presentation. The Oswan adapter supplies in-memory SRAM and stubs only file
  persistence, input, SDL locks and presentation/playback callbacks. The SDL
  include shim contains no emulator code. Its ROM loader intentionally supports
  only the 128/256 KiB no-save/SRAM benchmark inputs.

## Results and iterations

Median microseconds per emulated frame; smaller is better:

| Stage | Beacon Trail (CPU polling, mono tiles) | SRAM128 compatibility (color, sprites, audio, IRQ) | Hyper Voice/SDMA probe |
| --- | ---: | ---: | ---: |
| Original core | 384.377 | 652.678 | 593.112 |
| Audio batching alone | 314.282 | 602.221 | 505.145 |
| Add HALT scheduling; keep clock loop for short active-audio advances | 378.365 | 200.493 | 188.982 |
| Add tile rows and sprite spans | 274.336 | 110.885 | 88.456 |
| Skip inactive SDMA boundaries, fast silent-audio advances, thin LTO | 176.693 | 103.363 | 75.944 |
| Add per-line palette translation cache (final) | **173.411** | **88.594** | **67.787** |
| Beetle | 85.857 | 47.122 | Excluded: cartridge failed |
| Oswan | 85.225 | 96.186 | Not supported by adapter/comparison |

Final improvements are **2.22× / 7.37× / 8.75×** over the original core. On the
two mutually passing workloads, final time is **2.02× / 1.88× Beetle's**, compared
with **4.48× / 13.85×** originally. Final time is **2.03× / 0.92× Oswan's**.
This is useful progress, not parity with Beetle or a speed ranking for games.

The short-advance fallback initially gave back the audio-only win on the silent
CPU-polling demo. Subsequent silent-channel and inactive-SDMA paths recovered it.
An explicit ALU opcode pattern list and additional CPU/fetch inlining were also
tried and discarded: repeated checks did not show a useful consistent benefit.
There is no timing gate in normal CI; this host also exhibited an occasional
large scheduling outlier (the original compatibility run ranged 626–1,092 µs).

**Check execution before accepting a speed result.** Preflight reads CPU-written
RAM markers after warmup. All three cores reached Beacon Trail state `[3,8,0]`
and the compatibility cartridge's success marker `1`. Their timer counters can
differ and full RAM is not claimed identical across cores. The Hyper Voice
cartridge reached `[0x600d,1,0]` here, but Beetle reached `[0x0bad,0,0]` and halted
on the fixture's failure path. Beetle's approximately 28 µs measurement is retained
as excluded raw evidence and is **not** a valid comparison against our successful
67.8 µs path. This does not establish that Beetle generally lacks Hyper Voice.
The comparison script detects and excludes such reference failures automatically.

Every native stage, including the original executable, produced identical final
framebuffer, final-frame PCM, full RAM hashes, frame counts and sample counts for
each workload at the same warmup/measured frame count. This establishes unchanged
outputs for these runs; it is not a proof of correctness for all cartridges.

### Wasm adapter measurement

A separate isolated Chromium 151.0.7922.34 run compares original and optimized
Wasm packages on the same host. It includes the real adapter's RGBA copy and PCM
copy/drain on every frame. Both packages use the installed stable Rust toolchain
and wasm-pack 0.14.0 release/wasm-opt builds. Each of five alternating repetitions
warms up for 120 frames and measures 1,000 frames, excluding module initialization.
No other browser suite or profiling process was running during this measurement.

| Workload | Original µs/frame | Optimized µs/frame | Improvement |
| --- | ---: | ---: | ---: |
| Beacon Trail | 456.5 | 230.5 | 1.98× |
| SRAM128 compatibility | 660.7 | 139.9 | 4.72× |
| Hyper Voice/SDMA | 603.8 | 110.9 | 5.44× |

Final framebuffer/PCM hashes, audio counts and framebuffer byte counts agree
across both packages and all repetitions. The complete samples are in
[performance-wasm-results.json](performance-wasm-results.json). These are browser
Wasm adapter results, excluding canvas/GPU presentation, requestAnimationFrame
pacing and audio playback. No external core was compiled to Wasm for comparison.

## Changes and correctness protection

1. Audio jumps between its 128-clock PCM boundaries and ordinary sweep boundaries.
   Wave phases advance arithmetically; noise transitions still apply in order.
   Frequency writes that shorten an existing counter reset it on the next clock,
   rather than incorrectly carrying the overshoot through a modulus. Fast sweep
   and short active-audio advances retain the old clock loop.
2. `Cpu::step` keeps its one-clock HALT contract. `Machine::run_frame` skips idle
   calls only after a sustained HALT step, stopping at line end or enabled SDMA's
   next 117-mod-128 event. Interrupt, DMA stall and device clocks stay scheduled.
3. Backgrounds decode eight-pixel tile rows, sharing consecutive identical entries
   within the current line. Sprite composition visits each selected sprite's eight
   pixels, instead of scanning the sprite list for every display pixel. It keeps
   the 32-selected-sprite limit, wrap/window behavior and first-opaque-sprite rule.
4. Translation builds a small mono lookup and caches only used color identities
   for that translation boundary. Palette reads remain one line after identity
   composition. No persistent cache invalidation or memory-write hooks are needed.
5. Release builds use thin LTO and one codegen unit, measured as part of the last
   stages. This changes build cost/settings, not supported emulation features.

Focused differential tests compare batched sound against the retained clock loop
across register changes, sweep modes, noise taps, Hyper Voice and queue rollover;
compare 384 generated scenes/rows and palette translation against the old pixel
renderer; and compare full-machine halted scheduling against one-clock execution
with keypad input, interrupts/timers and active SDMA. Existing literal hardware
vectors and cartridge integration tests remain the accuracy regression layer.

## Architecture decision and remaining work

A complete rewrite is not justified by this evidence: local scheduling and
rendering changes removed most of the initial gap, without a JIT, threads, unsafe
memory access, disabled devices, or a change to timing/compatibility policies.
There remains a measurable approximately 2× gap against Beetle. A two-second
sampling profile of the final polling-demo executable still shows CPU decode,
ROM mapping/fetch and inlined machine scheduling prominently; rendering and
device advancement also remain. Inlined attribution is coarse, so this does not
justify assigning all remaining time to any one subsystem.

If continuing toward parity, profile representative running games when lawful
inputs and sufficient compatibility are available. Then evaluate caching mapped
ROM windows or synchronizing devices lazily at observable reads/writes and event
boundaries. Those changes need explicit invalidation and exact boundary tests.
Reconsider the execution/synchronization design if such measurements still show
a large gap. Do not substitute busy-loop recognition, frame skipping, or reduced
audio accuracy for equivalent work. The adapter measurements above establish a
Wasm improvement for these workloads; broader browser/CPU/game measurements remain
necessary before making general performance claims.

## Reproduce

From the project root, build the original core with today's identical benchmark
host and the optimized core. Temporary external sources stay outside the repo:

```sh
mkdir -p /tmp/ws-perf-original
git archive 9fcf981 | tar -x -C /tmp/ws-perf-original
mkdir -p /tmp/ws-perf-original/crates/ws-core/examples
cp crates/ws-core/examples/throughput.rs /tmp/ws-perf-original/crates/ws-core/examples/
cargo build --manifest-path /tmp/ws-perf-original/Cargo.toml -p ws-core --example throughput --release --locked
cargo build -p ws-core --example throughput --release --locked

git clone https://github.com/libretro/beetle-wswan-libretro.git /tmp/ws-perf-beetle
git -C /tmp/ws-perf-beetle checkout 4b01295838ea89e3f1355bbe4cb5cf98aa6108cd
make -C /tmp/ws-perf-beetle -j4 platform=osx ARCH=arm64 CC=clang CXX=clang++
clang -O2 -Wall -Wextra -Werror -I/tmp/ws-perf-beetle/libretro-common/include \
  tools/perf/libretro-throughput.c -o /tmp/ws-libretro-throughput

git clone https://github.com/gameblabla/oswan.git /tmp/ws-perf-oswan
git -C /tmp/ws-perf-oswan checkout efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33
clang -O2 -std=gnu99 -DSOUND_ON -DSOUND_EMULATION -DNOSDL_FB \
  -Itools/perf/oswan-include -I/tmp/ws-perf-oswan/main/emu \
  -I/tmp/ws-perf-oswan/main/headers -I/tmp/ws-perf-oswan/main/sound \
  tools/perf/oswan-throughput.c /tmp/ws-perf-oswan/main/emu/WS.c \
  /tmp/ws-perf-oswan/main/emu/WSApu.c /tmp/ws-perf-oswan/main/emu/WSRender.c \
  /tmp/ws-perf-oswan/main/emu/cpu/nec.c -o /tmp/ws-oswan-throughput

python3 tools/perf/compare.py \
  --native baseline=/tmp/ws-perf-original/target/release/examples/throughput \
  --native optimized=target/release/examples/throughput \
  --libretro-host /tmp/ws-libretro-throughput \
  --beetle /tmp/ws-perf-beetle/mednafen_wswan_libretro.dylib \
  --oswan /tmp/ws-oswan-throughput --output /tmp/ws-performance.json
```

Use fresh temporary directories. On Linux use the upstream Unix target, its `.so`
and `-ldl` for the libretro host. For core-only measurements omit both reference
options and `--oswan`. The scripts require Python 3 and a C compiler only when
building reference hosts; the Rust core adds no dependencies. Intermediate-stage
executables were measured during development; their raw samples document the
sequence, while the commands above reproduce the original/final comparison.

To repeat the Wasm comparison, build both original and optimized `ws-web` crates
with `wasm-pack build --target web --locked` into separate package directories,
then run (after `cd web && npm ci`, return to the repository root):

```sh
node tools/perf/wasm-throughput.mjs \
  --baseline /tmp/ws-original-pkg --optimized web/pkg \
  --output /tmp/ws-wasm-performance.json
```

By default Playwright uses its installed Chromium; `--browser /path/to/chromium`
selects an available executable explicitly. The script loads only the local
packages/ROMs into an isolated browser, with no player or network requests.

## Validation

- `cargo fmt --all -- --check`, workspace tests (**331 passed**, 38 suites),
  workspace/all-target Clippy with `-D warnings`, and `git diff --check`: passed.
- Original demo and fixture builders with `--check`: passed; ROMs unchanged.
- Wasm release build, TypeScript/Vite production build: passed.
- Production-preview browser smoke suite: passed with Chrome 157.0.8103.52.
  Full compatibility suite: passed with cached Chromium 151.0.7922.34, including
  actual Wasm PCM, CPU budgets, LCD pipeline, keypad/rotation, raw ROMs and console
  persistence. Temporary launchers changed only `executablePath`; no assertion
  was removed. Playwright's pinned Chromium could not be downloaded because of
  DNS `ENOTFOUND cdn.playwright.dev`.
- Chrome 157 compatibility run: failed at `keypad.mjs:129`, LCD rotated corner
  marker 5 (`238,238,238,255`). The focused check reproduced the identical failure
  with the pre-change `9fcf981` Wasm and unchanged frontend; the optimized build
  passes that check in Chromium 151. Tracked separately in
  [Issue #100](https://github.com/takahirox/wonderswan-rust/issues/100). This is not
  reported as a passing Chrome 157 compatibility run or a regression caused here.
- Full post-merge verification: none required for this Issue. No deployment or
  public-site performance result is claimed.
