# Indirect CALL/JMP timing (Issue #79)

## Oracle recorded before changing production charges

The old AI assumption preserved near/far memory JMP to odd 1041 at eight/eleven
clocks. Replace these with **nine/fourteen**. The
[NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A, printed pp. 73/76, gives decoded JMP register,
near memory, far memory 4/5/10, and CALL 5/6/12, plus one for odd destinations.
Subtract the normal aligned pointer reads and CALL stack writes already charged
by this emulator: inferred residuals **4 near, 8 far**, plus loaded IP parity.
Actual fetches, reads, writes, alignment and waits are added once. Appendix A's
decode/EA/no-hold qualifications still apply; full prefetch and effective-address
hazards are outside this change. These are serialized emulator expectations.

Literal internal-RAM vectors, zero prefixes, aligned pointer and stack, at
**odd opcode IP** (1001):

| Encoding | Even destination | Odd destination | Access clocks |
| --- | ---: | ---: | ---: |
| FF D0 (CALL AX) | 7 | 8 | 3 |
| FF 16 00 08 (CALL [0800]) | 10 | 11 | 6 |
| FF 1E 00 08 (CALL FAR [0800]) | 16 | 17 | 8 |
| FF E0 (JMP AX) | 6 | 7 | 2 |
| FF 26 00 08 (JMP [0800]) | 9 | 10 | 5 |
| FF 2E 00 08 (JMP FAR [0800]) | 14 | 15 | 6 |

Opcode/ModR/M are separate byte fetches. The following disp16 starts with
the same parity as the opcode, costing 1/2 at even/odd IP. At even opcode IP
subtract one from all memory-row clocks and accesses. Register rows apply at
either IP. Odd internal pointers add one near/two far; odd internal stacks
add one near CALL/two far CALL. No destination instruction is read or executed.

At 1000/1001 the return offsets are 1002/1003 for registers and 1004/1005 for
disp16 memory. Near operands, including SP, are sampled before decrementing SP.
Far CALL saves CS then return IP before reading either pointer word; overlapping
pointer words can change both CS and IP. Parity comes from that loaded IP.
Full FLAGS, other registers/segments, ports and unrelated memory are preserved.
SS selects stack writes despite overrides. Offset and physical addresses wrap
at 16/20 bits, retaining existing mapping/consolidation/ignored-ROM-write policy.
Real RET/RETF must restore the saved continuation; [Issue #81](return-timing.md)
corrects their execution residuals to 5/6 plus restored destination parity.

ROM byte costs at live A0 00/04/08/0C are 1/1/2/2, aligned word costs 2/1/4/2,
odd words 2/2/4/4. SRAM words cost 2 (4 with existing live SRAM wait bit).
Internal aligned words cost 1 only in accessible RAM; odd/wrapping words cost 2.
Prefixes add byte fetches and shift disp16 alignment; no EA-hazard charge is added.

## Absolute original cartridge oracle

The CC0 source copies twelve bodies to RAM 2000, one per six forms and target
parity. The timer observation at 2000 is IN AX,A8 (8 clocks); MOV DX,AX at 2002
costs 3. The loop starts at even 2004 with LEA BX,[BX+1] (4). Indirect dispatch
at odd 2007 costs register CALL 7/8, near memory CALL 10/11, far memory CALL
16/17; JMP 6/7, 9/10, 14/15. Memory encodings use disp16 0900 (two fetch
clocks), aligned internal pointers, and an aligned internal stack.
Register forms encode FF D6/E6; near memory FF 16/26 00 09; far memory
FF 1E/2E 00 09. Copied even/odd target offsets are CALL register 2032/2033,
CALL near memory 2034/2035, CALL far memory 2032/2033, JMP register 200A/200B,
JMP near/far memory 200C/200D. Source NASM listings establish these offsets.
Destination LEA BP,[BP+1] costs 4. Calls actually RET/RETF to odd 2009/200B
(8/10 including fetch, stack reads and restored destination parity). DEC CX costs 2; JNZ to even
2004 costs 6, final fallthrough 3. Terminal IN samples before its own clocks.

| Form | Even/odd loop clocks | Even/odd total clocks | Even/odd HBlank ticks |
| --- | --- | --- | --- |
| CALL register | 31 / 32 | 126984 / 131080 | 496–497 / 512–513 |
| CALL near memory | 34 / 35 | 139272 / 143368 | 544–545 / 560–561 |
| CALL far memory | 42 / 43 | 172040 / 176136 | 672–673 / 688–689 |
| JMP register | 22 / 23 | 90120 / 94216 | 352–353 / 368–369 |
| JMP near memory | 25 / 26 | 102408 / 106504 | 400–401 / 416–417 |
| JMP far memory | 30 / 31 | 122888 / 126984 | 480–481 / 496–497 |

Each total is 11 + 4096*loop − 3. Only unknown timer phase permits floor/ceil
at 256 clocks per tick. Every sample requires BX=BP=4096, CX=0, SP=2FFA and
marker 51A7 after actual continuation. Near CALL saves 2009/200B at 2FF8;
far CALL saves 200B at 2FF6 and CS=0 at 2FF8. JMP preserves both sentinel words;
near CALL preserves the lower sentinel. No measured setup modifies SI.

Copying, pointer/SI initialization, stack sentinels, timer setup, outer CALL/RETF,
result stores, comparisons and SRAM publication are outside the interval.
RAM 0400 holds completion 600D, boot count, capability 0/2 and pass count 1/7,
then twelve five-word records per pass (ticks/BX/BP/CX/marker). All 1024 bytes
clear on boot and mirror to SRAM. Retained completion is invalidated before
sampling. Mono runs 00; Color/Crystal 00/80/00/C0/00/E0/00. Both independent
consumers require poisoned-save invalidation and two fresh boots.

Existing IRQ, direct CALL/JMP, Jcc/JCXZ, LOOP, ordinary-I/O and word fixture
intervals contain no indirect dispatch. Dispatch instructions remain unchanged.
Issue #81 recalculates direct CALL and IRQ intervals for corrected returns; IRQ entry 32/35 and former-charge rejection
remain covered. Other measured bounds stay unchanged.
Historical observations remain historical. Full prefetch,
effective-address hazards, unrelated bus policies, commercial-title and physical
console validation remain separate gaps.
