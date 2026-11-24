# Programmable LCD timing (Issue #27)

## Source reconciliation, before scheduler implementation

Inspected on 2026-10-24:

- [Wonderful display definitions, pinned](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/display.xml):
  16 is the final line before restart; the default is 158. Even values are a
  safety recommendation, not a documented write mask. Register 17 controls STN
  (WS/WSC) vertical back porch and should normally be programmed to total minus
  three. It is independent of the total.
- [WSHWTest observations](https://github.com/FluBBaOfWard/WSHWTest/blob/main/README.md)
  (V0.2.2): startup tables distinguish ASWAN 16=9C/17=92, Sphinx
  16=9E/17=9B, and Sphinx2 16=9E/17=00. The author explicitly qualifies
  interpretations; these are neither complete firmware handoffs nor proof of
  Crystal write/read behavior.
- [ares scheduler, pinned](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/ppu/ppu.cpp)
  and [I/O, pinned](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/ppu/io.cpp):
  implementation corroboration for programmable totals and refresh reporting.
  Writes update register state directly; scheduling checks the total at a line
  boundary. Short-frame handling uses a minimum active height and comments retain
  uncertainty about line compare. Register 17 is stored without a Crystal gate.
  This is implementation evidence, not silicon measurement; no code was copied.

The old assumption was that 158 was the only supported total. The corrected
requirement is an independently programmable final-line byte, supported by the
Wonderful definitions. Preserve the existing synthetic 158 startup on every
model; do not reconstruct firmware from partial startup tables.

## Explicit compatibility policies where evidence is incomplete

- Writes read back immediately and affect the next line-end comparison. A write
  never changes elapsed clocks, current line, line phase or completed frame count.
  At line end, wrap if the current line is greater than or equal to the new total.
  Thus shortening below the current line finishes that line, then completes one
  frame; increases extend the current frame. CPU writes occur at the emulator's
  existing instruction boundary, without new intra-instruction bus timing.
- All byte totals, including odd and short values, are retained literally. A
  stable total T produces T+1 lines of 256 clocks. This short-total policy differs
  from ares's uncertain minimum-height treatment. No VBlank event is synthesized
  when line 144 is unreachable; unvisited framebuffer rows retain their pixels.
  Even full-height totals are the established use case. These edge policies
  guarantee bounded progress but do not claim measured physical accuracy.
- STN 17 retains independent byte writes, initially 155 (158 minus three).
  Its analog back-porch effects are outside the digital framebuffer subset.
  Crystal accepts/ignores writes and reads zero. The latter is a conservative
  model policy informed by the startup observation and STN-only definition,
  not proof of constant physical readback. Changing 16 never rewrites 17.
- A frame completes at the actual raster wrap. Rendering, scanline readback,
  line compare, HBlank timer, line-144 VBlank/timer and sprite latch share that
  raster state. Device clocks remain machine clocks, including across wraps.
  Instructions and DMA words complete before returning from run_frame, so their
  bounded overshoot belongs to the next raster frame.

The framebuffer stays 224 by 144. Stable cadence is 3,072,000 / (256*(T+1)) Hz;
158 retains the existing default cadence. The Wasm adapter reports current
register cadence, and the browser reads it for each scheduling decision, with
its existing four-frame catch-up bound and lifecycle/audio handling.

This work is independently authored and firmware-free. It does not establish
commercial-title compatibility, analog panel response, mid-frame silicon
latching, physical short/odd-total behavior, or precise Crystal readback.

## One-line shade/color translation (Issue #37)

Source reconciliation completed before implementation on 2026-10-24. The pinned
[Wonderful DISPLAY_LINE definition](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/display.xml)
says final color translation has a one-line delay and instructs software changing
LCD shades or color palettes to subtract one from DISPLAY_LINE. Thus translation
while DISPLAY_LINE is L targets composed row L−1, including row 143 during line
144. The old assumption, immediate composition-to-RGBA at current-line end, was
incorrect. Delaying finished RGBA or refetching old tile rows cannot satisfy this
requirement: translation must see later palette values and earlier identities.

Also inspected pinned ares
[screen](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/ppu/screen.cpp),
[memory](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/ppu/memory.cpp),
[DAC](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/ws/ppu/dac.cpp)
and scheduler sources: these illustrate tile identities, transparency, palette
lookup, backdrop and LCD controls but combine translation with drawing. They are
implementation context, not independent evidence of fetch phases or this delay;
no implementation was copied. Previously inaccessible WSdev/WSman excerpts are
not used as established requirements.

The primary definition establishes the row offset, not sampling phases, mono
palette-port timing, backdrop/LCD latching or runtime-mode transitions. The
following are explicit, deterministic compatibility policies, not silicon claims:

| State | Sampling policy |
| --- | --- |
| Screen 1 and Screen 2 scroll (10–13), sprite window bounds (0C–0F) | Readback reflects writes immediately. Composition uses the values sampled at entry to row L. |
| Tile/map data, Screen 2 window, layer enables, latched OAM, sprite selection/priority and transparency | Compose at line L end using current state. Retain the winning identity for every pixel of row L. Later writes never refetch or recomposite that row. |
| Mono palette ports 20–3F | Resolve tile index to a shade identity at composition; retain that shade (all three physical models in mono mode). |
| Mono shade LUT 1C–1F | Translate the retained shade with current LUT at line L+1 end. |
| Color palette RAM FE00–FFFF | Retain palette/index at composition; read RGB12 at line L+1 end for supported 2bpp planar and 4bpp planar/packed modes on Color/Crystal. |
| Backdrop 01 | Compose a shade identity in mono or palette/index in color; later backdrop changes affect newly composed rows only. |
| LCD enable 14 | Apply current enable at translation; disabled output is opaque white, including already pending identities. Composition continues while disabled. |
| Runtime mode 60 | Latch identity type per composed pixel. A pending mono row still uses the shade LUT after switching to color; a pending color row still uses internal palette RAM after switching to mono. New rows use the new mode. |

At each line end, translate the pending preceding row before composing the current
visible row. This happens before the bus advances to the next line. Entry to 144
still requests VBlank/timers and latches OAM at its original clock; row 143's
identities are already fixed before that latch and translate at the end of 144.
Line 0 has no preceding visible row. No fake row is emitted in VBlank, and no
additional device clocks or frames are introduced. Totals 156/158/180 keep all
144 rows and original frame cadence, IRQ/timer positions and instruction overshoot.

For short/odd totals the existing literal T+1-line policy remains. Only rows with
a visited successor line translate. A pending row at actual raster wrap is
discarded, so T<144 updates rows 0 through T−1 and leaves row T and all unvisited
rows unchanged; T=0 translates none. T=144 translates row 143. Odd full-height
totals work identically. A runtime shortening still translates the preceding row
at the current line end, then discards the current pending row if that boundary
wraps; extending before wrap permits the pending row's normal successor. Pending
state never crosses an actual frame wrap. Reset, restart, model changes and
successful ROM replacement clear it. Rejected loads/model changes preserve the
running machine, including pending identities. These policies retain bounded
progress without claiming exact physical behavior at unusual totals/transitions.

Sub-line fetch/write phases, analog panel response, contrast, icons and OAM-copy
accuracy remain outside this focused pipeline. Commercial-title impact is
unmeasured. The implementation uses one bounded row of identities, shared by
native and Wasm execution; browser ROM/save ownership and persistence are unchanged.

## Sequential ROM fetch timing (Issue #119)

The V30MZ fetch, decode and execution stages can overlap. Section 3.3 describes
16 bytes of instruction prefetch capacity. The core now accounts for both bytes
of an aligned 16-bit ROM instruction fetch in one transfer, including the case
where the second byte starts the next sequential instruction. Each transfer
uses the live A0.2 width and A0.3 wait setting. Control transfers, interrupt
entry and ROM mapping writes discard the pending lane. Byte values always come
from the live map. Issue #119 accounted for one word at a time. Issue #132
extends this to overlapping transfers. [V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)

In Final Fantasy II, the last HBlank handler before raster wrap reloads its
one-shot timer and then acknowledges the interrupt. Before the change, one
captured handler reloaded at line 158 clock 246 and acknowledged at line 0
clock 27. The new edge had already arrived when the acknowledgement cleared
it. This dropped a frame of scroll writes during a dialogue-window animation.
An original scheduler test reproduces the race using two ROM bus widths. The
byte-wide handler executes once before losing the next edge, while the word-wide
handler services all 158 line entries before frame wrap.

The local 6,000-frame replays compare the project with Beetle `4b012958` and
Oswan `efea1326`. Starts and buttons are applied at fixed emulated frame numbers.
Colors are normalized to RGB444. The detector flags an isolated frame when at
least 322 pixels change on each side and the two neighboring frames differ over
at most 10% of the smaller change, with an eight-pixel minimum tolerance.

| Replay measurement | Before | After | Beetle | Oswan |
| --- | ---: | ---: | ---: | ---: |
| Final Fantasy II isolated changes | 15 | 0 | 0 | 0 |
| Wild Card isolated changes | 16 | 2 | 1 | 1 |
| Wild Card frames per step in the opening transition | 8 | 6 | 2 | 2 |

The Final Fantasy II runs reach the opening dialogue and battle. The seven
large dialogue flashes had matching static neighboring frames across the
project and both references. All seven flash images were absent from the new
run. Wild Card's transition uses the same sequence of images in each core with
the hold duration shown above. The comparison covers generated frames before
browser presentation. A trial that reused the second lane only at instruction
boundaries left later battle-transition flashes. The retained timing change
also groups the instruction's own bytes. Physical-console timing is unmeasured.

The Final Fantasy II replay taps Start on frames 380 and 800 and A on frames
520 and 675. It then taps A every 90 frames from 2100 through 5430, followed
by Right, Down, Left, Up twice from frame 5500 in 48-frame intervals. Taps last
four emulated frames and directions last 24. Moving the recurring A taps by
−4, +4 and +17 frames produced 14, 16 and 14 isolated changes before the fix
and zero in each revised run. Wild Card taps A on frame 520 and every 130
frames from 970 through 5780.

Five interleaved 6,000-frame native throughput runs put the keypad fixture at
175.932 µs per frame, about 5.6% above the prior core's 166.542 µs and within
9.1% of Oswan's 161.259 µs. Beetle took 179.411 µs. The color/audio fixture
measured 7.717 → 7.632 µs and the Hyper Voice fixture 8.996 → 9.133 µs. The
project's final frame, RAM and PCM hashes matched before and after on the timed
fixtures. The frame and PCM counts matched as well.

## Sprite range capture (Issue #121)

The line-144 snapshot includes the sprite table entries, the seven-bit first
index and the count capped at 128. CPU reads of ports 05 and 06 return the latest
written values. Visible rows use the captured range. A range-only change also
invalidates the cached composition when the snapshot is taken.

In the local Wild Card replay, frame 3868 entered rendering with a previously
captured count of zero. At line 24 the cartridge switched the table base from
2C00 to 2E00 and wrote count 54. The live count exposed part of the old table
for that frame. A second observed transition raised the count from 4 to 58 at
line 95 and briefly drew a stale card. Capturing the range alongside the entries
removed all 14 isolated changes in the 14,000-frame replay.

| Isolated changes across 14,000 frames | Before | After | Beetle | Oswan |
| --- | ---: | ---: | ---: | ---: |
| Wild Card | 14 | 0 | 1 | 1 |
| Final Fantasy II | 0 | 0 | 0 | — |

The extended replays continue the Issue #119 inputs. Wild Card taps A every 137
frames from 6040 through 13849. Final Fantasy II taps A every 93 frames from
6060 through 12198, then holds Right, Down, Left and Up in a loop for 32 frames
each from 12300, with 110 frames between starts. The detector and RGB444
comparison are the same as in the earlier replays. Oswan's capture counts its
video callbacks.

Shifting Wild Card's recurring A taps by −7 or +5 frames produced 13 isolated
changes per run before the fix and zero after. The production Retro CRT player
also replayed frames 3800 through 4000 of the original sequence. Chromium
reported zero isolated changes and zero page errors during that check.

Keeping the snapshot operation off the per-line path brought the native keypad
median to 179.621 µs/frame. The previous core measured 177.219 µs/frame in the
same interleaved run. The three timed fixtures retained matching final frames,
RAM, PCM and frame/sample counts.

Beetle and Oswan both capture the selected range with the entries at line 142.
The project captures that state at its existing line-144 boundary. A probe that
moved only the entry capture to line 142 still reproduced all 14 changes.
[Beetle sprite capture](https://github.com/libretro/beetle-wswan-libretro/blob/4b01295838ea89e3f1355bbe4cb5cf98aa6108cd/mednafen/wswan/gfx.c) · [Oswan sprite capture](https://github.com/gameblabla/oswan/blob/efea1326dc4f0477a6a7b704f7f7f6f5a1cc6f33/main/emu/WS.c)

## ROM prefetch and sprite window timing (Issue #132)

The V30MZ manual describes overlapping instruction fetch, decode and execution,
up to sixteen bytes of prefetched code, and an early fetch at a conditional
branch destination. The core models this overlap as a bounded count of ROM
transfers that can be supplied by earlier internal execution clocks. It applies
to the 16-bit cartridge bus without wait states. Byte values still come from
the live cartridge map. A control transfer drops the previous path and can
supply one word at the destination. External interrupt entry and cartridge
mapping changes invalidate the pending fetches. Ordinary I/O retains separately
accounted timing. This model runs at instruction boundaries. Exact prefetch
phases and arbitration remain unmeasured. [V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)

The renderer captures the sprite window edges (ports 0C–0F) on entry to each
scanline. CPU readback reflects writes immediately. Pixel composition uses the
captured edges for the entire row, and the cache tracks changes to those edges.
[ares sprite window](https://raw.githubusercontent.com/ares-emulator/ares/master/ares/ws/ppu/window.cpp)

The old timing could delay a scanline interrupt into the next line. In Jamil's
opening, the delayed handler left characters visible above dialogue and changed
the shape of the bubble edges. Sampling the edges at line entry fixed the edges
while earlier sprite leakage persisted. Earlier interrupt service from prefetch
overlap fixed the remaining leakage. Four matched opening frames, including
the affected dialogue, agree pixel for pixel in RGB444 with Beetle and Oswan.
In the 7,600-frame Jamil replay, the isolated-change detector found the same
two scripted opening transitions as Beetle. In Albert's opening, the six
previously observed character disappearances are absent and the caption holds
for 164 frames, as in both references. Final Fantasy II and Wild Card produced
zero isolated changes in their 14,000-frame replays.

## Tilemap scroll during the Final Fantasy II battle warp (Issue #134)

The first scripted battle writes the two screens' X and Y offsets in each
HBlank handler. In the trace at visible row 0, the registers enter the row
with Y=$D0, then the handler writes Y=$CF for row 1. The following handlers
decrement Y each row so the scroll points to the same tilemap scanline. Using
each write on its current row selected the previous texture row and left
forest pixels in the margins of the warp. Both screens now use their entry
values for both axes, consistent with the early tilemap fetch and the ares
scanline sampler. [WSdev frame timing](https://ws.nesdev.org/wiki/Frame_timing) · [ares screen sampling](https://raw.githubusercontent.com/ares-emulator/ares/master/ares/ws/ppu/screen.cpp)

In the opening battle replay, the upper and lower black margins advance to
the center. From the first visible warp frame to the first entirely black
frame, the project now takes 41 frames, matching Beetle. The previous core
took 79 frames. Oswan took 66 presented frames and also left forest pixels
in the margins. In the 42-frame interval including the black frame,
31 aligned project frames match Beetle pixel for pixel in RGB444.
The first visible battle screen follows full black by 117 frames in the
project and 91 in Beetle. The register trace puts that later difference
between the end of the warp and the battle's layer enable.
