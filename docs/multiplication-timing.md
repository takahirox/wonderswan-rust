# Multiplication timing (Issue #85)

## Correction and independent oracle

The [NEC V30MZ preliminary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A, printed pp. 73/79, gives decoded register/memory
costs of 3/4 clocks for unsigned byte/word MUL, signed byte/word IMUL and both
immediate IMUL widths (NEC names signed/unsigned forms MUL/MULU). Conditions
include completed decoding, aligned words, no waits/holds and qualified addressing.

The former AI-authored assumption retained residuals 12/20 for byte/word group
forms and 12 for immediate forms. Correct this to **three residual clocks for
every form**: the normal aligned memory-source read already contributes one of
the manual's four clocks. This subtraction is an explicit inference for this
emulator's serialized scheduler. Fetches, transfers, alignment splits and waits
remain charged once by the existing bus. Arithmetic, FLAGS and private overflow
history are unchanged. [Issue #43 architectural evidence](multiplication-flags.md)
and [Issue #65 history requirements](byte-div-flags.md) remain applicable.

At even internal-RAM instruction addresses, independent literal totals are:

| Form | Register | Byte/aligned word [BX] | Odd word [BX] |
| --- | ---: | ---: | ---: |
| F6 /4 or /5 | 5 | 6 | n/a (byte still 6) |
| F7 /4 or /5 | 5 | 6 | 7 |
| 69, immediate word | 6 | 7 | 8 |
| 6B, sign-extended byte | 6 | 7 | 8 |

F6/F7 fetch two bytes. 69 fetches opcode, ModR/M and one aligned immediate
word; 6B fetches three bytes. Odd instruction addresses split 69's immediate
word into two transfers. Prefix/displacement fetches and mapped waits add their
actual costs. No test expectation invokes production timing/address helpers.
Tests assert charged accesses as well as complete state and unchanged source
memory, so duplicate reads and source writes are rejected.

Full prefetch/pipeline fidelity, effective-address dependencies and unrelated bus
policies remain separate gaps. Issue #87 subsequently corrects
[successful division timing](division-timing.md), retaining separate fault costs. The pinned
[WSTimingTest report](https://github.com/FluBBaOfWard/WSTimingTest/blob/025e606fc3b6615160ffdae485d992ffcad0c024/README.md)
describes aggregate instruction-loop measurements with their own fetch/loop
conditions; those totals are not isolated decoded costs.

## Original CC0 cartridge and absolute interval arithmetic

`fixtures/multiply-timing.asm` produces an original 128 KiB cartridge with 32 KiB
SRAM using existing tooling. No external code, ROM, firmware, assets or device
identity is included. Eighteen samples run in form order byte MUL, word MUL,
byte IMUL, word IMUL, 69, 6B; each uses register, even RAM and odd RAM sources.
Every repetition reloads AX and BX and incoming FLAGS F002. Unsigned input is
3*2, signed group input -3*7, immediate-word input -3*-257 and immediate-byte
input -3*-128. This prevents repeated products collapsing to zero or another
uninformative steady state. Word group results include DX; byte and immediate
forms must retain DX=5AA5. Native and browser consumers embed separate literal
products and full FLAGS, including model-specific unsigned ZF.

Every body is copied to internal 2800; source RAM is 3200 or 3201. Independently
summing the actual emitted instructions gives:

| Instruction | Address / condition | Serialized clocks |
| --- | --- | ---: |
| IN AX,A8 | 2800, start snapshot before advance | 8 |
| MOV [3000],AX | 2802, odd address immediate, aligned data | 5 |
| NOP | 2805, puts repeat at even 2806 | 2 |
| MOV AX,input | 2806, odd immediate | 4 |
| MOV BX,source/address | 2809, aligned immediate | 3 |
| PUSH word F002 | 280C, odd immediate, aligned stack | 5 |
| POPF | 280F | 3 |
| Multiplication | 2810, register / even RAM / odd RAM | see below |
| PUSHF; POP CX | capture full arithmetic FLAGS | 3 + 3 |
| LEA SI,[SI+1] | execution count | 4 |
| DEC BP | remaining iterations | 2 |
| JNZ 2806 | even target, taken / final | 6 / 3 |
| MOV [DI+2],AX; MOV [DI+4],DX | aligned product stores | 5 + 5 |
| IN AX,A8 | end snapshot before advance | outside interval |

The loop excluding multiplication is **33 clocks**. Thus the interval is
`8 + 5 + 2 + 1024*(33 + multiply) - 3 + 10`, or
**22 + 1024*(33 + multiply)**. Subsequent subtraction, state publication,
outer copying, CALL/RETF and comparisons lie outside that interval.

| Forms | Multiplication clocks, register/even/odd | Absolute HBlank ticks, register/even/odd |
| --- | --- | --- |
| F6 unsigned/signed | 5 / 6 / 6 | 152–153 / 156–157 / 156–157 |
| F7 unsigned/signed | 5 / 6 / 7 | 152–153 / 156–157 / 160–161 |
| 69 and 6B | 6 / 7 / 8 | 156–157 / 160–161 / 164–165 |

These are exact floor/ceiling bounds for arbitrary phase on a 256-clock HBlank
timer, not widened tolerances. The former charges add 9 clocks per byte/immediate
multiply and 17 per word group multiply: **36 or 68 timer ticks** over 1024
repetitions. Old bounds are therefore 188–189/192–193/192–193 (F6),
220–221/224–225/228–229 (F7), and 192–193/196–197/200–201 (69/6B).
The same final cartridge bytes are tested under both charge implementations.

RAM 0400 contains completion 600D, boot count, capability 0/2, pass count 1/7,
then eighteen eight-word records per pass:
`ticks/AX/DX/full FLAGS/executed SI/remaining BP/source BX/continuation`.
Expected counts are 1024/0. Marker 51A7 requires SP=3EFA and is followed by an
actual RETF to the next sample. Unused observations are zero. The CPU mirrors
all 7168 observation bytes to SRAM and halts after fresh completion.

Mono runs 00; Color/Crystal run 00/80/00/C0/00/E0/00. Both independent consumers
poison retained completion and records before each of two boots, inspect startup
invalidation after one frame, and require fresh completion within 400 frames.
The browser uses the production player's local save restoration and Restart,
with controlled RAF delivery. Native checks the entire RAM/SRAM mirror; browser
checks the persisted observations and local ROM/save handling. Timing failures
accumulate across **540 samples per consumer** (18 * 15 model/pass configurations
* 2 boots) before final rejection, so later passes cannot be concealed.

## Existing interval audit and limits

All fixture assembly and explicit measured opcode bodies were inspected for
multiplication. Existing multiplication and division-history cartridges exercise
state without an absolute multiply interval. `compat.asm` uses IMUL during setup,
before timers/interrupt loops; it has no absolute assertion for that setup.
Word/I/O, shift, direct/indirect CALL/JMP, return/IRET, IRQ, Jcc/JCXZ and LOOP
measured bodies contain no multiplication. Other 69/6B occurrences are port
addresses, not executed multiply opcodes. Demo assembly has no multiplication.
Consequently no existing measured cartridge interval or tolerance needs changing.
One older CPU accounting assertion assumed `F7 E0` must exceed a memory MOV.
Its bytes follow `90 B8 01 00 A1 00 08` at 1007: two fetches plus three residual
clocks give **5**, equal to the preceding aligned `A1 00 08` (opcode + odd
address-word fetch + aligned operand read + residual = 1+2+1+1). Replace only
that affected assertion with the literal five-clock multiply budget; retain its
other fetch/wait/HALT assertions. Existing fixture/demo bytes, unrelated assertions
and historical reports are preserved.

[Validation evidence](issue-85-validation.md) records executed controls, actual
observations, tested identities, mandatory checks and criterion review. This
bounded accounting validation does not establish physical-console pipeline timing
or commercial-catalog readiness. Physical-console measurements and human gameplay
remain optional and unperformed.
