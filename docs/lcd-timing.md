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
