# WonderSwan save states

The native `Machine` captures at power-on or between calls to `run_frame`.
`prepare_snapshot` fully decodes and checks a candidate against the running
cartridge. `apply_snapshot` installs it without executing the IPL or a reset.
The Wasm adapter exposes the same operations.

The payload stores the CPU registers, interrupt shadow, multiplication flag
latch and bus fetch timing, as well as RAM, I/O registers and IPL lockout. It
retains the framebuffer, pending raster row, sprite latch, scroll and window
latches, audio phases and counters, DMA, timers, latched keypad and interrupts,
and device timing. Cartridge save bytes, cartridge and console EEPROM
transfers, console protection, and the RTC registers and command transport are
included. Only the active physical console's EEPROM bytes are captured.

The loaded cartridge supplies ROM, parsed header, derived mapper window
geometry and the built-in model IPL. Video composition and translation caches,
their generation counters, and the tile usage map start fresh. The pending
raster row is still available to the delayed palette stage. Per-instruction bus
access accounting is reset at the next CPU step. Generated PCM, host input
sources, playback and display settings, and execution error messages are
omitted. The browser clears held input and queued sound and resets presentation
history and pacing on Load.

## File format version 1

`.wsstate` files begin with this 83-byte header. Envelope integers are
little-endian.

| Offset | Bytes | Field |
| --- | ---: | --- |
| 0 | 8 | ASCII magic `WSRSTATE` |
| 8 | 2 | Format version (1) |
| 10 | 1 | Physical model (0 mono, 1 Color, 2 SwanCrystal) |
| 11 | 32 | SHA-256 of the complete raw ROM file |
| 43 | 4 | Decoded length |
| 47 | 4 | Compressed length |
| 51 | 32 | SHA-256 of bytes 0 through 50 followed by the compressed payload |
| 83 | variable | One zlib stream with a Postcard-encoded Rust payload |

The first supported version caps the decoded payload at 1 MiB and the whole file
at 1,049,683 bytes. The decoder checks exact lengths, stream boundaries,
integrity, identity, storage geometry and device invariants before installation.
The payload order is defined by the Serde field declarations in `snapshot.rs`
and the device modules, including the Postcard enum discriminants. Wire layout
or incompatible emulation changes require a version bump. Keep a compatible
build and your original raw ROM to use older exports. Third-party state
formats are unsupported. Exports carry machine state and native progress.
The browser renders a fresh thumbnail on import using the current orientation.

The RTC retains its guest calendar, fractional milliseconds, alarm and
in-flight command. Applying the state rebases its host anchor to the supplied
current time, so time spent since capture does not advance the restored clock.
Future samples advance it as they normally would. The current RTC attachment
must match the captured configuration.

## Browser storage

The version 1 `wonderswan-rust:save-states` IndexedDB database scopes history by
the existing full ROM identity and the physical console model. It commits
metadata and PNG thumbnails in the same transaction as the compressed states.
The picker reads pages of 20 metadata records without fetching state payloads.
Ordinary save, clock, and physical-console keys and records retain their
existing formats in localStorage.

**Load** reads the selected state from IndexedDB, validates it and installs it
in the running machine. History grows when you choose **New state** and press
**Save**, or when you **Import** a state. Saving to a selected entry replaces
it in place.

The ordinary persistence path saves the restored cartridge data, RTC and
console EEPROM. Loading an older state can therefore replace newer native
progress. To keep your current progress, choose **New state** and press
**Save** before loading. Native storage errors still appear in the player, and
records found unreadable during ordinary native loading are protected from writes.

Data stays in this browser and site origin. One tab at a time is the supported
workflow. Quota failures, storage eviction, clearing site data, or abrupt
termination can affect persistence. Use exports to keep copies outside
browser storage. Deleting a state or removing a remembered ROM leaves
the ordinary save records available.

## Issue #150 validation

The checks used the repository's original CC0 cartridges and the local
production preview. Playwright ran headless Chromium 156.0.8078.4 on
macOS arm64. The Wasm build used Rustup Rust 1.99.0 and wasm-pack 0.14.0.

| Check | Result |
| --- | --- |
| `python3 fixtures/build.py --check` and `python3 bios/build.py --check` | Passed |
| `cargo fmt --all -- --check` and `git diff --check` | Passed |
| `cargo test --workspace --locked` | 348 passed |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | Passed |
| `npm ci`, `npm run build:wasm` and `npm run build` | Passed |
| `npm run test:browser` and `npm run test:compat` | Passed on production preview |

Replay covered all three physical models, IPL lockout, in-flight devices,
and invalid files. Browser checks used real IndexedDB, native-record failure
injection, and emulated touch in both game orientations at 375×812, 812×375
and 320×568. The Pages deployment and published smoke test are pending
post-merge verification.
