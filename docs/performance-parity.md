# Comparable core throughput (Issue #102)

This continues the [Issue #99 investigation](performance.md) from merged revision
`7dec186`. The working target was execution time within 10% of the fastest passing
reference on both mutually passing cartridges. The final implementation meets it
with both unchanged input and a moving-input replay, while preserving the measured
native and Wasm outputs. The claim is limited to these workloads and configurations.

## Final measurements

Median microseconds per emulated frame, five alternating repetitions of 6,000
frames after 120 warmup frames on the same macOS arm64 host:

| Input/workload | Before this change | Optimized | Beetle | Oswan | Optimized / fastest reference |
| --- | ---: | ---: | ---: | ---: | ---: |
| No keys: Beacon Trail | 174.383 | **89.351** | 86.403 | 86.438 | **1.034** |
| No keys: color/sprite/audio compatibility | 89.542 | **8.837** | 47.396 | 96.520 | **0.186** |
| Moving keys: Beacon Trail | 173.520 | **91.151** | 86.120 | 86.989 | **1.058** |
| Moving keys: color/sprite/audio compatibility | 89.186 | **8.848** | 47.385 | 96.565 | **0.187** |

Thus the demo is within approximately 3–6% of the faster reference, and the
compatibility cartridge is approximately 5.4× faster than Beetle in these runs.
The successful Hyper Voice/SDMA path takes 9.038/9.118 µs without/with replay,
versus 64.591/67.385 µs before this change. Beetle still reaches that probe's
failure marker; its failed-path timing remains excluded.

An earlier independent 3,000-frame moving-input series measured 91.092 µs here
and 85.064 µs in Beetle (ratio 1.071), also inside the target. No performance
threshold was added to normal CI.

The raw final samples, intermediate summaries, ROM SHA-256 identities and rejected
reference paths are in [performance-parity-results.json](performance-parity-results.json).
Reference revisions, compiler settings, models, audio formats and the native
measurement boundary remain as documented in the original investigation. Native
Rust uses release thin LTO and one codegen unit; references retain their existing
optimized builds with video and sound enabled. Input replay is a host workload,
not a core behavior or a cartridge-specific optimization.

The replay leaves warmup input clear, then holds Right, Down, Left and Up for
64 measured frames each, repeatedly. Beacon Trail changes tilemap cells through
ordinary CPU execution. Native baseline/optimized and Beetle finish the 3,000-frame
replay with CPU-written state `[4,14,1]`; Oswan finishes one movement step earlier
at `[3,14,1]`. Oswan samples input within its fixed-size execution slices and
presents 3,001 frames for 3,000 requested frames. This phase/quantization difference
is retained as a comparison limitation; cross-core framebuffer equality is not
claimed. Native baseline/optimized final RAM, framebuffer, final-frame PCM and
frame/sample counts agree in every measured repetition, including moving input.

## Iteration evidence

Two-second sampling profiles of the baseline, separately from throughput
measurement, show CPU execution/mapping and machine scheduling prominently in
the polling demo, and scanline composition prominently in the halted/audio
workload. Sanitized sample histograms are included in the result file. Inlined
attribution is coarse; the paired timing experiments below determine retention.

| Experiment | Main result | Decision |
| --- | --- | --- |
| Defer peripheral updates between observations | Polling demo 173.223 → 154.502 µs; compatibility essentially unchanged | Retain with explicit synchronization boundaries. |
| Reuse composed rows | Demo 170.373 → 102.209 µs; compatibility 87.781 → 24.580 µs | Retain dependency-based invalidation. |
| Cache ROM page translations | Demo 103.771 → 109.301 µs in the paired comparison | Discard the additional lookup/cache state. |
| Table-based SZP calculation | Demo 104.202 → 104.699 µs; no useful consistent gain | Discard; CPU arithmetic/flag implementation remains unchanged. |
| Reuse translated rows independently of composition | Demo 86.872 µs; compatibility 8.837 µs in that series | Retain delayed palette semantics. |
| Moving-input check | Demo 96.044 µs versus Beetle 85.816 µs | Continue; static-frame parity alone was insufficient. |
| Dirty map rows alone | Moving demo 97.912 µs; broad tile invalidation still dominated | Retain finer map dependencies only together with the next improvement. |
| Track referenced tile storage | Moving demo 91.092 µs versus Beetle 85.064 µs | Retain; unused stack storage no longer invalidates visible graphics. |
| Longer final static and replay series | Ratios 1.034 / 1.058 on the polling demo | Target achieved in both conditions. |

The demo places its stack inside an address range that can hold tile data.
Movement calls and pushes therefore exposed overly conservative invalidation of
all graphics for writes to tiles never used by the display. This was a general
dependency problem, not a reason to recognize the demo's code or address layout.
The retained implementation tracks every tile block decoded by the renderer.

## Architecture and correctness

Peripheral advancement now accumulates at most the remainder of a scanline when
SDMA is inactive. CPU time and raster readback advance immediately. Before any
other I/O observation/write, waveform RAM mutation, active GDMA operation, raster
boundary, frame return or fault return, pending device clocks are applied in
order. Enabled SDMA retains the eager event scheduler and its existing bus-stall
costs. The public single-step CPU contract and instruction timing are unchanged.

The LCD pipeline retains per-row pixel identities. It has separate composition
and translation generations, so the palette remains observable one line after
composition. Writes to active tilemaps invalidate only their eight scrolled/wrapped
display rows; tile, display/window/scroll/mode and mono-palette changes invalidate
composition as required. VBlank invalidates sprite-dependent composition when the
latched OAM bytes actually change. CPU and GDMA RAM writes share invalidation.

The renderer records used 32-byte tile blocks, covering one 4bpp or two 2bpp tiles.
Writes to never-referenced tile storage leave cached identities valid. A later
map, mode or scroll change reads the current RAM and records the new tile use.
Historical use is retained conservatively, so long sessions can cause extra
invalidation, never reuse pixels after a relevant write. The tracking array is
bounded at 2,048 entries. The identity cache contains 144 × 224 pixels.

Each pipeline belongs to one core-owned framebuffer. Correct translated rows
already reside there and can remain when both identity and translation inputs are
unchanged. The CPU, audio, interrupts, DMA, EEPROM/RTC and frame counters still
execute; the frontend continues receiving every requested frame. Reset/model/ROM
lifecycle operations replace the bus, pipeline and framebuffer together.

Focused tests cover eager/deferred device agreement across noise, ordinary/fast
sweep, wave writes and SDMA transitions; exact EEPROM completion and GDMA wave
replacement; cached versus forced-uncached rendering across CPU/DMA tile/map
writes, palette changes, mode/window/scroll changes and sprite latching; wrapped
and overlapping map rows; and an initially unused tile becoming visible and then
changing. Existing independent pixel and clock vectors remain unchanged.

## Wasm adapter results

Separate isolated Chromium 151.0.7922.34 runs use five alternating repetitions of
1,000 frames after 120 warmup frames, including the real adapter's RGBA and PCM
copy/drain. Module initialization, canvas/GPU presentation and audio playback are
outside this boundary. Both local packages use stable Rust, wasm-pack 0.14.0 and
release wasm-opt. No browser regression suite ran concurrently with measurement.

| Input/workload | Before µs/frame | Optimized µs/frame |
| --- | ---: | ---: |
| No keys: Beacon Trail | 236.2 | **116.6** |
| No keys: compatibility | 139.5 | **20.1** |
| No keys: Hyper Voice | 111.5 | **20.7** |
| Moving keys: Beacon Trail | 233.1 | **117.2** |
| Moving keys: compatibility | 138.9 | **21.1** |
| Moving keys: Hyper Voice | 111.7 | **20.1** |

Frame/sample byte counts and final video/PCM hashes agree across both packages
in every repetition. This establishes an additional approximately 2× demo and
6.6–6.9× compatibility speedup in the actual adapter. External cores were not
compiled to Wasm, so native cross-core parity is not asserted across runtimes.

## Reproduce

Use the native reference build commands from [the original report](performance.md#reproduce).
For the baseline, extract revision `7dec186` into a fresh temporary directory and
copy today's `crates/ws-core/examples/throughput.rs` into that checkout before its
release build. Rebuild both reference hosts from today's sources to enable input
replay. No external core source is included in the project.

```sh
cargo build -p ws-core --example throughput --release --locked
python3 tools/perf/compare.py \
  --native baseline=/tmp/ws-before/target/release/examples/throughput \
  --native optimized=target/release/examples/throughput \
  --libretro-host /tmp/ws-libretro-throughput \
  --beetle /tmp/ws-perf-beetle/mednafen_wswan_libretro.dylib \
  --oswan /tmp/ws-oswan-throughput --frames 6000 --output /tmp/ws-static.json
```

Repeat with `WS_PERF_INPUT_REPLAY=1` before `python3` for the moving workload.
The same environment variable works with `tools/perf/wasm-throughput.mjs` and
the original report's separate-package Wasm comparison command. Preflight requires
cartridge success; replay checks valid demo coordinates/score rather than its
stationary initial position. Output equality across native versions remains mandatory.

## Validation

- Workspace tests: **336 passed**, 38 suites. Formatting, all-target workspace
  Clippy with `-D warnings`, ROM builders' `--check` and whitespace checks passed.
- Wasm release and TypeScript/Vite production builds passed.
- Full compatibility and production smoke suites passed in cached Chromium
  151.0.7922.34. The complete smoke suite also passed against the baseline build
  with the repaired test setup.
- Validation exposed two pre-existing test setup races, separately scoped in
  [Issue #103](https://github.com/takahirox/wonderswan-rust/issues/103): short input
  immediately after resume before released input/frame-clock initialization, and
  a paused snapshot assigning the same width as existing CSS instead of causing
  a resize. Both original failures reproduce on `7dec186`. Test-only fixes wait
  with the existing RAF helper and request a real one-pixel width change. The
  baseline focused input and shader checks then pass; assertions and thresholds
  are preserved. The RAF helper allows two elapsed intervals because the first
  callback after load/resume initializes the player clock. An intermediate smoke
  repeat exposed the same short-input assumption at visibility resume and local
  file load; the final baseline and optimized full repeats pass with the complete
  synchronization repair. The optimized unmodified smoke suite also passed in
  Chrome 157.
- The test-only repair is isolated in
  [PR #104](https://github.com/takahirox/wonderswan-rust/pull/104); the performance
  branch follows that commit so its complete browser validation is reproducible.
- Browser launchers explicitly select isolated cached Chromium. The known Chrome
  157 corner-screenshot difference remains tracked in
  [Issue #100](https://github.com/takahirox/wonderswan-rust/issues/100); that
  already-reproduced baseline failure was not repeated as a new merge gate.
- No required post-merge verification. No commercial-game, other-host or
  public-deployment performance claim is made.
