# Reference audio playback comparison (Issue #113)

This records the research baseline. The subsequent
[clock-drift correction](audio-clock-drift.md) implements and validates the
output feedback identified here; the historical measurements below are retained.

The most useful next technique is **queue feedback and clock-drift correction**,
followed by audio-aware reduction of presentation cost. Replacing the current
buffer scheduler with an AudioWorklet is a separate experiment, not a necessary
first step: RetroArch's default Emscripten build also schedules small Web Audio
buffers. The current project already shares its contiguous scheduling principle,
but lacks the frontend's continuous rate feedback.

This is source research and a controlled sensitivity experiment at project
revision `979c75e`, following [the implemented stability fix](audio-stability-fix.md).
It does not change production code. Earlier [core throughput comparisons](performance.md)
bypassed actual playback and cannot rank these systems' audible stability or latency.

## Revisions and boundary

Sources inspected on 2026-11-17:

| Component | Pinned revision | Role |
| --- | --- | --- |
| Beetle WonderSwan | `4b01295838ea89e3f1355bbe4cb5cf98aa6108cd` | The previously measured core and its libretro adapter |
| RetroArch | `d78199ff8590f7dfd0efac9c32fd7a4984e7752a` | Frontend rate control and browser audio drivers, inspected separately |
| Gameblabla's Oswan | `efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33` | The previously measured core plus its actual SDL playback path |
| NitroSwan | `564e171fb93c2c11f98f38607493675727676c2c` | Nintendo DS Maxmod output and a PCM ring |
| NitroSwan's Sphinx submodule | `8b625f118caa5a58f09c9fb2a32e1d1b70746f78` | The audio-feedback consumer selected by that NitroSwan revision |

Cached Beetle/Oswan/NitroSwan files were checked against their pinned Git commits.
RetroArch files and the Sphinx dependency were fetched from primary upstream
sources at explicit revisions. Neither the native throughput host nor inspecting
a browser driver establishes that a particular installed Beetle frontend uses it.
No external implementation is copied, ported or added as a runtime dependency.

## Comparison

| Path | PCM consumption | Synchronization / pressure response | Relevant difference here |
| --- | --- | --- | --- |
| Beetle + frontend | Core hands batches to a frontend; frontend owns the device | Optional audio-buffer-based render skipping; frontend rate/latency controls | Observe actual queue pressure rather than only elapsed wall time |
| RetroArch RWebAudio | Scheduled `AudioBufferSourceNode` packets | Contiguous future tail, configured virtual buffer, frontend rate control | Similar primitive to this project; node allocation alone does not prove a redesign is needed |
| RetroArch AudioWorklet | Render-thread callback pulls an SPSC ring | Available-space feedback, bounded writes, underrun/drop counters | Continuous consumer and reusable storage; producer must still keep up |
| Oswan SDL | SDL callback consumes a PCM ring | Mutex/condition variable; callback can wait when short | Ring structure transfers; its shortage/overflow behavior should not be copied |
| NitroSwan Maxmod | Hardware-timed stream callbacks copy a PCM ring | Filtered ring error adds/skips mixer updates | Feedback principle transfers; the sample-generation shortcut does not preserve this project's PCM contract |
| This project | Main-thread timer submits scheduled Web Audio packets | 100 ms recovery reserve, timestamp reclamation, bounded resynchronization | Short-stall robustness is measured; ongoing clock correction is absent |

## Beetle: synthesis and playback are different responsibilities

