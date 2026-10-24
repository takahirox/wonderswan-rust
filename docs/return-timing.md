# Return timing (Issue #81)

## Independent oracle recorded before changing charges

The [NEC V30MZ manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
A13761EJ1V1UM00, Appendix A printed pp. 73/81 gives decoded C3/C2/CB/CA/CF
costs 6/6/8/9/10 under its aligned, decoded, no-wait conditions. Deduct the
one/two/three aligned stack transfers already charged by this emulator to infer
residuals **5/5/6/7/7**. Replace the previous AI-authored residuals 4/4/4/4/6.
Add actual opcode/immediate fetches, actual stack reads/waits once, and one
clock for an odd loaded IP. Applying the general branch destination condition
to restored CS:IP is an explicit inference; Chapter 4 printed pp. 47–48 explains
odd branch fetch alignment. These are serialized budgets, not pipeline or
physical-console measurements. There is no destination pre-read.

Literal internal RAM vectors (columns are even/odd destination):

| Form | Even opcode, even SP | Odd opcode, even SP | Even opcode, odd SP | Odd opcode, odd SP |
| --- | --- | --- | --- | --- |
| C3 | 7/8 | 7/8 | 8/9 | 8/9 |
| C2 | 9/10 | 8/9 | 10/11 | 9/10 |
| CB | 9/10 | 9/10 | 11/12 | 11/12 |
| CA | 12/13 | 11/12 | 14/15 | 13/14 |
| CF | 11/12 | 11/12 | 14/15 | 14/15 |

C2/CA immediate fetches cost two at an even opcode, one at an odd opcode.
SS selects every popped word despite overrides. Pop IP, then CS for far/IRET,
then FLAGS for IRET; normalize FLAGS to (value & 0x0FD5) | 0xF002.
Add immediate to SP after popping with 16-bit wrapping, including zero/odd/FFFF.
Other registers/segments and all memory stay unchanged. RET preserves full FLAGS;
IRET retains the existing IF shadow and TF activation/delivery policy.

ROM byte clocks for live A0 00/04/08/0C are 1/1/2/2 and aligned word clocks
2/1/4/2. Odd words use two lanes. SRAM uses two byte transfers per word,
or four with the existing color SRAM wait enabled. High internal RAM words
consolidate only when available in the selected model/mode. Prefix bytes and
immediate alignment are accounted once. Offset/physical/bank wraps retain the
existing bus policies; full prefetch and effective-address hazards remain gaps.

## Original CC0 cartridge and absolute interval

`fixtures/return-timing.asm` builds a 128 KiB/32 KiB SRAM cartridge. No external
code, assets, firmware or ROM is included. Ten bodies copied to internal RAM
2000 measure the five forms with even/odd restored destinations. Each runs
4096 returns, counting execution with LEA BX,[BX+1] and continuation with
LEA BP,[BP+1]. LOOP leaves full FLAGS unchanged; initial/restored FLAGS F046
has TF/IF clear. PUSHF captures full FLAGS outside the interval on a separate
scratch stack, preserving the popped frame for inspection.

First IN AX,A8 contributes 8 clocks and MOV DX,AX 3. Loop starts at even 2004:
LEA BX (4), frame pushes below, return, LEA BP (4), LOOP even 2004 (7, last 4).
PUSH SI and PUSH ES cost 3 each. PUSH imm16 at odd 2007 costs 4 (two fetch,
one execution, one aligned write). ES=0 supplies saved CS. C2/CA push FACE
padding first and discard it with immediate 2. CF pushes F046, CS, IP.
SP starts/ends 2FFA; targets are C3/CB 200A/B, C2/CF 200E/F, CA 2010/11.
Return opcodes are C3 2008, C2 200B, CB 2009, CA/CF 200C. Immediate
consolidation makes C2 cost 8/9; other return totals are 7/8, 9/10, 12/13,
11/12 respectively. Unreachable HLT padding sets loaded destination parity.

| Form | Frame pushes | Even/odd loop clocks | Total clocks | HBlank bounds |
| --- | ---: | --- | --- | --- |
| C3 | 3 | 25/26 | 102408/106504 | 400–401/416–417 |
| C2 | 4+3 | 30/31 | 122888/126984 | 480–481/496–497 |
| CB | 3+3 | 30/31 | 122888/126984 | 480–481/496–497 |
| CA | 4+3+3 | 37/38 | 151560/155656 | 592–593/608–609 |
| CF | 4+3+3 | 36/37 | 147464/151560 | 576–577/592–593 |

Total = 11 + 4096*loop − 3. Terminal IN samples before its clocks advance;
unknown timer phase permits floor/ceil at 256 clocks, with no added tolerance.
Setup, copying, outer CALL/RETF, flag capture, SP/frame comparisons, result
stores and publication lie outside the interval. Other harness costs remain
emulator approximations; the interval is not a full pipeline measurement.

RAM 0400 begins completion 600D, boot count, capability 0/2, passes 1/7;
then ten six-word records per pass: ticks/BX/BP/CX/full FLAGS/marker. Marker
51A7 requires exact SP, popped IP/CS/FLAGS, discarded padding and untouched
frame sentinels. All 1024 bytes clear at boot and mirror to SRAM. Independent
native and production Chromium consumers check absolute budgets, 4096/4096/0,
F046, marker, unused zero records, poisoned-save invalidation and two boots.
Mono uses 00; Color/Crystal use 00/80/00/C0/00/E0/00. Browser bytes remain local.

## Recalculated existing intervals with unchanged cartridge bytes

Direct CALL returns to even continuations: near gains 1 and far gains 2 clocks
per iteration. Loop costs become 30/31/38/39; 11+4096*loop−3 gives
122888/126984/155656/159752 clocks, **480–481/496–497/608–609/624–625** ticks.
Indirect CALL returns to odd 2009/200B: near gains 2, far gains 3. Loop costs
become 31/32/34/35/42/43 and totals 126984/131080/139272/143368/172040/176136,
**496–497/512–513/544–545/560–561/672–673/688–689** ticks.
IRQ IRET returns to odd 2005 and gains 2*1024 clocks: 64530/70674 clocks,
**252–253/276–277** ticks. Former IRQ entry subtracts 15*1024, giving
49170/55314 clocks and **192–193/216–217** ticks. Entry itself stays 32/35.
Indirect/direct JMP, Jcc/JCXZ, LOOP, ordinary-I/O and word-transfer intervals
contain no measured returns and keep their existing bounds. Historical reports
are unchanged. Actual validation and identities: [Issue #81](issue-81-validation.md).
