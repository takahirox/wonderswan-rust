# Browser audio stability investigation (Issue #110)

This is the historical baseline investigation. The subsequent implementation,
current validation and latency tradeoff are in [the audio stability fix](audio-stability-fix.md).

At baseline `9edf5b6`, the browser player's scheduling can interrupt audio even
when the core has substantial execution headroom. A controlled approximately
15 Hz presentation cadence produced **1,730 ms of gaps in eight seconds**.
A temporary scheduling/buffering/catch-up candidate restored approximately
75.47 emulated frames/s with **zero scheduled gaps** in that condition. It still
failed a stronger intermittent-stall condition, so it is evidence for the next
implementation, rather than a complete fix. This change adds diagnostics and
the investigation; production behavior is unchanged.

These results explain reproducible failure modes compatible with the reported
slow-PC symptoms. They do not identify the owner's particular hardware, game,
browser or audio driver as the cause. A core rewrite is not justified by these
measurements. Earlier [core throughput results](performance-parity.md) exclude
browser presentation and audio playback, and therefore cannot establish playback
stability.

## Source findings

- [The player loop](../web/src/main.ts) generates PCM only inside
  `requestAnimationFrame`. Each callback runs at most four emulated frames,
  clips an elapsed interval to 100 ms, and retains at most one frame of backlog.
  It updates display history before submitting each frame's audio.
- For these cartridges a frame is 256 × 159 / 3,072,000 = **13.25 ms**,
  or **75.471698 frames/s**. Four-frame catch-up requires at least
  **18.867925 callbacks/s**. At 15 Hz it produces at most 60 emulated frames/s,
  equivalent to 0.795 seconds of PCM per second; at 10 Hz, 40 frames/s and
  0.530 seconds of PCM. Buffering cannot sustain a consumer with that shortfall.
  Cartridges that change frame timing have a correspondingly different boundary.
- [Sound.enqueue](../web/src/audio.ts) schedules each packet at
  `max(previousEnd, currentTime + 25ms)`. If previously queued sound ends
  less than 25 ms ahead, this can insert a gap while that sound is still pending.
  The deterministic normal-cadence result is small (about 13 ms across 20 s),
  so this rule alone is insufficient to explain severe persistent breakup.
- A packet normally contains 318 stereo frames at 24 kHz: **13.25 ms**.
  The eight-source limit represents 106 ms of packet data, rather than a
  guaranteed 150 ms reserve. The separate 150 ms guard tests the scheduled
  tail before accepting another packet. Source membership is released by
  `onended`, whose event delivery can lag audio completion on a busy main thread.
  Exceeding either guard silently drops already-drained PCM.
- Wasm PCM and RGBA are copied into JavaScript arrays; each packet creates an
  `AudioBuffer` and `AudioBufferSourceNode`. LCD modes also update a WebGL history
  texture per emulated frame and present once per callback. These add main-thread
  work around the producer; their existence does not establish a bottleneck.

