# Shift and rotate flag correction (Issue #45)

Baseline `90ef271b149b95e3143d9459324d4820b8403d77` retained AF for
shifts and OF except for a masked count of one. Its zero-count shift SZP
calculation was already correct. Retention was an unsupported assumption.

The [NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§3.1.5, describes one-bit overflow and leaves multi-bit overflow and auxiliary
carry unspecified. Unspecified does not establish preservation. The pinned
[WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md)
provides the console-specific requirements: five-bit counts; CF retained only
when the masked count is zero; shifts clear AF and calculate SZP even at zero;
rotates preserve AF/SZP. At every masked count, left OF is final CF XOR result
MSB, and right OF is the XOR of the two highest result bits.

The [assembly experiments](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L2935)
compare operands and full flags with contrasting incoming flags. Inspected
excerpts include byte ROL and SAR and word RCR, with CL counts and explicit
masked-zero handling. The issue's inspected range covers byte operations and
word carry rotations; the report explicitly describes bits 7/15. Applying the
same semantics to other documented word and implicit-one/immediate encodings
is an inference from that report and the common operation, not a claim of
independently measured coverage of every encoding.

The report identifies unsigned multiplication ZF as the SOC difference; it
reports no shift/rotate model distinction. These rules apply to physical
WonderSwan, WonderSwan Color and SwanCrystal independently of video mode.
No physical execution or per-model measurement log was independently inspected.
Tests of these emulated models are not physical-console measurements. Commercial
title impact remains unmeasured. No external implementation or test code is
copied. Undocumented /6, division, single-step delivery and absolute timing
remain outside this correction. No game-specific behavior is introduced.

This record was written before production behavior changed. See
[executed validation](issue-45-validation.md) for baseline failures, corrected
checks, original CC0 probe evidence and acceptance review.
