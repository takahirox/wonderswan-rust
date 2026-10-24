# Direct JMP timing (Issue #73)

## Independent oracle recorded before the execution-cost edit

Old AI assumption: default one-clock execution suffices for E9/EA/EB.
Replacement: E9 near and EB short use four decoded execution clocks; EA far
uses seven. Add one for an odd destination IP. Source: [NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Appendix A conditions and Note 5, printed p. 73, Table A-1 printed p. 76
(document A13761EJ1V1UM00). These forms perform no operand-memory transfers
to subtract. This corrects the old assumption without reducing acceptance.

Actual opcode/immediate fetches remain separate and charged once, including
live waits. No destination instruction is read by JMP. Preserve Cpu::word and
Bus::word_transfer consolidation. Independent internal-RAM unprefixed totals:

| Opcode | Even opcode / even target | Odd opcode / even target | Odd target increment |
| --- | ---: | ---: | ---: |
| EB | 6 (2+4) | 6 (2+4) | 1 |
| E9 | 7 (3+4) | 6 (2+4) | 1 |
| EA | 12 (5+7) | 10 (3+7) | 1 |

Mapped ROM fetch clocks at A0 00/04/08/0C, for even/odd opcode:
EB 2/2/4/4 either parity; E9 3/3/6/6 versus 3/2/6/4;
EA 5/5/10/10 versus 5/3/10/6. Here the first sequence in each pair
is even opcode IP. Add execution 4/4/7 and target parity separately.
Prefixes fetch one byte apiece and change immediate alignment: at even start,
counts 0..7 produce E9 fetches 3/3/5/5/7/7/9/9 and EA
5/4/7/6/9/8/11/10; EB fetches 2..9. Target parity is independent.

State oracle: IP 1000 short displacements 00/02/03/FC/FD/80/7F
produce 1002/1004/1005/0FFE/0FFF/0F82/1081. Near displacements
0000/0002/0003/FFFC/FFFD/8000/7FFF produce
1003/1005/1006/0FFF/1000/9003/9002. At odd opcode 1001 each target
increments by one. At FFFF short FE/FF/02 produce FFFF/0000/0003;
near FFFD/FFFE/0000 produce FFFF/0000/0002. Immediate bytes wrap
within CS at IP FFFF; physical addresses wrap at 20 bits independently.
EA loads its literal CS:IP, including FFFF:0010 -> physical 00000.
All full FLAGS (including reserved bits), registers, stack, other segments,
memory and ports are preserved. Issue #77 corrects shared E8 CALL to four
residual execution clocks plus target parity and actual fetches/stack writes;
see [direct CALL timing](call-timing.md). Issue #79 corrects
[indirect CALL/JMP](indirect-control-timing.md) to residuals 4/8 plus loaded
destination parity and actual pointer/stack transfers.
Issue #75 now applies the taken odd-target clock to
[Jcc/JCXZ](jcc-timing.md). REP remains rejected. IRQ/TF entry and DMA
reservations remain separate.

These are serialized emulator expectations, not console measurements. Full
prefetch, effective-address hazards, return timing and external bus policy remain
separate work.

## Original absolute cartridge interval

The source-included CC0 `fixtures/jmp-timing.asm` copies six measured bodies
to RAM 2000. All JMP opcodes are at odd IP 2007; each jumps forward over
poison HLT bytes to even/odd destinations. The loop begins at even IP 2004.
Each of 4096 iterations runs LEA BX,[BX+1] (4 clocks), the tested JMP
(EB/E9 6 or 7, EA 10 or 11), LEA BP,[BP+1] (4), DEC CX (2), and
JNZ 2004 (6, final 3). BX counts executions and BP counts destinations.
The interval includes initial IN AX,A8 (8) and MOV DX,AX (3). Terminal
IN observes the timer before its clocks advance; publication/return are outside.

| Forms / target | Independent clocks | HBlank ticks at 256 clocks |
| --- | ---: | ---: |
| EB/E9 even | 11 + 4096*22 - 3 = 90120 | 352–353 |
| EB/E9 odd | 11 + 4096*23 - 3 = 94216 | 368–369 |
| EA even | 11 + 4096*26 - 3 = 106504 | 416–417 |
| EA odd | 11 + 4096*27 - 3 = 110600 | 432–433 |

Only unknown initial timer phase permits floor/ceil quantization. Independent
native and browser consumers require these absolute ranges, BX=BP=4096,
CX=0 and continuation marker 51A7. Relative parity alone cannot pass.
Mono runs 00; Color/Crystal execute 00/80/00/C0/00/E0/00. RAM 0400 holds
600D completion, retained fresh boot count, physical capability 0/2 and pass
count 1/7. Each pass has six five-word records (ticks, BX, BP, CX, marker).
All 512 bytes are cleared on boot and mirrored to SRAM. Startup invalidates
retained success before measurement. Consumers poison saves and restart.

The LOOP and IRQ measured intervals contain no E9/EA/EB. Issue #73 left
their fixture bytes and absolute assertions unchanged. LOOP retains 45064/49161,
49160/53257 and 94216/98312 clocks. Issue #75 corrects IRQ’s odd JNZ
from six to seven clocks on 1023 iterations: 62482/68626 clocks,
244–245/268–269 ticks and unchanged counts 1024/0/0. Unmeasured setup, reset and
post-HALT jumps now cost more without changing these intervals.
