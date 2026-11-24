# Original WonderSwan boot ROM

The WonderSwan, WonderSwan Color and SwanCrystal images in [generated](generated)
are assembled from this project's independently authored [boot.asm](boot.asm).
The source, builder, generated ROMs and associated data are dedicated to the
public domain under [CC0 1.0 Universal](LICENSE). The images contain only the
original instructions and deterministic FF padding.

The reset vector and overlay placement follow [WSdev Boot ROM](https://ws.nesdev.org/wiki/Boot_ROM).
The handoff uses the public [memory map](https://ws.nesdev.org/wiki/Memory_map),
[SoC controls](https://ws.nesdev.org/wiki/SoC), [mapper](https://ws.nesdev.org/wiki/Mapper)
and [Bandai 2003 banking](https://ws.nesdev.org/wiki/Bandai_2003),
[ROM footer](https://ws.nesdev.org/wiki/ROM_header) and
[internal EEPROM interface](https://ws.nesdev.org/wiki/EEPROM).
The incoming SP and DS values follow the project’s established compatibility
vectors, based on the [pinned WSHWTest observations](https://github.com/FluBBaOfWard/WSHWTest/blob/34e33d5bfaaabb8b09fa5a557e409efe9fbfd979/README.md#start-registers).
The [Wonderful Toolchain cartridge guide](https://wonderful.asie.pl/wiki/doku.php?id=wswan%3Aguide%3Awfconfig)
also documents bank-window access and the header settings. No original
manufacturer firmware, third-party code, disassembly or assets were used.

From the repository root with Python 3 and NASM installed, run
`python3 bios/build.py` to regenerate the images and their
[SHA-256 manifest](generated/SHA256SUMS), or `python3 bios/build.py --check`
to assemble and verify the committed bytes and hashes. The mono image is
4096 bytes at FF000. Both color images are 8192 bytes at FE000. The CPU
executes a far jump from FFFF0 to offset zero of the corresponding image.
Normal native, Wasm and Pages builds embed these committed images.

The IPL initializes the stack, compatibility I/O and ROM banks, issues an
internal EEPROM WEN command, and copies an 11-byte stub to RAM. It reads the
cartridge footer through ROM0 to set settings protection and pass the ROM
controls to the stub. The stub locks out the IPL and jumps to FFFF:0000. It occupies
scratch space at 1FF0 below the incoming stack. See the
[startup and reset policy](../docs/hardware-subset.md#built-in-ipl-startup-and-reset)
for emulator timing, deterministic defaults and deferred behavior.
