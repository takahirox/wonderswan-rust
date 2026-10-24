# Successful division timing (Issue #87)

## Correction and accounting

The [NEC V30MZ preliminary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A, printed pp. 73/77–78, gives decoded register
costs of 15/23 clocks for unsigned byte/word division and 17/24 for signed
byte/word division. Memory forms cost one additional normal aligned transfer.
NEC calls unsigned division DIVU and signed division DIV. Conditions include
completed decoding, no waits/holds, aligned words and qualified effective
addressing; these are not full instruction-fetch/pipeline measurements.

The former AI-authored requirement preserved width-only **24/32 residual clocks**
for successful byte/word division. Correct this to **15/17/23/24** in byte DIV,
byte IDIV, word DIV, word IDIV order. The bus already charges the source read,
including alignment splits and waits: adding the manual's memory cost as a
residual would count the normal transfer twice. Using the register costs as
residuals is an explicit inference for the existing serialized scheduler.
Actual fetch and source transfers remain charged once. Neither signedness nor
source location needs a separate decoder or model-specific dispatch.

Independent totals for two-byte instructions fetched from internal RAM are:

| Form | Register (former → corrected) | Even [BX] | Odd [BX] |
| --- | ---: | ---: | ---: |
| Byte DIV, F6 /6 | 26 → 17 | 18 | 18 |
| Byte IDIV, F6 /7 | 26 → 19 | 20 | 20 |
| Word DIV, F7 /6 | 34 → 25 | 26 | 27 |
| Word IDIV, F7 /7 | 34 → 26 | 27 | 28 |

Two independent opcode-byte fetches account for two clocks. A byte or aligned
RAM word source adds one; an odd word adds two. Displacements, prefixes, offset
and physical wrapping, unavailable RAM, mapped SRAM/ROM and live A0/60 waits
retain existing bus policies. Tests calculate those costs independently of the
production timing/address helpers and check charged accesses and unchanged
source snapshots, rejecting duplicate reads and source writes.

Successful and fault charge selection are separate. Zero divisors and quotient
overflow retain **24/32 fault residuals**, plus actual fetch/source/vector/stack
transfers. Two-byte register faults with aligned internal stack/vector return
31/39 clocks (24/32 + 2 + 5), preserving the baseline frame, registers, history
and diagnostics. This is a regression policy, **not established hardware fault
accuracy**. Fault classification, exceptional FLAGS/results, restart semantics
and exact fault timing remain unresolved. No successful charge is applied to
faults. AAM-zero behavior is also unchanged.

The arithmetic/full-FLAGS acceptance from Issues [#47](word-division-flags.md),
[#49](byte-idiv-flags.md) and [#65](byte-div-flags.md) remains applicable.
Multiplication overflow history, valid AL/AH/AX/DX aliases and actual subsequent
instructions are preserved. Unsigned byte DIV AH cannot succeed: AH=0 faults,
and AX/AH ≥256 otherwise. Signed boundary tests use ±127/±32767 without claiming
resolution of ambiguous exceptional limits.

Full prefetch/pipeline timing and effective-address hazards remain separate gaps.
The pinned [WSTimingTest report](https://github.com/FluBBaOfWard/WSTimingTest/blob/025e606fc3b6615160ffdae485d992ffcad0c024/README.md)
limits interpretation of aggregate loops. The pinned
[WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md)
identifies exceptional division discrepancies. Neither establishes this emulator's
fault policies or commercial-title impact as hardware-correct.

## Original probe and independent absolute budgets

`fixtures/divide-timing.asm` is original CC0-1.0 source using existing fixture
NASM tooling. Its 128 KiB cartridge has 32 KiB SRAM and no third-party program,
ROM, firmware, assets or device identity. Twelve samples run byte DIV, word DIV,
byte IDIV, word IDIV, each with register, even RAM and odd RAM sources. Every
repetition reloads AX=7, DX=0, BX=2 or the source address, and FLAGS=F002.
Both byte forms must produce AX=0103 and preserve DX=0; word forms produce
AX=3, DX=1. Full FLAGS are F006 for byte IDIV and F002 otherwise.

Each body is copied to internal 2800; source data is 3200/3201. The following
sum comes from emitted instructions, including immediate-word alignment:

| Instruction | Address / condition | Serialized clocks |
| --- | --- | ---: |
| IN AX,A8 | 2800; start snapshot before advance | 8 |
| MOV [3000],AX | 2802; odd address word, aligned data | 5 |
| NOP | 2805; repeat target even | 2 |
| MOV AX,7 | 2806; odd immediate | 4 |
| MOV DX,0 | 2809; aligned immediate | 3 |
| MOV BX,source/address | 280C; odd immediate | 4 |
| PUSH word F002 | 280F; aligned immediate/stack | 4 |
| POPF | 2812 | 3 |
| Division | 2813; register/even/odd memory | table above |
| PUSHF; POP CX | 2815/2816; full FLAGS before DEC | 3 + 3 |
| LEA SI,[SI+1] | 2817; execution count | 4 |
| DEC BP | 281A; remaining iterations | 2 |
| JNZ 2806 | 281B; taken/final | 6 / 3 |
| MOV [DI+2],AX; MOV [DI+4],DX | 281D/2820; result stores | 5 + 5 |
| IN AX,A8 | 2823; end snapshot before advance | outside interval |

The loop excluding division costs **36 clocks**. The complete interval is
`8 + 5 + 2 + 1024*(36 + division) - 3 + 10`, or
**22 + 1024*(36 + division clocks)**. Later subtraction, publication, copying,
CALL/RETF and comparisons are outside the interval. Exact floor/ceiling bounds
for arbitrary timer phase on a 256-clock HBlank timer are:

| Form | Corrected ticks, register/even/odd | Former ticks, register/even/odd |
| --- | --- | --- |
| Byte DIV | 212–213 / 216–217 / 216–217 | 248–249 / 252–253 / 252–253 |
| Word DIV | 244–245 / 248–249 / 252–253 | 280–281 / 284–285 / 288–289 |
| Byte IDIV | 220–221 / 224–225 / 224–225 | 248–249 / 252–253 / 252–253 |
| Word IDIV | 248–249 / 252–253 / 256–257 | 280–281 / 284–285 / 288–289 |

These are quantization bounds, without added tolerance. The former charges add
36/36/28/32 timer ticks per 1024 repetitions in the four-form order above.
Independent native and production Chromium consumers use separate literal
budgets/results, and run identical final probe bytes with both charge versions.

RAM 0400 contains completion 600D, boot count, capability 0/2 and pass count 1/7,
then twelve eight-word records per pass:
`ticks/AX/DX/full FLAGS/executed SI/remaining BP/source BX/continuation`.
Counts must be 1024/0. Marker 51A7 requires SP=3EFA, followed by actual RETF to
the next sample. Unused observations are cleared. All 7168 bytes mirror to SRAM;
only fresh completion then HALT is accepted.

Mono runs 00; Color/SwanCrystal run 00/80/00/C0/00/E0/00. Both consumers poison
retained completion and records before **each of two boots**, observe startup
invalidation after one frame and require fresh completion within 400 frames.
The production browser uses real local save restoration/Restart with controlled
RAF delivery. Native also compares the entire RAM/SRAM mirror. Each consumer
collects all **360 timing observations** (12 * 15 model/pass configurations *
2 boots) before reporting timing failures, so a first failure cannot hide later
models or restarts. Browser ROM/save handling must remain local.

## Existing interval audit and limits

All fixture/demo assembly was searched for division. Prior executed division
opcodes occur in `divide.asm`, which checks arithmetic/FLAGS/history without an
absolute division interval. Existing multiplication timing bodies have no
division. Shift, I/O, word transfers, direct/indirect CALL/JMP, returns/IRET,
Jcc/JCXZ, LOOP and IRQ timed bodies contain no division. Demo has none. Existing
fixture/demo bytes and every unrelated interval/tolerance remain unchanged.

Existing arithmetic matrices now select literal 15/17/23/24 residuals. Register
alias assertions become 17/19 or 25/26. Word scheduler budgets become 25/26.
The former **51-clock** byte-division-plus-trap assertions become **42 for DIV**
(15+2+25) and **44 for IDIV** (17+2+25). Entry's five accesses and 20 residual
clocks still total 25 independently; actual handler/IRET continuation remains
checked. Historical validation reports are preserved as historical evidence.

Normal `Machine::run_frame` coverage checks exact elapsed clocks, one-clock
frame overshoot, raster/frame/HBlank progression, one stereo sample pair and no
poison continuation instruction. Separate delivery tests cover IF shadows, TF,
IRQ priority, saved frames and actual IRET with separate entry costs. GDMA/SDMA
retain their capability/mode gates and seven-clock reservations without fetching
an instruction. Existing REP/HALT and broader device acceptance remain enabled.

[Executed validation and criterion review](issue-87-validation.md) records actual
failures/passes, artifacts and uncertainty. These original probes establish
bounded behavior; commercial-catalog readiness, physical-console measurements
and human gameplay remain unestablished and unperformed.
