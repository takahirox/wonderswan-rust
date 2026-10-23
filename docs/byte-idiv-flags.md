# Successful byte IDIV flags (Issue #49)

At baseline `f9464ad5f50dfc5f362fe6c1ea0cf3846394690b`, successful F6 /7
cleared only OF. Retaining the other incoming arithmetic flags was an
unsupported assumption. The corrected rule clears AF, CF and OF, sets SF from
quotient AL bit 7, ZF when AL is zero, and PF for even parity of AL. Unrelated
FLAGS, including caller TF/IF/DF, are preserved. Remainder AH does not
participate: AX=0001 divided by 02 produces AX=0100 and FLAGS F046 from F002;
AX=FFF9 divided by 02 produces AX=FFFD and FLAGS F082.

The change is inside ordinary successful byte-signed decoding. Operand reads
still precede AL/AH writeback, including accumulator divisor aliases. It applies
to emulated Mono, Color and SwanCrystal, independently of graphics mode. Byte
DIV, word DIV/IDIV, quotient/remainder calculation, divide-fault classification,
continuation, access charges and the existing 24-clock execution component are
unchanged. This accounting assertion does not establish absolute hardware timing.

## Sources and reconciliation

The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§3.1.5 (printed pages 27–29), specifies cleared division overflow and leaves
other division status flags unspecified. This does not establish retention.
The valid OF requirement remains in force.

The [WSCPUTest report pinned at
245b12c06dede4a2bb9c04c942ebca2044ba4930](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#idiv--div-signed-division-168)
reports the successful byte-signed rule. Its [register experiment and
calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5287)
compare actual full FLAGS under opposing incoming patterns, and calculate
successful SZP by indexing the low-byte table with AL. The calculator's F202
base contains its incoming IF: the emulator preserves caller control bits
instead of imposing F202. Its magnitude bound sends 128 to the exceptional
path. New successful boundary vectors therefore use ±127; the existing
emulator's −128 classification is preserved without claiming hardware accuracy.

These sources were inspected for this correction. The register experiments
are inspected source evidence, without independently inspected per-model
measurement logs. Applicability to mapped operands and graphics modes is an
instruction-level inference, supported by the manual's common operand forms
and the report's family scope. No physical-console experiment was performed.
The added test vectors and cartridge assembly are original CC0 work; no external
source or cartridge bytes were copied. This corrects the implementation's
assumption without changing the Issue's goal or relevant acceptance.

## Bounded automated acceptance

Fifteen independent literal AX/full-FLAGS/branch vectors run with incoming
F002/FFD7, covering opposite arithmetic and TF/IF/DF bits, zero, signed
quotients/remainders, all operand-sign combinations, exact/inexact division,
parity differences and ±127. Five static model/mode combinations and seven
sources (BL and even/odd RAM, ROM and SRAM with ES overriding a distinct DS)
make 1,050 decoded executions. Expectations never call production flag helpers.
They check complete registers, segments, memory, saves, ports, exact IP and
following MOV/JZ/JC/JO/JS/JP/JL decisions, and unchanged access/execution costs.
Separate successful AL/AH aliases check sampling before writeback. Native
TF assertions cover bit preservation; single-step interrupt delivery remains
outside this change.

Live mono → planar 2bpp (80) → mono → planar 4bpp (C0) → mono → packed 4bpp
(E0) → mono transitions run on Color/Crystal, with Mono remaining in its
supported mode. The runtime test reads back each selected mode; the static
matrix and cartridge retain their mono/planar-2bpp coverage. A normal
`Machine::run_frame` positioned one byte IDIV before frame end advances shared
devices by the returned 26 clocks exactly once (24 execution plus two fetches),
wraps one frame, produces one stereo audio sample, and stops at the next opcode.

The extended original `fixtures/divide.asm` keeps all 52 word operations and
adds 30 byte-IDIV operations per mode pass. It records actual AX, unchanged DX,
full FLAGS and Z/C/O/S/P/signed-LT branch observations. Mono runs one pass;
Color/Crystal run mono → color → mono. Native and production Chromium/Wasm
compare all 988 words against literals on each model and fresh restart. Each
boot clears volatile records, invalidates saved completion and increments a
fresh boot marker; poisoned stale observations cannot satisfy acceptance.
See [fixture provenance](../fixtures/README.md#successful-division-probe-issues-47-and-49)
and [actual commands and acceptance review](issue-49-validation.md).

Byte DIV multiplication history, signed exceptional results, fault flags and
classification, decimal adjustment and absolute timing remain unresolved
separate work. Automated emulator evidence does not establish physical-device
accuracy, commercial-title impact or commercial-catalog compatibility.
