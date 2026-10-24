# Direct CALL timing (Issue #77)

## Independent oracle recorded before the charge edit

The old AI preservation assumption gave `E8 02 00` at 0000:1000 five clocks
with an aligned internal stack and target 1005. Replace that with **nine**.
The old far CALL residual was six; replace it with **eight plus target parity**.
The [NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A conditions/Note 5 and Table A-1, printed pp. 73/76,
gives decoded near/far CALL 5/10 with aligned data and an even destination.
Chapter 4, pp. 47–48, adds transfers for odd words, including stack words.
Subtract one/two normal aligned stack transfers to infer residuals **4/8** in
this serialized emulator: actual fetches + actual stack writes + residual +
(destination IP & 1). This is an inference from the primary specification,
not a console measurement or a full prefetch model. No destination pre-read.

| Encoding | Even opcode, even/odd target | Odd opcode, even/odd target |
| --- | --- | --- |
| E8, aligned internal stack | 8 / 9 | 7 / 8 |
| 9A, aligned internal stack | 15 / 16 | 13 / 14 |

Odd internal stacks add one/two clocks. Opcode/immediate fetches are 3/2 for
E8 and 5/3 for 9A at even/odd IP. Prefix bytes cost one apiece and shift the
immediate alignment. At even start and prefix counts 0..7 the fetches are
E8 3/3/5/5/7/7/9/9 and 9A 5/4/7/6/9/8/11/10. ROM byte lane costs at live
A0 00/04/08/0C are 1/1/2/2; aligned immediate words cost 2/1/4/2 and odd
words 2/2/4/4. Existing mapping/wait/consolidation rules apply exactly once.

Near displacement 0000/0002/0003/FFFC/FFFD/8000/7FFF at 1000 gives
1003/1005/1006/0FFF/1000/9003/9002; at 1001 all targets increase by one.
The return IP is 1003/1004. At FFFF, post-immediate IP is 0002 and
FFFD/FFFE/0000 give FFFF/0000/0002. Far CALL loads its literal CS:IP,
including FFFF:0010 (physical 00000), and saves old CS before post-immediate
IP. Near saves only post-immediate IP. SS selects the stack even with segment
prefixes. SP subtraction and stack offsets wrap at 16 bits, physical addresses
at 20 bits. All other registers, full FLAGS and unrelated memory are preserved.
RET/RETF execution retains its existing approximate four clocks.

## Original absolute cartridge interval

Original CC0 `fixtures/call-timing.asm` copies four measured bodies to RAM 2000.
IN AX,A8 at 2000 costs 8; MOV DX,AX at 2002 costs 3. Loop 2004 executes
LEA BX,[BX+1] (4), CALL at odd 2007 (E8 7/8, 9A 13/14), destination
LEA BP,[BP+1] (4), RET/RETF (6/7 including fetch and stack reads), DEC CX (2),
JNZ even 2004 (6, final 3). The aligned stack is 2FFA before each measured
CALL. The copied targets are near 202A/202B and far 2034/2035; measured bytes
at 2007 are E8 20 00 / E8 21 00 / 9A 34 20 00 00 / 9A 35 20 00 00.
RET/RETF costs are retained emulator approximations, not a new hardware
claim. Terminal IN reads before its own clocks advance. Each interval is:

| Form / destination | Clocks | HBlank ticks (256 clocks) |
| --- | ---: | ---: |
| Near even | 11 + 4096*29 - 3 = 118792 | 464–465 |
| Near odd | 11 + 4096*30 - 3 = 122888 | 480–481 |
| Far even | 11 + 4096*36 - 3 = 147464 | 576–577 |
| Far odd | 11 + 4096*37 - 3 = 151560 | 592–593 |

Only unknown timer phase allows floor/ceil quantization. Outside the interval,
SUB DX,AX / MOV AX,DX publish elapsed ticks; comparisons require SP=2FFA,
near return word at 2FF8=200A, far words at 2FF6=200C and 2FF8=0000.
Failure halts before publishing marker 51A7. Thus marker certifies actual return
continuation and frame integrity. BX/BP must both be 4096 and CX zero.
Setup/copy/timer programming, outer CALL/RETF, result stores, stack comparisons,
mode changes and completion/SRAM publication are outside the measured interval.

RAM 0400 holds completion 600D, retained boot count, capability 0/2 and pass
count 1/7, then four five-word records per pass (ticks/BX/BP/CX/marker).
All 512 bytes are cleared and mirrored to SRAM. Boot invalidates retained
completion first. Mono runs 00; Color/Crystal run 00/80/00/C0/00/E0/00.
Independent native and production Chromium consumers poison retained saves,
check invalidation after one frame, all absolute results and fresh restart.

## Existing consumers

IRQ/JMP/LOOP/Jcc measured bytes contain no direct CALL inside their sampled
intervals. Their outer/setup calls change elapsed setup only. Preserve absolute
IRQ 62482/68626 (244–245/268–269 ticks), JMP 90120/94216/106504/110600,
LOOP 45064/49161, 49160/53257 and 94216/98312, and Jcc
106504/110600/94216. Ordinary I/O remains 11+2048*8/9/10, relative-word
odd-minus-even remains 4096 clocks. No tolerance widening or historical rewrite.
Indirect calls/jumps, return timing, effective-address hazards, full prefetch,
external bus policy and commercial-title/physical-console validation remain separate.
