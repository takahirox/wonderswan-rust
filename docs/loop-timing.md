# LOOP timing (Issue #71)

## Accounting and literal vectors recorded before the CPU change

The old AI assumption shared 4/1 decoded execution clocks across E0–E2,
producing internal 6/3 totals and ignoring target parity. Replace it with the
instruction-specific costs in the [NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Appendix A conditions, Note 5 and Table A-1 (printed pp. 73, 77): DBNZ (E2)
uses 5/2 decoded clocks; DBNZE/DBNZNE (E1/E0) use 6/3, taken/fallthrough.
Taken odd destinations add one clock. These register instructions have no
operand transfer to subtract. This corrects the earlier assumption using the
primary table, without changing the accepted hardware goal.

The current serialized CPU explicitly fetches opcode and displacement as two
bytes. Charge these actual fetches once, then execution; no destination fetch is
performed by the branch. These are emulator step expectations, not measurements
of the console pipeline. Full prefetch and external-bus policy remain separate.

| Opcode | Taken even | Taken odd | Fallthrough (either destination parity) |
| --- | ---: | ---: | ---: |
| E2 LOOP | 7 = 2 + 5 | 8 = 2 + 5 + 1 | 4 = 2 + 2 |
| E1 LOOPE | 8 = 2 + 6 | 9 = 2 + 6 + 1 | 5 = 2 + 3 |
| E0 LOOPNE | 8 = 2 + 6 | 9 = 2 + 6 + 1 | 5 = 2 + 3 |

Independent architectural vectors: incoming CX 0000/0001/0002/FFFF becomes
FFFF/0000/0001/FFFE. E2 takes all except incoming 1, regardless of ZF. E1
additionally requires ZF=1, E0 ZF=0. Full FLAGS and every other register and
memory byte stay unchanged. At IP 1000, displacements 02/03/FC/FD produce
1004/1005/0FFE/0FFF when taken, otherwise 1002. At IP FFFF the displacement
fetch wraps to CS:0000; FE/FF/02/03 produce FFFF/0000/0003/0004. CS is unchanged.

Mapped ROM fetch vectors use live A0 controls 00/04/08/0C: two distinct byte
fetches cost 2/2/4/4 regardless of ROM word width. E2 taken-even totals therefore
7/7/9/9; conditional taken-even 8/8/10/10. Mode/SRAM wait controls do not add
waits to internal fetches or register-only execution. Prefix bytes retain their
ordinary fetch charges; REP rejection and E3 JCXZ retain existing behavior.
Interrupt entry (32/35 internal), TF entry, DMA stalls and device advancement
remain outside the instruction execution component and must each occur once.

## Original absolute cartridge oracle

The CC0 `fixtures/loop-timing.asm` copies its measured bodies to RAM 2000 on
every model. Every pass samples E2, E1 and E0 at both even and odd taken targets,
then repeatedly samples their fallthrough paths. Color/Crystal transition
00→80→00→C0→00→E0→00; Mono uses 00. No interrupt is enabled during measurement.

Odd bodies insert a NOP (2 clocks) before the target to shift it from 2004
to 2005; it executes once inside the measured interval.
Taken bodies perform 4096 three-byte LEA BX,[BX+1] instructions (4 clocks each),
4095 taken LOOP instructions and one fallthrough. Flags are established outside
the interval and LEA preserves them. The interval includes initial IN AX,A8
(8 clocks) and MOV DX,AX (3); the final IN observation and publication are outside.

| Sample | Independent clocks | HBlank ticks (256 clocks, unknown phase) |
| --- | ---: | ---: |
| E2 even | 11 + 4096×4 + 4095×7 + 4 = 45064 | 176–177 |
| E2 odd | 13 + 4096×4 + 4095×8 + 4 = 49161 | 192–193 |
| E1/E0 even | 11 + 4096×4 + 4095×8 + 5 = 49160 | 192–193 |
| E1/E0 odd | 13 + 4096×4 + 4095×9 + 5 = 53257 | 208–209 |

Fallthrough bodies repeat MOV CX,imm (4), LEA BX,[BX+1] (4), TEST BP,BP
(3), tested LOOP (4 for E2, 5 otherwise), DEC SI (2), JNZ (6, final 3).
Thus E2 costs 11 + 4096×23 − 3 = 94216 (368–369 ticks); E1/E0 cost
98312 (384–385 ticks). E2 starts CX=1; conditional early stops start CX=2
with the opposite ZF. Unexpected taken branches reach a failure continuation.
These absolute ranges are derived independently of production timing helpers;
no tolerance is widened to accept an old implementation.

RAM 0400 contains completion 600D, boot count, model capability 0/2 and pass
count 1/7. Each pass has nine four-word samples: ticks, independent LEA count
4096, final CX (0 for taken/E2 fallthrough, 1 for conditional early stops), and
continuation marker 51A7. All 512 bytes are mirrored to SRAM. Startup invalidates
retained success before measurement; consumers poison saves and restart freshly.

The unchanged IRQ fixture's odd LOOP target 2005 adds 1023×2 + 1 = 2047 clocks:
61459/67603 clocks, or 240–241/264–265 ticks. See
[external IRQ timing](external-irq-timing.md) for the updated independent sum.
Historical Issue #69 observations remain historical.

Actual negative controls, native/browser observations, artifact identities,
checks and unperformed optional experiments are recorded in
[Issue #71 validation](issue-71-validation.md).
