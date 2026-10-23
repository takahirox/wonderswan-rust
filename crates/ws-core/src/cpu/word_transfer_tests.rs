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
    b.ports[0x60] = if color { 0x80 } else { 0 };
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    let mut c = Cpu {
        segments: [0; 4],
        ip: 0x1000,
        ..Cpu::default()
    };
    c.registers[4] = 0x2000;
    (c, b)
}

#[test]
fn literal_internal_ram_bus_costs_data_fetch_write_and_boundaries() {
    for (model, color) in modes() {
        let (mut c, mut b) = setup(model, color, &[]);
        // Values are independent of the accounting implementation. No total
        // instruction clocks here: only explicitly requested bus transfers.
        for (s, o, accessible, unavailable, value) in [
            (0, 0x0800, 1, 1, 0x1234),
            (0, 0x0801, 2, 2, 0x1234),
            (0, 0x3ffe, 1, 1, 0x1234),
            (0, 0x3fff, 2, 2, if color { 0x1234 } else { 0xff34 }),
            (0, 0x4000, 1, 2, if color { 0x1234 } else { 0xffff }),
            (0, 0xfffe, 1, 2, if color { 0x1234 } else { 0xffff }),
            (0, 0xffff, 2, 2, if color { 0x1234 } else { 0x12ff }),
            (0x0100, 0xffff, 2, 2, 0x12ff), // offset wraps SRAM -> RAM
            (0xffff, 0x000f, 2, 2, 0x12ff), // physical FFFFF -> 00000
            (0x0fff, 0x000f, 2, 2, if color { 0xff34 } else { 0xffff }),
        ] {
            let cost = if color { accessible } else { unavailable };
            let op = Operand::Memory(s, o);
            b.accesses.set(0);
            c.write(&mut b, op, true, 0x1234);
            assert_eq!(
                b.accesses.get(),
                cost,
                "write {model:?}/{color} {s:x}:{o:x}"
            );
            b.accesses.set(0);
            assert_eq!(c.read(&b, op, true), value);
            assert_eq!(b.accesses.get(), cost, "read {model:?}/{color} {s:x}:{o:x}");
            c.segments[1] = s;
            c.ip = o;
            b.accesses.set(0);
            assert_eq!(c.word(&b), value);
            assert_eq!(c.ip, o.wrapping_add(2));
            assert_eq!(
                b.accesses.get(),
                cost,
                "fetch {model:?}/{color} {s:x}:{o:x}"
            );
        }
        for offset in [0x800, 0x801] {
            b.accesses.set(0);
            c.write(&mut b, Operand::Memory(0, offset), false, 0xa5);
            assert_eq!(b.accesses.get(), 1);
            b.accesses.set(0);
            assert_eq!(c.read(&b, Operand::Memory(0, offset), false), 0xa5);
            assert_eq!(b.accesses.get(), 1);
        }
        // Adjacent byte requests cannot accidentally become a word.
        b.accesses.set(0);
        b.read(0x800);
        b.read(0x801);
        assert_eq!(b.accesses.get(), 2);
    }
}

