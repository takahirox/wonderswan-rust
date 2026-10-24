# Successful AAM/AAD flags (Issue #55)

The old decoder updated SZP alone for both conversions. This retained incoming
AF/CF/OF after AAM and omitted the final addition's AF/CF/OF after AAD. The
corrected rules for ordinary two-byte decoding are:

| Instruction | Result | Arithmetic FLAGS |
| --- | --- | --- |
| AAM, nonzero immediate base | AH = original AL / base; AL = original AL % base; incoming AH ignored | Clear AF/CF/OF; derive SF/ZF/PF from remainder AL |
| AAD, any immediate base including zero | AL = original AL + ((original AH × base) modulo 256), modulo 256; AH = 0 | All six flags come from that final byte ADD |

Control/fixed bits are preserved, including incoming IF. Execution components
remain 16 for AAM and 1 for AAD, plus the existing fetch accounting. Neither
handler advances devices directly. The zero-base AAM fault path is unchanged.

The instruction-specific evidence is the pinned independent
[WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#aam--cvtbd-88)
and [AAD experiment/calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5007):
its expected product accumulates in a byte before `calcAadResult` uses byte ADD
and captures FLAGS (lines 5105–5114). The
[AAM experiment](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5920)
compares complete results/FLAGS with contrasting incoming arithmetic flags and
varies incoming AH. Its successful calculator (lines 6024–6060) uses remainder
SZP. The calculator's F202 includes the experiment's IF; it is not an
unconditional emulator constant. Raw report and assembly were inspected; no
external implementation code was copied into the original CC0 fixture.

The [NEC V30MZ preliminary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§1.3 (printed p. 15), distinguishes the immediate second byte of CVTDB/CVTBD.
Section 3.1.5 (printed pp. 26–28) defines the ordinary byte-add carry, auxiliary
carry, signed overflow and SZP rules used after product truncation. It does not
independently establish all console-specific conversion flags. The report's
exceptional AAM ZF prose contradicts its calculator, so those disputed fault
semantics remain separate work and are not used as successful-conversion evidence.

This corrects the prior SZP-only assumption without changing the Issue's agreed
requirements. Applying these CPU rules to Mono/Color/Crystal and across graphics
modes is an instruction-level inference from the experiment sources, not an
independently inspected set of per-model silicon logs. Physical-console tests,
commercial-title impact, broader exceptional arithmetic and exact prefetch timing
remain unmeasured. The original [fixture and provenance](../fixtures/README.md#conversion-probe-issues-55-and-57)
and [actual validation record](issue-55-validation.md) describe automated evidence.
