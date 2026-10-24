# Successful byte DIV and multiplication history (Issue #65)

**Timing update (Issue #87):** The unchanged timing statements below describe
this earlier FLAGS correction. [Successful division timing](division-timing.md)
supersedes its width-only residuals with 15/17/23/24 clocks and updates the
current scheduler/trap assertions. Arithmetic, full FLAGS and history acceptance
remain applicable; historical validation reports remain unchanged.


At baseline `144eaef001109cf79d42f5b8426156fe2a6145ec`, successful unsigned
byte division (`F6 /6`) cleared OF and preserved the other incoming arithmetic
flags. There was no multiplication history. Both the preservation assumption
and the universal successful-division OF-clear assertion were incorrect for
WonderSwan byte DIV.

Successful byte DIV now clears AF/PF/SF, sets ZF exactly for a zero remainder
and an odd quotient, and restores CF and OF from the preceding explicit
multiplication's overflow result. Unrelated FLAGS are preserved. History is
private CPU state, independent of live FLAGS; POPF, arithmetic, intervening
word DIV/IDIV, byte IDIV and repeated successful byte DIV do not replace it.
Both CF and OF have the same multiplication value, so one boolean suffices.
The ordinary decoder updates it for byte/word MUL and IMUL (`F6/F7 /4`, `/5`)
and both immediate IMUL encodings (`69`, `6B`). Products, multiplication flags,
operand sampling, AL/AH writeback, IP, faults and existing execution/access
accounting retain their prior rules.

| History | Incoming FLAGS | AX / BL | Result AX | Full FLAGS |
| --- | --- | --- | --- | --- |
| Clear | F002 | 0006 / 02 | 0003 | F042 |
| Set | F002 | 0006 / 02 | 0003 | F843 |
| Clear | FFD7 | 0007 / 02 | 0103 | F702 |
| Set | FFD7 | 0007 / 02 | 0103 | FF03 |

## Evidence and reconciliation

The [NEC preliminary manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§3.1.5 (printed pp. 27–29), leaves several division flags unspecified and
specifies cleared division overflow. Unspecified does not establish retention.
The console-specific [WSCPUTest report at
245b12c06dede4a2bb9c04c942ebca2044ba4930](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#div--divu-unsigned-division-168)
reports the successful unsigned-byte rule above, including multiplication-derived
CF/OF. Its [experiment and calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5149)
change incoming FLAGS independently. The calculator seeds history with byte
`IMUL AL`, using AL=00 or 10, before arithmetic derives its expected quotient,
remainder and exact-odd ZF. This directly distinguishes history from live FLAGS.
Its F202 expected base includes the experiment's IF; the emulator preserves
caller control flags instead of imposing that constant.

The old universal OF assertion is narrowed to word DIV/IDIV and byte IDIV;
byte DIV now has independent tests of both restored OF values and branches.
The old byte-DIV preservation vector is corrected, retaining its result, state,
continuation and accounting checks. This evidence-based correction preserves
all valid word/signed requirements and does not relax acceptance to fit failures.

Byte IMUL is the directly demonstrated producer. Extending history updates to
all explicit multiplication forms listed above is an instruction-level inference
from the report's preceding-multiplication rule, not separate measured evidence.
Applicability to mapped operands, all emulated Mono/Color/SwanCrystal models
and graphics modes is likewise an instruction-level applicability inference.
No physical-console runs, independently inspected per-model measurement logs,
or device identities are claimed. Implicit conversion microcode effects remain
uncharacterized; this implementation introduces no history updates for them.

Unsigned `DIV AH` has no successful input: nonzero AH makes AX/AH at least
256, and zero AH divides by zero. Thus successful aliases cover unsigned AL
and signed AL/AH; unsigned AH explicitly retains fault coverage. This resolves
the Issue's alias wording mathematically without changing fault acceptance.

## Lifecycle policy and bounded validation

Cloning preserves history. Synthetic unloaded initialization and firmware-free
cartridge entry clear it deterministically. Reset, restart, successful cartridge
replacement and compatible model changes create fresh CPUs. Rejected loads and
model changes preserve running CPU/history and other state. Cleared initialization
is an emulator startup policy, not an established firmware-handoff value.

Independent literal native vectors cover both histories, opposing FLAGS,
zero, exact/inexact odd/even quotients and FF, register and even/odd mapped
RAM/ROM/SRAM divisors, ES overrides, unrelated state, exact IP, following MOV
and Z/C/O/S/P/signed-condition branches. Producer tests cover fit/overflow,
sign extension, aliases, replacement, intervening arithmetic and POPF, cloning,
repeated consumption and lifecycle boundaries. Runtime 00/80/00/C0/00/E0/00
mode transitions retain history. A normal machine frame boundary compares the
returned 26 clocks (24 execution plus two fetches), raster/frame and audio
advancement exactly once. Absolute timing remains approximate.

The extended original CC0 [cartridge](../fixtures/README.md#successful-division-probe-issues-47-and-49)
retains word DIV/IDIV and byte-IDIV records, adds byte DIV with both histories,
and checks cleared startup history before any producer. All three models and
fresh restarts compare actual CPU-written AX, full FLAGS, branch observations
and fresh counters in native and production Chromium/Wasm. Poisoned saved
observations and old completion cannot satisfy those comparisons.

Exceptional division flags/results and classification refinements, conversion
history effects, TF single-step delivery and absolute timing remain separate
gaps. Passing these original probes does not establish commercial-title impact
or commercial-game readiness. See [executed validation](issue-65-validation.md).
