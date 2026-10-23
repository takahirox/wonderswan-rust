// SPDX-License-Identifier: CC0-1.0
// Products and complete status words are independent, hand-calculated literals.
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

// Exercise the same literal arithmetic through registers, aligned/odd RAM,
// byte-wide mapped SRAM and word-wide mapped ROM, with ES overriding DS.
// No memory destination or stack access is permitted for multiplication.
fn group(signed: bool) {
    let vectors: &[(u8, u16, u16, u16, u16, u16)] = if signed {
        &[
            (0xf6, 0, 0xff, 0, 0xabcd, 0x40),
            (0xf6, 0xa5fd, 7, 0xffeb, 0xabcd, 0x40),
            (0xf6, 127, 1, 127, 0xabcd, 0x40),
            (0xf6, 128, 1, 0xff80, 0xabcd, 0x40),
            (0xf6, 127, 2, 254, 0xabcd, 0x841),
            (0xf6, 128, 255, 128, 0xabcd, 0x841),
            (0xf6, 128, 2, 0xff00, 0xabcd, 0x841),
            (0xf7, 0, 0xffff, 0, 0, 0x40),
            (0xf7, 0xfffd, 7, 0xffeb, 0xffff, 0x40),
            (0xf7, 0x7fff, 1, 0x7fff, 0, 0x40),
            (0xf7, 0x8000, 1, 0x8000, 0xffff, 0x40),
            (0xf7, 0x7fff, 2, 0xfffe, 0, 0x841),
            (0xf7, 0x8000, 0xffff, 0x8000, 0, 0x841),
            (0xf7, 0x8000, 2, 0, 0xffff, 0x841),
            (0xf7, 0xfffe, 0xfffd, 6, 0, 0x40),
        ]
    } else {
        &[
            (0xf6, 0, 255, 0, 0xabcd, 0),
            (0xf6, 0xa503, 2, 6, 0xabcd, 0),
            (0xf6, 255, 1, 255, 0xabcd, 0),
            (0xf6, 255, 255, 0xfe01, 0xabcd, 0x801),
            (0xf7, 0, 0xffff, 0, 0, 0),
            (0xf7, 3, 2, 6, 0, 0),
            (0xf7, 0xffff, 1, 0xffff, 0, 0),
            (0xf7, 0xffff, 0xffff, 1, 0xfffe, 0x801),
            (0xf7, 0x8000, 2, 0, 1, 0x801),
        ]
    };
    for (model, color) in modes() {
        for &(opcode, ax, source, product, high, arithmetic) in vectors {
            for (incoming, control) in [(0xf002, 0xf002), (0xffd7, 0xf702)] {
                for location in 0..7 {
                    let address: u16 = 0x800 + u16::from(location % 2 == 0);
                    let mut code = if location == 0 {
                        vec![opcode, if signed { 0xeb } else { 0xe3 }]
                    } else {
                        vec![
                            0x26,
                            opcode,
                            if signed { 0x2e } else { 0x26 },
                            address as u8,
                            (address >> 8) as u8,
                        ]
                    };
                    let length = code.len();
                    code.extend([0xb9, 0xcd, 0xab]); // exact following decode
                    let (mut c, mut b) = setup(model, color, &code);
                    c.registers[0] = ax;
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
                    expected[0] = product;
                    expected[2] = high;
                    let segments = c.segments;
                    let ram = b.ram.clone();
                    let ports = b.ports;
                    let save = b.cartridge.as_ref().unwrap().save.clone();
                    let operand_cost = if location == 0 {
                        0
                    } else if opcode == 0xf6 || (location != 3 && location != 4 && address & 1 == 0)
                    {
                        1
                    } else {
                        2
                    };
                    let access = if location == 0 { 2 } else { 5 + operand_cost };
                    let execution = if opcode == 0xf6 { 12 } else { 20 };
                    let cost = c.step(&mut b).unwrap();
                    assert_eq!(
                        c.registers, expected,
                        "{model:?}/{color} {code:x?} {ax:x}*{source:x}"
                    );
                    let status = arithmetic
                        | if !signed && model != Model::Mono {
                            0x40
                        } else {
                            0
                        };
                    assert_eq!(
                        c.flags,
                        control | status,
                        "{model:?}/{color} {code:x?} incoming {incoming:x}"
                    );
                    assert_eq!(c.ip, 0x1000 + length as u16);
                    assert_eq!(c.segments, segments);
                    assert!(!c.halted);
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
                }
            }
        }
    }
}

#[test]
fn mul_full_flags_products_and_mapped_sources() {
    group(false);
}
#[test]
fn imul_full_flags_products_and_mapped_sources() {
    group(true);
}

