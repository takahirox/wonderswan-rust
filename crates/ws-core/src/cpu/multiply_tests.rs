// SPDX-License-Identifier: CC0-1.0
// Products and complete status words are independent, hand-calculated literals.
// Literal opcode checks use instruction_step to observe FLAGS before TF entry.
// Ordinary delivery and saved arithmetic FLAGS are covered in trap_tests.
use super::*;

fn modes() -> [(Model, u8); 9] {
    [
        (Model::Mono, 0),
        (Model::Color, 0),
        (Model::Color, 0x80),
        (Model::Color, 0xc0),
        (Model::Color, 0xe0),
        (Model::Crystal, 0),
        (Model::Crystal, 0x80),
        (Model::Crystal, 0xc0),
        (Model::Crystal, 0xe0),
    ]
}

fn setup(model: Model, color: u8, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    if model != Model::Mono {
        b.output(0x60, color).unwrap();
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
                    let execution = 3;
                    let cost = c.instruction_step(&mut b).unwrap();
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
                    c.instruction_step(&mut b).unwrap();
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
                        let length = code.len();
                        code.extend([0xbe, 0xef, 0xbe]); // MOV SI,BEEF is the actual continuation
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
                        let fetch = if address.is_some() { length as u32 } else { 3 };
                        let access = fetch
                            + match address {
                                None => 0,
                                Some(0x800) => 1,
                                Some(_) => 2,
                            };
                        let cost = c.instruction_step(&mut b).unwrap();
                        assert_eq!(c.registers, expected, "{code:x?} source {source:x}");
                        assert_eq!(c.flags, control | status, "{model:?}/{color} {code:x?}");
                        assert_eq!(c.ip, 0x1000 + length as u16);
                        assert_eq!(c.segments, [0, 0, 0x20, 0x30]);
                        assert_eq!(b.accesses.get(), access);
                        assert_eq!(cost, 3 + access);
                        assert_eq!(b.cycles, 0);
                        assert!(b.ram == ram);
                        c.instruction_step(&mut b).unwrap();
                        expected[6] = 0xbeef;
                        assert_eq!(c.registers, expected);
                        assert_eq!(c.flags, control | status);
                        assert_eq!(c.ip, 0x1003 + length as u16);
                    }
                }
            }
        }
    }
}