#[test]
fn literal_instruction_totals_mov_rmw_stack_calls_interrupts_and_strings() {
    for (model, color) in modes() {
        for (address, read_cost, rmw_cost) in [(0x0800u16, 5, 7), (0x0801, 6, 9)] {
            for (opcode, expected) in [(0xa1, read_cost), (0xa3, read_cost)] {
                let [lo, hi] = address.to_le_bytes();
                let (mut c, mut b) = setup(model, color, &[opcode, lo, hi]);
                c.registers[0] = 0x3412;
                b.ram[address as usize..address as usize + 2].copy_from_slice(&[0x12, 0x34]);
                assert_eq!(c.step(&mut b).unwrap(), expected);
                assert_eq!(c.registers[0], 0x3412);
                assert_eq!(
                    &b.ram[address as usize..address as usize + 2],
                    &[0x12, 0x34]
                );
            }
            let [lo, hi] = address.to_le_bytes();
            let (mut c, mut b) = setup(model, color, &[0x83, 0x06, lo, hi, 1]);
            b.ram[address as usize] = 0xff;
            assert_eq!(c.step(&mut b).unwrap(), rmw_cost);
            assert_eq!(&b.ram[address as usize..address as usize + 2], &[0, 1]);
        }
        for (sp, push_cost, call_cost, ret_cost, irq_cost) in
            [(0x2000, 3, 5, 6, 17), (0x2001, 4, 6, 7, 20)]
        {
            let (mut c, mut b) = setup(model, color, &[0x50, 0x58, 0xe8, 0, 0, 0xc3]);
            c.registers[4] = sp;
            c.registers[0] = 0xabcd;
            assert_eq!(c.step(&mut b).unwrap(), push_cost);
            assert_eq!(c.registers[4], sp - 2);
            assert_eq!(c.step(&mut b).unwrap(), push_cost);
            assert_eq!(c.registers[0], 0xabcd);
            assert_eq!(c.step(&mut b).unwrap(), call_cost);
            assert_eq!(c.ip, 0x1005);
            assert_eq!(c.step(&mut b).unwrap(), ret_cost);
            b.ram[0x1c..0x20].copy_from_slice(&[0x00, 0x12, 0, 0]);
            b.ports[0xb4] = 0x80;
            c.flags |= IF;
            assert_eq!(c.step(&mut b).unwrap(), irq_cost);
            assert_eq!(c.ip, 0x1200);
            assert_eq!(c.registers[4], sp - 6);
            assert_eq!(c.flags & IF, 0);
        }
        for (odd, total) in [(0u16, 7), (1, 9)] {
            for reverse in [false, true] {
                let (mut c, mut b) = setup(model, color, &[0xf3, 0xa5]);
                c.registers[1] = 2;
                c.registers[6] = 0x800 + odd;
                c.registers[7] = 0x900 + odd;
                if reverse {
                    c.flags |= DF;
                }
                for _ in 0..2 {
                    let source = c.registers[6] as usize;
                    let dest = c.registers[7] as usize;
                    b.ram[source..source + 2].copy_from_slice(&[0x34, 0x12]);
                    assert_eq!(c.step(&mut b).unwrap(), total);
                    assert_eq!(&b.ram[dest..dest + 2], &[0x34, 0x12]);
                }
                assert_eq!(c.registers[1], 0);
                assert_eq!(c.ip, 0x1002);
                assert_eq!(
                    c.registers[6],
                    if reverse { 0x7fc + odd } else { 0x804 + odd }
                );
            }
        }
    }
}

