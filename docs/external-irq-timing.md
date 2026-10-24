# External IRQ entry timing (Issue #69)

## Accounting recorded before changing the CPU

The old assumption was that 12 execution clocks plus vector/stack transfers
(17 aligned internal clocks, 20 with an odd stack) represented complete external
IRQ entry. The [NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Table 5-1 and its note (printed p. 55), specifies 32 clocks from interruption to
handler entry, excluding waits and holds. Chapter 4, §§4.1–4.3 establishes
one-clock aligned internal words, split odd words, and two aligned vector reads.
This primary specification corrects the earlier AI assumption; no human approval
is required. Exact console bus phases remain unmeasured.

In this serialized emulator, the five normal transfers are already inside those
32 clocks: two vector reads plus FLAGS, CS and IP pushes. Use **27 + actual entry
transfers**, in both delivery paths. At an eligible TF boundary, add this to the
completed instruction, counting neither its execution nor its transfers again.
Handler instructions (including acknowledgement and IRET) execute on later steps.
GDMA and SDMA stalls retain their separate charges and precedence. Single-step-only
entry remains 25 aligned clocks (20 + transfers).

Independent literal vectors, chosen before the implementation:

| Entry / step | No wait | SRAM wait set |
| --- | ---: | ---: |
| Internal even stack | 32 = 27 + 2 + 3 | 32 |
| Internal odd stack | 35 = 27 + 2 + 6 | 35 |
| RAM NOP then boundary IRQ, even | 34 = 2 + 32 | 34 |
| RAM NOP then boundary IRQ, odd | 37 = 2 + 35 | 37 |
| Color/Crystal SRAM stack, either alignment | 35 = 27 + 2 + 6 | 41 = 27 + 2 + 12 |

Preserve Issue #39's bus classification, inaccessible RAM behavior, wrapping and
existing wait controls. Mono has no writable mode/wait register; its unresolved
external wait behavior remains the existing no-added-wait policy. Do not infer
new electrical behavior from this timing correction. Issue #67's frame ordering,
FLAGS normalization, IF/TF clearing, priority, shadows, REP/HALT and IRET semantics
remain in force. Full prefetch and instruction-wide timing remain separate work.

## Absolute cartridge budget

The source-included CC0 `irq-timing.asm` probe and its native/browser consumers
measures a bounded latched interrupt sequence against the HBlank timer. Its
harness, independent totals, quantization and actual validation are recorded here
and in [Issue #69 validation](issue-69-validation.md).

The timer latches one HBlank IRQ under CLI. The CPU reloads the counter to FFFF,
then executes 1024 deliveries without acknowledging until the final handler.
An internal RAM LOOP is protected by each STI/IRET shadow; its count and the
handler's independent count must both finish. Samples use even and odd stacks,
and repeat through 0→80→0→C0→0→E0→0 on Color/Crystal (Mono uses 0 only).
All instruction fetches are internal, so ROM width/wait cannot affect the sample.

The measured interval begins at the first IN's timer observation and ends at the
second IN's observation. The first IN contributes 8 clocks, MOV DX,AX 3, STI 2,
and final CLI 2. The second IN and result publication are outside the interval.
Each of 1024 iterations includes entry 32, INC BX 2, DEC SI 2, taken JNZ 7,
IRET 10 and taken LOOP 8 = **61** clocks. The LOOP target is odd RAM 2005;
JNZ at 2011 targets odd 2017 (handler bytes 43 4E 75 04 B0 80 E6 B6 CF at 200F).
The final JNZ falls through at 3 instead of 7 (subtract 4); final LOOP falls
through at 4 instead of 8 (subtract 4). Final MOV AL,80 + OUT B6,AL adds 3 + 8 = 11.
Total: **1024 × 61 + 15 − 4 − 4 + 11 = 62482** clocks. An odd stack adds three
split pushes and three split pops per iteration: **68626** clocks. The counter
changes once per 256 clocks, so independent consumers require **244–245** and
**268–269** ticks respectively. The former IRQ entry charge gives **184–185**
and **208–209** ticks, well outside the ranges. Harness instruction costs are
the existing serialized fetch/execution policy, not full silicon timing.

Issue #71 corrects the old AI assumption of LOOP 6/3 totals to **8/4** for
this odd target, using Appendix A's instruction-specific DBNZ costs and Note 5.
With fixture layout and bytes unchanged, 1023 taken iterations gain two clocks
each and the final fallthrough gains one: **2047** extra clocks. The prior
59412/65556 budgets and observed 232/256 ticks in [Issue #69 validation](issue-69-validation.md)
remain historical evidence under that old LOOP implementation.
[Issue #71 validation](issue-71-validation.md) records that correction’s observations.

During Issue #69 harness development an initial calculation counted taken JNZ
as 3 instead of the then-implemented 6 clocks. That historical harness correction
preceded the IRQ change. Issue #75 now corrects the old six-clock odd-target
assumption to seven using Appendix A Note 5; the unchanged bytes gain **1023**
clocks over 61459/67603. See [Jcc timing](jcc-timing.md) for the correction record.
An initial stack overlap with the caller's return address was also fixed by
placing sample stacks at 2800/2801.

RAM 0400 begins with completion 600D, fresh boot count, model capability (0/2),
and pass count (1/7). Each pass records two four-word samples: elapsed ticks,
handler count 1024, final LOOP count 0 and final handler countdown 0. The CPU
clears RAM and invalidates SRAM completion before sampling, then publishes all
128 words to SRAM. Consumers poison retained saves and check both fresh runs;
old completion alone cannot fabricate timing, count or continuation observations.

Issue #73 changes direct JMP costs outside this measured interval and handler.
Reinspection of assembled bytes finds IN/MOV/STI/LOOP/CLI and
INC/DEC/JNZ/MOV/OUT/IRET, with no direct JMP. At that revision the 61459/67603-clock
and 240–241/264–265-tick ranges and 1024/0/0 assertions were unchanged.
Those observations and the former-entry negative control remain historical in
[Issue #73 validation](issue-73-validation.md). Issue #75 keeps fixture bytes,
32/35-clock entry, 1024/0/0 results and restart checks, and corrects only the
JNZ-dependent absolute bounds above. Current observations and a repeated
former-entry negative control are in [Issue #75 validation](issue-75-validation.md).
