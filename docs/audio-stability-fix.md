# Audio production independent of presentation (Issue #112)

This implements the [Issue #110 findings](audio-stability.md). The player now
produces PCM from a timer using elapsed monotonic time; RAF only presents the
latest display state. Low presentation frequency no longer imposes a four-frame
audio-production ceiling. Audio is submitted before per-frame display work.
The LCD history still advances for every emulated frame.

The subsequent [reference playback comparison](audio-playback-comparison.md)
examines frontend buffering and the remaining long-term clock-drift limitation.

## Scheduling and resource limits

- The producer wakes within a requested maximum of 8 ms. Each turn starts at
  most eight frames and stops starting more once the turn has consumed 8 ms.
  A single frame can itself exceed that budget. Backlog is limited to 100 ms,
  preventing unbounded catch-up after a long stall. The next frame's current
  hardware cadence determines credit consumption, including live timing changes.
- Future audio packets remain contiguous. Startup or actual underrun establishes
  a **100 ms** scheduling reserve, rather than reapplying 25 ms to every packet.
  This adds 75 ms of initial/recovery lead relative to the old setting, in
  addition to frame generation and device latency. It is the principal tradeoff
  for tolerating the demonstrated main-thread stalls without another subsystem.
- Completed sources are reclaimed using their scheduled end time on the audio
  clock, independently of delayed `onended` delivery. Normal completion still
  releases them through `onended` when available.
- At most 32 sources are retained. If the existing tail exceeds 250 ms or the
  source bound is reached, the queue explicitly resynchronizes and accepts the
  incoming PCM. This exceptional recovery cancels old queued sound; it does not
  claim lossless arbitrary overproduction. `Sound.timing` exposes underrun and
  overrun counts, retained sources and queued duration for diagnostics. The
  accepted final packet can extend a tail beyond the threshold by one packet.
- Pause, visibility loss, menu entry, restart and model/cartridge changes cancel
  both the producer and presentation callbacks. Audio cleanup and user-gesture
  activation remain on their existing lifecycle paths. Muting allows emulation
  to continue without creating audio packets.

The producer remains on the main thread. Sustained CPU demand above real-time
capacity, sufficiently long DOM/GPU stalls, browser timer suspension and device
or driver dropouts remain outside the demonstrated envelope. A worker remains
a possible next step if measured stalls exceed an acceptable reserve. There is
no CPU interpreter or hardware timing change in this fix.

## Before and after

Eight-second requested windows, real Wasm/Web Audio, macOS arm64 and isolated
Chromium 151.0.7922.34. The historical baseline is `9edf5b6`; the final tree
includes main `cab3a40`. Explicit display choices keep the comparable conditions
independent of main's display-default change.

| Condition | Before frames/s / gaps, ms | After frames/s / gaps, ms |
| --- | ---: | ---: |
| Clean, approximately 15 Hz presentation | 58.99 / 1,729.7 | 75.47 / **0** |
| Clean, recurring 80 ms main-thread block | 73.38 / 348.9 | 75.47 / **0** |
| Clean, approximately 10 Hz presentation | Not measured in baseline browser run | 75.48 / **0** |
| LCD Light, approximately 15 Hz presentation | Not measured | 75.60 / **0** |
| LCD Light, CPU 8× slower, recurring 80 ms block | Not measured | 75.43 / **0** |
| Retro CRT, CPU 8× slower, recurring 80 ms block | Not measured | 75.45 / **0** |

All **11** final compatibility-player profiles have zero scheduled gaps, zero
drained PCM loss and zero cancellation of future PCM. The two CPU-active Hyper
Voice profiles also pass these assertions. Peak observed sources are at most
11, below the retained bound of 32. Requested display modes remain active;
falling back to Clean cannot silently satisfy an effect profile.

A further **30-second** Retro CRT/8× CPU/recurring-80-ms-block run produces and
schedules **29.998 seconds** of PCM with zero gaps, loss or cancellation and a
source peak of 10. Its actual wall window is 30.082 seconds, including a final
block before the next catch-up. Frame counts and PCM supply vary slightly with
measurement endpoints; all playback-gap assertions require zero, not a tolerance.

The old 75 ms temporary candidate still had a 77.1 ms gap in the recurring-stall
case. The implemented clock separation, 100 ms reserve and timestamp reclamation
address that demonstrated failure; merely raising the old four-frame ceiling was
not shipped as the fix.

## Validation method

The production-player diagnostic now has `--check`: it asserts no gaps between
observed packet reservations, no drained PCM loss, no cancellation of future PCM,
bounded sources, and production within 120 ms of the actual measurement duration.
That tolerance allows the final packet/timer/stall phase; it is not a playback-gap
tolerance. All tested playback gaps must be zero. Source observations, nonzero
samples, actual Wasm execution and framebuffer probe signals remain mandatory.

The historical 9edf5b6 results remain in [audio-stability-results.json](audio-stability-results.json).
Final measurements on the integrated local-ROM-only player are in
[audio-stability-fix-results.json](audio-stability-fix-results.json), including
revision context and asset/ROM hashes. Main's merged Retro CRT default and local
ROM changes are retained. The unchanged release Wasm artifact is reused; only a
test-fixture reference changed in main's Rust source since the measured baseline.

The normal smoke test now waits for actual observed Wasm frames while holding
or releasing input, rather than assuming RAF advances emulation. CPU timing and
LCD tests use a shared controlled host clock for producer timers and presentation;
their original hardware observations, poisoned retained data, restart assertions
and exact PCM checks remain. The LCD cadence test additionally checks one pending
producer, one pending presentation, bounded catch-up, and cancellation of both.

[audio-queue.mjs](../tools/perf/audio-queue.mjs) exercises the actual compiled
Sound class for 120 seconds of deterministic PCM with all ended events withheld.
It checks adjacent packet boundaries, timestamp-based reclamation, resource
bounds, real starvation recovery, deliberate queue overflow accepting every
incoming packet, and pause/resume/mute cleanup. The observed retained peak is nine
sources, with no underruns or overruns during correctly paced production.

Mandatory checks pass: production TypeScript/Vite build, full production smoke
and compatibility suites on the integrated main, all 14 checked real-player
profiles, queue regressions, changed JavaScript syntax, artifact/result consistency,
relative documentation links and whitespace. No Rust runtime or fixture bytes
changed; unrelated Rust suites were not rerun. The historical 45-case replay
remains baseline evidence and is not a current-implementation test.

An initial pre-integration smoke run failed a mobile short-tap expectation because
its RAF waits no longer implied CPU progress. Replacing those waits with observed
Wasm frames resolved it; the full final suite passes. Main integration required a
README conflict resolution retaining the local-ROM-only content. Both complete
browser suites were rerun after integration, including main's updated LCD/CRT and
local-read acceptance. No physical-device or acoustic results are claimed.

## Reproduction

From the root, using the existing frontend dependencies and generated CC0
fixtures, build the frontend and start production preview as described in the
[README](../README.md#production-preview-and-browser-acceptance). For example,
run `npm run build` and `./node_modules/.bin/vite preview --host 127.0.0.1 --port 4173 --strictPort`
from `web`. Then run from the root:

```sh
web/node_modules/.bin/tsc --ignoreConfig web/src/audio.ts --target ES2022 --module ES2022 --lib ES2022,DOM --outDir /tmp/ws-audio-current --skipLibCheck
cp /tmp/ws-audio-current/audio.js /tmp/ws-audio-current/audio.mjs
node tools/perf/audio-queue.mjs --module /tmp/ws-audio-current/audio.mjs
node tools/perf/audio-player.mjs --browser /path/to/chromium --check --output /tmp/ws-audio-current.json
node tools/perf/audio-player.mjs --browser /path/to/chromium --check --rom fixtures/generated/sound-direct.ws --profiles clean-cpu8,lcd-jitter80-cpu8 --output /tmp/ws-audio-direct.json
```

Omit `--browser` to use Playwright's installed Chromium. `--profiles` selects
focused cases and `--seconds` selects the measurement duration; the default
matrix includes Clean, LCD Light, Retro CRT, CPU slowdown, 15/10 Hz presentation
and recurring 80 ms main-thread stalls. Historical response-patch candidates
require explicit profile selection and the historical build.

Also run the production smoke and compatibility suites documented in the README.
They require the existing Playwright browser installation; this host selected
isolated cached Chromium 151 explicitly. No authenticated browser profile is used.

These are scheduling/PCM observations, without acoustic capture or a claim about
every old PC, game, browser engine or long-term clock drift. No post-merge
verification is required for this Issue.