#[test]
fn literal_io_costs_immediate_dx_strings_and_byte_lanes() {
    for (model, color) in modes() {
        let (_, mut b) = setup(model, color, &[]);
        b.output_word(0xa4, 0x1234).unwrap();
        b.output_word(0xa6, 0x5678).unwrap();
        for (port, data, cost) in [(0xa4, 0x1234, 1), (0xa5, 0x7812, 2)] {
            b.accesses.set(0);
            assert_eq!(b.input_word(port).unwrap(), data);
            assert_eq!(b.accesses.get(), cost, "literal I/O bus clocks");
        }
        for (port, data, cost) in [(0xa4, 0x1234, 1), (0xa5, 0xabcd, 2)] {
            b.accesses.set(0);
            b.output_word(port, data).unwrap();
            assert_eq!(b.accesses.get(), cost);
        }
        assert_eq!(&b.ports[0xa4..0xa8], &[0x34, 0xcd, 0xab, 0x56]);
        for port in [0xa4, 0xa5] {
            b.accesses.set(0);
            b.output(port, 0x5a).unwrap();
            assert_eq!(b.accesses.get(), 1);
            b.accesses.set(0);
            assert_eq!(b.input(port).unwrap(), 0x5a);
            assert_eq!(b.accesses.get(), 1);
        }
        // Every documented implemented pair, plus deliberately ineligible pairs.
        for (port, readable, writable, value) in [
            (0xa4, true, true, 0x1234),
            (0xa6, true, true, 0x1234),
            (0xa8, true, false, 0),
            (0xaa, true, false, 0),
            (0xba, true, true, 0),
            (0xbc, true, true, 0x1234),
            (0x80, true, true, 0x0234),
            (0x82, true, true, 0x0234),
            (0x84, true, true, 0x0234),
            (0x86, true, true, 0x0234),
            (0x92, true, false, 0),
            (0x96, true, false, 0),
            (0x98, true, false, 0),
            (0x9a, true, false, 0),
        ] {
            for opcode in [0xe5, 0xe7, 0xed, 0xef] {
                let output = opcode & 2 != 0;
                let (mut c, mut b) = setup(model, color, &[opcode, port as u8]);
                c.registers[2] = port;
                c.registers[0] = 0x1234;
                if writable {
                    b.output(port, 0x34).unwrap();
                    b.output(port + 1, 0x12).unwrap();
                }
                b.accesses.set(0);
                let result = c.step(&mut b);
                if (output && !writable) || (!output && !readable) {
                    assert!(result.is_err());
                } else {
                    assert_eq!(
                        result.unwrap(),
                        if opcode & 8 == 0 { 4 } else { 3 },
                        "{port:x}/{opcode:x}"
                    );
                    if !output {
                        assert_eq!(c.registers[0], value);
                    }
                }
            }
        }
        for (port, data, eligible) in [
            (0x40, 0x1234, true),
            (0x44, 0x1234, true),
            (0x46, 0x1234, true),
            (0x4a, 0x1234, true),
            (0x4e, 0x1234, true),
            (0x64, 0x1234, true),
            (0x66, 0x1234, true),
            (0x6a, 0x0034, true),
            (0x42, 0x0004, false),
            (0x4c, 0x0004, false),
            (0x50, 0x0004, false),
        ] {
            let (mut c, mut b) = setup(model, color, &[0xef, 0xed]);
            c.registers[2] = port;
            c.registers[0] = if port == 0x6a { 0x0034 } else { 0x1234 };
            if model == Model::Mono {
                assert!(c.step(&mut b).is_err());
                continue;
            }
            assert_eq!(c.step(&mut b).unwrap(), if eligible { 3 } else { 4 });
            assert_eq!(c.step(&mut b).unwrap(), if eligible { 3 } else { 4 });
            assert_eq!(c.registers[0], data);
        }
        for (port, value, cost) in [(0xa5, 0x3412, 4), (0xbe, 0, 4), (0x16, 0, 4), (0xc0, 0, 4)] {
            let (mut c, mut b) = setup(model, color, &[0xef]);
            c.registers[2] = port;
            c.registers[0] = value;
            assert_eq!(c.step(&mut b).unwrap(), cost);
        }
        // Adjacent byte OUTs and INs still each cost one I/O clock.
        let (mut c, mut b) = setup(model, color, &[0xee, 0xee, 0xec, 0xec]);
        c.registers[2] = 0xa4;
        for _ in 0..4 {
            assert_eq!(c.step(&mut b).unwrap(), 3);
        }
        for opcode in [0x6d, 0x6f] {
            for reverse in [false, true] {
                let (mut c, mut b) = setup(model, color, &[0xf3, opcode]);
                c.registers[1] = 2;
                c.registers[2] = 0xa4;
                c.registers[6] = 0x800;
                c.registers[7] = 0x900;
                b.ram[0x7fe..0x804].copy_from_slice(&[0x34, 0x12, 0x34, 0x12, 0x34, 0x12]);
                b.output_word(0xa4, 0x1234).unwrap();
                if reverse {
                    c.flags |= DF;
                }
                for _ in 0..2 {
                    assert_eq!(c.step(&mut b).unwrap(), 7);
                }
                assert_eq!(c.registers[1], 0);
                assert_eq!(
                    c.registers[6],
                    if opcode == 0x6f {
                        if reverse {
                            0x7fc
                        } else {
                            0x804
                        }
                    } else {
                        0x800
                    }
                );
                assert_eq!(
                    c.registers[7],
                    if opcode == 0x6d {
                        if reverse {
                            0x8fc
                        } else {
                            0x904
                        }
                    } else {
                        0x900
                    }
                );
                if opcode == 0x6d {
                    assert_eq!(&b.ram[0x900..0x902], &[0x34, 0x12]);
                    let second = if reverse { 0x8fe } else { 0x902 };
                    assert_eq!(&b.ram[second..second + 2], &[0x34, 0x12]);
                } else {
                    assert_eq!(b.input_word(0xa4).unwrap(), 0x1234);
                }
            }
        }
        // Low lane side effects precede high lane failure; boundaries never wrap
        // into a different register and silently become eligible.
        for port in [0xbfu16, 0xffff, 0x6a] {
            let (mut c, mut b) = setup(model, color, &[0xef]);
            c.registers[2] = port;
            c.registers[0] = 0x0134;
            if port == 0xbf {
                assert!(c.step(&mut b).is_ok());
                assert_eq!(b.ports[0xc0], 1);
            } else {
                assert!(c.step(&mut b).is_err());
                if port == 0x6a && model != Model::Mono {
                    assert_eq!(
                        b.ports[0x6a], 0x34,
                        "low lane written before high validation failure"
                    );
                    assert_eq!(b.ports[0x6b], 0);
                    assert_eq!(
                        b.accesses.get(),
                        3,
                        "failed word keeps both attempted lane charges and opcode"
                    );
                }
            }
        }
    }
}

