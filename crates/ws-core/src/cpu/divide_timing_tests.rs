// SPDX-License-Identifier: CC0-1.0
use super::*;

fn setup(model: Model, mode: u8, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    if model != Model::Mono {
        b.output(0x60, mode).unwrap();
    }
    b.ram.fill(0xa5);
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    let c = Cpu {
        registers: [7, 0x5678, 0, 2, 0x2000, 0x820, 0x840, 0x860],
        segments: [0, 0, 0, 0],
        ip: 0x1000,
        flags: 0xf002,
        ..Cpu::default()
    };
    (c, b)
}
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

// Literal totals and complete architectural state, without production timing helpers.
fn successful(opcode: u8, extension: u8, totals: [u32; 3], ax: u16, dx: u16, flags: u16) {
    for (model, mode) in modes() {
        for (profile, total) in totals.into_iter().enumerate() {
            let code = [
                opcode,
                if profile == 0 { 0xc3 } else { 7 } | extension << 3,
                0xb9,
                0xcd,
                0xab,
            ];
            let (mut c, mut b) = setup(model, mode, &code);
            c.registers[3] = if profile == 0 {
                2
            } else {
                0x800 + u16::from(profile == 2)
            };
            b.ram[0x800..0x803].copy_from_slice(&[2, 2, 0]);
            if profile == 1 {
                b.ram[0x801] = 0;
            }
            let mut expected = c.registers;
            expected[0] = ax;
            expected[2] = dx;
            let ram = b.ram.clone();
            let ports = b.ports;
            let segments = c.segments;
            let cost = c.instruction_step(&mut b).unwrap();
            assert_eq!(c.registers, expected);
            assert_eq!(c.flags, flags);
            assert_eq!(c.ip, 0x1002);
            assert_eq!(c.segments, segments);
            assert_eq!(b.ports, ports);
            assert!(b.ram == ram);
            assert_eq!(b.accesses.get(), total - totals[0] + 2);
            assert_eq!(cost, total, "{model:?}/{mode:x}/profile {profile}");
            c.instruction_step(&mut b).unwrap();
            expected[1] = 0xabcd;
            assert_eq!(c.registers, expected);
            assert_eq!(c.ip, 0x1005);
        }
    }
}
#[test]
fn successful_byte_div_literal_totals_and_state() {
    successful(0xf6, 6, [17, 18, 18], 0x0103, 0, 0xf002);
}
#[test]
fn successful_byte_idiv_literal_totals_and_state() {
    successful(0xf6, 7, [19, 20, 20], 0x0103, 0, 0xf006);
}
#[test]
fn successful_word_div_literal_totals_and_state() {
    successful(0xf7, 6, [25, 26, 27], 3, 1, 0xf002);
}
#[test]
fn successful_word_idiv_literal_totals_and_state() {
    successful(0xf7, 7, [26, 27, 28], 3, 1, 0xf002);
}

