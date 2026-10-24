# Issue #63 validation

Implemented on 2026-10-24 in the assigned GitWeave worktree, starting at
`2f20399a08dc75b1c354085f46a461e8533c59fa`; evaluation baseline
`4b79ff5ac8612d50fc53f84fa27133f1e068efc3`. Read live GitHub Issue #63 and its
comments with `gh issue view`; the body matches the execution input and there
are no comments. No upstream diagnosis was supplied. Development flow and review
guidelines were read. No push, PR, merge, remote comment or Issue modification
was performed. The source Issue already includes the footer correction, so no
remote specification edit is needed within this node's authorization.

[RTC policies](rtc-calendar.md) record snapshot/interrupted-write decisions before
production implementation, source reconciliation, attachment, time, format and
remaining hardware gaps. The manufacturer's sections 2–4, pinned libws header and
routines, swanlink producer and independent mapper experiment were inspected.
The original-board inventory was read as direct board documentation, not proof
of a universal header-to-hardware mapping. No console pass log was inspected.

## Baseline discrimination and resolved failures

The final `rtc-calendar.ws` was executed against the exact evaluation baseline
using a temporary archive outside this artifact worktree. A native integration
test required the fresh CPU completion word and then the twelve CPU-written
calendar/status bytes for Mono mode 00. It failed (cargo exit 101):

- Actual: `00 FF FF FF FF FF FF FF FF FF FF FF`.
- Expected: `00 6A 24 02 28 03 A3 59 58 92 34 56`.

This is real CPU IN/OUT execution through the mapper, not host-generated register
results. The corrected native integration test passes three boots on all models,
requiring incremented CPU boot counters 1/2/3, with Mono 00 and both Color models 00/80/00/C0/00/E0/00. Initial CPU observations
are cold `82 00 01 01 00 00 00 00`; after a controlled 61-second interval they
are `6A 24 02 28 03 92 35 57`, read before the fixture writes new values.

During fixture development, an unconditional Color-only port write stopped Mono;
it now checks model capability. The first source data reads also incorrectly
used RAM DS; explicit CS overrides now read the source-included ROM data. The
exact-baseline discrimination above uses the final corrected fixture.
The first full native suite exposed the old broad compatibility probe's implicit
absence assumption (82 versus FF); explicit absent configuration preserves that
assertion and all original Issue #53 FF assertions. No old expected byte was
replaced with an equipped-clock expectation.

The first Wasm build selected Homebrew Rust without its Wasm target. Rebuilding
with the already installed Rustup stable toolchain and `wasm32-unknown-unknown`
target passed.
Clippy initially rejected a manual range check; the equivalent inclusive range
check now passes. The default preview port 4173 was occupied; this worktree's
production preview uses 4183 without interrupting the other server.

The first full browser compatibility run passed. Strengthening the CPU boot
counter then exposed a harness race: loading immediately after synthetic dialog
Close could race its queued close listener, pausing the new cartridge. One run
timed out awaiting Running; the isolated run caught untouched counter FFFF in the
new cartridge slot. The harness now waits for the native close event before
loading. The isolated complete calendar check passes, including fresh boot
counters and the full-storage failure branch; a final full suite rerun subsequently passed.
The production player and core were not modified to satisfy this test race.

## Acceptance evidence for Coordinator review

