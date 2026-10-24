# Jcc and JCXZ timing (Issue #75)

## Independent oracle recorded before the execution-cost edit

The old AI assumption kept taken odd-target Jcc/JCXZ at six internal clocks.
Replacement: all 70–7F conditions and E3 use four decoded clocks when taken,
one otherwise, plus one only for a taken odd destination. Primary source:
[NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A conditions/Note 5 and Table A-1, printed pp. 73–76.
The two actual byte fetches remain separate. Internal totals are **6 even,
7 odd, 3 fallthrough**, regardless of opcode parity. These are serialized
emulator expectations, not physical-console measurements. No destination read,
fetch consolidation, external-bus policy or prefetch implementation is added.
[Direct CALL](call-timing.md) now uses its reconciled residual execution costs;
indirect CALL/JMP and other instruction execution costs remain separate.

Before production costs change, literal state vectors at IP 1000 with
00/02/03/FC/FD/80/7F displacements give taken destinations
1002/1004/1005/0FFE/0FFF/0F82/1081. At IP 1001 they give
1003/1005/1006/0FFF/1000/0F83/1082. Fallthrough is 1002/1003.
At IP FFFF, FE/FF/02/03 give FFFF/0000/0003/0004; the displacement
fetch wraps within CS. FFFF:000F fetches physical FFFFF then 00000.
Bank crossing 2800:7FFF fetches 2FFFF then 30000; next IP is 8001.
Full FLAGS, all registers (including CX for JCXZ), CS/other segments,
stack, memory and ports must be preserved. JCXZ is taken only for CX=0000;
0001/0002/FFFF fall through independently of FLAGS.

Jcc truth vectors enumerate all 32 CF/PF/ZF/SF/OF combinations. Literal
32-bit masks specify each opcode's outcomes, with bit index C+2P+4Z+8S+16O.
Other FLAGS bits are tested both clear and set, including reserved bits.
Mapped ROM byte lane clocks for A0 00/04/08/0C are 1/1/2/2; two-byte
fetch totals are 2/2/4/4. A FFFFF→00000 split is 2/2/3/3. Each of zero
through seven segment/BUSLOCK prefixes adds its actual byte lane charge;
REP remains rejected. Opcode parity introduces no new charge.

## Original CC0 absolute cartridge interval

`fixtures/jcc-timing.asm` copies six bodies to RAM 2000. Samples are JZ
taken-even, taken-odd, fallthrough, then the same three JCXZ paths. CX is
0/0/1, SI is the independent 4096-iteration counter. Each iteration executes
LEA BX,[BX+1] (4), CMP CX,0 (4), tested branch (6/7/3), LEA BP,[BP+1]
(4), DEC SI (2), and JNZ to even 2004 (6, final 3). Tested opcode is 200A;
taken targets are 2010/2011 over poison HLTs. Fallthrough reaches 200C;
its unused displacement is +3 and would skip BP's increment if wrongly taken.
BX and BP independently count execution and reached destinations. CX must
remain 0/0/1; final SI=0 is required to reach the continuation marker.
Initial IN AX,A8 contributes 8 clocks and MOV DX,AX 3; terminal IN observes
the timer before advancing its own clocks. Publication and RETF are outside.

| Path (either instruction) | Independent clocks | 256-clock HBlank ticks |
| --- | ---: | ---: |
| Taken even | 11 + 4096×26 − 3 = 106504 | 416–417 |
| Taken odd | 11 + 4096×27 − 3 = 110600 | 432–433 |
| Fallthrough | 11 + 4096×23 − 3 = 94216 | 368–369 |

Only unknown initial timer phase permits floor/ceil quantization. Native and
independent browser consumers require absolute bounds, BX=BP=4096, unchanged
CX and continuation 51A7. Relative parity differences alone cannot pass.
Mono runs 00; Color/Crystal run 00/80/00/C0/00/E0/00. Result layout matches
the JMP probe: completion, fresh boot count, capability, pass count, then six
five-word samples per pass (ticks, BX, BP, CX, marker). All 512 result bytes
are cleared and mirrored to SRAM; retained completion is invalidated before
sampling. Poisoned saves and fresh restart are required in both consumers.

## Affected existing IRQ oracle

Unchanged IRQ handler bytes at RAM 200F are 43 4E 75 04 B0 80 E6 B6 CF.
JNZ targets odd 2017 on 1023 taken iterations. Add 1023 to the previous
61459/67603 clocks: **62482/68626**, requiring **244–245/268–269** ticks.
Former IRQ execution 12 rather than 27 would produce **47122/53266** clocks,
**184–185/208–209** ticks. Keep internal IRQ entry 32/35 and counts 1024/0/0.
The JMP and LOOP probes' measured JNZ targets are even, so their absolute
budgets, counts, CX and continuation bounds remain unchanged. Historical
validation observations remain historical; current observations belong in
[Issue #75 validation](issue-75-validation.md).
