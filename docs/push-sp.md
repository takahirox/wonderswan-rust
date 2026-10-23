# PUSH SP sampling (Issue #51)

The original implementation sampled every register source before decrementing
SP. That incorrectly stored incoming SP for both `54` and `FF F4`: incoming
`2000` produced bytes `00 20` at SS:1FFE. V30MZ PUSH SP must instead decrement
SP by two with 16-bit wrapping and store that updated value through SS. The
correct bytes in this example are `FE 1F`; subsequent POP AX obtains `1FFE`.

The pinned [WSCPUTest report's stack section](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md#pushpop-sp-tofrom-stack)
describes this ordering. Its independently authored
[stack experiment](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L7119)
compares a pushed-and-popped SP against incoming SP minus two. The primary
[NEC V30MZ hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
Appendix A, printed page 81 (PDF page 83), lists both register encodings and
one execution clock each. This is the rationale for correcting the old sampling
assumption. Applying the stack experiment's rule to FF F4 is an instruction-level
inference from the two encodings, rather than a separately inspected silicon
experiment for that encoding. No independently inspected per-model silicon logs
are available. These sources were inspected during implementation; no third-party
code, firmware or cartridge bytes were incorporated.

Both decoders now pass operand identity to `push_operand`. Register SP supplies
the wrapping decremented value to the existing `push` helper, which performs
the single actual SP update and existing SS word write. Other registers and
FF /6 memory sources retain their normal sampling before that write. A memory
value equal to entry SP does not trigger the exception, including when the
source aliases the destination. Segment overrides select explicit memory
sources but cannot redirect the stack write. PUSHA, immediate and segment
pushes, calls, interrupts, and other users of the value-based helper retain
their paths. FLAGS and unrelated state are preserved.

Independent unit literals cover both encodings, SP 2000/2001/0002/0000,
little-endian bytes, POP AX, exact IP/continuation, opposite FLAGS, all unrelated
registers/segments and all RAM outside the destination. SS=F100 makes the wrapped
FFFE destination visible at physical 00FFE–00FFF. Distinct ES/CS/SS/DS plus ES
overrides guard SS selection. Other register pushes, immediate/segment and
memory pushes, PUSHA's entry-SP slot at SS:1FF6 and final SP=1FF0, and POPA's
skip of the saved SP slot are checked separately. Prior POP, CALL/RET and
interrupt-frame regressions remain in the full suite.

Static tests run Mono in mono mode and Color/Crystal in 00/80/C0/E0 modes
(mono, planar 2bpp, planar 4bpp, packed 4bpp). Live Color/Crystal tests use
00 → 80 → 00 → C0 → 00 → E0 → 00. With code and stack in accessible internal
RAM, 54 still costs 3/4 total clocks for even/odd destinations; FF F4 costs 4/5.
These preserve one execution clock plus existing fetch/word-transfer charges,
not measured absolute timings. Normal `Machine::run_frame` tests place the
instruction at a frame boundary and assert exactly its returned clocks, one
frame wrap and one stereo sample, along with actual written bytes.

The original source-included CC0 [cartridge](../fixtures/push-sp.asm) repeats
both encodings and all four SP vectors under opposite arithmetic/DF inputs in
every runtime phase. Native and production Chromium/Wasm expectations compare
actual CPU-written stored/popped values, pushed/restored SP and full FLAGS
against independent literals, on all models and fresh restart. Volatile results
are cleared, saved completion is invalidated, and fresh execution/phase counters
are required; poisoned stale-success saves cannot satisfy the checks. See
[fixture layout and provenance](../fixtures/README.md#push-sp-probe-issue-51)
and [actual validation results](issue-51-validation.md).

This is bounded emulator regression coverage. Physical-console comparisons,
additional browser engines, commercial-title compatibility and human gameplay
remain unperformed. Absolute timing and single-step delivery remain separate
work. Required post-merge verification: none.
