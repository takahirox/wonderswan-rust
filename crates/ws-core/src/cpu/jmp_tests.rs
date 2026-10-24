// SPDX-License-Identifier: CC0-1.0
use super::*;

fn setup(model: Model) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    b.ram.fill(0xa5);
    (
        Cpu {
            registers: [
                0x1234, 0x4567, 0x5678, 0x789a, 0x2800, 0x9abc, 0xbcde, 0xdef0,
            ],
            segments: [0x10, 0, 0x20, 0x30],
            ip: 0x1000,
            ..Cpu::default()
        },
        b,
    )
}

#[test]
fn literal_relative_and_far_vectors_preserve_full_state_across_live_modes() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for waits in [0, 0x0a] {
                if model != Model::Mono {
                    b.output(0x60, mode | waits).unwrap();
                }
                for (opcode, vectors) in [
                    (
                        0xeb,
                        &[
                            (0, 0x1002),
                            (2, 0x1004),
                            (3, 0x1005),
                            (0xfc, 0x0ffe),
                            (0xfd, 0x0fff),
                            (0x80, 0x0f82),
                            (0x7f, 0x1081),
                        ][..],
                    ),
                    (
                        0xe9,
                        &[
                            (0, 0x1003),
                            (2, 0x1005),
                            (3, 0x1006),
                            (0xfffc, 0x0fff),
                            (0xfffd, 0x1000),
                            (0x8000, 0x9003),
                            (0x7fff, 0x9002),
                        ][..],
                    ),
                    (
                        0xea,
                        &[
                            (0, 0),
                            (0x10, 0x10),
                            (0x11, 0x11),
                            (0x1000, 0x1000),
                            (0x1001, 0x1001),
                            (0xfffe, 0xfffe),
                            (0xffff, 0xffff),
                        ][..],
                    ),
                ] {
                    for &(operand, target) in vectors {
                        for start in [0x1000, 0x1001] {
                            for flags in [0, 0xffff, 0xf002, 0xfcd7] {
                                for cs in if opcode == 0xea {
                                    &[0, 0x3456, 0xffff][..]
                                } else {
                                    &[0][..]
                                } {
                                    c.ip = start;
                                    c.segments[1] = 0;
                                    c.flags = flags;
                                    let mut code = vec![opcode, operand as u8];
                                    if opcode != 0xeb {
                                        code.push((operand >> 8) as u8);
                                    }
                                    if opcode == 0xea {
                                        code.extend_from_slice(&u16::to_le_bytes(*cs));
                                    }
                                    b.ram[start as usize..start as usize + code.len()]
                                        .copy_from_slice(&code);
                                    let target = if opcode == 0xea {
                                        target
                                    } else {
                                        target + start - 0x1000
                                    };
                                    let base = match (opcode, start & 1) {
                                        (0xeb, _) => 6,
                                        (0xe9, 0) => 7,
                                        (0xe9, _) => 6,
                                        (0xea, 0) => 12,
                                        _ => 10,
                                    };
                                    let ram = b.ram.clone();
                                    let ports = b.ports;
                                    let registers = c.registers;
                                    assert_eq!(c.instruction_step(&mut b).unwrap(), base + u32::from(target & 1),
                                        "{model:?}/{mode:x}/{opcode:x}/{start:x}/{operand:x}/{target:x}");
                                    let fetch = match (opcode, start & 1) {
                                        (0xeb, _) => 2,
                                        (0xe9, 0) => 3,
                                        (0xe9, _) => 2,
                                        (0xea, 0) => 5,
                                        _ => 3,
                                    };
                                    assert_eq!(b.accesses.get(), fetch, "no destination fetch");
                                    assert_eq!(
                                        (c.segments, c.ip, c.flags),
                                        ([0x10, *cs, 0x20, 0x30], target, flags)
                                    );
                                    assert_eq!(c.registers, registers);
                                    assert!(!c.halted);
                                    assert_eq!(c.interrupt_shadow, 0);
                                    assert!(b.ram == ram);
                                    assert_eq!(b.ports, ports);
                                    assert_eq!(b.cycles, 0);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

// Install bytes at independently specified physical addresses, including mapped
// bank crossings. Never use CPU physical-address or timing helpers as the oracle.
fn mapped_code(b: &mut Bus, addresses: &[u32], code: &[u8]) {
    let mut cart =
        crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(262144, 2, 0)).unwrap();
    cart.bytes.fill(0x0f); // destination must not be read/executed early
    b.ports[0xc2] = 0;
    b.ports[0xc3] = 1;
    for (&address, &value) in addresses.iter().zip(code) {
        if address < 0x10000 {
            b.ram[address as usize] = value;
        } else {
            let index = match address >> 16 {
                2 => address & 0xffff,
                3 => 0x10000 | (address & 0xffff),
                _ => 0x30000 | (address & 0xffff),
            };
            cart.bytes[index as usize] = value;
        }
    }
    b.insert_cartridge(cart);
}

#[test]
fn literal_mapped_rom_banks_width_waits_and_prefix_alignment() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode | 0x0a).unwrap();
            }
            for (control, lane, word_even, word_odd) in
                [(0, 1, 2, 2), (4, 1, 1, 2), (8, 2, 4, 4), (12, 2, 2, 4)]
            {
                b.output(0xa0, control).unwrap();
                for start in [0x1000u16, 0x1001] {
                    for count in 0..=7 {
                        for opcode in [0xeb, 0xe9, 0xea] {
                            for odd in [0u16, 1] {
                                // Prefixes move the opcode and immediate-word alignment.
                                let mut code =
                                    [0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0][..count].to_vec();
                                let target = 0x1020 + odd;
                                let len = if opcode == 0xeb {
                                    2
                                } else if opcode == 0xe9 {
                                    3
                                } else {
                                    5
                                };
                                let operand = if opcode == 0xea {
                                    target
                                } else {
                                    target - (start + count as u16 + len)
                                };
                                code.extend_from_slice(&[opcode, operand as u8]);
                                if opcode != 0xeb {
                                    code.push((operand >> 8) as u8);
                                }
                                if opcode == 0xea {
                                    code.extend_from_slice(&[0xff, 0xff]);
                                }
                                let addresses: Vec<u32> = (0..code.len())
                                    .map(|i| 0x20000 + u32::from(start) + i as u32)
                                    .collect();
                                mapped_code(&mut b, &addresses, &code);
                                b.output(0xa0, control).unwrap();
                                // Literal lane costs supplied above: an explicit immediate
                                // word starts opposite the opcode parity. Prefixes are bytes.
                                let word = if (usize::from(start) + count) % 2 == 1 {
                                    word_even
                                } else {
                                    word_odd
                                };
                                let fetch = (count as u32 + 1) * lane
                                    + match opcode {
                                        0xeb => lane,
                                        0xe9 => word,
                                        _ => 2 * word,
                                    };
                                let execution = if opcode == 0xea { 7 } else { 4 };
                                c.segments[1] = 0x2000;
                                c.ip = start;
                                c.flags = 0xfcd7;
                                let registers = c.registers;
                                let ram = b.ram.clone();
                                let ports = b.ports;
                                assert_eq!(c.step(&mut b).unwrap(), fetch + execution + u32::from(odd),
                                    "{model:?}/{mode:x}/A0={control:x}/{opcode:x}/{start:x}/{count}/{odd}");
                                assert_eq!(b.accesses.get(), fetch);
                                assert_eq!(
                                    (c.ip, c.segments, c.flags),
                                    (
                                        target,
                                        [
                                            0x10,
                                            if opcode == 0xea { 0xffff } else { 0x2000 },
                                            0x20,
                                            0x30
                                        ],
                                        0xfcd7
                                    )
                                );
                                assert_eq!(c.registers, registers);
                                assert!(b.ram == ram);
                                assert_eq!(b.ports, ports);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn literal_ip_immediate_bank_and_twenty_bit_wraps() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            let (mut c, mut b) = setup(model);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (control, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
                b.output(0xa0, control).unwrap();
                for (cs, ip, addresses) in [
                    (
                        0x2000,
                        0xffff,
                        [0x2ffff, 0x20000, 0x20001, 0x20002, 0x20003],
                    ),
                    (
                        0x2800,
                        0x7fff,
                        [0x2ffff, 0x30000, 0x30001, 0x30002, 0x30003],
                    ),
                    (
                        0x2800,
                        0x7ffe,
                        [0x2fffe, 0x2ffff, 0x30000, 0x30001, 0x30002],
                    ),
                    (
                        0x2000,
                        0xfffe,
                        [0x2fffe, 0x2ffff, 0x20000, 0x20001, 0x20002],
                    ),
                    (0xffff, 0x000f, [0xfffff, 0, 1, 2, 3]),
                    (0xffff, 0x000e, [0xffffe, 0xfffff, 0, 1, 2]),
                ] {
                    for opcode in [0xeb, 0xe9, 0xea] {
                        for operand in if opcode == 0xeb {
                            &[0u16, 2, 0x7f, 0x80, 0xfe, 0xff][..]
                        } else {
                            &[0u16, 2, 0x7fff, 0x8000, 0xfffd, 0xfffe, 0xffff][..]
                        } {
                            let mut code = vec![opcode, *operand as u8];
                            if opcode != 0xeb {
                                code.push((operand >> 8) as u8);
                            }
                            if opcode == 0xea {
                                code.extend_from_slice(&[0xff, 0xff]);
                            }
                            mapped_code(&mut b, &addresses, &code);
                            b.output(0xa0, control).unwrap();
                            c.segments[1] = cs;
                            c.ip = ip;
                            c.flags = 0xfcd7;
                            let registers = c.registers;
                            let ram = b.ram.clone();
                            let ports = b.ports;
                            let relative = if opcode == 0xeb {
                                (*operand as u8 as i8) as u16
                            } else {
                                *operand
                            };
                            let target = if opcode == 0xea {
                                *operand
                            } else {
                                ip.wrapping_add(code.len() as u16).wrapping_add(relative)
                            };
                            // Explicit independent fetch counts for each boundary layout.
                            let words = match (cs, ip) {
                                (0xffff, 0xf) => {
                                    if opcode == 0xea {
                                        2
                                    } else {
                                        1
                                    }
                                }
                                (0xffff, 0xe) => {
                                    if opcode == 0xea {
                                        lane + 3
                                    } else {
                                        lane + 1
                                    }
                                }
                                (_, 0xfffe | 0x7ffe) => {
                                    if opcode == 0xea {
                                        4 * lane
                                    } else {
                                        2 * lane
                                    }
                                }
                                _ => {
                                    let aligned_word =
                                        if control & 4 != 0 { lane } else { 2 * lane };
                                    if opcode == 0xea {
                                        2 * aligned_word
                                    } else {
                                        aligned_word
                                    }
                                }
                            };
                            let fetch = if opcode == 0xeb {
                                if cs == 0xffff && ip == 0xf {
                                    lane + 1
                                } else {
                                    2 * lane
                                }
                            } else {
                                lane + words
                            };
                            assert_eq!(c.step(&mut b).unwrap(), fetch + if opcode == 0xea { 7 } else { 4 } + u32::from(target & 1),
                                "{model:?}/{mode:x}/{control:x}/{cs:x}:{ip:x}/{opcode:x}/{operand:x}");
                            assert_eq!(b.accesses.get(), fetch);
                            assert_eq!(
                                (c.ip, c.segments, c.flags),
                                (
                                    target,
                                    [0x10, if opcode == 0xea { 0xffff } else { cs }, 0x20, 0x30],
                                    0xfcd7
                                )
                            );
                            assert_eq!(c.registers, registers);
                            assert!(b.ram == ram);
                            assert_eq!(b.ports, ports);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn literal_internal_prefix_fetch_alignment_and_rep_rejection() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for start in [0x1000, 0x1001] {
            for count in 0..=7 {
                for (opcode, even_fetches, odd_fetches) in [
                    (0xeb, [2, 3, 4, 5, 6, 7, 8, 9], [2, 3, 4, 5, 6, 7, 8, 9]),
                    (0xe9, [3, 3, 5, 5, 7, 7, 9, 9], [2, 4, 4, 6, 6, 8, 8, 10]),
                    (0xea, [5, 4, 7, 6, 9, 8, 11, 10], [3, 6, 5, 8, 7, 10, 9, 12]),
                ] {
                    for odd in [0, 1] {
                        let (mut c, mut b) = setup(model);
                        let mut code = [0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0][..count].to_vec();
                        let target = 0x1040 + odd;
                        let len = if opcode == 0xeb {
                            2
                        } else if opcode == 0xe9 {
                            3
                        } else {
                            5
                        };
                        let operand = if opcode == 0xea {
                            target
                        } else {
                            target - start - count as u16 - len
                        };
                        code.extend_from_slice(&[opcode, operand as u8]);
                        if opcode != 0xeb {
                            code.push((operand >> 8) as u8);
                        }
                        if opcode == 0xea {
                            code.extend_from_slice(&[0xff, 0xff]);
                        }
                        b.ram[start as usize..start as usize + code.len()].copy_from_slice(&code);
                        c.ip = start;
                        c.flags = 0xfcd7;
                        let registers = c.registers;
                        let ram = b.ram.clone();
                        let fetch = if start == 0x1000 {
                            even_fetches[count]
                        } else {
                            odd_fetches[count]
                        };
                        assert_eq!(
                            c.step(&mut b).unwrap(),
                            fetch + if opcode == 0xea { 7 } else { 4 } + u32::from(odd)
                        );
                        assert_eq!(b.accesses.get(), fetch);
                        assert_eq!((c.ip, c.flags, c.registers), (target, 0xfcd7, registers));
                        assert_eq!(
                            c.segments,
                            [0x10, if opcode == 0xea { 0xffff } else { 0 }, 0x20, 0x30]
                        );
                        assert!(b.ram == ram);
                    }
                }
            }
        }
        for opcode in [0xe8, 0xe9, 0xea, 0xeb, 0xff, 0x74, 0xe3] {
            for prefix in [0xf2, 0xf3] {
                let (mut c, mut b) = setup(model);
                b.ram[0x1000..0x1004].copy_from_slice(&[prefix, opcode, 0, 0]);
                let registers = c.registers;
                let ram = b.ram.clone();
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!((c.ip, c.registers), (0x1002, registers));
                assert!(b.ram == ram);
            }
        }
    }
}

#[test]
fn shared_call_indirect_jmp_jcc_and_jcxz_retain_existing_cost_and_state() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for (code, flags, cx, target, cost, call) in [
            (&[0xe8, 2, 0][..], 0xfcd7, 0, 0x1005, 5, true),
            (&[0xe8, 3, 0][..], 0xf002, 1, 0x1006, 5, true),
            (&[0xff, 0xe0][..], 0xfcd7, 0, 0x1234, 6, false),
            (&[0xff, 0xe0][..], 0xf002, 1, 0x1234, 6, false),
            (&[0xff, 0x26, 0, 8][..], 0xf002, 1, 0x1041, 8, false),
            (&[0xff, 0x2e, 0, 8][..], 0xf002, 1, 0x1041, 11, false),
            (&[0x74, 3][..], 0xf042, 0, 0x1005, 6, false),
            (&[0x74, 3][..], 0xf002, 1, 0x1002, 3, false),
            (&[0xe3, 3][..], 0xfcd7, 0, 0x1005, 6, false),
            (&[0xe3, 3][..], 0xf002, 1, 0x1002, 3, false),
        ] {
            let (mut c, mut b) = setup(model);
            c.registers[1] = cx;
            c.segments[2] = 0;
            c.segments[3] = 0;
            c.flags = flags;
            b.ram[0x800..0x804].copy_from_slice(&[0x41, 0x10, 0x56, 0x34]);
            b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
            let mut ram = b.ram.clone();
            let mut registers = c.registers;
            if call {
                registers[4] -= 2;
                ram[0x27fe..0x2800].copy_from_slice(&[3, 0x10]);
            }
            assert_eq!(c.step(&mut b).unwrap(), cost, "{code:x?}");
            assert_eq!((c.ip, c.flags, c.registers), (target, flags, registers));
            assert_eq!(
                c.segments,
                [0x10, if code[1] == 0x2e { 0x3456 } else { 0 }, 0, 0]
            );
            assert!(b.ram == ram);
        }
    }
}

#[test]
fn normal_run_frame_charges_one_jump_for_raster_timer_and_audio() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model);
        b.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap(),
        );
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
            for start in [0x1000u16, 0x1001] {
                for (opcode, base) in [
                    (0xeb, 6),
                    (0xe9, if start == 0x1000 { 7 } else { 6 }),
                    (0xea, if start == 0x1000 { 12 } else { 10 }),
                ] {
                    for odd in [0u16, 1] {
                        let target = 0x1040 + odd;
                        let len = if opcode == 0xeb {
                            2
                        } else if opcode == 0xe9 {
                            3
                        } else {
                            5
                        };
                        let operand = if opcode == 0xea {
                            target
                        } else {
                            target - start - len
                        };
                        let mut code = vec![opcode, operand as u8];
                        if opcode != 0xeb {
                            code.push((operand >> 8) as u8);
                        }
                        if opcode == 0xea {
                            code.extend_from_slice(&[0x10, 0]);
                        }
                        m.bus.ram[start as usize..start as usize + code.len()]
                            .copy_from_slice(&code);
                        m.bus.ram[0x1040..0x1042].fill(0x43); // extra INC BX must not execute
                        m.bus.ram[0x1140..0x1142].fill(0x43);
                        m.cpu.segments[1] = 0;
                        m.cpu.ip = start;
                        m.cpu.flags = 0xfcd7;
                        let registers = m.cpu.registers;
                        let cost = base + u32::from(odd);
                        let remaining =
                            crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                        m.advance((remaining - u64::from(cost)) as u32);
                        m.bus.output_word(0xa4, 2).unwrap();
                        m.bus.output(0xa2, 1).unwrap();
                        let before = m.bus.cycles;
                        let frame = m.frame_number();
                        m.audio_samples();
                        m.run_frame().unwrap();
                        assert_eq!(m.bus.cycles, before + u64::from(cost));
                        assert_eq!((m.frame_number(), m.bus.raster_line), (frame + 1, 0));
                        assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                        assert_eq!(m.audio_samples(), [0.0, 0.0]);
                        assert_eq!(
                            (m.cpu.segments[1], m.cpu.ip, m.cpu.registers, m.cpu.flags),
                            (
                                if opcode == 0xea { 0x10 } else { 0 },
                                target,
                                registers,
                                0xfcd7
                            )
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn jump_irq_tf_priority_shadows_frames_and_real_iret_continuation() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (code, next, next_cs, cost, fetch) in [
                (&[0xeb, 2][..], 0x1004u16, 0, 6, 2),
                (&[0xeb, 3][..], 0x1005, 0, 7, 2),
                (&[0xe9, 1, 0][..], 0x1004, 0, 7, 3),
                (&[0xe9, 2, 0][..], 0x1005, 0, 8, 3),
                (&[0xea, 4, 0x10, 0x10, 0][..], 0x1004, 0x10u16, 12, 5),
                (&[0xea, 5, 0x10, 0x10, 0][..], 0x1005, 0x10, 13, 5),
            ] {
                for (pending, tf, shadow) in [
                    (true, false, false),
                    (true, false, true),
                    (true, true, true),
                    (false, true, false),
                ] {
                    for (sp, irq, trap) in [(0x2800, 32, 25), (0x2801, 35, 28)] {
                        let (mut c, mut b) = setup(model);
                        if model != Model::Mono {
                            b.output(0x60, mode).unwrap();
                        }
                        c.segments[2] = 0;
                        c.registers[4] = sp;
                        c.flags = 0xf202 | if tf { TF } else { 0 };
                        c.interrupt_shadow = u8::from(shadow);
                        let flags = c.flags;
                        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                        b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                        b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                        b.ram[0x300] = 0xcf;
                        b.ram[0x400] = 0xcf;
                        b.output(0xb2, 0x80).unwrap();
                        if pending {
                            b.request(7);
                        }
                        let before_instruction = pending && !shadow;
                        let entry_now = before_instruction || tf;
                        let entry_cost = if pending { irq } else { trap };
                        assert_eq!(
                            c.step(&mut b).unwrap(),
                            if before_instruction {
                                irq
                            } else if tf {
                                cost + entry_cost
                            } else {
                                cost
                            }
                        );
                        let saved_ip = if before_instruction { 0x1000 } else { next };
                        let saved_cs = if before_instruction { 0 } else { next_cs };
                        if entry_now {
                            assert_eq!(
                                (c.segments[1], c.ip, c.registers[4]),
                                (0, if pending { 0x400 } else { 0x300 }, sp - 6)
                            );
                            let frame = (sp - 6) as usize;
                            assert_eq!(
                                &b.ram[frame..frame + 6],
                                &[
                                    saved_ip as u8,
                                    (saved_ip >> 8) as u8,
                                    saved_cs as u8,
                                    (saved_cs >> 8) as u8,
                                    flags as u8,
                                    (flags >> 8) as u8
                                ]
                            );
                            assert_eq!(c.flags, flags & !0x300);
                            assert_eq!(
                                b.accesses.get(),
                                entry_cost - if pending { 27 } else { 20 }
                                    + if before_instruction { 0 } else { fetch }
                            );
                            if pending {
                                b.output(0xb6, 0x80).unwrap();
                            }
                            c.step(&mut b).unwrap();
                            assert_eq!(
                                (c.segments[1], c.ip, c.flags, c.registers[4]),
                                (saved_cs, saved_ip, flags, sp)
                            );
                        } else {
                            assert_eq!((c.segments[1], c.ip, c.flags), (next_cs, next, flags));
                            assert_eq!(b.accesses.get(), fetch);
                            assert_eq!(c.step(&mut b).unwrap(), irq);
                            assert_eq!((c.segments[1], c.ip), (0, 0x400));
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn jump_after_separate_dma_reservations_retains_model_and_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (code, cost, fetch) in [
                (&[0xeb, 3][..], 7, 2),
                (&[0xe9, 2, 0][..], 8, 3),
                (&[0xea, 5, 0x10, 0, 0][..], 13, 5),
            ] {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.flags = 0xf002;
                    c.registers[1] = 2;
                    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                    if model == Model::Mono {
                        assert!(b.output(if sound { 0x52 } else { 0x48 }, 0x80).is_err());
                    } else {
                        b.output(0x60, mode).unwrap();
                        b.ram[0x800..0x802].copy_from_slice(&[0x51, 0xa2]);
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
                            let before = (c.ip, c.registers, c.flags);
                            assert_eq!(c.step(&mut b).unwrap(), 7);
                            assert_eq!((c.ip, c.registers, c.flags), before);
                            assert_eq!(b.accesses.get(), 0);
                            if !sound {
                                assert_eq!(&b.ram[0x900..0x902], &[0x51, 0xa2]);
                            }
                        }
                    }
                    assert_eq!(c.step(&mut b).unwrap(), cost);
                    assert_eq!(b.accesses.get(), fetch);
                    assert_eq!((c.ip, c.registers[1], c.flags), (0x1005, 2, 0xf002));
                    if model != Model::Mono && !sound && mode == 0 {
                        assert_eq!(
                            &b.ram[0x900..0x902],
                            &[0xa5, 0xa5],
                            "GDMA requires color enable"
                        );
                    }
                }
            }
        }
    }
}
