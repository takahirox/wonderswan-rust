# AAA/AAS results, flags and clocks (Issue #57)

The old decoder adjusted the whole AX by 0106, then masked AL. A carry or borrow
from AL could change AH a second time. It updated AF/CF alone and retained the
default one-clock execution component. The corrected one-byte instructions use
this sequence on every emulated model:

1. Determine adjustment from incoming AF or the original AL low nibble > 9.
2. Mask AL to its low nibble.
3. If adjusted, add six to AL and increment AH for AAA (37), or subtract six
   from AL and decrement AH for AAS (3F). Each byte wraps independently.
4. Retain only AL's low nibble. Set PF, clear OF, set AF/CF/ZF to the adjustment
   condition and SF to its inverse. Preserve control/fixed flags.

These are instruction-specific flags, independent of the result's sign, zero
and parity. For example adjusted AAA 120B → 1301 sets ZF/PF and clears SF;
unadjusted AB00 → AB00 clears ZF and sets SF/PF. With incoming F002 the complete
FLAGS are F057 when adjusted and F086 otherwise. Incoming IF/TF/DF are preserved.

## Sources and reconciliation

The pinned independent [WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#aaa--adjba)
describes both instructions' flags. Its [AAA calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L6506)
and [AAS calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L6641)
mask AL first and adjust AL/AH separately. Their expected FLAGS include the
experiment's incoming IF: copying their F206 constant unconditionally would
incorrectly change emulator control state. The raw pinned report and assembly
were inspected; no external implementation source is included in the CC0 probe.

The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Appendix A, Table A-1 (printed pp. 73–74), lists nine execution clocks for ADJBA
and ADJBS, with decoding completed, no waits/hold requests and the other stated
conditions. Both handlers now return nine execution clocks plus existing fetch
charges: ten total for unprefixed RAM/fast ROM, eleven for waited ROM. Prefix
fetches retain their current costs. Only Machine scheduling advances devices.
This is the repository's bounded timing policy, not an exact prefetch/pipeline model.

This replaces the whole-word, AF/CF-only and default-cost assumptions. The Issue
already requires the corrected behavior, so no specification or remote edit was
needed. DAA/DAS are covered by the subsequent [Issue #59 correction](decimal-adjust.md);
exceptional division and prefetch redesign remain separate work.
Applying these CPU rules across Mono/Color/Crystal and supported graphics modes
is an instruction-level inference. No independent per-model silicon logs were
inspected; physical-console measurements, human gameplay and commercial-title
impact remain unmeasured.

## Independent expected tables and vectors

The exhaustive native oracle uses these literal nibble tables, without production
adjustment/ALU helpers. The high nibble of original AL has no effect. A suffix `+`
means increment AH, `−` means decrement AH; entries without a suffix preserve AH.
Suffixed entries have arithmetic FLAGS 0055, others 0084. These include all six
arithmetic flags, with OF clear and PF set.

| Original low nibble | AAA, AF=0 | AAA, AF=1 | AAS, AF=0 | AAS, AF=1 |
| --- | --- | --- | --- | --- |
| 0 | 0 | 6+ | 0 | A− |
| 1 | 1 | 7+ | 1 | B− |
| 2 | 2 | 8+ | 2 | C− |
| 3 | 3 | 9+ | 3 | D− |
| 4 | 4 | A+ | 4 | E− |
| 5 | 5 | B+ | 5 | F− |
| 6 | 6 | C+ | 6 | 0− |
| 7 | 7 | D+ | 7 | 1− |
| 8 | 8 | E+ | 8 | 2− |
| 9 | 9 | F+ | 9 | 3− |
| A | 0+ | 0+ | 4− | 4− |
| B | 1+ | 1+ | 5− | 5− |
| C | 2+ | 2+ | 6− | 6− |
| D | 3+ | 3+ | 7− | 7− |
| E | 4+ | 4+ | 8− | 8− |
| F | 5+ | 5+ | 9− | 9− |

Independent AH rows for inputs 00/01/FE/FF are 01/02/FF/00 when incremented
and FF/00/FD/FE when decremented. The native test combines the literal tables
with all 256 incoming AL values, both AF states, these four AH inputs and two
opposing arithmetic/control profiles on all nine static model/mode configurations
(73,728 decoded instructions). Separate literal AX/full-FLAGS vectors cover all
Issue reproductions, adjusted nonzero AL, unadjusted zero AL and byte wrapping;
these also check full unrelated state, continuation and fetch/wait charges.

The extended original [conversion probe](../fixtures/README.md#conversion-probe-issues-55-57-and-59)
records actual AX, full PUSHF and five Jcc observations against independent
literals on every model, all supported runtime round trips and fresh restart.
The [validation record](issue-57-validation.md) distinguishes executed evidence
from hardware/commercial uncertainties.
