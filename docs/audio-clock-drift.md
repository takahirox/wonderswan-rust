# Bounded audio clock-drift correction (Issue #115)

The [reference comparison](audio-playback-comparison.md) identified a persistent
clock mismatch that a fixed jitter reserve cannot absorb indefinitely. At the
`949dacf` baseline, a simulated audio clock 100 ppm faster than the producer
caused the first underrun after about 1,000 seconds; 100 ppm slower caused an
overrun reset after about 1,500 seconds. The new output feedback keeps the
scheduled reserve stable without changing the CPU, hardware timing, core sample
rate or generated PCM values.

## Implementation and tuning

[Sound](../web/src/audio.ts) measures its scheduled tail ahead of the current
audio clock. A two-second low-pass filter smooths that lead, and proportional
plus integral feedback targets the existing 100 ms pre-packet reserve. The
proportional gain is 0.2/s and the integral gain 0.01/s². Integration and rate
slew use processed PCM duration, so several packets submitted after one delayed
timer callback do not imply that additional host time passed.

Output correction is limited to **0.995–1.005**, with a slew limit of 500 ppm per
second of input PCM before AudioParam float32 rounding. The integral term is
bounded and stops accumulating further in the saturation direction. Controller
state resets on starvation, overflow recovery and lifecycle queue cleanup.
`Sound.timing.playbackRate` exposes the currently selected correction alongside
existing queue/recovery diagnostics.

