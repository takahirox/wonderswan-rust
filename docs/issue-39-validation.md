# Issue #39 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`515b83b95bcfce1a0c92403af0e15ab22b052bf5` (workflow checkpoints on evaluation
baseline `a665b477bb347a21285881732054252c76f410ef`). The implementation coordinator
read live Issue #39 and the development/review guidance. No remote Issue, PR,
branch or comment was modified; this node does not push or merge.

## Source reconciliation before implementation

[The transfer table](internal-word-timing.md) was written before implementation.
The NEC V30MZ hardware manual chapter 4, Table 4-1, supplies the primary CPU
interface evidence: byte/even-word/odd-word no-wait bus costs 1/1/2, odd-stack
penalties, I/O alignment and two aligned interrupt-vector words. Pinned Wonderful
timer, EEPROM, DMA, sound and Hyper Voice register definitions identify the
implemented console word pairs. Pinned ares classification corroborates internal
word width versus SRAM/cartridge I/O byte width; its wait constants are not
adopted as hardware proof. Sources were inspected, not executed on hardware.
Previously inaccessible WSdev text and search snippets establish no requirements.

The old two-byte-charge assumption is replaced for accessible internal RAM and
explicit documented internal word registers. Width is distinct from permission
to write a byte lane: counters/monitors retain read-only errors, masks remain,
reserved lanes stay reserved, and BE/BF, A2/A3, 16/17 and other unrelated byte
register pairs receive no inferred discount. Unmapped RAM retains its FF/read,
ignored-write and byte-accounting policy. These unresolved electrical/alias
questions and broad console I/O decoding are explicitly labeled, not silently
promoted to hardware facts. The browser measurement correction below preserves the relevant acceptance.

## Implementation and tests

The existing read/operand-word helper now consolidates accessible aligned RAM
as well as the existing live-A0 ROM cases. The CPU word-write helper consolidates
only internal RAM. MOV, read-modify-write, push/pop, calls/returns, vector reads,
interrupt stack entry and memory strings reach these helpers without individual
opcode timing patches. Port word helpers classify the original port/model/
direction before dispatch and execute low then high exactly once. Immediate/DX
IN/OUT and INS/OUTS share them. Failed lanes preserve errors, preceding side
effects and attempted byte charges. Byte operations remain independent.

No device scheduling or public API changes: `Cpu::step` still returns execution
plus bus costs to `Machine::advance` once. Both DMA engines retain their mapped
accesses and independent cost paths. The explicit opcode/prefix fetch
approximation is preserved; no complete prefetch or absolute instruction timing
is claimed.

Five new core tests use independent literal bus/data and approximate instruction
vectors across Mono and Color/Crystal in both mono/color modes. They cover byte,
even/odd RAM reads/writes/operand fetches; 3FFE/3FFF/4000/FFFE/FFFF availability
edges; offset and 20-bit wraps; SRAM/ROM/internal boundaries; all four live ROM
width/wait combinations; SRAM and cartridge I/O waits; and ignored ROM writes.
Instruction totals separately cover MOV, RMW, aligned/odd stacks, near call/RET,
interrupt entry, REP MOVSW/CMPSW/STOSW/LODSW/SCASW, both DF directions, immediate/DX
word I/O, REP INS/OUTS, masks, reserved lanes, read-only errors, high-lane failure
with retained low-lane effects, adjacent unrelated byte operations, cartridge
boundaries and unsupported FFFF port wrapping.

Existing BUSLOCK vectors retained all data/register/flags/interrupt assertions.
Their literal bus totals were rederived for aligned RAM operands, stack words
and immediate-word fetches. The following MOV's immediate alignment depends on
the preceding instruction's length. Seven-prefix/shadow and interrupted REP tests
still assert the entire prefix boundary, restart IP, stack and final contents.
These are required timing corrections, not relaxed behavior checks.

## Original relative timing cartridge

`fixtures/word-timing.asm` and generated cartridge are original CC0-1.0 work,
128 KiB with 32 KiB SRAM and ordinary reset/header/checksum, without imported
assets, firmware, special core paths or new dependencies. A single RAM loop
runs 4096 byte/even/odd transfers for RAM reads, RAM writes, internal IN and OUT.
Each sample keeps instruction lengths, addresses and control flow equal; opcode
changes occur only between samples. The odd I/O BB/BC lanes are valid and do not
start an EEPROM operation. BC/BD is the documented aligned command word.

The primary bus rule independently predicts byte minus aligned = 0 and odd minus
aligned = 4096 / 256 = 16 HBlank ticks. Each elapsed timer observation quantizes
two endpoints; differences of observations allow +/-2 ticks (bounds -2..2 and
14..18). Neither expectation nor tolerance was fitted to emulator output.
Common fetch/execution overhead cancels within each group; no absolute CPU or
physical pipeline accuracy is claimed.

Native execution checks CPU RAM and all fourteen mirrored SRAM words after twenty
frames on all three models, then repeats after restart. Browser execution selects
the local cartridge through the real production player/Wasm on each model,
checks all four relative result groups and the incremented boot/completion
markers after restart, and asserts zero subsequent network requests. Boot clears
volatile markers, increments the retained count and invalidates saved completion
before measuring. Old save data cannot satisfy the fresh-boot assertions.

SHA-256: `002bedf305b3fd2c4bde2db74c365d0970b4c8deb5ec8ce5e4c82dec9c64f73c`.
All 26 previous cartridges and the demo remain byte-identical.

### Baseline failures actually executed

Before implementation, ran the first three new core tests against unchanged
production code. All failed (exit 101): aligned Mono RAM word write cost 2 versus
required 1; aligned MOV total 6 versus required 5; timer word IN immediate total
5 versus required 4. Bus and approximate instruction totals are separate tests.

