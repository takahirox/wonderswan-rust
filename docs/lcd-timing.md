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
| Tile/map data, scroll, windows, layer enables, latched OAM, sprite selection/priority and transparency | Compose at line L end using current state; retain the winning identity for every pixel of row L. Later writes never refetch or recomposite that row. |
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
