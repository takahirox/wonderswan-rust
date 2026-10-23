// SPDX-License-Identifier: CC0-1.0
use super::*;
fn modes() -> [(Model, bool); 5] {
    [
        (Model::Mono, false),
        (Model::Color, false),
        (Model::Color, true),
        (Model::Crystal, false),
        (Model::Crystal, true),
    ]
}

fn setup(model: Model, color: bool, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    if model != Model::Mono {
        b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
    }
    b.ram.fill(0xa5);
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    let c = Cpu {
        registers: [0x1234, 0x5678, 0xabcd, 0x9876, 0x2000, 0x820, 0x840, 0x860],
        segments: [0, 0, 0x20, 0x30],
        ip: 0x1000,
        ..Cpu::default()
    };
    (c, b)
}

// signed, AX, DX, divisor, quotient, remainder, literal full FLAGS for F002/FFD7.
type Vector = (bool, u16, u16, u16, u16, u16, [u16; 2]);
const VECTORS: &[Vector] = &[
    (
        false,
        0x0000,
        0x0000,
        0x0002,
        0x0000,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0006,
        0x0000,
        0x0002,
        0x0003,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        false,
        0x0008,
        0x0000,
        0x0002,
        0x0004,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0007,
        0x0000,
        0x0002,
        0x0003,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0009,
        0x0000,
        0x0002,
        0x0004,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0001,
        0x0000,
        0x0002,
        0x0000,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0000,
        0x0001,
        0x0002,
        0x8000,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        false,
        0x0002,
        0x0001,
        0x0002,
        0x8001,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        false,
        0xffff,
        0x0000,
        0x0001,
        0xffff,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        false,
        0xfffe,
        0x0001,
        0x0002,
        0xffff,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        false,
        0xffff,
        0x0001,
        0x0002,
        0xffff,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        false,
        0xffff,
        0xfffe,
        0xffff,
        0xffff,
        0xfffe,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x0000,
        0x0000,
        0x0002,
        0x0000,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x0006,
        0x0000,
        0x0002,
        0x0003,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0x0008,
        0x0000,
        0x0002,
        0x0004,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x0007,
        0x0000,
        0x0002,
        0x0003,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        true,
        0xfffa,
        0xffff,
        0x0002,
        0xfffd,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0x0006,
        0x0000,
        0xfffe,
        0xfffd,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0xfffa,
        0xffff,
        0xfffe,
        0x0003,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0xfff9,
        0xffff,
        0x0002,
        0xfffd,
        0xffff,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x0007,
        0x0000,
        0xfffe,
        0xfffd,
        0x0001,
        [0xf002, 0xf702],
    ),
    (
        true,
        0xfff9,
        0xffff,
        0xfffe,
        0x0003,
        0xffff,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x7fff,
        0x0000,
        0x0001,
        0x7fff,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0x8002,
        0xffff,
        0x0001,
        0x8002,
        0x0000,
        [0xf002, 0xf702],
    ),
    (
        true,
        0x8001,
        0xffff,
        0x0001,
        0x8001,
        0x0000,
        [0xf042, 0xf742],
    ),
    (
        true,
        0x0000,
        0x0001,
        0x0004,
        0x4000,
        0x0000,
        [0xf002, 0xf702],
    ),
];

