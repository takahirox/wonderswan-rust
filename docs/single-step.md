# V30MZ single-step reconciliation (Issue #67)

This decision record was written before changing CPU REP/HALT handling or trap
entry accounting. Expectations below are independent architectural traces, not
values obtained from the implementation. No physical-console run was performed.

## Sources and applicability

- [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
  chapter 5, printed pp. 55–63, especially Table 5-1 and §§5.2–5.4;
  chapter 6, pp. 65–66. BRK is TF, IE is IF, PS/PC are CS/IP.
- [NEC related V-series instruction manual, U11301EJ5V0UMJ1](https://datasheets.chipdb.org/NEC/V20-V30/U11301EJ5V0UMJ1.PDF),
  HALT p. 87, MOV p. 98, POP p. 118 and REP pp. 123–128 (printed).
  This manual describes older cores; its general segment-register inhibition
  does not override V30MZ §5.3's explicit SS-only list.
- [Independent experiment assembly](https://github.com/asiekierka/ws-test-suite/blob/5d8950d6b271a585c252af3ced93fbd2045f4752/src/mono/cpu/interrupt_timing/tests.s)
  and [comparison harness](https://github.com/asiekierka/ws-test-suite/blob/5d8950d6b271a585c252af3ced93fbd2045f4752/src/mono/cpu/interrupt_timing/main.c):
  `do_brk_delay_on_popf` and `do_brk_no_delay_on_popf_sti` compare against
  labels following NOP and STI respectively. Inspected sources, not console passes.
- [ares dispatch](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/component/processor/v30mz/instruction.cpp),
  [strings](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/component/processor/v30mz/instructions-string.cpp),
  [miscellaneous](https://github.com/ares-emulator/ares/blob/c76aaffa6e69778baf9a66d1da04298ac2ed7d1d/ares/component/processor/v30mz/instructions-misc.cpp),
  and execution/move implementations at the same revision: dispatch samples BRK
  before execution, suppresses polling for prefixes/SS loads, polls after string
  elements and HALT, and rewinds all prefixes on interrupted repetition.
  This is corroborating implementation evidence, not hardware measurement.
  Its unconditional tail BRK poll is not a priority oracle: after a synchronous
  entry (or NMI poll), the sampled BRK value can still be true. This change
  instead consumes a higher-priority boundary once, following V30MZ Table 5-1
  and entry's cleared IF/TF, rather than manufacturing a nested trap.

## Direct requirements and corrections

Old assumption: TF can remain set indefinitely while application instructions
execute. Corrected: sample TF before an instruction; enabling it with POPF or
IRET first traps after the following instruction. Entry fetches the vector before
stack writes, saves normalized FLAGS, CS, IP (in that push order), and clears live
IF/TF. IRET reactivates stepping; a handler can clear stacked TF to stop it.
The saved FLAGS, rather than live handler FLAGS, retain arithmetic/control flags.

V30MZ §5.3 suppresses the boundary after MOV SS (register or memory) and POP SS;
other segment transfers are negative controls. Prefix bytes never introduce a
boundary. STI's IF delay applies only to external INT; it does not delay TF.
Synchronous interrupts outrank NMI, external INT, then TF (Table 5-1). A selected
higher-priority entry consumes that boundary; do not fabricate a second vector-1
frame around its handler. No new NMI source is introduced.

## Independent boundary traces

All examples start at 0000:1000 with SS=0, SP=2000 unless specified. A trap frame
has IP at 1FFA, CS=0000 at 1FFC, normalized FLAGS at 1FFE; SP=1FFA. Handler IF/TF
are clear. No handler opcode executes as part of entry.

| Sequence / initial state | Expected observation and continuation |
| --- | --- |
| POPF; NOP; MOV BX,1234, stacked F102 | POPF ends at 1001 without entry; NOP traps with IP=1002, FLAGS=F102, BX unchanged; IRET resumes MOV. |
| POPF; STI; NOP, stacked F102 | Trap after STI, IP=1002, FLAGS=F302 despite external-INT shadow. |
| IRET restores TF and 0000:1000 | No trap on enabling IRET; one application instruction, then trap. Repeated IRET leaves exactly one frame per application instruction. |
| MOV SS,AX; NOP (TF already set) | No entry after SS load; trap after NOP. New SS used for all three saved words. |
| F0 26 F3 A4; CX=2, SI=0800, DI=0900, FLAGS=F102 | First element copies byte 0800 to 0900; CX=1, SI=0801, DI=0901, trap IP=1000. IRET restarts all prefixes. Second element copies 0801 to 0901; CX=0, SI=0802, DI=0902, trap IP=1004. No skipped/duplicate element. |
| Same REP, CX=0 | No data transfer/register progress; one completion trap, IP=1004. |
| REPE CMPSB with first unequal pair, CX=3 | One comparison, CX=2, SI/DI advance once; trap IP after opcode, with comparison FLAGS. |
| HLT; MOV BX,1234 with TF set | Trap at HLT completion, saved IP=1001, BX unchanged; entry cancels pending HALT before sustained standby. IRET resumes MOV. |
| Sustained HALT idle or DMA-only step | No new instruction, no TF sample/entry; preserve CPU state. |
| Pending eligible external IRQ before instruction, IF/TF set | IRQ entry before executing application code; save current IP and IF/TF once. IRET restores stepping; no trap around IRQ entry. |
| IRQ becomes eligible at a TF boundary | External IRQ wins; saved post-instruction IP/FLAGS, no second trap frame. |
| INT/INTO/BOUND/divide fault with TF set | Only synchronous vector entry. Preserve existing division-fault restart IP/results; IRET does not invent a trap in the handler. |

REP element cadence is an integration inference supported by the related manual's
repeat operation (execute primitive, decrement count, reexecute) and ares's
explicit per-element BRK poll. V30MZ §5.4 alone describes hardware interruption,
so it is insufficient proof of TF cadence. Zero count still completes the
prefixed instruction. HALT completion polling follows §5.2's next-instruction
rule and ares; chapter 6 lists hardware/reset release from *sustained* standby,
which does not establish an exception at the initial completion boundary.
Neither inference asserts exact silicon phases. Physical comparisons could
refine them; automated vectors keep the chosen behavior explicit.

## Accounting and scheduling decision

Table 5-1 assigns 25 clocks to software interrupt entry, including vector/stack
bus cycles and excluding inserted waits/holds. For this emulator's existing
word-transfer accounting, aligned internal vector reads cost 2 and the three
aligned stack writes cost 3. Trap entry therefore adds 20 execution clocks plus
actual entry bus charges (25 in the aligned internal case); odd/waiting stack
accesses add their existing charges. Do not add another five baseline transfers
on top of 25. A RAM NOP costs 1 execution + 1 fetch, so NOP plus aligned trap
returns 27. Mixed F0/26/F3 MOVSB costs 3 execution + 4 fetch + 2 data + 25 entry
= 34. Zero-count mixed REP costs 1 + 4 + 25 = 30. HLT plus aligned trap is
1 + 1 + 25 = 27 under the existing approximate HLT instruction model.

Keep existing external-IRQ/synchronous costs outside this task. Machine advances
all returned instruction/entry costs once through its normal scheduler. DMA
retains precedence and consumes only its own cost. Per-bus-phase IRQ arrival,
prefetch, general timing accuracy and exact HLT clock phases remain unmeasured.
The CPU's trap decision is local to instruction execution; no pending trap may
leak across cloning/reset/restart/replacement/model operations. Architectural
FLAGS and the existing external-INT shadow remain the relevant persistent state.