Every new `AudioBufferSourceNode` receives a fixed playback rate before it is
scheduled. Existing sources are never retuned. Reservation and reclamation use
`sampleFrames / sampleRate / acceptedPlaybackRate`; the code reads the assigned
AudioParam value back so float32 rounding cannot accumulate timestamp error.
This follows the Web Audio specification's
[playback-rate semantics](https://www.w3.org/TR/2021/REC-webaudio-20210617/#dom-audiobuffersourcenode-playbackrate)
and [duration convention](https://www.w3.org/TR/2021/REC-webaudio-20210617/#dom-audiobuffersourcenode-start).
Source buffer data and sample counts are unchanged. The audio engine performs
the small output-rate resampling; there is no core sample skipping or duplication.

The first PI candidate passed constant-drift cases but failed when startup burst
delivery and repeated stalls were combined: integral accumulation during a
saturated positive correction subsequently drained the reserve too far. Adding
anti-windup removed those recoveries without weakening the tests. Halving the
integral gain was also evaluated as a temporary compiled-module candidate; it
offered little additional headroom, so the original gain with anti-windup was
retained. The production implementation adds no worker, worklet or dependency.

## Clock and jitter regressions

The actual compiled Sound class runs for one virtual hour per case with a
float32 AudioParam model. All ended events are withheld to preserve the previous
timestamp-reclamation stress. The suite covers 0/±100/±1,000 ppm, a sign reversal
halfway through the hour, and both drift directions combined with bursts after
approximately 80 ms stalls. Each burst-delivery vector withholds six 13.25 ms
arrivals and delivers seven packets together; the interval from the preceding
delivery can reach 92.75 ms. All generated packets must be scheduled.

The eight final cases have **zero underruns, overrun resets, scheduled gaps and
cancelled future PCM**. Rates and per-packet slew stay bounded; the final minute's
mean correction matches the inverse clock ratio within 100 ppm. Pause/resume
after learning a non-unity rate must restart at unity; mute clears the queue.
The ordinary 120-second continuity, deliberate starvation/overflow and lifecycle
regressions remain mandatory and also pass.

| Imposed constant audio-clock difference | Baseline underruns / resets in one hour | Corrected underruns / resets | Corrected settled mean rate |
| --- | ---: | ---: | ---: |
| 0 ppm | 0 / 0 | 0 / 0 | 1.0000000 |
| +100 ppm | 3 / 0 | 0 / 0 | 0.9999000 |
| −100 ppm | 0 / 2 | 0 / 0 | 1.0001000 |
| +1,000 ppm | 35 / 0 | 0 / 0 | 0.9990010 |
| −1,000 ppm | 0 / 23 | 0 / 0 | 1.0010010 |

The reversal case converges to 1.0010010. The fast/slow burst cases converge to
0.9990013 / 1.0010013. Constant-drift minimum start lead is at least 95.5 ms;
the adversarial fast startup/burst case reaches approximately 4.76 ms before
recovering its reserve without a discontinuity. That last figure comes from a
zero-CPU-cost model and is not a hardware deadline guarantee.

These imposed clock rates are a sensitivity test, not measurements of a real
device oscillator. The prior baseline data remains preserved in
[audio-playback-clock-results.json](audio-playback-clock-results.json).

## Real Web Audio validation

The production-player observer now calculates each packet's effective duration
using the real AudioParam rate. Scheduled PCM quantity is counted from source
sample frames, separately from playback duration. It checks that old rates stay
fixed and that no scheduled hole, drained PCM loss or early cancellation is
hidden by rate correction.

Two additional browser profiles inject an adapter sample-rate mismatch of
24,024 / 23,976 Hz while the core still produces its original 24 kHz PCM. This
makes actual Web Audio packets initially shorter/longer relative to the producer
without changing the browser clock. It exercises the real resampling, parameter
and scheduling APIs; it is not a real hardware-clock measurement. Each profile
runs for 60 seconds, with the final quarter's mean rate required within 500 ppm
of the compensating ratio. A combined profile adds CPU slowdown, Retro CRT and
recurring 80 ms main-thread stalls to the fast mismatch.

The existing low-RAF, display/CPU-load and recurring-stall cases still use real
Wasm execution, nonzero PCM and framebuffer probe signals. Core-generated PCM
values continue to be checked exactly by the production compatibility suite.

Final measured mean rates are 0.9989990 / 1.0010149 for the fast/slow mismatch,
against expected 0.9990010 / 1.0010010. The combined fast-mismatch/CRT/8× CPU/stall
profile measures 0.9989766 over its final quarter. All **17 final browser cases**
pass strict continuity, PCM quantity/cancellation, fixed-rate, resource and
real-time-supply assertions. This includes the eleven-case normal/load matrix,
two CPU-active Hyper Voice cases, both 60-second mismatch cases, a 30-second CRT
stress case and the 60-second combined case. Instrumented submission intervals
reach 96 ms; injected 80 ms blocks reach approximately 81.9 ms in those runs.

One initial eight-second CRT/8× CPU/stall run failed: it produced 7.910 seconds
of PCM in an 8.003-second window and had one 125.95 ms scheduled gap. No drained
PCM was rejected or cancelled. That run lacked arrival/injection telemetry, so
its cause is not established and it is not reported as passed. The diagnostic
was extended to record actual block/submission intervals and gap details. The
subsequent full matrix and longer focused/combined runs above pass unchanged
strict assertions, using the same production code. This is an observed
scheduling-sensitive limit, not proof of continuity under arbitrary CPU/GPU or
host scheduling load. The failed record is preserved with the passing evidence.

Results, source/module/build identities and the initial failure are in
[audio-clock-drift-results.json](audio-clock-drift-results.json).

## Tradeoffs and limits

The initial/recovery reserve remains 100 ms; correcting drift does not require
another fixed latency increase. Output pitch follows the small rate adjustment.
The ±0.5% hard limit is approximately ±9 cents at its extremes; no claim of
inaudibility or subjective sound quality is made. Corrections for a constant
±0.1% mismatch settle near ±0.1%, with transient overshoot bounded by the limit.

The producer is still on the main thread, and the previous work/backlog and
resource limits remain. Long stalls, sustained under-speed, or mismatches beyond
the controller range can still require counted recovery. This change does not
claim lossless arbitrary overload, physical-device latency, other-browser
behavior or measured stability on a particular old PC. A continuous worklet
resampler remains a separate option if allocations or audio quality warrant it.

## Reproduction

Build the frontend and start production preview as described in the
[README](../README.md#production-preview-and-browser-acceptance). From the root:

```sh
web/node_modules/.bin/tsc --ignoreConfig web/src/audio.ts --target ES2022 --module ES2022 --lib ES2022,DOM --outDir /tmp/ws-audio-drift --skipLibCheck
cp /tmp/ws-audio-drift/audio.js /tmp/ws-audio-drift/audio.mjs
node tools/perf/audio-queue.mjs --module /tmp/ws-audio-drift/audio.mjs --drift-output /tmp/ws-audio-drift-model.json
node tools/perf/audio-player.mjs --browser /path/to/chromium --check --output /tmp/ws-audio-player.json
node tools/perf/audio-player.mjs --browser /path/to/chromium --check --seconds 60 --profiles clean-rate-fast,clean-rate-slow --output /tmp/ws-audio-rate.json
node tools/perf/audio-player.mjs --browser /path/to/chromium --check --seconds 60 --profiles crt-rate-fast-jitter80-cpu8 --output /tmp/ws-audio-combined.json
```

The default browser matrix omits the longer mismatch profiles; select them
explicitly. `--revision` records an explicit source label in browser outputs.
Omit `--browser` to use Playwright's installed Chromium. This host uses isolated
cached Chromium 151.0.7922.34, without an authenticated browser profile.

Mandatory validation passes: frontend TypeScript/Vite build; default queue and
eight one-hour clock/jitter/lifecycle cases; full production smoke and
compatibility suites; the 17 final real-player cases; diagnostic syntax;
source/build/JSON consistency; relative links; and whitespace. Wasm is unchanged
and reused. Unrelated Rust suites, other engines, physical-PC clock measurement
and acoustic quality/latency capture were not run. The rejected initial PI model
and the separate initial real stress failure are documented above rather than
rewritten as passing results. Required post-merge verification: none.