#[test]
fn word_division_full_flags_results_mapped_sources_and_branches() {
    for (model, color) in modes() {
        for &(signed, ax, dx, source, quotient, remainder, flags) in VECTORS {
            for (pattern, incoming) in [0xf002, 0xffd7].into_iter().enumerate() {
                for location in 0..7 {
                    let opcode = 0xf7;
                    let address: u16 = 0x800 + u16::from(location % 2 == 0);
                    let mut code = if location == 0 {
                        vec![opcode, if signed { 0xfb } else { 0xf3 }]
                    } else {
                        vec![
                            0x26,
                            opcode,
                            if signed { 0x3e } else { 0x36 },
                            address as u8,
                            (address >> 8) as u8,
                        ]
                    };
                    let length = code.len();
                    code.extend([0xb9, 0xcd, 0xab, 0x74, 2, 0x90, 0x90]); // exact following decode
                    let (mut c, mut b) = setup(model, color, &code);
                    c.registers[0] = ax;
                    c.registers[2] = dx;
                    c.registers[3] = source;
                    c.flags = incoming;
                    let mut cart = crate::cartridge::Cartridge::parse(
                        &crate::cartridge::tests::rom(131072, 2, 0),
                    )
                    .unwrap();
                    let bytes = source.to_le_bytes();
                    match location {
                        1 | 2 => {
                            b.ram[address as usize..address as usize + 2].copy_from_slice(&bytes)
                        }
                        3 | 4 => {
                            c.segments[0] = 0x1000;
                            cart.save[address as usize..address as usize + 2]
                                .copy_from_slice(&bytes);
                        }
                        5 | 6 => {
                            c.segments[0] = 0xf000;
                            cart.bytes[65536 + address as usize..65538 + address as usize]
                                .copy_from_slice(&bytes);
                        }
                        _ => {}
                    }
                    b.insert_cartridge(cart);
                    b.output(0xa0, 4).unwrap(); // no ROM wait, word bus
                    let mut expected = c.registers;
                    expected[0] = quotient;
                    expected[2] = remainder;
                    let segments = c.segments;
                    let rom = b.cartridge.as_ref().unwrap().bytes.clone();
                    let ram = b.ram.clone();
                    let ports = b.ports;
                    let save = b.cartridge.as_ref().unwrap().save.clone();
                    let operand_cost = if location == 0 {
                        0
                    } else if location != 3 && location != 4 && address & 1 == 0 {
                        1
                    } else {
                        2
                    };
                    let access = if location == 0 { 2 } else { 5 + operand_cost };
                    let execution = 32;
                    let cost = c.step(&mut b).unwrap();
                    assert_eq!(
                        c.registers, expected,
                        "{model:?}/{color} {code:x?} {dx:04x}:{ax:04x}/{source:x}"
                    );
                    assert_eq!(
                        c.flags, flags[pattern],
                        "{model:?}/{color} {code:x?} incoming {incoming:x}"
                    );
                    assert_eq!(c.ip, 0x1000 + length as u16);
                    assert_eq!(c.segments, segments);
                    assert!(!c.halted);
                    assert_eq!(c.interrupt_shadow, 0);
                    assert_eq!(b.cartridge.as_ref().unwrap().bytes, rom);
                    assert_eq!(b.accesses.get(), access);
                    assert_eq!(cost, execution + access);
                    assert_eq!(b.cycles, 0);
                    assert_eq!(b.ports, ports);
                    assert!(b.ram == ram);
                    assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                    c.step(&mut b).unwrap();
                    expected[1] = 0xabcd;
                    assert_eq!(c.registers, expected);
                    assert_eq!(c.ip, 0x1000 + length as u16 + 3);
                    let saved_flags = c.flags;
                    for (jcc, taken) in [
                        (0x74, flags[0] == 0xf042),
                        (0x72, false),
                        (0x70, false),
                        (0x78, false),
                        (0x7a, false),
                        (0x7c, false),
                    ] {
                        c.ip = 0x1000 + length as u16 + 3;
                        b.ram[c.ip as usize] = jcc;
                        c.step(&mut b).unwrap();
                        assert_eq!(c.ip, 0x1000 + length as u16 + if taken { 7 } else { 5 });
                        assert_eq!(c.flags, saved_flags);
                    }
                }
            }
        }
    }
}

#[test]
fn issue_47_reproduction() {
    let (mut c, mut b) = setup(Model::Mono, false, &[0xf7, 0xf3]);
    c.registers[0] = 6;
    c.registers[2] = 0;
    c.registers[3] = 2;
    c.flags = 0xf002;
    c.step(&mut b).unwrap();
    assert_eq!((c.registers[0], c.registers[2]), (3, 0));
    assert_eq!(c.flags, 0xf042);
}