Also temporarily substituted only the original HEAD `bus.rs` and `cpu.rs`, ran
`cargo test -p ws-core --test word_timing --locked`, and restored both edited files
in a `finally` block. The new source-included cartridge failed on baseline Mono
with its CPU-written 0BAD self-check marker versus required 600D (exit 101).
The same test passes with the implementation on Mono/Color/Crystal and restart.
The worktree was not reset and no baseline source was committed.

During test authoring, corrected independently counted RMW totals to include the
aligned displacement fetch (7/9), RET's existing execution overhead (6/7 total),
and the valid Hyper Voice control value to keep unsupported high bits clear.
None alters the hardware transfer table or weakens acceptance.

## Browser analyser correction established by primary evidence

The first two complete compatibility attempts stopped at the existing live RMS
check. Added assertion context identified `sound-direct/color`: input RMS
`5.561359479300365e-8`, output `1.951226569361258e-8`, while the preceding real
Wasm PCM checks had passed. The old assumption was that a stereo analyser's
mono RMS measures energy even for the fixture's opposite left/right channels.
The [W3C Web Audio specification, time-domain down-mixing](https://www.w3.org/TR/webaudio/#time-domain-down-mixing)
requires the analyser to downmix to mono regardless of its channel settings;
[stereo-to-mono mixing](https://www.w3.org/TR/webaudio/#down-mixing) averages L+R.
Thus valid opposite samples cancel. The connection to this timing change is an
inference: changed instruction phases can remove brief unequal-write transitions
that let the old check pass. Such transitions are not a valid energy requirement.

The replacement requirement measures the same isolated left channel before and
after gain, using standard ChannelSplitterNode observation branches. Production
audio routing, amplitude/silence and gain tolerances, lifecycle checks, exact
left/right PCM assertions and opposite-channel checks remain intact. This fixes
a test measurement assumption rather than altering correct audio to create
artificial mono energy. Primary evidence establishes the correction without
human approval or an independent AI re-review; the source Issue is not edited
because this execution node explicitly forbids remote changes.

## Required checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 27 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 115 core tests, 17 integration tests, one adapter test, doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass, zero warnings. |
| `git diff --check` | Pass; repeated on final files before commit. |
| `npm run build:wasm` in `web` | Pass with installed Rustup toolchain PATH. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium production smoke suite. |
| `npm run test:compat` in `web` | Pass: complete production Chromium suite, including the new timing cartridge and fresh restart on all models. |

`npm ci` restored locked dependencies without lockfile changes. The first Wasm
build attempt used Homebrew Rust, whose sysroot lacks the Wasm target. Selecting
the already installed Rustup stable toolchain (which has the target) resolved it;
no model/provider or allowance change was involved. The default preview port
4173 was occupied, so this worktree's production build is served at
`http://127.0.0.1:4189/wonderswan-rust/` through `WS_PREVIEW_URL`. The production
builds finished before the full smoke suite, then the compatibility suite ran
sequentially. After the documented analyser correction, the entire compatibility
suite passed (not just the formerly failing check). Production Rust code did not change after those builds.

## Coordinator acceptance review

The implementation coordinator reviewed the final production diff, unit vectors,
assembly/generated bytes, browser assertions, documentation and actual outcomes
against every pre-merge criterion. The live-analyser measurement defect was
resolved using primary evidence; no blocking findings remain.

| Acceptance | Reviewed implementation / executed evidence |
| --- | --- |
| Source-backed transfer rules before implementation | Explicit accessible RAM/internal word pairs/SRAM/ROM table, 1/1/2 bus costs, valid byte lanes, read-only/model gates and unresolved behavior recorded before code. |
| Ordinary CPU memory/stack/vector paths | Central read/fetch/write helpers, MOV/RMW, calls/RET, vector reads plus three stack writes, all word memory strings, REP/DF and existing interrupt-shadow/BUSLOCK boundary tests pass. |
| Word I/O and preserved byte semantics | Immediate/DX and INS/OUTS share preclassified word dispatch. All explicit pairs, masks, low-before-high effects, errors, reserved bytes, independent bytes and cartridge waits are covered. |
| Independent costs, data and boundaries | Literal bus/approximate total vectors on five model/mode combinations; RAM availability, offset/physical/port wraps, mixed regions and all A0 width/wait cases. Original core and cartridge baseline failures actually executed. |
| Original cartridge, native and production Wasm | CC0 source/image/provenance; 4096-transfer calibrated RAM and valid internal I/O loops; independently derived zero/16-tick differences with quantization; fresh CPU markers/restart pass on all three models. |
| Existing hardware and browser behavior | Full workspace plus complete production smoke/compatibility suites pass. These retain live A0, startup, IRQ/REP, delayed LCD, exact PCM/Hyper Voice/GDMA/SDMA, both EEPROM controllers/SRAM, raw banking, input/frame continuity and browser save isolation/ownership checks. Shared-clock and separate DMA cost paths are unchanged. |
| Required commands | Every listed builder/format/test/Clippy/whitespace/Wasm/production/Chromium command passed. Final core tests also passed after the last vector additions. |
| Documentation and final review | Hardware matrix, explicit source/uncertainty table, fixture provenance/hash and this validation record updated. Existing cartridges/demo byte identity checked. Relative documentation file links and JavaScript syntax checks pass. |

No required post-merge verification. Physical consoles, commercial titles,
proprietary firmware/ROMs, human gameplay and another browser engine were not
used or claimed. Full prefetch, absolute instruction accuracy, electrical aliases,
ROM-width mismatch data and speculative external wait corrections remain outside
the agreed scope. No usage reset, allowance purchase or model/provider switch was
performed.
