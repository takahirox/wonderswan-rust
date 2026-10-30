# Issue #91 source reconciliation and validation

Scope: [Issue #91](https://github.com/takahirox/wonderswan-rust/issues/91), read
from GitHub while open on 2026-10-30. No upstream technical diagnosis was supplied.
The source reconciliation below was written before implementing destructive
behavior. No other emulator implementation was consulted.

## Source reconciliation

Confirmed **public definitions**, distinguished from physical test results:

- [Wonderful internal register definitions](https://github.com/WonderfulToolchain/hardware-definitions/blob/fa44ee8b4a656f5382b5cbf580159c236f01c6b1/target/wswan/eeprom.xml)
  define BA data, BC command, BE ERASE/WRITE/READ at bits 6/5/4,
  READY/DONE at 1/0, and protection of bytes 60 onward at bit 7.
- [libws erase-word assembly](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/e1f2a59c5e61382b1b5339098414c143c2aea5f0/libws/src/eeprom/eeprom_erase_word.s)
  uses start/opcode 1/3 with control 40, then a software delay and READY poll.
  Its [command helper](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/e1f2a59c5e61382b1b5339098414c143c2aea5f0/libws/src/eeprom/eeprom_utils.s)
  converts byte to word addresses and shifts the start/opcode by the handle's
  address width. The [API](https://github.com/WonderfulToolchain/target-wswan-syslibs/blob/e1f2a59c5e61382b1b5339098414c143c2aea5f0/libws/include/ws/eeprom.h)
  describes an erased word as FFFF and WDS as locking writes and erases. These
  are documented software sequences, not measurements on each console model.
- The [original internal EEPROM experiment](https://github.com/asiekierka/ws-test-suite/blob/7dfa0e2e869d08386b685d6a56df0bcfaf181b47/src/mono/eeprom/internal/main.c)
  questions whether words other than zero can be erased. It substitutes WRITE
  FFFF for **all** internal erase calls, so even its word-zero pass expectations
  cannot be reported as a direct-ERASE hardware result. Its internal command
  coverage does not certify WRAL or ERAL. The definitions, library and experiment
  share authorship; they are independent of this emulator, not three independent
  physical confirmations. Generated libws port documentation repeats the register
  definitions and adds no independent evidence. WSdev returned HTTP 403 during
  this review and supplies no additional evidence here.

Plausible, deliberately limited **implementation policy**:

- ERASE at word zero is the intersection of the documented software command and
  the experiment's stated potential exception. Accept start/opcode 1/3 with six
  or ten address bits and BE control 40; write FFFF at completion only when WEN
  was set at acceptance. Keep ordinary WRITE's completion-time protection check,
  READY progression, unchanged DONE/read buffer, shared clock and lifecycle rules.
  Applying this policy to Mono, Color and Crystal is an emulator choice; the
  sources do not prove direct ERASE on all three physical models.
- Use the existing 3072-clock programming interval, including locked requests.
  libws's conservative software delay/poll is not a measured chip latency; it
  cannot determine an exact value. Reusing the bounded approximation avoids a
  second clock or invented precision. No bulk operation is scheduled.

**Unverified behavior and resolution for each operation**:

| Operation | Resolution |
| --- | --- |
| ERASE word zero | Implement real two-byte FFFF mutation under the conservative policy above. |
| ERASE nonzero | Specific unsupported diagnostic; no mutation or pending command. Do not redirect to zero or silently use WRITE. The questioned restriction is not claimed as a universal hardware rule. Ordinary WRITE FFFF remains available. |
| WRAL (opcode 0/subcommand 1) | Specific unsupported diagnostic: internal-controller support/effects unverified. No evidence establishes its control sequence, settings interaction or affected range. No write-all mutation. |
| ERAL (opcode 0/subcommand 2) | Specific unsupported diagnostic for the same evidence gap; no full-memory or user-area erase. |

A generic Microwire instruction table and cartridge controller cannot resolve
internal bulk support. Malformed formats, mixed controls and busy requests keep
explicit errors. Rejection is an emulator diagnostic policy, not a claim about
silicon's response. Precise protection races and physical timing remain unknown.

## Validation

The implementation adds a pending word-zero ERASE operation to the existing
internal controller; it does not combine controllers or introduce bulk mutation.
Start-bit/width validation precedes command decoding; unsupported nonzero ERASE,
WRAL and ERAL get separate diagnostics. The fixture counter moves from byte 00
to 02, leaving completed ERASE bytes 00/01 visible in exported/persisted records.

| Check | Actual result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 55 generated original CC0 images reproduce, including the two extended EEPROM images and eight new rejection images. |
| `python3 demo/build.py --check` | Pass: unchanged 131072-byte demo, SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 283 core unit tests, all integration tests, Wasm crate unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass. |
| `npm ci` in `web` | Pass; lockfile unchanged, audit reports zero vulnerabilities. |
| `npm run build:wasm` in `web` | Pass with installed Rustup stable/Wasm target, wasm-pack 0.14.0 and wasm-opt. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass against this worktree's production preview: three-model gameplay, input, lifecycle, recovery, audio and mobile checks. |
| `npm run test:compat` in `web` | Pass: full production Chromium suite, including EEPROM CPU/Wasm/persistence, gameplay, video, audio, both DMAs, RTC, all existing CPU/timing probes, cartridge saves and failure recovery. |
| Focused native and Chromium EEPROM checks | Pass, including the final additional assertions that erased bytes survive reopen/reload before the cartridge executes again. |

Browser suites use `WS_PREVIEW_URL=http://127.0.0.1:4189/wonderswan-rust/`
with a dedicated strict-port production preview from this worktree. The Wasm
build prepends `/opt/homebrew/opt/rustup/bin` to PATH to select the already
installed Rustup compiler/target. No agent model/provider or allowance changed.
After strengthening reopen assertions, the two native CPU integration tests,
Clippy and the focused `consoleChecks` Chromium module were rerun successfully.

## Acceptance review

| Requirement | Evidence |
| --- | --- |
| Defensible support and explicit unresolved operations | Source reconciliation above, specific non-destructive diagnostics and exact support limits in README/support/fixture documentation. Nonzero ERASE is not asserted to fail universally on hardware. |
| Decode, widths, models and actual mutation | Unit vectors on Mono/Color/Crystal in applicable six/ten-bit modes; original CPU ERASE sequences in both widths. Complete record comparisons establish exactly two changed bytes and isolation from other models. |
| Status, captured state, locks and protection | READY at 0/3071/3072 clocks, both initial DONE states, unchanged read buffer, independent data latch, changed BC/BA/video mode, BF/reserved bits, WDS/WEN and sticky BE.7 during pending operations. Rejected nonzero cases include byte 5E/60, Mono 7E and Color/Crystal 7FE. Existing WRITE completion-time protection checks remain covered. |
| Shared clock and bounded completion | Existing timer/audio/GDMA/SDMA test now runs both WRITE and ERASE, with RAM and live-wait ROM sound DMA sources on Color/Crystal. It asserts completion, scanlines, timer IRQ, DMA bytes/counters and emitted audio. Original Mono/Color/Crystal CPU probes poll through normal instruction clocks. No second clock or per-cycle erase loop is added. |
| Lifecycle and failure safety | Tests cancel an erase at clock 3071 on restart, ROM replacement, reset, model switch and successful console import. Completed erases survive lifecycle changes. Invalid ROM/import and incompatible model transitions preserve pending work; distinct complete records and cartridge save markers are compared. |
| Original CC0 fixtures | Extended settings/protected-header probe self-checks direct ERASE, lock, buffers, status and mode/protection behavior; eight new CPU OUT probes separately reject nonzero boundaries, bulk candidates, mixed controls, missing start and busy requests. Native tests check CPU markers and all console/save bytes. No commercial ROM, BIOS, real-user dump or copied external test code. |
| Real Wasm and browser persistence | Optimized bindings/Wasm execute every rejection probe on every model and compare every completed console/save byte. Production UI runs both successful cartridges, checks green/gray pass pixels and distinct model records, shared counters, retained FF erase bytes, outgoing lifecycle flush, reload and new-page reopen. Periodic persistence is disabled in that test context. Corrupt/wrong-size records are preserved; denied/full storage leaves cartridge persistence usable. |
| Scope and regressions | Existing ordinary transfers, physical capacities, settings records and cartridge controller retained. No new UI, implicit wipe, tracing framework, dependency or unrelated ROM/save changes. Full workspace tests and both complete Chromium suites pass. |

Reviewed the final source/diff and validation against the Issue and repository
review guidelines. No pre-merge blocker remains for this explicitly bounded
support policy. This is an implementation-node review, not a PR approval/merge.

The longer CPU probe needs a third native frame for a fully rendered pass pixel;
its CPU result/storage assertions already completed in the second. Early test
setup failures were corrected to respect Mono's unavailable Color port and ROM
replacement's separate cartridge-save lifecycle. Final checks above reflect the
corrected tests, not those initial failures.

Storage restrictions are injected failures. CPU execution, Wasm, pixels and
browser records are real. No physical console was tested; optional independent
measurements remain unperformed. Exact latency, direct ERASE beyond zero, bulk
support and silicon invalid-command/protection-race responses remain unverified.

## Required post-merge verification

- Pending: confirm the main GitHub Pages workflow succeeds for the merge commit;
  record the Actions run and revision.
- Pending: run the original EEPROM probe on the public player, checking supported
  operations, status, model isolation and reload persistence. Browser results
  must remain distinct from physical hardware fidelity.

These deployment-only checks are not pre-merge gates. This node does not push,
open a PR, merge, comment, or perform physical-console tests.