#[test]
fn accumulator_and_high_divisor_aliases_sample_before_writeback() {
    for (model, color) in modes() {
        // DIV AX and IDIV AX, successful operands only.
        for (modrm, ax, dx, quotient, remainder, flags) in [
            (0xf0, 7, 0, 1, 0, 0xf742),
            (0xf8, 0xfff9, 0xffff, 1, 0, 0xf742),
        ] {
            let (mut c, mut b) = setup(model, color, &[0xf7, modrm, 0x90]);
            c.registers[0] = ax;
            c.registers[2] = dx;
            c.flags = 0xffd7;
            let mut registers = c.registers;
            registers[0] = quotient;
            registers[2] = remainder;
            assert_eq!(c.step(&mut b).unwrap(), 34);
            assert_eq!(c.registers, registers);
            assert_eq!(c.flags, flags);
            assert_eq!(c.ip, 0x1002);
        }
        // Signed DX alias samples the negative divisor before replacing DX.
        // FFFF:FFFA / FFFF = +6, remainder zero.
        let (mut c, mut b) = setup(model, color, &[0xf7, 0xfa]);
        c.registers[0] = 0xfffa;
        c.registers[2] = 0xffff;
        c.flags = 0xffd7;
        c.step(&mut b).unwrap();
        assert_eq!((c.registers[0], c.registers[2], c.flags), (6, 0, 0xf702));
    }
}

#[test]
fn runtime_modes_and_normal_machine_scheduler_advance_division_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model, false, &[0xf7, 0xf3, 0xf7, 0xfb]);
        b.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap(),
        );
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0] {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            for (ip, ax, dx, expected) in [(0x1000, 6, 0, 3), (0x1002, 0xfffa, 0xffff, 0xfffd)] {
                m.cpu.ip = ip;
                m.cpu.registers[0] = ax;
                m.cpu.registers[2] = dx;
                m.cpu.registers[3] = 2;
                m.cpu.flags = 0xfed7;
                // One DIV's unchanged 32 execution + 2 fetch costs ends the frame.
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                m.advance((remaining - 34) as u32);
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + 34);
                assert_eq!(m.frame_number(), frame + 1);
                assert_eq!(m.bus.raster_line, 0);
                assert_eq!(m.bus.accesses.get(), 2);
                assert_eq!(m.cpu.ip, ip + 2);
                assert_eq!(
                    (m.cpu.registers[0], m.cpu.registers[2], m.cpu.flags),
                    (expected, 0, 0xf642)
                );
            }
        }
    }
}

// AX, signed byte divisor, resulting AX, full FLAGS for F002/FFD7,
// literal Z/C/O/S/P/signed-LT branch bitmap. No production flag helpers.
type ByteVector = (u16, u8, u16, [u16; 2], u16);
const BYTE_VECTORS: &[ByteVector] = &[
    (0x0000, 0x02, 0x0000, [0xf046, 0xf746], 17),
    (0x0001, 0x02, 0x0100, [0xf046, 0xf746], 17),
    (0x0006, 0x02, 0x0003, [0xf006, 0xf706], 16),
    (0x0008, 0x02, 0x0004, [0xf002, 0xf702], 0),
    (0x0007, 0x02, 0x0103, [0xf006, 0xf706], 16),
    (0xfffa, 0x02, 0x00fd, [0xf082, 0xf782], 40),
    (0x0006, 0xfe, 0x00fd, [0xf082, 0xf782], 40),
    (0xfffa, 0xfe, 0x0003, [0xf006, 0xf706], 16),
    (0xfff9, 0x02, 0xfffd, [0xf082, 0xf782], 40),
    (0x0007, 0xfe, 0x01fd, [0xf082, 0xf782], 40),
    (0xfff9, 0xfe, 0xff03, [0xf006, 0xf706], 16),
    (0x007f, 0x01, 0x007f, [0xf002, 0xf702], 0),
    (0xff81, 0x01, 0x0081, [0xf086, 0xf786], 56),
    (0xfff8, 0x02, 0x00fc, [0xf086, 0xf786], 56),
    (0x007e, 0x01, 0x007e, [0xf006, 0xf706], 16),
];