[sound.c](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/mednafen/wswan/sound.c#L329)
finishes timestamp-based Blip buffers and drains interleaved stereo PCM.
[libretro.c](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/libretro.c#L1390)
executes the machine, obtains PCM and repeatedly calls the frontend's batch
callback until the batch has been accepted. The core does not own a sound-card
callback or independently guarantee uninterrupted playback. A frontend that
never accepts data would also prevent that upload loop from completing.

Optional frameskip registers audio-buffer occupancy and likely-underrun feedback.
Auto mode uses likely underrun; manual mode compares occupancy with a threshold.
The skip flag is passed to video rendering while emulation and sound generation
continue. There is a limit of 30 consecutive skipped frames. This is useful
precedent for reducing visual work under pressure without discarding CPU/audio time.

When this option is enabled and the frontend supports monitoring, native-rate
playback requests eight frame periods of minimum frontend audio latency:
8 × 13.25 ms = 106 ms, rounded upward to a multiple of 32 ms: **128 ms**.
When disabled, the request is zero, meaning the frontend's own setting, not
zero physical latency. Frameskip defaults to disabled in
[the core options](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/libretro_core_options.h#L389).
Thus 128 ms is an optional request, not Beetle's universal/default measured delay.

The adapter also has an optional approximately-60-Hz output mode: it executes
five machine frames for four frontend calls, caches their PCM and redistributes
it. Its advertised rate is actually 4/5 × 75.471698 Hz, about **60.377358 Hz**.
That preserves production across the conversion; simply producing only 60
WonderSwan frames per second is not equivalent. Configurable synthesis output
rates and low-pass filtering are further accuracy/work differences, separate
from the host playback policy.

## RetroArch: rate feedback is the major missing mechanism here

The inspected [audio driver](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/audio/audio_driver.c#L1677)
samples writable space, compares it with half the reported buffer, and uses the
error to adjust the resampling ratio. It includes audio-paced special handling
and a slower sink-rate estimate; this is more than a fixed buffer enlargement.
Its [design documentation](https://docs.libretro.com/development/cores/dynamic-rate-control/)
explains the resampling approach and notes the near-monitor-rate scope of its
VSync synchronization model. WonderSwan's approximately 75.47 Hz must not simply
be forced to 60 Hz to reuse that model. For this timer-driven player, the
transferable idea is stabilizing audio occupancy against clock differences.

### Browser default: RWebAudio

[Makefile.emscripten](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/Makefile.emscripten#L67)
defaults RWebAudio on and AudioWorklet off. The
[C driver](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/audio/drivers/rwebaudio.c#L73)
aggregates/deinterleaves into reusable temporary storage in nominal 10 ms blocks
at the actual context rate. Its
[JavaScript side](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/os/emscripten/library_rwebaudio.js#L76)
creates a buffer and source per block, preserves an existing future tail, and
adds a 10 ms offset only when establishing a new tail. It has another 10 ms
browser-dependent offset for its non-Chromium path. These offsets are not the
entire pipeline latency: the configured virtual buffer and device also matter.

Writable space is derived from configured buffer duration minus the scheduled
tail lead. It is exposed to the frontend's rate controller. The time estimate
uses `performance.now()` calibrated against `AudioContext.currentTime`, with a
recalibration hook. Blocking/nonblocking behavior depends on build flags and
has bounded waits. It is therefore inaccurate to describe all RetroArch browser
audio as AudioWorklet-based or as automatically immune to a blocked producer.

### Optional AudioWorklet path

[audioworklet.c](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/audio/drivers/audioworklet.c#L41)
uses an SPSC PCM ring pulled by a render callback. It reports writable space and
buffer size, counts underruns/dropped frames, and fills shortages with silence.
The producer does not take the consumer's lifetime lock; a teardown lock remains.
Writes and initialization have bounded failure/wait paths. These details matter
as much as choosing an AudioWorklet.

Visible capacity is configured latency × device rate × stereo float frame size.
One external-write-block build path allocates another **128 ms** reserve; the
driver subtracts that reserve from its exposed writable-space accounting.
Capacity, feedback setpoint, initial lead and audible latency are distinct numbers.

The inspected [build instructions](https://github.com/libretro/RetroArch/blob/d78199ff8590f7dfd0efac9c32fd7a4984e7752a/pkg/emscripten/README.md#L198)
require threads for this Wasm AudioWorklet driver. Proxying the frontend to a
pthread is a separate option, incompatible with RWebAudio in that build. The
threaded configuration requires cross-origin isolation headers. A custom JS
AudioWorklet using transferred messages would be a different design; these
Emscripten flags are not universal requirements of every AudioWorklet. Verify
the intended transport and hosting before adopting that architecture.

## Oswan: a ring does not guarantee safe underrun handling

[sound_SDL.c](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/sound/sound_SDL.c#L14)
provides an SDL callback that copies from separate left/right ring arrays under
a mutex. In the normal build it requests 44,800 Hz and 512-frame callbacks,
about 11.43 ms. [The ring](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WSApu.h#L6)
holds 32 × 512 = 16,384 frames, about 365.71 ms of capacity. Its native-audio
variant requests 12 kHz, 256-frame callbacks and an 8,192-frame ring. These are
source configuration/capacity figures, not measured output delay or guaranteed
SDL-obtained settings.

On shortage, the callback waits on a condition variable. Its shortage branch
does not subsequently execute the copy loop or explicitly fill the requested
output; it is not a model for a nonblocking real-time consumer. The inspected
[producer](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WSApu.c#L303)
wraps the write index without a full-ring guard. Its ring-length calculation
cannot distinguish a completely full ring from an empty one when indices match.
These are source observations, not an acoustic demonstration of a particular failure.

Its source synthesis is at scanline cadence near 12 kHz; the normal path repeats
each value alternately three/four times. That is a different quality/cadence
tradeoff from our 24 kHz PCM. Only the producer/consumer separation and reusable
storage are candidate ideas; the callback waiting and overwrite policy are rejected.

## NitroSwan: device-clocked consumption with occupancy feedback

[Main.c](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/source/Main.c#L224)
opens a Maxmod hardware-timed stereo stream using 24 kHz and a 640-frame buffer
from [Sound.h](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/source/Sound.h#L17).
Automatic callbacks occur as half the stream buffer is consumed: nominally 320
frames / 24 kHz = 13.33 ms. The separate
[PCM ring](https://github.com/FluBBaOfWard/NitroSwan/blob/564e171fb93c2c11f98f38607493675727676c2c/source/Sound.s#L22)
has 2,048 frames, about 85.33 ms capacity, with its write pointer initially
1,024 frames ahead, about 42.67 ms. These layers are not an end-to-end latency measurement.

The callback filters the read/write distance error and communicates a signed
sample deficit. The pinned
[Sphinx consumer](https://github.com/FluBBaOfWard/Sphinx/blob/8b625f118caa5a58f09c9fb2a32e1d1b70746f78/WSVideo.s#L1664)
uses it to add or skip mixer updates at scanline boundaries. This is concrete
feedback against producer/consumer mismatch, but should not be copied as an
alteration to this project's synthesis cadence. Preserve our core's sample and
hardware timing, and evaluate correction at the output/resampling boundary instead.

## Current-project clock sensitivity

The current producer follows `performance.now()` and emits fixed-rate PCM;
the consumer follows the AudioContext clock. The 100 ms reserve absorbs short
jitter, but there is no continuous feedback correcting a persistent difference.
The optional `--drift-output` extension to
[audio-queue.mjs](../tools/perf/audio-queue.mjs) runs the actual compiled Sound
class for one virtual hour under five imposed audio-clock rates. Production is
perfectly periodic every 13.25 ms, ended events are withheld, and no CPU, device
or browser-render-quantum behavior is simulated. This is a sensitivity model,
not measurement of any real device's clock or a reference-player benchmark.

| Audio clock relative to producer | First recovery, wall s | Underruns / overrun resets | Scheduled gaps, ms | Cancelled future PCM, ms |
| --- | ---: | ---: | ---: | ---: |
| 0 ppm | None | 0 / 0 | 0 | 0 |
| +100 ppm (+0.01%) | 1,000.004 | 3 / 0 | 300.00 | 0 |
| −100 ppm (−0.01%) | 1,500.006 | 0 / 2 | 200.00 | 500.00 |
| +1,000 ppm (+0.1%) | 100.011 | 35 / 0 | 3,500.39 | 0 |
| −1,000 ppm (−0.1%) | 150.003 | 0 / 23 | 2,300.00 | 5,750.07 |

The first recovery agrees with a simple independent calculation: 100 ms of
reserve drains in 1,000 seconds at 100 ppm; the margin from 100 to the 250 ms
overrun threshold fills in 1,500 seconds at −100 ppm. A larger fixed reserve
delays this boundary but does not remove it. On overflow, the model accounts for
stopped future PCM and the new recovery gap; those quantities are not additive
partitions of silence. Nominal-clock and existing 120-second queue/lifecycle
checks still pass. Raw results and source/module identities are in
[audio-playback-clock-results.json](audio-playback-clock-results.json).

This identifies a limitation worth testing, not evidence that the owner's PC
has this clock error. It does not invalidate the previous real-browser 80 ms
stall results, which evaluated a different failure mode over shorter windows.

## Recommended next work

1. Measure queue lead over time, underruns, cancelled PCM, producer turn cost and
   the observed audio/performance-clock relationship. Then evaluate a bounded,
   slowly varying output-rate correction driven by queue occupancy. Keep CPU,
   raster, DMA and core PCM timing unchanged. If scheduled packet playback rates
   change, reservation durations and completion timestamps must change with them;
   updating rates while keeping old end times would break current queue accounting.
2. Evaluate audio-pressure-based presentation cost reduction, following Beetle's
   feedback principle. Prefer cheaper output effects/resolution before changing
   per-emulated-frame LCD history. Never discard CPU/audio time as a render shortcut.
   Use hysteresis and verify both recovery behavior and the visual tradeoff.
3. Compare the existing scheduled-buffer path with a reusable PCM ring and a
   long-lived AudioWorklet consumer. Measure allocations/GC, real CPU time,
   starvation/cancellation, latency and lifecycle behavior on the same workloads.
   Isolate the producer in a worker only if its main-thread stalls remain the
   measured limitation. A ring/worklet alone cannot generate missing PCM.
4. Tune latency after those measurements. RetroArch's 128 ms optional request,
   its virtual capacity, NitroSwan's ring sizes and our 100 ms recovery lead are
   different quantities. They do not establish which player has lower physical
   latency. Compare equivalent device rates, settings and cartridge success paths.

## Reproduction and validation

The following baseline commands describe checkout `949dacf`. On current main,
the same drift option validates corrected behavior and expanded scenarios; see
[the implemented correction](audio-clock-drift.md) for current expectations.

From the root with existing frontend dependencies:

```sh
web/node_modules/.bin/tsc --ignoreConfig web/src/audio.ts --target ES2022 --module ES2022 --lib ES2022,DOM --outDir /tmp/ws-audio-reference --skipLibCheck
cp /tmp/ws-audio-reference/audio.js /tmp/ws-audio-reference/audio.mjs
node tools/perf/audio-queue.mjs --module /tmp/ws-audio-reference/audio.mjs
node tools/perf/audio-queue.mjs --module /tmp/ws-audio-reference/audio.mjs --drift-output /tmp/ws-audio-clock-results.json
```

The drift option is opt-in, preserves the existing default queue regressions,
and checks nominal continuity, drift-induced recoveries and resource bounds.
Its expected sensitivity results describe the pinned baseline, not a desired
regression target after future drift correction. Normalization/resampling is
not prototyped or claimed fixed by this report.

Primary source revisions, numeric conversions, script syntax, both diagnostic
modes, saved JSON consistency, relative links and whitespace were checked.
No production browser/Rust suites were rerun for report/diagnostic-only changes.
No RetroArch/SDL playback application or DS runtime was executed, and no physical
audio capture was used. Reference playback stability, actual device
latency and sound quality were not measured or ranked. Post-merge verification: none.