#[test]
fn immediate_imul_full_flags_boundaries_sign_extension_and_aliases() {
    // source, literal immediate bytes, low product, arithmetic FLAGS
    for (model, color) in modes() {
        for (opcode, source, imm, result, status) in [
            (0x69, 0u16, &[0xff, 0xff][..], 0, 0x40),
            (0x69, 0xfffd, &[7, 0], 0xffeb, 0x40),
            (0x69, 0x7fff, &[1, 0], 0x7fff, 0x40),
            (0x69, 0x8000, &[1, 0], 0x8000, 0x40),
            (0x69, 0x8000, &[0xff, 0xff], 0x8000, 0x841),
            (0x69, 0x7fff, &[2, 0], 0xfffe, 0x841),
            (0x69, 0xfffe, &[0xfd, 0xff], 6, 0x40),
            (0x69, 0x100, &[0, 1], 0, 0x841),
            (0x6b, 0, &[0x80], 0, 0x40),
            (0x6b, 0xfffd, &[7], 0xffeb, 0x40),
            (0x6b, 0x7fff, &[1], 0x7fff, 0x40),
            (0x6b, 0x8000, &[1], 0x8000, 0x40),
            (0x6b, 0x8000, &[0xff], 0x8000, 0x841),
            (0x6b, 0x7fff, &[2], 0xfffe, 0x841),
            (0x6b, 0x100, &[0x80], 0x8000, 0x40),
            (0x6b, 0x101, &[0x80], 0x7f80, 0x841),
            (0x6b, 0xfffe, &[0xfd], 6, 0x40),
        ] {
            for (incoming, control) in [(0xf002, 0xf002), (0xffd7, 0xf702)] {
                for (modrm, dest) in [(0xcb, 1), (0xdb, 3), (0xc0, 0), (0xd2, 2)] {
                    for address in [None, Some(0x800u16), Some(0x801)] {
                        let mut code = if let Some(a) = address {
                            vec![
                                0x26,
                                opcode,
                                0x06 | (dest as u8) << 3,
                                a as u8,
                                (a >> 8) as u8,
                            ]
                        } else {
                            vec![opcode, modrm]
                        };
                        code.extend(imm);
                        let (mut c, mut b) = setup(model, color, &code);
                        c.flags = incoming;
                        if let Some(a) = address {
                            b.ram[a as usize..a as usize + 2]
                                .copy_from_slice(&source.to_le_bytes());
                        } else {
                            c.registers[(modrm & 7) as usize] = source;
                        }
                        let mut expected = c.registers;
                        expected[dest] = result;
                        let ram = b.ram.clone();
                        let fetch = if address.is_some() {
                            code.len() as u32
                        } else {
                            3
                        };
                        let access = fetch
                            + match address {
                                None => 0,
                                Some(0x800) => 1,
                                Some(_) => 2,
                            };
                        let cost = c.step(&mut b).unwrap();
                        assert_eq!(c.registers, expected, "{code:x?} source {source:x}");
                        assert_eq!(c.flags, control | status, "{model:?}/{color} {code:x?}");
                        assert_eq!(c.ip, 0x1000 + code.len() as u16);
                        assert_eq!(c.segments, [0, 0, 0x20, 0x30]);
                        assert_eq!(b.accesses.get(), access);
                        assert_eq!(cost, 12 + access);
                        assert_eq!(b.cycles, 0);
                        assert!(b.ram == ram);
                    }
                }
            }
        }
    }
}

#[test]
fn mul_runtime_mode_changes_follow_physical_model_and_shared_clock_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, b) = setup(model, false, &[0xf6, 0xe3, 0xf7, 0xeb, 0x6b, 0xdb, 0xfe]);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0, 0x80] {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            m.cpu.ip = 0x1000;
            m.cpu.registers[0] = 3;
            m.cpu.registers[3] = 2;
            m.cpu.flags = 0xffd7;
            for (cost, ip, access, flags) in [
                (
                    14,
                    0x1002,
                    2,
                    if model == Model::Mono { 0xf702 } else { 0xf742 },
                ),
                (22, 0x1004, 2, 0xf742),
                (15, 0x1007, 3, 0xf742),
            ] {
                let before = m.bus.cycles;
                let actual = m.cpu.step(&mut m.bus).unwrap();
                assert_eq!(actual, cost);
                assert_eq!(m.bus.accesses.get(), access);
                assert_eq!(m.cpu.ip, ip);
                assert_eq!(m.cpu.flags, flags);
                assert_eq!(m.bus.cycles, before);
                m.advance(actual);
                assert_eq!(m.bus.cycles, before + u64::from(cost));
                assert_eq!(u64::from(m.bus.raster_line), m.bus.cycles / 256);
            }
            assert_eq!(m.cpu.registers[0], 12);
            assert_eq!(m.cpu.registers[2], 0);
            assert_eq!(m.cpu.registers[3], 0xfffc);
        }
    }
}