#[test]
fn byte_idiv_full_flags_results_mapped_sources_and_branches() {
    for (model, color) in modes() {
        for &(ax, source, result, flags, branches) in BYTE_VECTORS {
            for (pattern, incoming) in [0xf002, 0xffd7].into_iter().enumerate() {
                for location in 0..7 {
                    let opcode = 0xf6;
                    let address: u16 = 0x800 + u16::from(location % 2 == 0);
                    let mut code = if location == 0 {
                        vec![opcode, 0xfb]
                    } else {
                        vec![0x26, opcode, 0x3e, address as u8, (address >> 8) as u8]
                    };
                    let length = code.len();
                    code.extend([0xb9, 0xcd, 0xab, 0x74, 2, 0x90, 0x90]); // exact following decode
                    let (mut c, mut b) = setup(model, color, &code);
                    c.registers[0] = ax;
                    let dx = c.registers[2]; // IDIV byte must preserve all of DX.
                    c.registers[3] = 0x5a00 | source as u16;
                    c.flags = incoming;
                    let mut cart = crate::cartridge::Cartridge::parse(
                        &crate::cartridge::tests::rom(131072, 2, 0),
                    )
                    .unwrap();
                    let bytes = [source];
                    match location {
                        1 | 2 => {
                            b.ram[address as usize..address as usize + 1].copy_from_slice(&bytes)
                        }
                        3 | 4 => {
                            c.segments[0] = 0x1000;
                            cart.save[address as usize..address as usize + 1]
                                .copy_from_slice(&bytes);
                        }
                        5 | 6 => {
                            c.segments[0] = 0xf000;
                            cart.bytes[65536 + address as usize..65537 + address as usize]
                                .copy_from_slice(&bytes);
                        }
                        _ => {}
                    }
                    b.insert_cartridge(cart);
                    b.output(0xa0, 4).unwrap(); // no ROM wait, word bus
                    let mut expected = c.registers;
                    expected[0] = result;
                    let segments = c.segments;
                    let rom = b.cartridge.as_ref().unwrap().bytes.clone();
                    let ram = b.ram.clone();
                    let ports = b.ports;
                    let save = b.cartridge.as_ref().unwrap().save.clone();
                    // One charge per byte: 2 fetches, or prefix + opcode +
                    // ModR/M + odd displacement word (5 fetches) + operand (1).
                    let access = if location == 0 { 2 } else { 6 };
                    let execution = 24;
                    let cost = c.step(&mut b).unwrap();
                    assert_eq!(
                        c.registers, expected,
                        "{model:?}/{color} {code:x?} {dx:04x}:{ax:04x}/{source:x}"
                    );
                    assert_eq!(
                        c.flags, flags[pattern],
                        "{model:?}/{color} {code:x?} incoming {incoming:x}"
                    );
                    assert_eq!(c.ip, 0x1000 + length as u16);
                    assert_eq!(c.segments, segments);
                    assert!(!c.halted);
                    assert_eq!(c.interrupt_shadow, 0);
                    assert_eq!(b.cartridge.as_ref().unwrap().bytes, rom);
                    assert_eq!(b.accesses.get(), access);
                    assert_eq!(cost, execution + access);
                    assert_eq!(b.cycles, 0);
                    assert_eq!(b.ports, ports);
                    assert!(b.ram == ram);
                    assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                    c.step(&mut b).unwrap();
                    expected[1] = 0xabcd;
                    assert_eq!(c.registers, expected);
                    assert_eq!(c.ip, 0x1000 + length as u16 + 3);
                    let saved_flags = c.flags;
                    for (jcc, taken) in [
                        (0x74, branches & 1 != 0),
                        (0x72, branches & 2 != 0),
                        (0x70, branches & 4 != 0),
                        (0x78, branches & 8 != 0),
                        (0x7a, branches & 16 != 0),
                        (0x7c, branches & 32 != 0),
                    ] {
                        c.ip = 0x1000 + length as u16 + 3;
                        b.ram[c.ip as usize] = jcc;
                        c.step(&mut b).unwrap();
                        assert_eq!(c.ip, 0x1000 + length as u16 + if taken { 7 } else { 5 });
                        assert_eq!(c.flags, saved_flags);
                    }
                }
            }
        }
    }
}