#[test]
fn literal_costs_for_each_memory_word_string_and_direction() {
    for (model, color) in modes() {
        for (opcode, even_cost, odd_cost) in [
            (0xa5, 7, 9),
            (0xa7, 7, 9),
            (0xab, 6, 7),
            (0xad, 6, 7),
            (0xaf, 6, 7),
        ] {
            for reverse in [false, true] {
                for (odd, total) in [(0u16, even_cost), (1, odd_cost)] {
                    let (mut c, mut b) = setup(model, color, &[0xf3, opcode]);
                    b.ram[0x7fe..0x806].fill(0x34);
                    b.ram[0x8fe..0x906].fill(0x34);
                    c.registers[0] = 0x3434;
                    c.registers[1] = 2;
                    c.registers[6] = 0x800 + odd;
                    c.registers[7] = 0x900 + odd;
                    if reverse {
                        c.flags |= DF;
                    }
                    for _ in 0..2 {
                        assert_eq!(c.step(&mut b).unwrap(), total);
                    }
                    assert_eq!(c.registers[1], 0);
                    assert_eq!(c.ip, 0x1002);
                    assert_eq!(c.registers[0], 0x3434);
                    assert_eq!(&b.ram[0x900..0x902], &[0x34, 0x34]);
                }
            }
        }
    }
}

#[test]
fn literal_external_width_wait_and_boundary_vectors_stay_byte_wide_where_required() {
    for (model, color) in modes() {
        let (mut c, mut b) = setup(model, color, &[]);
        let mut cart =
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap();
        cart.bytes.fill(0x31);
        cart.save.fill(0x56);
        b.insert_cartridge(cart);
        for (control, byte_cost, even_cost, odd_cost, mixed_cost) in [
            (0, 1, 2, 2, 2),
            (4, 1, 1, 2, 2),
            (8, 2, 4, 4, 3),
            (12, 2, 2, 4, 3),
        ] {
            b.output(0xa0, control).unwrap();
            for (s, o, data, cost) in [
                (0x2000, 0, 0x3131, even_cost),
                (0x2000, 1, 0x3131, odd_cost),
                (0x2000, 0xffff, 0x3131, odd_cost), // offset wrap, no combining
                (0x2800, 0x7fff, 0x3131, odd_cost), // bank window boundary
                (0xffff, 0xf, 0x0031, mixed_cost),  // physical wrap to RAM
                (0x1fff, 0xf, 0x3156, mixed_cost),  // SRAM/ROM boundary
            ] {
                b.accesses.set(0);
                assert_eq!(c.read(&b, Operand::Memory(s, o), true), data);
                assert_eq!(b.accesses.get(), cost);
                c.segments[1] = s;
                c.ip = o;
                b.accesses.set(0);
                assert_eq!(c.word(&b), data);
                assert_eq!(b.accesses.get(), cost);
            }
            b.accesses.set(0);
            assert_eq!(b.read(0x20000), 0x31);
            assert_eq!(b.accesses.get(), byte_cost);
            // Read-only ROM write attempts retain the old byte accounting.
            b.accesses.set(0);
            c.write(&mut b, Operand::Memory(0x2000, 0), true, 0xabcd);
            assert_eq!(b.accesses.get(), odd_cost);
            assert_eq!(b.read(0x20000), 0x31);
        }
        for (wait, byte_cost, word_cost, crossing_cost) in [(0, 1, 2, 2), (2, 2, 4, 3)] {
            b.ports[0x60] = if color { 0x80 | wait } else { wait };
            for offset in [0, 1] {
                b.accesses.set(0);
                c.write(&mut b, Operand::Memory(0x1000, offset), true, 0x5656);
                assert_eq!(b.accesses.get(), word_cost);
                b.accesses.set(0);
                assert_eq!(c.read(&b, Operand::Memory(0x1000, offset), true), 0x5656);
                assert_eq!(b.accesses.get(), word_cost);
            }
            b.accesses.set(0);
            assert_eq!(b.read(0x10000), 0x56);
            assert_eq!(b.accesses.get(), byte_cost);
            b.accesses.set(0);
            assert_eq!(
                c.read(&b, Operand::Memory(0x0fff, 0xf), true),
                if color { 0x5600 } else { 0x56ff }
            );
            assert_eq!(b.accesses.get(), crossing_cost);
        }
        for (wait, word_cost, boundary_cost, failed_cost) in [(0, 2, 2, 1), (8, 4, 3, 2)] {
            b.ports[0x60] = if color { 0x80 | wait } else { wait };
            b.accesses.set(0);
            b.output_word(0xc0, 0x0201).unwrap();
            assert_eq!(b.accesses.get(), word_cost);
            b.accesses.set(0);
            assert_eq!(b.input_word(0xc0).unwrap(), 0x0201);
            assert_eq!(b.accesses.get(), word_cost);
            b.accesses.set(0);
            b.output_word(0xbf, 0x0300).unwrap();
            assert_eq!(b.accesses.get(), boundary_cost);
            assert_eq!(b.ports[0xc0], 3);
            b.accesses.set(0);
            assert!(b.output_word(0xffff, 0xffff).is_err());
            assert_eq!(b.accesses.get(), failed_cost);
            assert_eq!(
                b.ports[0], 0,
                "failed wrapped low lane cannot write port zero"
            );
        }
    }
}
