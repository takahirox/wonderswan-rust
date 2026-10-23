# Issue #33 validation

Executed on 2026-10-24 in the assigned GitWeave worktree, starting at
`941b76e03679f62bf071a81ae3b21ecb12e24e9c` (workflow checkpoints on evaluation
baseline `83df7ffa4c068cb30799401e3b07c6efb4c05e46`). The implementation coordinator
read the open source Issue and comments from GitHub, repository guidance, and the
pinned WSHWTest README and assembly before implementation. No remote Issue, PR,
branch or comment was modified.

## Correction and evidence limits

The old AI-authored assumption was that zero SP/DS and mandatory cartridge
initialization sufficed. The corrected requirement supplies the reported entry
values: SP=2000 on all models, DS=FF00 on WonderSwan and FE00 on Color/Crystal,
including mono-compatible cartridges on those physical color models.
The [startup policy](hardware-subset.md#firmware-free-startup-and-reset) links the
pinned primary experiment documentation and assembly and records applicability.
The experiment stores SP before replacing it, but later overwrites that stored
value with its new SP; the SP requirement therefore rests on the author's reported
observation. DS is captured before initialization. SS is overwritten first and
cannot be established from that capture. No Issue requirement was weakened or
changed, and no uncertain startup I/O tables were adopted.

These sources were inspected through GitHub, not executed on a physical console.
Other CPU defaults, RAM and device initialization remain explicitly synthetic.
No firmware bytes or services are supplied at the incoming DS base. Complete
authentic post-firmware state, authentication and broader firmware services remain
unimplemented. Commercial-title impact is unmeasured.

## Implementation and independent tests

One shared machine handoff inserts the cartridge using the existing mapper,
header-seeded A0 and internal EEPROM policy, then installs a fresh CPU with the
bounded SP/DS values. Load, restart and compatible model changes all use it.
`Cpu::default`, unloaded new/reset/restart/model changes keep zero SP/DS.
FFFF:0000, footer execution and all other deterministic defaults are preserved.
No core/browser API, dependency, persistence implementation or timing logic changed.

Two new unit tests cover unloaded lifecycle state and literal entry vectors for
every supported physical-model/header combination, both mapper interfaces, all
four A0 width/wait combinations and both EEPROM protection header values. Each
successful load, restart, same-model restart, replacement and directed compatible
model change starts with previously changed SP/DS. Tests assert untouched CPU
defaults, mapper windows, header controls, protection, fresh versus retained SRAM,
and independent retained console records. Existing failed-load/incompatible-change
tests now deliberately change SP/DS and compare the full CPU snapshot, in addition
to their live RAM, ports, cycles, cartridge/save and console-record assertions.

`fixtures/startup.asm` and four generated 128 KiB images are original CC0-1.0
source/data. The images combine mono-compatible/Color-only headers with
common/Bandai 2003 mappers. They execute the normal footer jump and CPU stores,
establish ES=0 explicitly, and capture SP and DS before replacing either. Literal
self-checks and independent host literals assert SP=2000 and DS=FF00/FE00.
RAM and SRAM expose those raw captures, success, incoming A0 and final-bank CS.
The probe ends with SP=3456, DS=1234 and HLT to detect stale values on later entry.
Its retained SRAM boot count distinguishes fresh runs from old saved markers.

The native integration test checks all images/models, load, restart, same-model
restart, cross-cartridge replacements on each model and every directed compatible
transition. The production-browser module uses actual file chooser loads, player
model/Restart controls and real save flushes across two replacement passes. It
also executes the compiled production Wasm adapter separately for each image/model,
its direct restart API, replacements and every directed compatible transition.
Both paths check CPU-written captures, rather than replacing machine state or
injecting expected register values. Cartridge/save bytes stay browser-local.

| New cartridge | SHA-256 |
| --- | --- |
| `startup-mono-common.ws` | `f366c872fbfcb6d280d7adc3e1884149afd564397aadd328dd2fceb15a6c5798` |
| `startup-mono-2003.ws` | `256234ec047d0d4b92421404dec9e5bb2fac2a08afaa19d3fb444e799b61b6b3` |
| `startup-color-common.ws` | `8e8cdcb399a113fe692b4edb56bfac85ed22b9eea3c3f3651aa3e0e1a778741c` |
| `startup-color-2003.ws` | `b2b2a5776f5374d731a556f7d96ee1a8139f3a67040a067433228b79ce7fb633` |

All 20 existing fixture cartridges and the playable demo remain byte-identical.
Fixture provenance, startup documentation, API comments and README are updated.
No downloaded cartridge, proprietary firmware, commercial asset, copied emulator
implementation or title-specific path was added.

## Checks actually run

| Command | Result |
| --- | --- |
| `python3 fixtures/build.py --check` | Pass: all 24 cartridges. |
| `python3 demo/build.py --check` | Pass: unchanged demo SHA-256 `34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19`. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 99 core unit tests, 14 integration tests, one adapter unit test and doc tests. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `git diff --check` | Pass, including the final validation record and staged diff before commit. |
| `npm run build:wasm` in `web` | Pass using the README Rustup PATH selection. |
| `npm run build` in `web` | Pass: TypeScript and production Vite build. |
| `npm run test:browser` in `web` | Pass: complete Chromium suite, including three-model gameplay, input, lifecycle, audio, mobile/fullscreen and recovery. |
| `npm run test:compat` in `web` | Pass: complete Chromium suite, including the new SP/DS probes and all existing compatibility/persistence checks. |

`npm ci` restored locked dependencies without lockfile changes. The production
preview is `http://127.0.0.1:4291/wonderswan-rust/`, supplied through
`WS_PREVIEW_URL`. A targeted production-browser run of the new probe passed before
the full suites. Both complete suites then ran sequentially against that preview
after the production build and exited successfully. JavaScript syntax, all 46
relative Markdown file targets in the four changed documents, and whitespace
checks passed. No native, build or browser failures occurred.

## Coordinator source-Issue review

The implementation coordinator reviewed the final diff, original fixture source,
regenerated bytes, test assertions and actual validation against every acceptance
item under the repository review guidelines:

- Explicit cartridge handoff supplies the literal physical-model SP/DS values
  before execution; unloaded defaults, FFFF:0000, footer execution and common/
  extended mapper state remain distinct and covered.
- Literal lifecycle tests cover every model, both compatible header types,
  changed SP/DS, restart, replacement, same-model restart and all directed
  compatible transitions, with native CPU and production Wasm marker checks.
- Failed loads and incompatible model changes preserve live CPU/bus/save/console
  state. A0 header controls, internal EEPROM protection, cartridge saves and
  independent retained console records are covered by new and existing tests.
- Original source-included CC0 probes capture both incoming registers through
  explicit destination addressing and execute through normal cartridge banking.
  Real browser file loads, persistence and adapter execution observe CPU-written
  values against independent literals.
- All required native/build commands and both complete sequential production
  Chromium suites passed. Existing gameplay, audio/PCM, GDMA/SDMA/Hyper Voice,
  timing, input/orientation, SRAM/cartridge EEPROM/internal EEPROM and browser
  persistence checks pass.
- Startup documentation, API comments and fixture provenance record the bounded
  correction, primary-source applicability, synthetic defaults and unresolved
  firmware/authentication limitations. Core/browser ownership and local ROM/save
  handling are preserved; no unrelated subsystem or dependency was added.

No blocking findings remain and no mandatory checks were skipped.

Optional additional startup experiments, another browser engine, physical-device
execution, commercial ROMs and human gameplay were not performed. Required
post-merge verification: none. This node does not push, open a PR, merge or comment.