#[test]
fn mul_runtime_mode_changes_follow_physical_model_and_shared_clock_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, b) = setup(model, 0, &[0xf6, 0xe3, 0xf7, 0xeb, 0x6b, 0xdb, 0xfe]);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            m.cpu.ip = 0x1000;
            m.cpu.registers[0] = 3;
            m.cpu.registers[3] = 2;
            m.cpu.flags = 0xffd7;
            for (cost, ip, access, flags) in [
                (
                    5,
                    0x1002,
                    2,
                    if model == Model::Mono { 0xf702 } else { 0xf742 },
                ),
                (5, 0x1004, 2, 0xf742),
                (6, 0x1007, 3, 0xf742),
            ] {
                let before = m.bus.cycles;
                let actual = m.cpu.instruction_step(&mut m.bus).unwrap();
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

// Literal pre-edit totals: two byte fetches plus three residual clocks;
// 69 has a single aligned immediate-word transfer, 6B a byte transfer.
fn literal_timing(code: &[u8], cost: u32, ax: u16, dx: u16, bx: u16, flags: u16) {
    let (mut c, mut b) = setup(Model::Mono, 0, code);
    c.registers[0] = 3;
    c.registers[3] = 2;
    c.flags = 0xffd7;
    let actual = c.instruction_step(&mut b).unwrap();
    assert_eq!(
        (
            c.registers[0],
            c.registers[2],
            c.registers[3],
            c.flags,
            c.ip
        ),
        (ax, dx, bx, flags, 0x1000 + code.len() as u16)
    );
    assert_eq!(actual, cost);
}
#[test]
fn literal_byte_mul_timing() {
    literal_timing(&[0xf6, 0xe3], 5, 6, 0xabcd, 2, 0xf702);
}
#[test]
fn literal_word_imul_timing() {
    literal_timing(&[0xf7, 0xeb], 5, 6, 0, 2, 0xf742);
}
#[test]
fn literal_immediate_byte_imul_timing() {
    literal_timing(&[0x6b, 0xdb, 0xfe], 6, 3, 0xabcd, 0xfffc, 0xf742);
}
#[test]
fn literal_immediate_word_imul_timing() {
    literal_timing(&[0x69, 0xdb, 0xfe, 0xff], 6, 3, 0xabcd, 0xfffc, 0xf742);
}

#[test]
fn literal_memory_totals_and_accumulator_source_aliases() {
    for (code, ax, dx, result, high, flags, cost) in [
        (&[0xf6, 0x27][..], 3, 0xabcd, 6, 0xabcd, 0xf002, 6),
        (&[0xf7, 0x27][..], 3, 0xabcd, 6, 0, 0xf002, 6),
        (&[0xf7, 0x2f][..], 3, 0xabcd, 6, 0, 0xf042, 6),
        (&[0xf6, 0xe0][..], 3, 0xabcd, 9, 0xabcd, 0xf002, 5), // AL sampled
        (&[0xf6, 0xe4][..], 0x0203, 0xabcd, 6, 0xabcd, 0xf002, 5), // AH sampled
        (&[0xf6, 0xec][..], 0xfffd, 0xabcd, 3, 0xabcd, 0xf042, 5), // signed AH
        (&[0xf7, 0xe0][..], 0xffff, 0xabcd, 1, 0xfffe, 0xf803, 5), // AX square
        (&[0xf7, 0xe2][..], 3, 2, 6, 0, 0xf002, 5),           // DX sampled before high write
        (&[0xf7, 0xea][..], 0xfffd, 7, 0xffeb, 0xffff, 0xf042, 5),
    ] {
        for odd in [false, true] {
            let mut followed = code.to_vec();
            followed.extend([0xb9, 0xcd, 0xab]); // MOV CX,ABCD after the aliased source
            let (mut c, mut b) = setup(Model::Mono, 0, &followed);
            c.segments[3] = 0;
            c.registers[0] = ax;
            c.registers[2] = dx;
            c.registers[3] = 0x800 + u16::from(odd);
            b.ram[0x800 + usize::from(odd)..0x802 + usize::from(odd)].copy_from_slice(&[2, 0]);
            let ram = b.ram.clone();
            let segments = c.segments;
            let ports = b.ports;
            let mut registers = c.registers;
            registers[0] = result;
            registers[2] = high;
            let extra = u32::from(odd && code[0] == 0xf7 && code[1] < 0xc0);
            assert_eq!(c.instruction_step(&mut b).unwrap(), cost + extra);
            assert_eq!(
                (c.registers[0], c.registers[2], c.flags, c.ip),
                (result, high, flags, 0x1002)
            );
            assert_eq!(c.registers, registers);
            assert_eq!(c.segments, segments);
            assert_eq!(b.ports, ports);
            assert!(b.ram == ram);
            c.instruction_step(&mut b).unwrap();
            registers[1] = 0xabcd;
            assert_eq!(c.registers, registers);
            assert_eq!((c.ip, c.flags), (0x1005, flags));
            assert!(b.ram == ram);
        }
    }
}

#[test]
fn multiplication_mapped_fetch_wait_prefix_and_addressing_accounting() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model, 0, &[]);
        mapped_code(&mut b, &[], &[]);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for wait in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                }
                for (a0, rom_byte, rom_even_word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                    b.output(0xa0, a0).unwrap();
                    let sram_byte = if model != Model::Mono && wait != 0 {
                        2
                    } else {
                        1
                    };
                    for cs in [0u16, 0x1000, 0x2000] {
                        for segment in [0x20u16, 0x1000, 0x3000] {
                            for count in 0..=7 {
                                for (opcode, extension) in [
                                    (0xf6, 4),
                                    (0xf7, 4),
                                    (0xf6, 5),
                                    (0xf7, 5),
                                    (0x69, 0),
                                    (0x6b, 0),
                                ] {
                                    for bp in [false, true] {
                                        let mut code = [0x26, 0xf0, 0x2e, 0x36, 0x3e, 0xf0, 0x26]
                                            [..count]
                                            .to_vec();
                                        code.extend([
                                            opcode,
                                            (if bp { 0x46 } else { 7 }) | extension << 3,
                                        ]);
                                        if bp {
                                            code.push(1);
                                        }
                                        let imm_at = code.len();
                                        if opcode == 0x69 {
                                            code.extend([0xfe, 0xff]);
                                        }
                                        if opcode == 0x6b {
                                            code.push(0xfe);
                                        }
                                        c.segments = [segment, cs, segment, segment];
                                        let selected = if count == 3 { cs } else { segment };
                                        c.ip = 0x1000;
                                        c.registers[0] = 3;
                                        c.registers[3] = 0x800;
                                        c.registers[5] = 0x800;
                                        c.flags = 0xf002;
                                        let addresses: Vec<u32> = (0..code.len())
                                            .map(|i| u32::from(cs) * 16 + 0x1000 + i as u32)
                                            .collect();
                                        if cs == 0x1000 {
                                            for (&a, &v) in addresses.iter().zip(&code) {
                                                b.write(a, v);
                                            }
                                        } else {
                                            mapped_code(&mut b, &addresses, &code);
                                        }
                                        let address =
                                            u32::from(selected) * 16 + 0x800 + u32::from(bp);
                                        if selected >= 0x2000 {
                                            mapped_code(&mut b, &[address, address + 1], &[2, 0]);
                                        } else {
                                            b.write(address, 2);
                                            b.write(address + 1, 0);
                                        }
                                        let ram = b.ram.clone();
                                        let save = b.cartridge.as_ref().unwrap().save.clone();
                                        let rom = b.cartridge.as_ref().unwrap().bytes.clone();
                                        let mut regs = c.registers;
                                        regs[0] = if opcode == 0x69 || opcode == 0x6b {
                                            0xfffc
                                        } else {
                                            6
                                        };
                                        if opcode == 0xf7 {
                                            regs[2] = 0;
                                        }
                                        let flags = if extension == 5
                                            || opcode < 0xf6
                                            || model != Model::Mono
                                        {
                                            0xf042
                                        } else {
                                            0xf002
                                        };
                                        let lane = if cs == 0 {
                                            1
                                        } else if cs == 0x1000 {
                                            sram_byte
                                        } else {
                                            rom_byte
                                        };
                                        let mut fetch = code.len() as u32 * lane;
                                        // 69's immediate is fetched as a word; all other bytes here are separate.
                                        if opcode == 0x69 && imm_at % 2 == 0 {
                                            if cs == 0 {
                                                fetch -= 1;
                                            } else if cs >= 0x2000 {
                                                fetch -= 2 * rom_byte - rom_even_word;
                                            }
                                        }
                                        let operand = if opcode == 0xf6 {
                                            if selected == 0x1000 {
                                                sram_byte
                                            } else if selected >= 0x2000 {
                                                rom_byte
                                            } else {
                                                1
                                            }
                                        } else if selected == 0x1000 {
                                            2 * sram_byte
                                        } else if selected >= 0x2000 {
                                            if bp {
                                                2 * rom_byte
                                            } else {
                                                rom_even_word
                                            }
                                        } else if bp {
                                            2
                                        } else {
                                            1
                                        };
                                        let cost = c.instruction_step(&mut b).unwrap();
                                        assert_eq!(
                                            b.accesses.get(),
                                            fetch + operand,
                                            "one operand read, no writes or duplicate reads"
                                        );
                                        assert_eq!(cost, fetch+operand+3,"{model:?}/{mode:x}/{wait}/{a0:x}/{cs:x}/{segment:x}/{code:x?}");
                                        assert_eq!(
                                            (c.registers, c.ip, c.flags),
                                            (regs, 0x1000 + code.len() as u16, flags)
                                        );
                                        assert_eq!(c.segments, [segment, cs, segment, segment]);
                                        assert!(b.ram == ram);
                                        assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                                        assert_eq!(b.cartridge.as_ref().unwrap().bytes, rom);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for opcode in [0xf6, 0xf7, 0x69, 0x6b] {
            for prefix in [0xf2, 0xf3] {
                c.segments[1] = 0;
                c.ip = 0x1000;
                b.ram[0x1000..0x1002].copy_from_slice(&[prefix, opcode]);
                let state = (c.registers, c.segments, c.flags, c.multiplication_overflow);
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!(
                    (c.registers, c.segments, c.flags, c.multiplication_overflow),
                    state
                );
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn multiplication_distinct_segments_and_displacements() {
    for (code, physical, access, cost) in [
        (&[0xf7, 0x27][..], 0xb00, 3, 6),
        (&[0xf7, 0x66, 0][..], 0xa00, 4, 7),
        (&[0xf7, 0xa7, 0, 1][..], 0xc00, 4, 7),
        (&[0x26, 0xf7, 0x66, 0][..], 0x900, 5, 8),
        (&[0x36, 0xf7, 0x27][..], 0xa00, 4, 7),
        (
            &[
                0x26, 0xf0, 0x36, 0x3e, 0xf0, 0x26, 0xf0, 0x69, 0x07, 0xfe, 0xff,
            ][..],
            0x900,
            12,
            15,
        ),
    ] {
        let (mut c, mut b) = setup(Model::Mono, 0, code);
        c.segments = [0x10, 0, 0x20, 0x30];
        c.registers[0] = 3;
        c.registers[3] = 0x800;
        c.registers[5] = 0x800;
        b.ram[physical..physical + 2].copy_from_slice(&[2, 0]);
        let ram = b.ram.clone();
        assert_eq!(c.instruction_step(&mut b).unwrap(), cost, "{code:x?}");
        assert_eq!(b.accesses.get(), access);
        assert_eq!(
            (c.registers[0], c.registers[2], c.flags, c.ip),
            if code.len() > 7 {
                (0xfffc, 0xabcd, 0xf042, 0x1000 + code.len() as u16)
            } else {
                (6, 0, 0xf002, 0x1000 + code.len() as u16)
            }
        );
        assert!(b.ram == ram);
    }
}

#[test]
fn multiplication_operand_offset_bank_twenty_bit_and_available_ram_boundaries() {
    use super::call_tests::mapped_code;
    for (model, mode) in modes() {
        let (mut c, mut b) = setup(model, mode, &[]);
        mapped_code(&mut b, &[], &[]);
        for (segment, offset, first, second) in [
            (0u16, 0x3ffeu16, 0x3ffeu32, 0x3fff),
            (0, 0x3fff, 0x3fff, 0x4000),
            (0, 0x8000, 0x8000, 0x8001),
            (0, 0xffff, 0xffff, 0),
            (0x1000, 0xffff, 0x1ffff, 0x10000),
            (0x0fff, 0xf, 0xffff, 0x10000),
            (0x2fff, 0xf, 0x2ffff, 0x30000),
            (0xffff, 0xf, 0xfffff, 0),
        ] {
            for (opcode, extension) in [
                (0xf6, 4),
                (0xf7, 4),
                (0xf6, 5),
                (0xf7, 5),
                (0x69, 0),
                (0x6b, 0),
            ] {
                c.segments = [0, 0, 0, segment];
                c.ip = 0x1000;
                c.registers[0] = 2;
                c.registers[2] = 0xabcd;
                c.registers[3] = offset;
                c.flags = 0xf002;
                let mut code = vec![opcode, 7 | extension << 3];
                if opcode == 0x69 {
                    code.extend([2, 0]);
                }
                if opcode == 0x6b {
                    code.push(2);
                }
                b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
                b.ram[first as usize & 0xffff] = 0x81;
                b.ram[second as usize & 0xffff] = 0x80;
                if (0x10000..0x20000).contains(&first) {
                    b.write(first, 0x81);
                }
                if (0x10000..0x20000).contains(&second) {
                    b.write(second, 0x80);
                }
                let ram = b.ram.clone();
                let save = b.cartridge.as_ref().unwrap().save.clone();
                let available = if model != Model::Mono && mode != 0 {
                    0x10000
                } else {
                    0x4000
                };
                let read = |a: u32| {
                    if a < available {
                        ram[a as usize]
                    } else if a < 0x10000 {
                        0xff
                    } else if a < 0x20000 {
                        save[a as usize & 0x7fff]
                    } else {
                        0x0f
                    }
                };
                let wide = opcode != 0xf6;
                let input = if wide {
                    u16::from_le_bytes([read(first), read(second)])
                } else {
                    u16::from(read(first))
                };
                // Whole-integer arithmetic independent of CPU multiplication and FLAGS helpers.
                let signed = extension == 5 || opcode < 0xf6;
                let value = if !signed {
                    i64::from(input)
                } else if wide {
                    i64::from(input as i16)
                } else {
                    i64::from(input as u8 as i8)
                };
                let product = 2 * value;
                let overflow = if signed {
                    if wide {
                        !(-32768..=32767).contains(&product)
                    } else {
                        !(-128..=127).contains(&product)
                    }
                } else if wide {
                    product > 65535
                } else {
                    product > 255
                };
                let flags = 0xf002
                    | if signed || model != Model::Mono {
                        0x40
                    } else {
                        0
                    }
                    | if overflow { 0x801 } else { 0 };
                let transfer =
                    if !wide || (first & 1 == 0 && second == first + 1 && second < available) {
                        1
                    } else {
                        2
                    };
                let fetch = if opcode < 0xf6 { 3 } else { 2 };
                assert_eq!(
                    c.instruction_step(&mut b).unwrap(),
                    fetch + transfer + 3,
                    "{model:?}/{mode:x}/{segment:x}/{offset:x}/{code:x?}"
                );
                assert_eq!(b.accesses.get(), fetch + transfer);
                assert_eq!(
                    (c.registers[0], c.registers[2], c.flags, c.ip),
                    (
                        product as u16,
                        if opcode == 0xf7 {
                            (product >> 16) as u16
                        } else {
                            0xabcd
                        },
                        flags,
                        0x1000 + code.len() as u16
                    )
                );
                assert!(b.ram == ram);
                assert_eq!(b.cartridge.as_ref().unwrap().save, save);
            }
        }
    }
}

#[test]
fn multiplication_fetch_offset_and_twenty_bit_wraps() {
    use super::call_tests::mapped_code;
    for (cs, ip, addresses) in [
        (0u16, 0xffffu16, [0xffff, 0, 1, 2]),
        (0x2000, 0xffff, [0x2ffff, 0x20000, 0x20001, 0x20002]),
        (0x2fff, 0xf, [0x2ffff, 0x30000, 0x30001, 0x30002]),
        (0xffff, 0xf, [0xfffff, 0, 1, 2]),
    ] {
        let (mut c, mut b) = setup(Model::Color, 0x80, &[]);
        for code in [
            &[0xf6, 0xe3][..],
            &[0xf7, 0xe3],
            &[0xf6, 0xeb],
            &[0xf7, 0xeb],
            &[0x69, 0xc3, 0xfe, 0xff],
            &[0x6b, 0xc3, 0xfe],
        ] {
            let addresses = &addresses[..code.len()];
            mapped_code(&mut b, addresses, code);
            for (a0, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
                b.output(0xa0, a0).unwrap();
                c.segments[1] = cs;
                c.ip = ip;
                c.registers[0] = 3;
                c.registers[2] = 0xabcd;
                c.registers[3] = 3;
                c.flags = 0xf002;
                let ram = b.ram.clone();
                // All 69 immediates in these literal boundary cases start odd;
                // neither cross-window nor offset-wrapped bytes consolidate.
                let fetch = addresses
                    .iter()
                    .map(|a| if *a >= 0x20000 { lane } else { 1 })
                    .sum::<u32>();
                assert_eq!(c.instruction_step(&mut b).unwrap(), fetch + 3);
                assert_eq!(b.accesses.get(), fetch);
                assert_eq!(
                    (c.registers[0], c.registers[2], c.flags, c.ip),
                    (
                        if code[0] < 0xf6 { 0xfffa } else { 9 },
                        if code[0] == 0xf7 { 0 } else { 0xabcd },
                        0xf042,
                        ip.wrapping_add(code.len() as u16)
                    )
                );
                assert!(b.ram == ram);
            }
        }
    }
}

#[test]
fn normal_run_frame_multiplications_cross_frame_with_exact_timer_audio_and_continuation() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model, 0, &[]);
        mapped_code(&mut b, &[], &[]);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            for (code, count, cost, value, flags) in [
                (
                    &[0xf6, 0xe3][..],
                    0,
                    5,
                    0x0081,
                    if model == Model::Mono { 0xf002 } else { 0xf042 },
                ),
                (&[0xf7, 0xeb][..], 0, 5, 0x8081, 0xf042),
                (&[0x6b, 0xc3, 0xfe][..], 0, 6, 0xfffe, 0xf042),
                (&[0x69, 0xc3, 0xfe, 0xff][..], 0, 6, 0xfffe, 0xf042),
                (&[0xf7, 0x2f][..], 0, 7, 0x4101, 0xf843),
            ] {
                m.cpu.ip = 0x1000;
                m.cpu.flags = 0xf002;
                m.cpu.halted = false;
                m.cpu.interrupt_shadow = 0;
                m.cpu.registers[0] = 0x8081;
                m.cpu.registers[2] = 0xabcd;
                m.cpu.registers[1] = count;
                m.cpu.registers[3] = if code[1] == 0x2f { 0x801 } else { 1 };
                m.cpu.segments = [0, 0, 0, 0];
                m.bus.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                m.bus.ram[0x1000 + code.len()] = 0x43; // poison: INC BX must not execute
                m.bus.ram[0x801..0x803].copy_from_slice(&[0x81, 0x80]);
                let ram = m.bus.ram.clone();
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                // One clock of overshoot proves atomic progression across the frame edge.
                m.advance((remaining - cost + 1) as u32);
                m.bus.output_word(0xa4, 2).unwrap();
                m.bus.output(0xa2, 1).unwrap();
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + cost);
                assert_eq!(
                    (m.frame_number(), m.bus.raster_line, m.bus.cycles % 256),
                    (frame + 1, 0, 1)
                );
                assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                assert_eq!(m.audio_samples(), [0.0, 0.0]);
                assert_eq!(
                    (
                        m.cpu.ip,
                        m.cpu.registers[0],
                        m.cpu.registers[1],
                        m.cpu.registers[3],
                        m.cpu.flags
                    ),
                    (
                        0x1000 + code.len() as u16,
                        value,
                        count,
                        if code[1] == 0x2f { 0x801 } else { 1 },
                        flags
                    )
                );
                assert_eq!(
                    m.cpu.registers[2],
                    if code[1] == 0x2f {
                        0x3f7f
                    } else if code[0] == 0xf7 {
                        0xffff
                    } else {
                        0xabcd
                    }
                );
                assert!(m.bus.ram == ram);
            }
        }
    }
}

#[test]
fn multiplications_preserve_if_shadow_tf_irq_entry_and_dma_reservations() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            // Delivery follows the complete multiplication, with separate entry charge.
            for pending in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[0xf7, 0xeb]);
                c.segments = [0, 0, 0, 0];
                c.registers[4] = 0x2800;
                c.registers[0] = 3;
                c.registers[3] = 2;
                c.registers[1] = 31;
                c.flags = if pending { 0xf302 } else { 0xf102 };
                c.interrupt_shadow = u8::from(pending);
                b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                b.ram[0x300] = 0xcf;
                b.ram[0x400] = 0xcf;
                if pending {
                    b.output(0xb2, 0x80).unwrap();
                    b.request(7);
                }
                assert_eq!(c.step(&mut b).unwrap(), 5 + if pending { 32 } else { 25 });
                assert_eq!(
                    (c.ip, c.registers[0], c.registers[4], c.flags),
                    (if pending { 0x400 } else { 0x300 }, 6, 0x27fa, 0xf042)
                );
                assert_eq!(
                    &b.ram[0x27fa..0x2800],
                    &[2, 0x10, 0, 0, 0x42, if pending { 0xf3 } else { 0xf1 }]
                );
                if pending {
                    b.output(0xb6, 0x80).unwrap();
                }
                assert_eq!(c.step(&mut b).unwrap(), 11);
                assert_eq!(
                    (c.ip, c.registers[4], c.flags),
                    (0x1002, 0x2800, if pending { 0xf342 } else { 0xf142 })
                );
            }
            for sound in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[0x6b, 0xc0, 1]);
                mapped_code(&mut b, &[], &[]);
                c.registers[0] = 0x40;
                if model == Model::Mono {
                    assert!(b.output(if sound { 0x52 } else { 0x48 }, 0x80).is_err());
                } else {
                    for (port, value) in if sound {
                        vec![(0x4a, 0), (0x4b, 8), (0x4e, 1), (0x52, 0x83)]
                    } else {
                        vec![
                            (0x40, 0),
                            (0x41, 8),
                            (0x44, 0),
                            (0x45, 9),
                            (0x46, 2),
                            (0x48, 0x80),
                        ]
                    } {
                        b.output(port, value).unwrap();
                    }
                    if sound {
                        b.tick(117);
                    }
                    if sound || mode != 0 {
                        let state = (c.registers, c.segments, c.ip, c.flags);
                        assert_eq!(c.step(&mut b).unwrap(), 7);
                        assert_eq!(b.accesses.get(), 0);
                        assert_eq!((c.registers, c.segments, c.ip, c.flags), state);
                    }
                }
                assert_eq!(c.step(&mut b).unwrap(), 6);
                assert_eq!((c.ip, c.registers[0], c.flags), (0x1003, 0x40, 0xf042));
            }
        }
    }
}