#[test]
fn issue_49_reproduction() {
    let (mut c, mut b) = setup(Model::Mono, false, &[0xf6, 0xfb]);
    c.registers[0] = 6;
    c.registers[3] = 2;
    c.flags = 0xf002;
    c.step(&mut b).unwrap();
    assert_eq!(c.registers[0], 3);
    assert_eq!(c.flags, 0xf006);
}

#[test]
fn byte_idiv_aliases_sample_before_writeback_and_byte_div_stays_unchanged() {
    for (model, color) in modes() {
        for (modrm, ax, result, flags) in [
            (0xf8, 0x0007, 0x0001, 0xf702), // AL = +7
            (0xf8, 0xfff9, 0x0001, 0xf702), // AL = -7
            (0xfc, 0xfffa, 0x0006, 0xf706), // AH = -1
            (0xfc, 0xff81, 0x007f, 0xf702), // AH = -1, +127 result
            (0xf3, 0x0007, 0x0103, 0xf7d7), // byte DIV BL: retain prior behavior
        ] {
            let (mut c, mut b) = setup(model, color, &[0xf6, modrm, 0x90]);
            c.registers[0] = ax;
            c.registers[3] = 2;
            c.flags = 0xffd7;
            let mut registers = c.registers;
            registers[0] = result;
            let segments = c.segments;
            let ram = b.ram.clone();
            assert_eq!(c.step(&mut b).unwrap(), 26);
            assert_eq!(c.registers, registers);
            assert_eq!(c.segments, segments);
            assert!(b.ram == ram);
            assert_eq!(c.flags, flags);
            assert_eq!(c.ip, 0x1002);
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, 0x1003);
        }
    }
}

#[test]
fn byte_idiv_runtime_modes_and_normal_machine_scheduler_advance_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model, false, &[0xf6, 0xfb, 0x90]);
        b.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap(),
        );
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0] {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            for (ax, result, flags) in [
                (0x0006, 0x0003, 0xf706),
                (0xfff9, 0xfffd, 0xf782),
                (0x0001, 0x0100, 0xf746),
            ] {
                m.cpu.ip = 0x1000;
                m.cpu.registers[0] = ax;
                m.cpu.registers[3] = 2;
                m.cpu.flags = 0xffd7;
                // Compare the actual returned CPU cost with the normal scheduler.
                let mut cpu = m.cpu.clone();
                let returned = cpu.step(&mut m.bus).unwrap();
                assert_eq!(returned, 26); // unchanged 24 execution + 2 fetch
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                m.advance((remaining - returned as u64) as u32);
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples(); // discard samples from positioning the clock
                m.run_frame().unwrap();
                assert_eq!(m.audio_samples().len(), 2); // one stereo sample at frame end
                assert_eq!(m.bus.cycles, before + returned as u64);
                assert_eq!(m.frame_number(), frame + 1);
                assert_eq!(m.bus.raster_line, 0);
                assert_eq!(m.bus.accesses.get(), 2);
                assert_eq!(m.cpu.ip, 0x1002);
                assert_eq!((m.cpu.registers[0], m.cpu.flags), (result, flags));
                assert_eq!(m.cpu.registers, cpu.registers);
            }
        }
    }
}
