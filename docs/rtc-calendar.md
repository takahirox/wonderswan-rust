# Retained cartridge RTC (Issue #63)

Implementation policy recorded before production changes: reads snapshot the
complete selected register at command start. Writes stage bytes and commit only
at the final scheduled acknowledgment; a replacement command, restart, model
change or successful import cancels incomplete writes. Reset acts at its first
scheduled stage for either direction; the mapper's counted host byte is not a
chip payload. These phases and the retained 64-clock stage delay are explicit
emulation policies, without console measurements.

The manufacturer S-3511A manual sections 2–4 governs register behavior. Pinned
libws definitions and routines referenced by Issue #63 govern host payload order,
AM/PM mask and polling. Calendar values use BCD; 12-hour midnight/noon are 00
AM/PM. The PM flag is also returned in 24-hour mode. Invalid seconds are retained
until the next second carry, which corrects them to zero and carries into minutes.
Other invalid fields correct on completed write; impossible month days become
the first of the next month without changing the independently written weekday.

RTC attachment and mapper capability are independent. Raw footer 01 defaults to
an equipped RTC and the existing extended banking policy; 00 defaults to common
banking with no RTC. This supported producer convention is not universal retail
board detection. Explicit absent configuration leaves the mapper available.
Existing SRAM size codes and raw ROM bytes/checksum remain unchanged.

## Clock ownership and retention

`Machine::sync_rtc(now_ms)` injects an absolute integer millisecond clock. The first
sample establishes an anchor without changing calendar values. Later samples
advance only past the previous high-water mark. Backward or repeated samples
neither advance nor move that mark, preventing replay after clock adjustment.
The core never reads the OS clock, and CPU/device/frame execution never advances
the calendar. Frontends must inject this clock; native callers can use controlled
samples. A completed calendar write or chip reset restarts the subsecond phase
but retains the host anchor. Fractional milliseconds within a second survive
partitioned synchronization, export/import and ordinary restart/model changes.

The browser injects `Date.now()` before executing frames, on its independent
one-second persistence timer and on lifecycle flushes. Pause, tab suspension and
reopen therefore catch up without executing corresponding game frames. The
calendar intentionally starts at the chip's cold state, not the host's local
calendar date. There is no timezone/DST conversion. Two-digit years repeat the
2000–2099 leap-year cycle after 99; 00 is again a leap year. Catch-up reduces full
100-year cycles arithmetically and walks at most 36,524 days; weekday advances
by the full elapsed days modulo seven. Even maximum u64 elapsed milliseconds
have bounded work, without discarding elapsed intervals.

`restart` and compatible `change_model` retain completed chip registers and time
anchor via cartridge ownership, cancelling all transport. Full `reset` unloads
the cartridge. Successful ROM loads create a new cold chip; failed loads preserve
the running state. `set_rtc_present(false)` explicitly configures an absent chip
without modifying ROM bytes, mapper capability, checksum or saves. Reattaching
creates a cold chip; frontends may restore a previously exported record.

## Export and browser storage

`rtc_data()` returns empty for absent/unloaded cartridges, otherwise a 25-byte
record: ASCII RTC plus version 01; binary year/month/day/weekday/hour/minute;
raw masked BCD seconds; status; two INT bytes in host order; little-endian u16
subsecond milliseconds; anchor-present byte; little-endian u64 host anchor.
`load_rtc_data()` validates version, exact length, calendar ranges, status reserved
bits, fraction and anchor flag before replacing anything. Invalid seconds remain
representable because their correction is delayed. Failed imports leave retained
state and transport intact; successful imports cancel transport.

The Wasm adapter exposes the same state and validates finite, nonnegative,
JavaScript-safe integer millisecond samples. `configure_rtc("auto"|"absent")`
and the player's Cartridge clock setting select attachment independently of
banking. The setting can be chosen before load or changed during play.

Browser key `wonderswan:rtc:v1:<raw-ROM-SHA-256>` contains a JSON envelope
`{version:1,data:<base64-core-record>}`. Neither model nor filename selects this
slot. It is independent of cartridge SRAM/EEPROM, console EEPROM and audio
preferences; none of their formats/capacities change. Restore precedes first CPU
execution. Decode, version, storage-access or core-import failures block writes
to that slot for the session and show a clock-specific notice. Absent mode never
reads or writes the equipped clock slot. Failed ROM validation never restores or
replaces the target clock record. Storage write failures report without stopping
execution or changing existing data.

## References, correction and remaining gaps

The old footer-byte-13-as-universal-mapper interpretation is corrected: the
[pinned swanlink producer](https://github.com/WonderfulToolchain/wonderful-i8086/blob/a77672a50369b0dfd6f646a8deedbf77c114c04e/support/swanlink.py)
sets bit 8 of the two-byte flags from `--rtc`. The
[original-board inventory](https://www.yaronet.com/topics/161842-hardware-type-de-pcb-cartouches-wonderswan)
records both populated and unpopulated RTCs on 2003 boards. The compatibility
policy still selects existing common/extended banking from raw 00/01, while
attachment is explicit and independent. This correction is already in Issue #63;
no remote Issue edit or approval is necessary or authorized by this node.

Register semantics follow the [manufacturer manual, sections 2–4](https://abcelectronique.org/composants/telechargement_datasheet.php?id=836836&part-number=S-3511ASeries),
with [pinned libws payload masks/order](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/b3f2366c458421775cc4a18b4925d12a8d9c058b/libws/include/ws/cart/rtc.h)
and [polling routines](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/b3f2366c458421775cc4a18b4925d12a8d9c058b/libws/src/cart/cart_rtc.c).
Cold status is 82 and INT is 8000; explicit reset clears both. POWER cannot be
written and clears on chip reset. Voltage-drop waveforms are not simulated.
Status writes retain only 6A writable bits and the existing POWER flag.
Both alarm/frequency setting commands retain the single write-only INT register;
read-direction mapper transfers retain FF pull-up policy. Alarm/periodic output,
cartridge IRQ wiring and test-mode circuitry remain unsupported. No universal
retail detection, measured transport/snapshot/interrupted-write phase, physical
console pass, human gameplay or broad commercial readiness is claimed.