#[test]
fn multiplication_register_forms_mapped_fetches_at_both_parities() {
    use super::call_tests::mapped_code;
    for (model, mode) in modes() {
        let (mut c, mut b) = setup(model, mode, &[]);
        mapped_code(&mut b, &[], &[]);
        for wait in [0, 2] {
            if model != Model::Mono {
                b.output(0x60, mode | wait).unwrap();
            }
            for cs in [0u16, 0x1000, 0x2000] {
                for start in [0x1000u16, 0x1001] {
                    for (a0, lane, word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                        b.output(0xa0, a0).unwrap();
                        for code in [
                            &[0xf6, 0xe3][..],
                            &[0xf7, 0xe3],
                            &[0xf6, 0xeb],
                            &[0xf7, 0xeb],
                            &[0x69, 0xc3, 0xfe, 0xff],
                            &[0x6b, 0xc3, 0xfe],
                        ] {
                            let addresses: Vec<u32> = (0..code.len())
                                .map(|i| u32::from(cs) * 16 + u32::from(start) + i as u32)
                                .collect();
                            if cs == 0x1000 {
                                for (&a, &v) in addresses.iter().zip(code) {
                                    b.write(a, v);
                                }
                            } else {
                                mapped_code(&mut b, &addresses, code);
                            }
                            c.segments[1] = cs;
                            c.ip = start;
                            c.registers[0] = 3;
                            c.registers[2] = 0xabcd;
                            c.registers[3] = 2;
                            c.flags = 0xf002;
                            let byte = if cs == 0 {
                                1
                            } else if cs == 0x1000 {
                                if model != Model::Mono && wait != 0 {
                                    2
                                } else {
                                    1
                                }
                            } else {
                                lane
                            };
                            let mut fetch = code.len() as u32 * byte;
                            if code[0] == 0x69 && start & 1 == 0 {
                                if cs == 0 {
                                    fetch -= 1;
                                } else if cs >= 0x2000 {
                                    fetch -= 2 * byte - word;
                                }
                            }
                            let ram = b.ram.clone();
                            let save = b.cartridge.as_ref().unwrap().save.clone();
                            assert_eq!(c.instruction_step(&mut b).unwrap(), fetch + 3);
                            assert_eq!(b.accesses.get(), fetch);
                            let signed = code[1] == 0xeb || code[0] < 0xf6;
                            assert_eq!(
                                (c.registers[0], c.registers[2], c.flags, c.ip),
                                (
                                    if code[0] < 0xf6 { 0xfffc } else { 6 },
                                    if code[0] == 0xf7 { 0 } else { 0xabcd },
                                    if signed || model != Model::Mono {
                                        0xf042
                                    } else {
                                        0xf002
                                    },
                                    start + code.len() as u16
                                )
                            );
                            assert!(b.ram == ram);
                            assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                        }
                    }
                }
            }
        }
    }
}