#[test]
fn division_mapped_fetch_wait_prefix_and_addressing_accounting() {
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
                                for (opcode, extension) in
                                    [(0xf6, 6), (0xf7, 6), (0xf6, 7), (0xf7, 7)]
                                {
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
                                        c.segments = [segment, cs, segment, segment];
                                        let selected = if count == 3 { cs } else { segment };
                                        c.ip = 0x1000;
                                        c.registers[0] = 7;
                                        c.registers[2] = 0;
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
                                        regs[0] = if opcode == 0xf6 { 0x0103 } else { 3 };
                                        if opcode == 0xf7 {
                                            regs[2] = 1;
                                        }
                                        let flags = if opcode == 0xf6 && extension == 7 {
                                            0xf006
                                        } else {
                                            0xf002
                                        };
                                        let residual = match (opcode, extension) {
                                            (0xf6, 6) => 15,
                                            (0xf6, 7) => 17,
                                            (0xf7, 6) => 23,
                                            _ => 24,
                                        };
                                        let lane = if cs == 0 {
                                            1
                                        } else if cs == 0x1000 {
                                            sram_byte
                                        } else {
                                            rom_byte
                                        };
                                        // All F6/F7 instruction bytes are separately fetched.
                                        let fetch = code.len() as u32 * lane;
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
                                        assert_eq!(cost, fetch+operand+residual,"{model:?}/{mode:x}/{wait}/{a0:x}/{cs:x}/{segment:x}/{code:x?}");
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
        for opcode in [0xf6, 0xf7] {
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
fn division_distinct_segments_and_displacements() {
    for (code, physical, access, cost) in [
        (&[0xf7, 0x37][..], 0xb00, 3, 26),
        (&[0xf7, 0x76, 0][..], 0xa00, 4, 27),
        (&[0xf7, 0xb7, 0, 1][..], 0xc00, 4, 27),
        (&[0x26, 0xf7, 0x76, 0][..], 0x900, 5, 28),
        (&[0x36, 0xf7, 0x37][..], 0xa00, 4, 27),
        (
            &[0x26, 0xf0, 0x36, 0x3e, 0xf0, 0x26, 0xf0, 0xf7, 0x3f][..],
            0x900,
            10,
            34,
        ),
    ] {
        let (mut c, mut b) = setup(Model::Mono, 0, code);
        c.segments = [0x10, 0, 0x20, 0x30];
        c.registers[3] = 0x800;
        c.registers[5] = 0x800;
        b.ram[physical..physical + 2].copy_from_slice(&[2, 0]);
        let mut regs = c.registers;
        regs[0] = 3;
        regs[2] = 1;
        let ram = b.ram.clone();
        let segments = c.segments;
        assert_eq!(c.instruction_step(&mut b).unwrap(), cost, "{code:x?}");
        assert_eq!(b.accesses.get(), access);
        assert_eq!(c.registers, regs);
        assert_eq!(c.segments, segments);
        assert_eq!((c.flags, c.ip), (0xf002, 0x1000 + code.len() as u16));
        assert!(b.ram == ram);
    }
}

#[test]
fn division_register_forms_mapped_fetches_at_both_parities() {
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
                    for (a0, lane, _word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                        b.output(0xa0, a0).unwrap();
                        for (code, residual, ax, dx, flags) in [
                            (&[0xf6, 0xf3][..], 15, 0x0103, 0, 0xf002),
                            (&[0xf7, 0xf3][..], 23, 3, 1, 0xf002),
                            (&[0xf6, 0xfb][..], 17, 0x0103, 0, 0xf006),
                            (&[0xf7, 0xfb][..], 24, 3, 1, 0xf002),
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
                            c.registers[0] = 7;
                            c.registers[2] = 0;
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
                            let fetch = code.len() as u32 * byte;
                            let ram = b.ram.clone();
                            let save = b.cartridge.as_ref().unwrap().save.clone();
                            assert_eq!(c.instruction_step(&mut b).unwrap(), fetch + residual);
                            assert_eq!(b.accesses.get(), fetch);
                            assert_eq!(
                                (c.registers[0], c.registers[2], c.flags, c.ip),
                                (ax, dx, flags, start + 2)
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
#[test]
fn division_operand_mapping_and_offset_boundaries() {
    use super::call_tests::mapped_code;
    for (model, mode) in modes() {
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
            for (opcode, extension, residual) in
                [(0xf6, 6, 15), (0xf6, 7, 17), (0xf7, 6, 23), (0xf7, 7, 24)]
            {
                let (mut c, mut b) = setup(model, mode, &[opcode, 7 | extension << 3]);
                mapped_code(&mut b, &[], &[]);
                c.segments[3] = segment;
                c.registers[3] = offset;
                // Unavailable RAM reads FF; signed division must therefore
                // also handle a -1 source. Derive results independently below.
                b.ram[first as usize & 0xffff] = 0x81;
                b.ram[second as usize & 0xffff] = 0x80;
                if (0x10000..0x20000).contains(&first) {
                    b.write(first, 0x81);
                }
                if (0x10000..0x20000).contains(&second) {
                    b.write(second, 0x80);
                }
                let ram = b.ram.clone();
                let cart = b.cartridge.as_ref().unwrap();
                let save = cart.save.clone();
                let rom = cart.bytes.clone();
                let mut regs = c.registers;
                let wide = opcode == 0xf7;
                let available = if model != Model::Mono && mode != 0 {
                    0x10000
                } else {
                    0x4000
                };
                let read = |address: u32| {
                    if address < available {
                        ram[address as usize]
                    } else if address < 0x10000 {
                        0xff
                    } else if address < 0x20000 {
                        save[address as usize & 0x7fff]
                    } else {
                        0x0f
                    }
                };
                let input = if wide {
                    u16::from_le_bytes([read(first), read(second)])
                } else {
                    u16::from(read(first))
                };
                let divisor = if extension == 6 {
                    i64::from(input)
                } else if wide {
                    i64::from(input as i16)
                } else {
                    i64::from(input as u8 as i8)
                };
                let q = 7 / divisor;
                let rem = 7 % divisor;
                regs[0] = if wide {
                    q as u16
                } else {
                    ((rem as u16) << 8) | u16::from(q as u8)
                };
                if wide {
                    regs[2] = rem as u16;
                }
                let flags = if !wide && extension == 7 {
                    if q == 0 {
                        0xf046
                    } else {
                        assert_eq!(q, -7);
                        0xf086
                    }
                } else if rem == 0 && q & 1 != 0 {
                    0xf042
                } else {
                    0xf002
                };
                // Only aligned, contiguous available RAM words consolidate in
                // these cases; every external or wrapped word starts odd.
                let transfer =
                    if !wide || (first & 1 == 0 && second == first + 1 && second < available) {
                        1
                    } else {
                        2
                    };
                assert_eq!(
                    c.instruction_step(&mut b).unwrap(),
                    2 + transfer + residual,
                    "{model:?}/{mode:x}/{segment:x}:{offset:x}/{opcode:x}/{extension}"
                );
                assert_eq!(b.accesses.get(), 2 + transfer);
                assert_eq!(c.registers, regs);
                assert_eq!((c.flags, c.ip), (flags, 0x1002));
                assert!(b.ram == ram);
                let cart = b.cartridge.as_ref().unwrap();
                assert_eq!(cart.save, save);
                assert_eq!(cart.bytes, rom);
            }
        }
    }
}

#[test]
fn division_fetch_offset_bank_and_twenty_bit_wraps() {
    use super::call_tests::mapped_code;
    for (cs, ip, addresses) in [
        (0u16, 0xffffu16, [0xffff, 0]),
        (0x2000, 0xffff, [0x2ffff, 0x20000]),
        (0x2fff, 0xf, [0x2ffff, 0x30000]),
        (0xffff, 0xf, [0xfffff, 0]),
    ] {
        for (opcode, modrm, residual, ax, dx, flags) in [
            (0xf6, 0xf3, 15, 0x0103, 0, 0xf002),
            (0xf6, 0xfb, 17, 0x0103, 0, 0xf006),
            (0xf7, 0xf3, 23, 3, 1, 0xf002),
            (0xf7, 0xfb, 24, 3, 1, 0xf002),
        ] {
            let (mut c, mut b) = setup(Model::Color, 0x80, &[]);
            mapped_code(&mut b, &addresses, &[opcode, modrm]);
            for (a0, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
                b.output(0xa0, a0).unwrap();
                c.segments[1] = cs;
                c.ip = ip;
                c.registers[0] = 7;
                c.registers[2] = 0;
                c.registers[3] = 2;
                c.flags = 0xf002;
                let ram = b.ram.clone();
                let segments = c.segments;
                let fetch = addresses
                    .iter()
                    .map(|a| if *a >= 0x20000 { lane } else { 1 })
                    .sum::<u32>();
                assert_eq!(c.instruction_step(&mut b).unwrap(), fetch + residual);
                assert_eq!(b.accesses.get(), fetch);
                assert_eq!(
                    (c.registers[0], c.registers[2], c.flags, c.ip),
                    (ax, dx, flags, ip.wrapping_add(2))
                );
                assert_eq!(c.segments, segments);
                assert!(b.ram == ram);
            }
        }
    }
}

#[test]
fn division_faults_keep_baseline_charge_frame_registers_and_history() {
    for (model, mode) in modes() {
        for (opcode, modrm, ax, dx, bx, cost) in [
            (0xf6, 0xf3, 7, 0, 0, 31),
            (0xf6, 0xfb, 7, 0, 0, 31),
            (0xf7, 0xf3, 7, 0, 0, 39),
            (0xf7, 0xfb, 7, 0, 0, 39),
            (0xf6, 0xf3, 0x100, 0, 1, 31),
            (0xf6, 0xfb, 128, 0, 1, 31),
            (0xf6, 0xfb, 0xff7f, 0, 1, 31),
            (0xf7, 0xf3, 0, 1, 1, 39),
            (0xf7, 0xfb, 0x8000, 0, 1, 39),
            (0xf7, 0xfb, 0x7fff, 0xffff, 1, 39),
        ] {
            for history in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[opcode, modrm, 0x43]);
                c.registers[0] = ax;
                c.registers[2] = dx;
                c.registers[3] = bx;
                c.flags = 0xffd7;
                c.multiplication_overflow = history;
                let mut regs = c.registers;
                regs[4] = 0x1ffa;
                b.ram[..4].copy_from_slice(&[0, 3, 0, 0]);
                b.ram[0x300] = 0xcf;
                let mut ram = b.ram.clone();
                ram[0x1ffa..0x2000].copy_from_slice(&[0, 0x10, 0, 0, 0xd7, 0xff]);
                // Baseline policy: width residual + 2 fetches + 5 vector/stack
                // transfers. This does not assert silicon fault correctness.
                assert_eq!(c.step(&mut b).unwrap(), cost);
                assert_eq!(b.accesses.get(), 7);
                assert_eq!(c.registers, regs);
                assert_eq!(c.segments, [0, 0, 0, 0]);
                assert_eq!(
                    (c.ip, c.flags, c.multiplication_overflow),
                    (0x300, 0xfcd7, history)
                );
                assert!(b.ram == ram);
                assert!(!c.halted);
                assert_eq!(c.step(&mut b).unwrap(), 11);
                assert_eq!((c.ip, c.flags, c.registers[4]), (0x1000, 0xffd7, 0x2000));
                assert_eq!(c.multiplication_overflow, history);
            }
        }
    }
}

#[test]
fn normal_run_frame_divisions_cross_frame_with_exact_timer_audio_and_continuation() {
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
            for (code, cost, ax, dx, flags) in [
                (&[0xf6, 0xf3][..], 17, 0x0103, 0, 0xf002),
                (&[0xf6, 0xfb][..], 19, 0x0103, 0, 0xf006),
                (&[0xf7, 0xf3][..], 25, 3, 1, 0xf002),
                (&[0xf7, 0xfb][..], 26, 3, 1, 0xf002),
                (&[0xf7, 0x37][..], 27, 3, 1, 0xf002),
                (&[0xf7, 0x3f][..], 28, 3, 1, 0xf002),
            ] {
                m.cpu.ip = 0x1000;
                m.cpu.flags = 0xf002;
                m.cpu.registers[0] = 7;
                m.cpu.registers[2] = 0;
                m.cpu.registers[3] = if code[1] < 0xc0 { 0x801 } else { 2 };
                m.bus.ram[0x1000..0x1002].copy_from_slice(code);
                m.bus.ram[0x1002] = 0x43; // INC BX must not execute.
                m.bus.ram[0x801..0x803].copy_from_slice(&[2, 0]);
                let ram = m.bus.ram.clone();
                let mut regs = m.cpu.registers;
                regs[0] = ax;
                regs[2] = dx;
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
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
                assert_eq!(m.bus.accesses.get(), if code[1] < 0xc0 { 4 } else { 2 });
                assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                assert_eq!(m.audio_samples(), [0.0, 0.0]);
                assert_eq!((m.cpu.ip, m.cpu.flags), (0x1002, flags));
                assert_eq!(m.cpu.registers, regs);
                assert!(m.bus.ram == ram);
            }
        }
    }
}
#[test]
fn divisions_preserve_if_shadow_tf_irq_entry_and_dma_reservations() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            // Delivery follows the complete division, with separate entry charge.
            for (opcode, modrm, cost, ax, dx, arithmetic) in [
                (0xf6, 0xf3, 17, 0x0103, 0, 0x02),
                (0xf6, 0xfb, 19, 0x0103, 0, 0x06),
                (0xf7, 0xf3, 25, 3, 1, 0x02),
                (0xf7, 0xfb, 26, 3, 1, 0x02),
            ] {
                for pending in [false, true] {
                    let (mut c, mut b) = setup(model, mode, &[opcode, modrm]);
                    c.segments = [0, 0, 0, 0];
                    c.registers[4] = 0x2800;
                    c.registers[0] = 7;
                    c.registers[2] = 0;
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
                    assert_eq!(
                        c.step(&mut b).unwrap(),
                        cost + if pending { 32 } else { 25 }
                    );
                    assert_eq!(
                        (c.ip, c.registers[0], c.registers[4], c.flags),
                        (
                            if pending { 0x400 } else { 0x300 },
                            ax,
                            0x27fa,
                            0xf000 | arithmetic
                        )
                    );
                    assert_eq!(
                        &b.ram[0x27fa..0x2800],
                        &[
                            2,
                            0x10,
                            0,
                            0,
                            arithmetic as u8,
                            if pending { 0xf3 } else { 0xf1 }
                        ]
                    );
                    assert_eq!(c.registers[2], dx);
                    if pending {
                        b.output(0xb6, 0x80).unwrap();
                    }
                    assert_eq!(c.step(&mut b).unwrap(), 11);
                    assert_eq!(
                        (c.ip, c.registers[4], c.flags),
                        (
                            0x1002,
                            0x2800,
                            if pending {
                                0xf300 | arithmetic
                            } else {
                                0xf100 | arithmetic
                            }
                        )
                    );
                }
            }
            for sound in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[0xf7, 0xf3]);
                mapped_code(&mut b, &[], &[]);
                c.registers[0] = 7;
                c.registers[2] = 0;
                c.registers[3] = 2;
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
                assert_eq!(c.step(&mut b).unwrap(), 25);
                assert_eq!((c.ip, c.registers[0], c.flags), (0x1002, 3, 0xf002));
            }
        }
    }
}
