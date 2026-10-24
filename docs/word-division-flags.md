# Successful word DIV and IDIV flags (Issue #47)

At evaluation baseline `2669967f37dbbb65410ce8dcc496c1d1689a1693`, the
successful F7 /6 and /7 path cleared OF and retained every other arithmetic
flag. Retention was an unsupported assumption. The corrected word operation
clears AF, CF, OF, PF and SF, and sets ZF exactly when the remainder is zero
and bit zero of the quotient is one. All unrelated FLAGS bits are preserved.
This includes negative odd signed quotients: −6 / 2 produces FFFD:0000 in
AX:DX and sets ZF, while a zero quotient does not set ZF. This is not ordinary
sign/zero/parity calculation on either output.

The correction applies to the emulator's Mono, Color and SwanCrystal model
configurations, in monochrome and color graphics modes and across runtime mode
changes. Both register and mapped memory divisors use the existing ordinary
decoder. No model/game-specific dispatch is needed. Operand sampling precedes
AX/DX writeback, so accumulator and high-word divisor aliases remain valid.
Quotient/remainder calculation, divide-fault classification, operand reads,
source memory, instruction continuation, transfer costs, execution costs and
shared-clock scheduling are unchanged.

## Sources and rationale

The [NEC V30MZ preliminary hardware manual](https://www.renesas.com/en/document/lbr/v30mztm-hardware-preliminary),
§3.1.5 (printed pages 27–29), specifies cleared division overflow and leaves
other division status flags undefined. Undefined does not establish preservation.
The valid OF requirement and existing regression obligations remain in force.
The manual also documents common register/memory instruction forms and even/odd
word addressing; it does not establish deterministic console-specific flags.

The independent [WSCPUTest report pinned at
245b12c06dede4a2bb9c04c942ebca2044ba4930](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/README.md)
provides the successful 32/16 unsigned and signed flag rules above. Its
[independent unsigned calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5607)
and [signed calculator](https://github.com/FluBBaOfWard/WSCPUTest/blob/245b12c06dede4a2bb9c04c942ebca2044ba4930/WSCpuTest.asm#L5822)
explicitly start successful expected flags at F202 and conditionally add 0040
for zero remainder and odd quotient, after signed result correction. F202
includes the experiment's incoming IF; this implementation preserves the
caller's unrelated control bits instead of forcing that experimental constant.
The signed calculator routes quotient magnitude 8000 to its exceptional path;
new successful-boundary vectors therefore use −32767 and −32766, leaving that
existing emulator classification outside this correction.

The report identifies unsigned multiplication ZF as a SOC difference and gives
the same word-division rules for the family. Applying those rules to all emulated
models, video modes and mapped operands combines that experiment with the
manual's common instruction forms; it is not a separately inspected per-model
or memory-operand measurement log.

These sources were inspected for this change. No physical experiment was run
and no physical-device identities or per-model measurements were independently
verified. The original CC0 fixture and literal test expectations were authored
for this repository; no external assembly or cartridge was copied.

## Bounded acceptance

Independent literal vectors cover full FLAGS, quotient and remainder for both
forms under opposite arithmetic flags and TF/IF/DF combinations. Cases include
zero, odd/even quotient, exact/inexact division, high-word unsigned dividends,
all signed operand sign combinations, negative odd exact quotients and successful
signed/unsigned boundaries. Register, AX/DX aliases and aligned/odd RAM, ROM and
SRAM sources with ES overriding a distinct DS are checked through decoding.
Tests preserve all unrelated registers, segments, RAM, ROM, saves and ports;
check exact IP, subsequent MOV and Z/C/O/S/P/signed-LT branches; and assert exact
existing access/execution charges. A normal `Machine::run_frame` test positions
each form one instruction before frame completion and verifies exactly one
34-clock advance through the shared scheduler across live mode transitions.

The original Issue #47 version of `fixtures/divide.asm` records actual CPU-written
AX, DX, full FLAGS and six branch decisions for 52 operations per mode pass. Mono runs
one pass; Color/Crystal run mono → color → mono. Native and production
Chromium/Wasm tests compare every RAM/SRAM or browser-save word against literals
on every model and after restart. Each boot clears volatile records, invalidates
saved completion and increments the retained boot counter. Poisoned observations
with stale success markers cannot satisfy the fresh execution checks.

[Issue #49](byte-idiv-flags.md) subsequently corrects successful byte IDIV
flags and extends the same original fixture while retaining the word assertions.
The 52-operation and 628-word counts above describe the Issue #47 version.

[Issue #65](byte-div-flags.md) subsequently resolves successful byte-DIV
multiplication history. Signed exceptional results, fault
flags/classification, decimal adjustment, single-step delivery and absolute
instruction timing remain separate unresolved work. Issue #47 preserved byte
and fault behavior; Issue #49 preserves byte DIV and the fault paths. Passing
these emulator tests does not establish complete CPU readiness, commercial-title
impact or commercial-game compatibility. See [actual validation](issue-47-validation.md).
