# Browser throughput and playback parity (Issue #117)

The optimized project is within 3.1% of the fastest native reference on the
CPU-active keypad fixture and faster than both references on the color/audio
fixture. In Chromium, the project also executes both passing fixtures faster
than Beetle when both adapters deliver PCM and an RGBA image. Both cores sustain
continuous playback in the common frontend through reduced presentation cadence
and recurring main-thread stalls of 80 ms.

## Measurements and changes

The native results are medians of five interleaved runs of 6,000 frames after
120 warmup frames. This uses the native core comparison from
[Issue #102](performance-parity.md), with the currently shipped keypad fixture
adding continuous CPU and memory activity. Every timed reference passed the
cartridge marker check. The optimized core retains identical frame, RAM and PCM
hashes and counts to `3f0d7d7` on the timed fixtures.

| Fixture | Before, µs/frame | After, µs/frame | Beetle, µs/frame | Oswan, µs/frame |
| --- | ---: | ---: | ---: | ---: |
| Color/sprite/audio SRAM | 8.936 | 7.646 | 46.964 | 96.749 |
| Active keypad scan | 177.801 | 165.626 | 179.216 | 160.783 |
| Hyper Voice SDMA | 9.326 | 9.292 | Compatibility failure | Outside adapter scope |

Sampling the active keypad run put instruction execution and mapped reads among
the main costs. A trial that inlined mapped reads showed no consistent gain and
was discarded. Deferring device synchronization past keypad I/O and unrelated
RAM writes reduced the keypad time to 173.168 µs/frame. CPU accounting still
happens on each access. The selected 64-byte wave RAM page and its selector
still flush pending device cycles when they change. Calculating the ROM wrap
mask and leading gap at load time produced the final result. The fixed-size
ROM backing and raw-ROM regressions preserve top alignment, the unused leading
gap on 3/6 MiB cartridges, and bank behavior for CPU and DMA accesses.

Browser playback also pays for presentation. CRT had zero frame-history decay
but still ran the history pass on every emulated frame. It now uses the latest
source texture directly. Clean and CRT capture the latest framebuffer just before
presentation, coalescing intermediate captures when several frames arrive between
paints. LCD modes continue to update history on each emulated frame. Starting or
pausing a cartridge still presents the current framebuffer. The frontend uses a
typed view of the Wasm adapter's owned byte copy for canvas upload.

With 120 ms stalls, the original 100 ms reserve eventually ran out and the
100 ms catch-up window repeatedly discarded wall time. A real underrun now raises
the reserve and backlog ceiling to 160 ms for that playback session. The
100 ms startup reserve returns on pause, mute or context suspension. The first
underrun still creates a gap and adds 60 ms of scheduled lead. The bounded
output-rate correction and per-turn CPU and source-count limits still apply.

### Production before and after

Each row comes from a 12-second trial with the audible SRAM fixture. Both builds
run the same preset, CPU multiplier and injected workload for each pair.

| Playback condition | Gaps before → after | Scheduled gap time before → after | Framebuffer captures before → after | WebGL draws before → after |
| --- | ---: | ---: | ---: | ---: |
| CRT, ordinary presentation | 0 → 0 | 0 → 0 ms | 905 → 906 | 1,811 → 906 |
| CRT, 10 Hz presentation | 0 → 0 | 0 → 0 ms | 906 → 120 | 1,025 → 120 |
| Clean, 10 Hz presentation | 0 → 0 | 0 → 0 ms | 906 → 119 | — |
| LCD Light, CPU ×8, recurring 80 ms stalls | 0 → 0 | 0 → 0 ms | 906 → 906 | 1,669 → 1,673 |
| CRT, CPU ×8, recurring 80 ms stalls | 0 → 0 | 0 → 0 ms | 906 → 773 | 1,666 → 773 |
| CRT, CPU ×8, recurring 120 ms stalls | 6 → 1 | 719.0 → 160.9 ms | 852 → 713 | 1,554 → 713 |

At 120 ms, emulation rises from 70.97 to 74.58 frames/s over the measured
window. The single revised gap occurs about 0.63 seconds into the trial.
Recovery deliberately starts with more lead, so that first gap is longer than
the baseline's first gap (103.7 ms). There are no later gaps in this run, and
the final run schedules every generated PCM sample without cancellation. The
reduction in WebGL draw counts directly quantifies avoided work. After the
change, RAF callback times include framebuffer capture that previously ran in
the emulation loop.

In a separate 45-second production run with 120 ms stalls, the player again
recovers from a single gap at 0.62 seconds. It schedules all 44.957 seconds of
generated PCM, averages 75.39 emulated frames/s and retains at most 20 sources.
The longest observed submission interval is 138.7 ms. This trial establishes
the observed load envelope and separately records the initial recovery gap.

## Comparison boundary

The browser comparison compiles the pinned Beetle core to Wasm with Emscripten.
The Rust release package and Beetle run in the same Chromium session with the
same frame pump, real `AudioContext`, gain/filter, preset, PCM sample rate and
injected load. The independent host reads each adapter's PCM and RGBA for
throughput, alternates order between repetitions and excludes initialization and
presentation from that timing. Beetle's RGB565-to-RGBA and PCM conversion are
included because these are the formats required by this common frontend.

| Chromium workload | Project, µs/frame | Beetle, µs/frame |
| --- | ---: | ---: |
| Color/sprite/audio SRAM | 16.62 | 102.85 |
| Active keypad scan | 195.03 | 319.92 |

Real-time trials use the audible color/sprite fixture. Both cores pass the boot
marker and produce nonzero PCM. Tests compare gaps on the browser's audio clock,
source PCM counts, minimum scheduling lead, and emulated frame rate. The shared
host measures the cores under equivalent playback. The production trials
compare the browser player before and after the change under the same profile
settings. Full Oswan/SDL and RetroArch playback, device-captured sound and old-PC
hardware remain unmeasured. The earlier study covers NitroSwan's Nintendo DS
playback path. [Reference playback study](audio-playback-comparison.md)

Ten-second shared-host trials use the optimized display and the same Wasm
binaries for both reserve-policy runs. Both cores schedule every generated
sample in every trial. A stall can still reduce the number of samples generated.

| Shared condition | Project gaps, fixed → adaptive | Beetle gaps, fixed → adaptive | Adaptive emulated fps, Project / Beetle |
| --- | ---: | ---: | ---: |
| Clean, normal load | 0 → 0 | 0 → 0 | 75.38 / 75.46 |
| Clean, 10 Hz presentation, CPU ×8 | 0 → 0 | 0 → 0 | 75.38 / 75.38 |
| CRT, CPU ×8, recurring 80 ms stalls | 0 → 0 | 0 → 0 | 75.38 / 75.37 |
| CRT, CPU ×8, recurring 120 ms stalls | 6 → 1 | 5 → 1 | 75.27 / 75.28 |

At 120 ms the project and Beetle schedule approximately 170 ms and 166 ms of
total gaps with the adaptive policy, compared with 652 ms and 621 ms with the
fixed policy. Their minimum submission leads in the adaptive trial are 89.6 ms
and 87.2 ms. These figures include the warm start with the initial 100 ms
reserve. The project cartridge tests independently cover exact sample values.
The two cores use different synthesis implementations.

## Reproduction

Build the Rust release example, the release Wasm package, and production
frontend. [Build instructions](../README.md#build-and-run-locally)

Build the native reference binaries using the original comparison commands.
[Reference build instructions](performance.md#reproduce)

The Beetle revision is `4b01295838ea89e3f1355bbe4cb5cf98aa6108cd` and the Oswan
revision is `efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33`.

```sh
mkdir -p /tmp/ws-parity-before
git archive 3f0d7d7 | tar -x -C /tmp/ws-parity-before
cargo build --manifest-path /tmp/ws-parity-before/Cargo.toml -p ws-core --example throughput --release --locked
python3 tools/perf/compare.py --native baseline=/tmp/ws-parity-before/target/release/examples/throughput \
  --native final=target/release/examples/throughput \
  --libretro-host /tmp/ws-libretro-throughput \
  --beetle /tmp/ws-perf-beetle/mednafen_wswan_libretro.dylib \
  --oswan /tmp/ws-oswan-throughput --frames 6000 --keypad --output /tmp/native.json

# Build Wasm in a separate Beetle checkout.
git clone https://github.com/libretro/beetle-wswan-libretro.git /tmp/ws-beetle-wasm
git -C /tmp/ws-beetle-wasm checkout 4b01295838ea89e3f1355bbe4cb5cf98aa6108cd
emmake make -C /tmp/ws-beetle-wasm -j4 platform=emscripten
cp /tmp/ws-beetle-wasm/mednafen_wswan_libretro_emscripten.bc /tmp/beetle.a
emcc -O2 -I /tmp/ws-beetle-wasm/libretro-common/include \
  tools/perf/beetle-browser.c /tmp/beetle.a --no-entry \
  -s MODULARIZE=1 -s EXPORT_ES6=1 -s ENVIRONMENT=web -s ALLOW_MEMORY_GROWTH=1 \
  -s 'EXPORTED_FUNCTIONS=["_malloc","_free","_bench_open","_bench_close","_bench_step","_bench_pcm","_bench_pcm_values","_bench_sample_rate","_bench_frame_rate","_bench_marker","_bench_rgba"]' \
  -s 'EXPORTED_RUNTIME_METHODS=["HEAPU8","HEAPF32"]' -o /tmp/beetle.mjs

web/node_modules/.bin/tsc --ignoreConfig web/src/audio.ts web/src/display.ts \
  --target ES2022 --module ES2022 --lib ES2022,DOM --outDir /tmp/ws-frontend --skipLibCheck
node tools/perf/browser-parity.mjs --beetle /tmp/beetle.mjs --frontend /tmp/ws-frontend \
  --frames 6000 --seconds 10 --check --output /tmp/browser-parity.json

# With the production preview running (omit --check on the historical baseline).
node tools/perf/audio-player.mjs --seconds 12 --check \
  --profiles crt,crt-10hz,clean-10hz,lcd-jitter80-cpu8,crt-jitter80-cpu8,crt-jitter120-cpu8 \
  --output /tmp/player.json
```

The recorded browser runs use cached headless Chromium 151.0.7922.34 on macOS
arm64. `--browser /path/to/chromium` selects a specific executable. The shared
browser host serves only explicitly listed local files in an isolated context.
Individual native, shared-host, production and drift results, together with build
and fixture hashes, are available for inspection. [Measurement data](browser-playback-parity-results.json)

## Validation

The Rust format, workspace test and Clippy checks passed. The release Wasm
package and TypeScript/Vite production build passed. The production preview
passed both browser suites, including CRT pixels and draw counts, per-frame LCD
history, exact PCM across the sound paths, and pause, reset, visibility and
input behavior. The queue tests passed eight one-hour virtual-clock scenarios
and repeated 120 ms stall recovery. The reproduction commands above cover the
timed core comparison and both browser measurements.
