# Ordinary IN/OUT timing (Issue #61)

This table was recorded before changing the decoder. All values describe the
retained explicit-fetch scheduling policy, not complete silicon pipeline timing.

| Encodings | Operation | RAM fetch clocks | Byte / eligible even word total | Split word total |
| --- | --- | --- | --- | --- |
| E4 / E5 | IN AL / AX, imm8 | 2 | 8 | 9 (E5) |
| E6 / E7 | OUT imm8, AL / AX | 2 | 8 | 9 (E7) |
| EC / ED | IN AL / AX, DX | 1 | 7 | 8 (ED) |
| EE / EF | OUT DX, AL / AX | 1 | 7 | 8 (EF) |

| Transfer class | Raw I/O charge T | DX total | Immediate total |
| --- | --- | --- | --- |
| Internal byte, including valid odd ports | 1 | 7 | 8 |
| Eligible even internal word | 1 | 7 | 8 |
| Odd or ineligible internal word | 2 | 8 | 9 |
| Cartridge byte, 60.3 clear / set | 1 / 2 | 7 / 8 | 8 / 9 |
| Cartridge word (always byte wide), 60.3 clear / set | 2 / 4 | 8 / 10 | 9 / 11 |
| BF/C0 word crossing, 60.3 clear / set | 2 / 3 | 8 / 9 | 9 / 10 |

For ROM instruction fetches use the actual F rather than the RAM column:
`total = F + 6 + (T - 1) = F + T + 5`. Every opcode, immediate and prefix
byte is fetched separately, so A0.2 does not combine an opcode with its port
operand. Each ROM fetch costs 1 + A0.3, regardless of A0.2. Prefixes retain their
existing fetch charge. SRAM waits at 60.1 remain in the existing memory/DMA paths;
ordinary I/O has no memory operand. Each additional I/O lane and applicable wait
is charged once. Failed lanes keep existing errors and preceding side effects.
Raw bus accounting, eligibility/masks, INS/OUTS and DMA costs are unchanged.

## Specification reconciliation

The old AI-authored assumption was that a default one-clock execution component,
plus one or two fetches and one ordinary I/O transfer (3/4 clocks), established
ordinary IN/OUT accuracy. It did not. The
[NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary)
(A13761EJ1V1UM00), Appendix A, Table A-1, printed pp. 78–80, specifies six clocks
for immediate and DW (DX) IN/OUT, for its decoded, no-wait/even-word conditions.
Chapter 4, sections 4.1–4.3, Table 4-1, printed pp. 47–50, describes the one-clock
normal transfer and split/extra-wait bus cycles. The integration inference here
is that the normal data transfer belongs inside those six clocks. Because the
emulator already accounts for T separately, the decoder must contribute five,
while retaining separately charged fetches F. Adding six to the existing charges
would count the normal transfer twice. This is a correction supported by primary
evidence, with no human approval or independent AI re-review gate.

The [pinned WSTimingTest report](https://github.com/FluBBaOfWard/WSTimingTest/blob/025e606fc3b6615160ffdae485d992ffcad0c024/README.md)
contains measured loop totals involving prefetch, branches and harness code.
Those totals are not individual-instruction acceptance constants here.

The [existing transfer classification](internal-word-timing.md) remains binding.
Full prefetch, intra-instruction device ordering, exact silicon timing and
commercial-title impact remain unmeasured. Physical-console tests and human
play are optional and unperformed. See [executed validation](issue-61-validation.md).