| Issue criterion | Implementation and automated evidence |
| --- | --- |
| Separate presence and mapper capability | Cartridge owns optional RTC independently of mapper; raw 01 defaults equipped, explicit absent keeps extended banking. Common configuration rejects RTC access. Native tests preserve raw bytes/header diagnostics; full existing SRAM/EEPROM/raw-ROM suites retain capacities, save lengths, banking and checksums. |
| Calendar/status/reset | Seven focused native tests plus independent CPU fixture cover BCD fields, 12/24-hour and PM reads, POWER/read-only/reserved masks, cold/reset distinctions, both reset directions, write-only INT state and invalid-field correction including delayed invalid-second carry. Alarm/periodic IRQ output remains unsupported. |
| Existing transport | Original native and browser `rtc.ws` tests retain full FF expectations under explicit absent configuration. Literal counts, READY masking, first-write preload, terminal consumption and 64-clock scheduled-stage approximation are preserved. Read snapshot and staged write policies are documented. |
| Controlled time | Core absolute millisecond injection with high-water rollback handling; browser synchronization on frame entry, independent timer and lifecycle flush. Partitioned calls, fractional retention, no CPU/register/cycle/frame/framebuffer/audio/console advancement and maximum-u64 bounded catch-up are tested. |
| Calendar vectors | Independent literals cover second/minute/day rollover, 2024 leap day, common February, 30/31-day months, weekday wrap, noon/midnight, 24-hour readback, status/reset and invalid fields. Two-digit-year rollover repeats 2000–2099; year 00 leap behavior is tested. |
| Retention/storage | Restart/all compatible model transitions retain completed state and cancel transport. Versioned validated core/Wasm records; browser JSON namespace keyed by raw-ROM SHA-256 independent of model and existing saves. Automated reopen/offline, cartridge isolation, absent non-destruction, malformed/unsupported/failed import, denied/full storage and failed ROM load checks. |
| CPU and production browser | Original CC0 source-included fixture uses normal bounded IN/OUT polling. Independent host assertions inspect actual CPU-written SRAM on every model/mode; restart and reopen verify initial retained reads. Stale SRAM completion is poisoned; retained CPU boot counters must advance through 1/2/3/4 across restart/model/reopen, so identical saved result bytes cannot satisfy the check. Exact final fixture fails on baseline and passes on corrected core. |
| Required checks | See executed commands below. Existing CPU/timing/interrupt, graphics/input/frame, audio/PCM, DMA, SRAM/EEPROM, firmware-free startup, browser lifecycle and ownership/provenance checks remain in the full suites. |
| Documentation and final review | Hardware table, transport history, attachment/time/storage policy, correction rationale, fixture provenance and this record updated. Coordinator must review every row and resolve findings before merge; this implementation node does not claim that downstream review has occurred. |

## Executed commands

Results below refer to the final functional implementation; subsequent test-only
strengthening (fresh CPU boot counters and full clock storage) is included in the
final native and browser compatibility checks. Build output and source
fixtures use no proprietary ROM or firmware.

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Passed. |
| `python3 demo/build.py --check` | Passed. |
| `cargo fmt --all -- --check` | Passed. |
| `cargo test --workspace --locked` | Passed on final revision; 209 tests across the workspace. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed on final revision. |
| `git diff --check` | Passed. |
| `npm ci` in web | Passed; zero reported vulnerabilities. |
| `npm run build:wasm` in web | Passed with Rustup stable bin first in PATH. |
| `npm run build` in web | Passed, TypeScript and Vite production build. |
| `npm run test:browser` in web | Passed against local production preview on 4183. |
| `npm run test:compat` in web | Passed on final revision, including fresh counters, close-event synchronization and denied/full clock storage. |

The browser suites run sequentially with
`WS_PREVIEW_URL=http://127.0.0.1:4183/wonderswan-rust/`. Calendar checks control
only host Date.now and RAF delivery; they execute the genuine production
Wasm/player and use its actual save/import/persistence paths. Additional adapter
checks instantiate the same optimized Wasm and generated bindings. There are no
CPU/register/result/storage mocks; denied/full access is fault-injected around
real localStorage. Calendar/status results are independent literal arrays.

Final calendar cartridge SHA-256:
`1079af95c7dba8fe8853c2c3235e1b8713a92c9fc523d065fce6ed1e0fc28cfa`.
The final fixture additionally records its CPU execution counter at SRAM 0100.
This revision closes a stale-result weakness in identical reopen observations;
its exact-baseline failure was rerun after adding the counter.
The original RTC cartridge remains byte-identical at
`9877e29977646ec062e9e856a42d1da045fc8f505cc57ad795a480e6d5123251`.
All other existing generated fixture bytes are unchanged.

## Limits and downstream state

No proprietary cartridge testing, commercial impact measurement, physical-console
measurement, console identity, human gameplay or human sign-off is claimed.
Other browser engines and baseline browser discrimination were not performed;
they are optional. Exact transport/snapshot/interrupted-write phases, retail
board detection, alarm/periodic interrupt delivery, cartridge IRQ wiring and
factory firmware remain visible material gaps. No post-merge check is required.
Coordinator review of every criterion remains the downstream pre-merge step.
