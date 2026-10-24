# DAA/DAS results, flags and clocks (Issue #59)

The old 27/2F handlers retained incoming OF, added an extra DAS CF when the low
adjustment borrowed, and used the default one-clock execution component. The
corrected ordinary one-byte instructions use the original AL and incoming flags:

1. High adjustment is incoming CF or original AL > 99; low adjustment is incoming
   AF or the original low nibble > 9 (all instruction values here are hexadecimal).
2. Add (DAA) or subtract (DAS) the selected 60 and 06, modulo 256. Preserve AH.
3. CF and AF equal the high and low conditions, independently of adjustment
   overflow/borrow. SZP comes from final AL. OF is the signed overflow of byte
   `CMP final_AL, original_AL`, replacing incoming OF. Preserve control/fixed bits.
4. Return ten execution clocks plus the existing fetch/wait charges. The Machine
   scheduler alone advances shared devices.

For incoming AX/FLAGS 007A/F002, DAA produces 0080/F892. DAS 0000/F012 produces
00FA/F096 without CF. DAA 0000/F802 produces 0000/F046, clearing OF.

## Sources and reconciliation

The pinned independent [WSCPUTest report](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#daa--adj4a)
and [DAA calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L6186-L6234)
and [DAS calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L6350-L6398)
supply the instruction-specific flag requirements. Both calculators determine
adjustments from original AL and AF/CF, use byte CMP of final versus original AL
for OF, and obtain SZP from final AL. The raw pinned assembly was inspected,
including complete-FLAGS comparisons under contrasting inputs. The report uses
short descriptions; the calculators establish the exact CMP-based OF rule.
Their F202 constant
includes the experiment's IF; the emulator preserves its own incoming control
bits instead of copying that constant. No external implementation code was copied
into the original CC0 tests or cartridge.

The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Appendix A, Table A-1 (printed pp. 73–74), specifies ten execution clocks for
ADJ4A/ADJ4S after decoding, with no waits/hold requests and its stated conditions.
The handlers now add that component to existing fetch charges: eleven total for
unprefixed RAM/fast ROM, twelve for waited ROM. Prefix fetch costs remain in force.
This retains the repository's bounded timing policy; exact pipeline/prefetch
accuracy is not established.

These sources replace the preserved-OF, borrow-derived-CF and default-cost
assumptions. The live Issue already requires these corrected rules, so no remote
specification edit was necessary. Their application across Mono/Color/Crystal and
supported graphics modes is an instruction-level inference, without independent
per-model silicon logs. The external experiment was inspected, not executed here.
Physical-console measurements, human gameplay and commercial-title impact remain
unmeasured. Broader prefetch and exceptional arithmetic remain separate work.

## Independent literal oracle and coverage

[decimal_adjust_vectors.txt](../crates/ws-core/src/cpu/decimal_adjust_vectors.txt)
contains 2,048 original literal rows: opcode, original AL, incoming AF/CF, final AL
and full FLAGS with controls clear. Rows are ordered by 27/2F, AF/CF 00/01/10/11,
then AL 00–FF. They were calculated offline with Python integer arithmetic,
independently of production adjustment/ALU helpers. The derivation is reproducible
with this pseudocode (numbers in the code are decimal unless prefixed):

```python
high = bool(incoming & 1) or original >= 154
low = bool(incoming & 16) or original % 16 >= 10
adjustment = 96 * high + 6 * low
final = (original + adjustment if opcode == 0x27 else original - adjustment) % 256
signed = lambda byte: byte if byte < 128 else byte - 256
difference = signed(final) - signed(original)
flags = (0xf002 + high + 16 * low + 128 * (final >= 128)
         + 64 * (final == 0) + 4 * (bin(final).count('1') % 2 == 0)
         + 2048 * (difference < -128 or difference > 127))
```

Signed subtraction range supplies OF independently of the production bitwise
formula. The native test checks table ordering and completeness, then decodes
all rows in nine model/mode configurations with AH 00/AB and opposing OF/SZP and
control profiles: 73,728 executions. A separate 37-row literal suite includes all
three exact Issue reproductions, the 09/0A, 79/7A, 7F/80, 99/9A, F9/FA/FF boundaries,
CF-only, AF-only and combined adjustments. Three incoming profiles and four fetch
locations exercise 3,996 executions, checking full FLAGS, one-byte consumption,
MOV continuation, preserved unrelated state and actual fetch charges. RAM and ROM
are both checked with waits disabled/enabled; AH preservation is also exhaustive.

Normal Machine scheduling checks all 37 vectors in Mono and both seven-phase
runtime round trips (555 executions), observing eleven returned clocks, one timer
decrement, frame/raster wrap and one stereo PCM frame. A separate GDMA test checks
its seven-clock word charge preceding the eleven-clock instruction and its mode-00
gate. Prefix tests check segment override/BUSLOCK, unsupported REP policy and
adjusted full FLAGS saved through INT3/IRET, then HALT continuation, in all nine
static configurations.

The [original conversion cartridge](../fixtures/README.md#conversion-probe-issues-55-57-and-59)
adds 36 DAA/DAS vectors under contrasting incoming status/AH (72 conversions per
phase). It executes real JZ/JC/JO/JS/JP decisions and captures AX/full PUSHF before
changing flags. Native and browser tests contain independent literal observations;
neither parses cartridge expectations nor calls production helpers. Prior AAA/AAS
and AAM/AAD vectors remain unchanged. Both fresh boots clear volatile observations
and invalidate poisoned retained success. See [executed validation](issue-59-validation.md)
for baseline failures, corrected configurations reached, required commands and
remaining evidence limits.
