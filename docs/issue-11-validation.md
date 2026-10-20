# Issue #11 validation

Source: [Issue #11](https://github.com/takahirox/wonderswan-rust/issues/11).
Work performed in the assigned GitWeave worktree on 2026-10-20. No push, PR,
merge, Issue edit or external comment was made. No commercial ROM, proprietary
firmware, copied emulator implementation or downloaded game asset was used.

## Mandatory pre-merge checks

| Check | Actual result |
| --- | --- |
| `python3 demo/build.py --check` | Pass; existing demo bytes unchanged (SHA-256 34aed3c2432d18d0f3a83bae391a8dafb1fa26b558bea7c667ba3d5f0f46fa19). |
| `python3 fixtures/build.py --check` | Pass; all five committed CC0 probes reproduce from NASM/Python source. |
| `cargo fmt --all -- --check` | Pass. |
| `cargo test --workspace --locked` | Pass: 34 core unit tests, 1 compatibility integration test covering five cartridges × three models, 4 existing demo integration tests; 39 total. |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Pass. |
| `npm ci` in `web` | Pass, locked dependencies; zero reported vulnerabilities. |
| `npm run build:wasm` with rustup toolchain first on PATH | Pass; release wasm-pack/wasm-opt build. |
| `npm run build` in `web` | Pass; TypeScript no-emit checks and Vite production build. |
| `npm run test:browser` against production preview | Pass; unchanged existing suite, all three model playthroughs plus desktop/mobile and recovery checks. |
| `npm run test:compat` against production preview | Pass; save isolation/reopen/reload/banked persistence, layer colors, real audio buffers/lifecycle and unavailable-storage/audio paths. |
| `git diff --check` and relative documentation links | Pass; whitespace and all changed documentation relative links checked. |

Preview is owned by this task at
`http://127.0.0.1:4187/wonderswan-rust/`; port 4173 was already occupied and its
existing process was left alone. Browser checks use headless Chromium/Playwright.
The final suites run sequentially. Initial runs had incorrect EEPROM command,
transparent-tile and buffer-cap expectations; those assertions were corrected to
the documented values. The first new browser check could inspect stale pixels
while a file was being read/hashed; it now waits for Running and all three expected
layer colors. A concurrent run of the existing timed keyboard suite missed a
movement; the unchanged suite passed when run alone. The first Wasm command used
Homebrew Rust without the target; the README's existing rustup PATH selection
resolved it. No check has been replaced by a fabricated or ROM-specific result.

## Acceptance evidence

- **Header/mapper/save validation:** typed header plus separate field/value,
  declared/file-size and checksum errors. Tests accept every implemented save
  type only with functioning memory/protocol support. Unsupported values fail.
  Header 00 is called the common interface, not assumed physical chip identity.
- **SRAM/persistence:** all documented SRAM codes 01–05 (32, 128, 256, 512 KiB),
  byte writes, FF initialization, C1 banks, mirrors and internal-RAM separation.
  The SRAM128 cartridge saves independent bytes in banks 0/1. Native restart,
  compatible model change and save reimport tests retain bytes. The browser
  compares SHA-256 slots against independent Node hashes, tests identical
  filenames with different ROMs, renamed identical ROMs, page reopen/reload,
  pause/restart/model changes, corrupt slots and denied storage, and asserts
  local loading/persistence send no network requests.
- **EEPROM/2003:** 128/1024/2048-byte EEPROM command/data/read/status tests cover
  WEN/WDS, write latch, READ/WRITE/ERASE, WRAL/ERAL, invalid/abort and mapper-specific
  done-bit behavior. Register tests cover C0–C3 plus CA–CF/D0–D5, aliases, high
  ROM bank boundaries, GPO directions and data, read-only self mapping and
  explicit flash-write rejection. RTC transport has absent-chip FF payloads and
  byte-clock busy/ready transitions; no calendar service is claimed. A separate
  64 MiB synthetic image boots/restarts from its actual final bank.
- **CPU/interrupt/timebase:** byte ADC/SBB exhaustive result/carry/overflow/AF
  vectors; rotations/shifts, signed/unsigned multiplication/division, divide
  faults, decimal adjustment, extended strings, generic pointers/stack/LOCK,
  far control flow and INT/INTO/IRET stack/flags. Interrupt priority, request
  latching, enable/ack, IF/STI/SS shadows, HLT wake, line/VBlank and both timers
  are tested. Native deterministic frame tests pass; instruction/fetch/memory,
  ROM wait and operation costs differ (no fixed four-clock instruction model).
- **Video:** native vectors cover mono and 2bpp/4bpp color, planar/packed layout,
  banked tile access, 12-bit RGB, palette transparency, both backgrounds, scroll,
  flips, inclusive windows, sprite ordering/priority/flips/windows/32-per-line
  clipping and VBlank sprite latching. Model gates are tested. Cartridge/browser
  pixels verify green Screen 1, red Screen 2 and a blue sprite from CPU writes.
- **Sound:** native vectors test every wave channel, disabled silence, stereo
  volume, voice, sweep wrap, eight independently calculated noise state vectors,
  reset, sample clock and queue cap. Browser instrumentation observes real
  AudioContext source buffers with nonzero samples, bounded source count,
  a single reused context and pause/resume/reset/mute/visibility cleanup.
  Missing AudioContext and injected playback failures leave gameplay usable.
- **Integration/UI:** every compatibility cartridge runs on all three models,
  executing CPU → mapper/save/interrupt/input → video/audio. Existing demo
  completion/restart tests remain unchanged and pass on all models. Existing
  browser smoke covers game-first UI, controls, local ROM load/error recovery,
  debug, menus/focus, fullscreen, async loading races and mobile touch layout.
  The production `/wonderswan-rust/` asset base and Pages workflow are preserved.
- **Scope/provenance:** no emulator dependencies/framework changes. All hardware
  state and rendering/sample generation remain in Rust; storage, input, canvas
  and Web Audio remain browser adapters. [Fixture source/license](../fixtures/README.md)
  and the [exact support matrix](hardware-subset.md) document the implemented
  subset and known gaps; no blanket commercial compatibility claim is made.

## Optional validation

Physical-console comparison and external homebrew comparisons were not performed.
Headless Web Audio buffer checks establish sample delivery and lifecycle behavior,
not subjective sound quality on real speakers. Exact cycle/prefetch behavior,
LCD response and raster/subpixel accuracy are not claimed.

## Required post-merge verification — pending

1. **Pending:** verify the existing `main` Pages workflow deploys successfully;
   record its run URL, deployed commit and result.
2. **Pending:** open the deployed public emulator, locally select an original
   CC0 compatibility probe and verify save operations, color and sound; reload
   and reselect it to confirm persistence; verify Debug Mode/error recovery.

Only live deployment/site checks are deferred. This worktree/node does not push,
merge, deploy, comment, or mark the source Issue complete.
