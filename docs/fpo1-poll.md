# FPO1 and POLL console profile (Issue #41)

Source reconciliation recorded before implementation, 2026-10-24.
The earlier assessment in Issue #35 treated FPO1 byte consumption and console
POLL applicability as unresolved prerequisites. Issue #41 corrects that policy:
D8–DF consume exactly one following byte and do nothing else; 9B completes
without waiting or exception on WonderSwan, WonderSwan Color and SwanCrystal.

The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§§1.3/3.6, describes FPO1 as NOP. Appendix A nevertheless depicts memory and
displacement fields; that table alone does not settle console byte consumption.
The [pinned WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md)
explicitly reports two-byte NOPs for all eight opcodes and nonblocking POLL.
Its [experiment source](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L7909)
checks POLL followed by ADC; FPO1 cases at line 8643 use C0 as the second byte.
The broader 256-byte vectors implement the report's stated rule; they are not
additional physical observations. [Pinned ares decoding](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/component/processor/v30mz/instructions-misc.cpp)
corroborates one following-byte consumption, without proving hardware behavior.
No external implementation code is copied.

The manual's §2.3.1 POLLB description continues execution when the input is low.
We use an effective POLLB-low console profile, without adding an external-device
input. The report author's permanently-low wiring explanation is a conjecture,
not verified electrical wiring. No physical execution or per-model measurement
log was inspected. Per-model silicon differences, electrical pin behavior,
full prefetch and absolute timing remain unverified; commercial-title impact
is unmeasured. Single-step delivery and unrelated unsupported encodings remain
separate gaps.

Appendix A supplies execution components of one clock for FPO1 and nine for
nonwaiting POLL. Within the existing additive approximation, each consumed byte
is fetched independently through the mapped CPU byte path. Thus internal-RAM
fetch costs are two and one respectively, giving approximate totals three and
ten before prefixes. Live ROM waits apply per byte; word-bus width does not
merge these independent byte fetches. These totals are not measured physical
instruction timing. IP and 20-bit address wrapping use the existing helpers.
The ordinary step returns execution plus fetch cost for shared-clock devices to
advance once; DMA retains its independent boundary/stall accounting.