Web Audio schedules sources on the `AudioContext.currentTime` timeline, and
computes output on a rendering thread distinct from the control thread. Scheduling
from JavaScript late cannot recover an already missed output interval. See the
W3C Recommendation's [source start behavior](https://www.w3.org/TR/2021/REC-webaudio-20210617/#dom-audioscheduledsourcenode-start)
and [thread model](https://www.w3.org/TR/2021/REC-webaudio-20210617/#control-thread-and-rendering-thread).

## Deterministic evaluation

[audio-scheduling.mjs](../tools/perf/audio-scheduling.mjs) imports the actual
compiled `Sound` class and uses a controllable Web Audio clock. Its producer
replays the elapsed-time/catch-up rules after an initial callback at time zero.
It validates the real buffer length and deinterleaving. Temporary module copies
test five variants over nine arrival/event-delivery patterns, each for 20 seconds.
The mocked clock has no audio device, render quantum or CPU execution cost.

All candidates preserve an existing future packet end instead of reapplying
the lead to every packet; only an exhausted queue establishes a new lead.

| Variant / arrival pattern | Emulated frames/s | Dropped packets | Scheduled gaps, ms |
| --- | ---: | ---: | ---: |
| Current / 30 Hz | 75.45 | 0 | 13.17 |
| Contiguous, 25 ms lead / 30 Hz | 75.45 | 0 | 0 |
| Current / 15 Hz | 60.00 | 0 | 4,086.33 |
| Contiguous, 25 ms lead / 15 Hz | 60.00 | 0 | 4,072.67 |
| Current / 45 ms jitter | 75.45 | 0 | 41.50 |
| Contiguous, 75 ms lead, eight sources / 45 ms jitter | 75.45 | 0 | 0 |
| Current / 80 ms jitter | 72.80 | 0 | 752.50 |
| Contiguous, 75 ms lead, eight sources / 80 ms jitter | 72.80 | 51 | 1,368.42 |
| Contiguous, 75 ms lead, 16 sources / 80 ms jitter | 72.80 | 0 | 732.42 |
| Above, eight-frame catch-up / 15 Hz | 75.45 | 0 | 0 |
| Above, eight-frame catch-up / 80 ms jitter | 75.45 | 1 | 76.50 |

Jitter replaces every 15th 60 Hz arrival with a 45 ms interval, or every 30th
arrival with an 80 ms interval. These are arrival intervals, not an additional
pause of that length. The 45 ms condition fits the four-frame budget in this
replay; 80 ms does not. Larger buffers alone cannot recover discarded emulation
time. Enlarging only the lead can instead hit the source-count guard.

Event ordering matters as well. Delivering completed-source events after each
RAF batch gives the current 80 ms case **52 dropped packets and 1,428.25 ms of
gaps**, versus no drops and 752.50 ms when ended events are delivered first.
The 75 ms/eight-source candidate then drops 158 packets. The fully combined
candidate peaks at 16 sources and drops three packets. This models possible
callback delay, rather than predicting a particular browser event order.

## Real production-player evaluation

Environment: macOS 27.2.1 arm64, isolated headless Chromium 151.0.7922.34,
Node 26.12.1, viewport 1280 × 900, actual release Wasm and production Vite build.
Native RAF cadence was approximately 120 Hz. Each profile used a fresh browser
context, one second of warmup and an eight-second requested measurement window.
All final profiles preserved nonzero real PCM, running AudioContexts, and the
cartridge's green/red/blue framebuffer signals. No page errors occurred.

[audio-player.mjs](../tools/perf/audio-player.mjs) observes real source start
times, durations and nonzero buffers, counts PCM produced through the Wasm
adapter, and times RAF callbacks and `run_frame`. The CPU profiles use the
official [DevTools CPU slowdown factor](https://github.com/ChromeDevTools/devtools-protocol/blob/master/json/browser_protocol.json).
The low-frequency profiles gate each callback independently. The stall profiles
block the main thread for 80 ms every 500 ms while Web Audio continues running.

The temporary browser candidate applies three exact, asserted replacements to
the served JavaScript response: contiguous scheduling with 75 ms initial/recovery
lead, at most 16 sources, and at most eight emulated frames per RAF. It retains
the 150 ms tail guard and 100 ms elapsed clipping. Files on disk and the Wasm
are unchanged by the candidate.

Compatibility cartridge (`sram128-2003.ws`, color/sprites/IRQ/wavetable audio):

| Profile | Emulated frames/s | Produced / scheduled PCM, s | Gaps, ms | RAF callback p95, ms | Peak sources |
| --- | ---: | ---: | ---: | ---: | ---: |
| Current, Clean | 75.58 | 8.016 / 8.016 | 1.6 | 0.3 | 4 |
| Current, LCD Light | 75.47 | 8.003 / 8.003 | 1.2 | 0.6 | 4 |
| Current, Clean, CPU 8× slower | 75.58 | 8.016 / 8.016 | 0.2 | 2.2 | 6 |
| Current, LCD Light, CPU 8× slower | 75.52 | 8.016 / 8.016 | 0 | 4.2 | 6 |
| Current, Clean, approximately 15 Hz | 58.99 | 6.254 / 6.254 | 1,729.7 | 0.5 | 6 |
| Current, Clean, recurring 80 ms block | 73.38 | 7.857 / 7.698 | 348.9 | 0.3 | 8 |
| Candidate, Clean, approximately 15 Hz | 75.48 | 8.003 / 8.003 | 0 | 0.7 | 12 |
| Candidate, Clean, recurring 80 ms block | 74.74 | 8.003 / 7.990 | 77.1 | 0.3 | 13 |

The CPU-active `sound-direct.ws` probe continuously executes CPU Hyper Voice
writes instead of primarily waiting in HALT. Under 8× CPU slowdown it preserved
its green assertion signal and nonzero PCM, and measured **75.47 / 75.53 frames/s**
in Clean / LCD Light, with **zero scheduled gaps** and RAF p95 **2.6 / 4.9 ms**.
This covers a different producer workload; neither probe represents all games.

The approximately 15 Hz result is especially diagnostic: callbacks take only
0.5 ms at p95, yet production runs at about 59 frames/s. The four-frame policy,
rather than insufficient core throughput in this workload, imposes the limit.
Changing only scheduling/queue/catch-up rules restores supply without core edits.

The real stalled current player loses 0.159 seconds of drained PCM as well as
emulation time. The candidate reduces aggregate gaps by about 78%, but still
loses one packet and has one 77.1 ms gap. Its largest individual gap is larger
than the current player's 69.3 ms maximum: lower aggregate silence alone is
not sufficient acceptance. Its 75 ms reserve cannot reliably cover an 80 ms block
plus delivery phase and processing. The remaining count/time guards also need
explicit overflow handling.

## Recommended implementation

1. Correct scheduling continuity and make underrun/overflow observable. Keep
   future packets adjacent; on starvation perform an explicit bounded recovery.
   Test both silent intervals and PCM loss. Count queue capacity by scheduled
   time/PCM as well as resources; completed nodes awaiting `onended` must not
   consume the entire playable reserve. Raising the lead alone is rejected.
2. Give PCM production priority over expensive display work. Retain a bounded
   execution budget, but avoid throwing away emulation time merely because
   presentation is below 19 Hz. Evaluate reduced presentation/history updates
   with a defined visual tradeoff and preserve all emulated CPU/audio timing.
   Eight-frame catch-up demonstrates the cause; it is not a universal fix for
   arbitrary stalls or genuinely insufficient CPU capacity.
3. Establish a bounded, adjustable jitter reserve together with compatible
   count/time limits. The tested 75 ms candidate adds 50 ms of initial/recovery
   delay relative to the current 25 ms setting, uses more nodes, and remains
   inadequate for the 80 ms test. An implementation should pass the normal,
   low-frequency, intermittent-stall, delayed-ended and lifecycle cases without
   silent drops before choosing its latency default. Long-run clock drift and
   pause/mute/restart cleanup also need validation.
4. If display/DOM stalls still exhaust an acceptable latency budget, separate
   the producer from main-thread presentation with a worker and evaluate an
   AudioWorklet consumer with bounded PCM buffering. The
   [AudioWorkletProcessor runs on the rendering thread](https://www.w3.org/TR/2021/REC-webaudio-20210617/#AudioWorkletProcessor),
   but moving only the consumer does not supply PCM while its producer is blocked.
   This is a subsequent architectural experiment, not evidence that the CPU
   interpreter needs replacement. Sustained execution slower than real time
   still requires reducing work or optimizing the measured workload.

Clean is a reasonable low-cost display choice: it removed WebGL work and reduced
measured callback p95 in the CPU-loaded profiles. Both display modes remained
real-time here; this does not prove an old GPU is responsible for the report.

## Reproduction and evidence

Run from the repository root with existing frontend dependencies and generated
CC0 fixtures. To reproduce the historical browser measurements, build revision
`9edf5b6` in a separate checkout as documented in the
[README](../README.md#build-and-run-locally). The candidate response replacements
are deliberately tied to this baseline's minifier output and fail if it changes.

```sh
mkdir -p /tmp/ws-audio-baseline
git show 9edf5b6:web/src/audio.ts > /tmp/ws-audio-baseline/audio.ts
web/node_modules/.bin/tsc --ignoreConfig /tmp/ws-audio-baseline/audio.ts --target ES2022 --module ES2022 --lib ES2022,DOM --outDir /tmp/ws-audio-compiled --skipLibCheck
cp /tmp/ws-audio-compiled/audio.js /tmp/ws-audio-compiled/audio.mjs
node tools/perf/audio-scheduling.mjs --module /tmp/ws-audio-compiled/audio.mjs --output /tmp/ws-audio-scheduling.json
```

In another terminal, run the production preview:

```sh
cd web
./node_modules/.bin/vite preview --host 127.0.0.1 --port 4173 --strictPort
```

Then, from the repository root:

```sh
node tools/perf/audio-player.mjs --browser /path/to/chromium --profiles clean,lcd,clean-cpu8,lcd-cpu8,clean-15hz,clean-jitter80,candidate-15hz,candidate-jitter80 --revision 9edf5b6 --output /tmp/ws-audio-player.json
node tools/perf/audio-player.mjs --browser /path/to/chromium --rom fixtures/generated/sound-direct.ws --profiles clean-cpu8,lcd-cpu8 --output /tmp/ws-audio-direct.json
```

Omit `--browser` to use Playwright's installed browser. This host used cached
Chromium 151; the previously recorded Chrome 157 screenshot issue is unrelated
to this measurement. Build artifact hashes, ROM hashes, per-profile timings and packet
statistics are saved in [audio-stability-results.json](audio-stability-results.json).
The real-player records are profile aggregates; deterministic rows cover all
candidate/pattern combinations. Timing observations add instrumentation overhead
and have browser timer granularity; they are not isolated GPU execution timings.

`gap_ms` sums positive holes between consecutive source reservation intervals,
excluding initial startup lead and unobserved window boundaries. Injected timer
blocks lie outside the timed RAF callback bodies. `already_starved_ms` counts the part before the
next submission; `early_reanchor_gap_ms` counts holes inserted while a previous
tail is still future. They are distinct diagnostics, not additive partitions of
all gaps. Produced-minus-scheduled PCM measures draining/queue loss, independently
of holes. Queue occupancy can shift across measurement endpoints. Actual windows
vary slightly and a measurement can end during a block before catch-up occurs;
use the saved wall/audio durations rather than treating frame rates as exact.

Validation: both diagnostic scripts pass `node --check`; compiling the actual
`audio.ts` succeeds; all 45 deterministic rows and analytic sanity assertions
pass; the final eight compatibility profiles and two CPU-active profiles pass
their PCM, display and error assertions. An initial harness run timed out because
a shared RAF gate interfered with Playwright's polling; gates were corrected to
track callbacks independently before the final measurements. `npm run build`
passes and reproduces the measured bundle and Wasm artifact hashes. No production
regression suite was rerun for these diagnostic/documentation-only additions.

These tests measure the real scheduling API and PCM supply, without acoustic
capture. They do not measure physical speaker output, driver dropouts, listening
quality, a particular older CPU/GPU, other browser engines or commercial games.
They establish reproducible project-side defects without claiming all causes of
the owner's symptom are identified. No post-merge verification is required.
