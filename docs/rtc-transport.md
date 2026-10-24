# Bandai 2003 RTC transport correction (Issue #53)

Source reconciliation recorded on 2026-10-24 before production edits.

The old implementation assumed reset required no host bytes, read terminal READY
represented an extra byte, and supplying READY invalidated a supported command.
Those assumptions are replaced with mapper host counts for commands 10–1B:
`1,1,1,1,7,7,3,3,2,2,2,2`. READY is read-only on command start, so 93 starts 13
with READY cleared. A read's terminal READY contains the final byte with ACTIVE
clear; consuming it clears READY without another scheduled stage. Writes preload
the first byte before command start and only count later READY writes while
ACTIVE is set. Terminal write acknowledgment adds no host byte.

Evidence inspected for this change:

- The [pinned independent mapper experiment](https://github.com/asiekierka/ws-test-suite/blob/5d8950d6b271a585c252af3ced93fbd2045f4752/src/mono/rtc/mapper/main.c)
  specifies the counts above, consumes terminal reads, counts preloaded writes,
  and tests READY clearing after 93. This is inspected experimental source,
  without an inspected physical-console pass log. Its equipped mapper setup
  does not itself demonstrate absent-chip behavior; using these mapper-controlled
  counts for an absent RTC is an explicit architectural inference.
- The [pinned libws routines](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/b3f2366c458421775cc4a18b4925d12a8d9c058b/libws/src/cart/cart_rtc.c)
  read READY data before checking ACTIVE, and preload the first write. The tests
  use independently written bounded polling and literal expectations; no
  external source code or binary is incorporated.
- The [pinned register definitions](https://github.com/WonderfulToolchain/hardware-definitions/blob/46783132a59e576c11456f5997620594eca1c390/target/wswan/cart/rtc.xml)
  identify CA control, CB data, READY bit 7 and ACTIVE bit 4.
- The [S-3511A manufacturer manual, section 2, table 2](https://www.rockby.com.au/DSheets/10110.pdf)
  makes reset's R/W bit irrelevant. Chip reset semantics and mapper host counting
  are distinct: counting one reset host byte does not make it writable chip data.
  Commands 1A/1B retain the existing bounded mapper transport; this does not
  establish an additional chip command or expand uncertain chip functionality.

The existing 64-clock scheduled-stage approximation is retained, not established
silicon latency. On reads, a scheduled stage makes one byte available and updates
ACTIVE; on writes it acknowledges the previously supplied byte. Unsupported
commands still fail explicitly after masking only READY. Mapper availability,
command readback and absent-chip FF data are retained.

Historically, Issue #53 did not implement RTC attachment/detection, date/calendar
progression, RTC persistence, alarms or cartridge IRQ integration. No physical
console measurements, human gameplay or commercial compatibility are claimed.
Issue #63 adds an [optional retained calendar](rtc-calendar.md), with the original
FF vectors preserved under explicit absent configuration.
See [executed validation](issue-53-validation.md) and
[original fixture provenance](../fixtures/README.md#rtc-transport-probe-issue-53).
