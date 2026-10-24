# V30MZ multiplication flags (Issue #43)

The [NEC V30MZ preliminary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§3.1.5, printed pp. 27–28, leaves AF/PF/SF/ZF undefined for multiplication.
The old implementation implicitly interpreted that as preservation. Undefined
values do not establish preservation. The console-specific rule comes from the
independent [WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md)
and its [assembly](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm):

| Ordinary decoded instructions | AF/PF/SF | ZF | CF/OF |
| --- | --- | --- | --- |
| Unsigned F6/F7 /4 | Clear | Clear on WonderSwan; set on Color/Crystal | Unsigned product exceeds destination width |
| Signed F6/F7 /5, 69, 6B | Clear | Set on every model | Product exceeds signed destination range |

The source selects unsigned flags from physical-model identification (port A0
bit 1, lines 64–70), then compares products and full flags under opposing incoming
status patterns (from line 4346). Graphics-mode changes do not change the SOC.
The prior assessment's unresolved immediate-IMUL prerequisite is also corrected:
the pinned source includes both immediate-word and sign-extended-byte tests
(from lines 4832 and 4953), using the same signed full-flag expectations.
These two corrected assumptions do not weaken earlier acceptance.

The report/source is an independent hardware experiment reference. No physical
execution or per-model measurement log was independently inspected. These rules
are bounded to multiplication on the three WonderSwan models; they do not settle
division quirks, other undefined arithmetic flags, single-step delivery, absolute
instruction timing or commercial-title compatibility. Existing multiplication
products, destinations, CF/OF rules, control flags, bus accounting and shared-clock
scheduling are retained. No external implementation code is copied.

The original CC0 probe and independent native/browser assertions are described
in [fixture provenance](../fixtures/README.md#multiplication-probe-issue-43).
[Validation evidence](issue-43-validation.md) separates baseline discrimination,
checks actually executed, final review and optional unperformed hardware checks.

Issue #65 also retains each explicit multiplication's overflow in private CPU
history for [successful byte DIV](byte-div-flags.md). Byte IMUL is directly
demonstrated by that experiment; extending history production to the other
explicit forms is an instruction-level inference. Live multiplication flags and
products are unchanged. No claim is made about implicit conversion microcode.

Issue #85 corrects the former 12/20 (immediate 12) residual-clock assumption to
three for every explicit form, with transfers charged once. See the
[multiplication accounting rationale](multiplication-timing.md); products,
complete FLAGS, sampling and history behavior above remain required.
