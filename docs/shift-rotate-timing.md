# Shift and rotate timing (Issue #83)

## Independent pre-edit oracle

The [NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A printed pp. 73 and 81–83, lists decoded
register costs 1/3/3 and memory costs 3/5/5 for implicit-one/CL/immediate
counts. There is no per-count term. Subtract the two already charged aligned
operand transfers to infer residuals **1/3/3** for either destination.
Replace the AI-authored preservation assumption `2 + masked_count`.
Section 1.3 requires memory read and writeback even for zero counts.

Literal unprefixed internal-register totals are **3/5/6**; byte or aligned
word `[BX]` totals are **5/7/8**; odd internal words add two clocks. Actual
opcode, ModR/M, displacement, prefix and immediate fetches and operand transfers
and waits are charged once. Immediate counts are byte fetches at either parity.
These totals include serialized fetches; the manual assumes completed decoding,
aligned words, no waits/holds and qualified addressing. Full prefetch/pipeline,
effective-address hazards, undocumented /6 and unrelated bus policies remain gaps.

The existing Issue #45 independent whole-integer/carry-ring result and full-FLAGS
vectors remain the architectural oracle, with opposing incoming FLAGS, all seven
operations, both widths, counts 0/1/2, width/ring boundaries, 31/32/33/255.
Timing expectations do not call production timing helpers. CL/CX/CH sampling,
all registers/segments, untouched RAM/SRAM, exact IP and continuation are retained.

## Original CC0 timing cartridge

`fixtures/shift-timing.asm` builds the original 128 KiB/32 KiB SRAM cartridge.
It uses no downloaded code, firmware, title, assets or device identity. Seventy
samples cover ROL/ROR/RCL/RCR/SHL/SHR/SAR, byte then word, in that order, each
with implicit one, CL=0, CL=31, immediate=32 and immediate=255. Every loop reloads
its operand and full incoming FLAGS F002, so the final result is discriminating
rather than an eventual zero produced by repeated shifts. Inputs are 81/8001 for
one and CL=31, 40/4000 for masked zero, FF/FFFF for immediate=255; byte forms
retain AH=A5. Independent native and browser literals check all results/full FLAGS.

The body is copied to internal 2800. Its exact measured instruction sequence is:

| Instruction | Address / condition | Serialized clocks |
| --- | --- | ---: |
| IN AX,A8 | 2800, start snapshot before advance | 8 |
| MOV DX,AX | 2802 | 3 |
| MOV AX,input | 2804, immediate starts odd | 4 |
| PUSH word F002 | 2807, aligned immediate and stack | 4 |
| POPF | 280A, aligned stack | 3 |
| Shift AX/AL | 280B, implicit-one / CL / immediate | 3 / 5 / 6 |
| PUSHF, POP SI | full shift FLAGS before counter DEC | 3 + 3 |
| LEA BX,[BX+1] | executed instruction count | 4 |
| DEC BP | remaining iterations | 2 |
| JNZ 2804 | even target, taken / final | 6 / 3 |
| MOV [DI+2],AX | after final loop, aligned result store | 5 |
| IN AX,A8 | end snapshot before advance | outside interval |

Thus loop costs are **32/34/35** and the total interval is
`8 + 3 + 1024*loop - 3 + 5 = 13 + 1024*loop`.
This is 32781/34829/35853 clocks. A 256-clock HBlank timer and arbitrary phase
permit precisely **128–129 / 136–137 / 140–141** ticks. No tolerance is added.
The former charges instead produce loop costs 34/33/64/34/65 across the five
profiles: 34829/33805/65549/34829/66573 clocks, or
**136–137 / 132–133 / 256–257 / 136–137 / 260–261** ticks. Every profile rejects
the former accounting. These sums are independent of production timing helpers.
They validate serialized accounting, not complete physical pipeline timing.

Setup/copying, outer CALL/RETF, comparisons and publication are outside the
interval. PUSHF captures every iteration's FLAGS before DEC changes them.
Results at RAM 0400 are completion 600D, boot count, capability 0/2, pass count
1/7, then seventy seven-word records per pass:
`ticks/result/full FLAGS/executed BX/remaining BP/raw CX/continuation`.
Expected counters are 1024/0 and CX=1/0/31/32/255. Marker 51A7 requires exact
SP=3EFA after each body, followed by actual RETF continuation. Missing/extra
execution changes counters or results. All 7168 observation bytes clear at boot
and mirror through CPU instructions to SRAM. Unused records must remain zero.

Mono runs mode 00. Color/Crystal run 00/80/00/C0/00/E0/00. Native and production
Chromium consumers invalidate poisoned retained completion after one frame,
require fresh completion and all records within 1400 frames, and perform two
boots. Before the second restart, observations are poisoned again while preserving
only the boot count. Chromium restores this through actual page/load/save handling,
then uses Restart before delivering RAF. Timing failures accumulate over all 2100
samples (70 × 15 model/pass configurations × 2 boots) before rejection. State
failures remain immediate. Neither consumer writes successful CPU observations.

## Existing measured-interval audit

All existing fixture sources and explicit opcode bodies were inspected for
shift/rotate instructions in measured intervals. Direct CALL, indirect CALL/JMP,
RET/RETF/IRET, external IRQ, Jcc/JCXZ, JMP, LOOP, ordinary I/O and internal-word
measurement bodies contain no affected shifts. Their absolute budgets and
quantization remain unchanged. Compatibility startup uses ROL AX,1 outside any
measured timing interval; keypad selection uses SHL AL,4 and SHR DL,1 without an
absolute timing assertion. Issue #45's probe measures results/FLAGS/branches,
not absolute intervals. Other occurrences of C0/D0–D3 are I/O ports or data,
not decoded shift opcodes. Demo source contains no shift/rotate instructions.
No existing consumer budget needs recalculation, and no tolerance changes.
Existing fixture/demo bytes and historical reports are preserved.

Executed checks, control observations and tested identities are recorded in
[Issue #83 validation](issue-83-validation.md).
