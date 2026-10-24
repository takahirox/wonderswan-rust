// SPDX-License-Identifier: CC0-1.0
use super::*;

fn setup(model: Model) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    b.ram.fill(0xa5);
    let c = Cpu {
        registers: [0x1234, 0, 0x5678, 0x789a, 0x2800, 0x9abc, 0xbcde, 0xdef0],
        segments: [0x10, 0, 0x20, 0x30],
        ip: 0x1000,
        ..Cpu::default()
    };
    (c, b)
}

#[test]
fn literal_loop_matrix_preserves_full_state_across_live_modes() {
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
                for (opcode, zf, taken_even, taken_odd, fall) in [
                    (0xe2, false, 7, 8, 4),
                    (0xe2, true, 7, 8, 4),
                    (0xe1, false, 8, 9, 5),
                    (0xe1, true, 8, 9, 5),
                    (0xe0, false, 8, 9, 5),
                    (0xe0, true, 8, 9, 5),
                ] {
                    for (cx, after) in [(0, 0xffff), (1, 0), (2, 1), (0xffff, 0xfffe)] {
                        for (displacement, target, taken_cost) in [
                            (2, 0x1004, taken_even),
                            (3, 0x1005, taken_odd),
                            (0xfc, 0x0ffe, taken_even),
                            (0xfd, 0x0fff, taken_odd),
                        ] {
                            for flags in [0x0000, 0xffff] {
                                c.ip = 0x1000;
                                c.flags = (flags & !ZF) | if zf { ZF } else { 0 };
                                c.registers[1] = cx;
                                b.ram[0x1000..0x1002].copy_from_slice(&[opcode, displacement]);
                                let ram = b.ram.clone();
                                let ports = b.ports;
                                let mut registers = c.registers;
                                registers[1] = after;
                                let saved_flags = c.flags;
                                // Independent truth table: CX must survive decrement and
                                // the selected conditional opcode must admit incoming ZF.
                                let take = cx != 1 && (opcode == 0xe2 || (opcode == 0xe1) == zf);
                                assert_eq!(c.instruction_step(&mut b).unwrap(), if take { taken_cost } else { fall },
                                    "{model:?}/{mode:x}/{waits:x}/{opcode:x}/{cx:x}/{zf}/{displacement:x}");
                                assert_eq!(b.accesses.get(), 2, "only the actual two fetches");
                                assert_eq!(c.registers, registers);
                                assert_eq!(c.segments, [0x10, 0, 0x20, 0x30]);
                                assert_eq!(c.ip, if take { target } else { 0x1002 });
                                assert_eq!(c.flags, saved_flags);
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

#[test]
fn literal_loop_mapped_fetch_waits_bank_and_ip_wraps() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode | 0x0a).unwrap();
            }
            for (control, fetch) in [(0, 2), (4, 2), (8, 4), (12, 4)] {
                for (opcode, flags, even, odd, fall) in [
                    (0xe2, 0xf002, 5, 6, 2),
                    (0xe1, 0xf042, 6, 7, 3),
                    (0xe0, 0xf002, 6, 7, 3),
                ] {
                    for (cs, ip, addresses, next, targets) in [
                        (
                            0x2000,
                            0x1000,
                            [0x21000, 0x21001],
                            0x1002,
                            [0x1000, 0x1001, 0x1004, 0x1005],
                        ),
                        (
                            0x2800,
                            0x7fff,
                            [0x2ffff, 0x30000],
                            0x8001,
                            [0x7fff, 0x8000, 0x8003, 0x8004],
                        ),
                        (0x2000, 0xffff, [0x2ffff, 0x20000], 1, [0xffff, 0, 3, 4]),
                        (0xffff, 0xf, [0xfffff, 0], 0x11, [0xf, 0x10, 0x13, 0x14]),
                    ] {
                        for (disp, target) in [0xfe, 0xff, 2, 3].into_iter().zip(targets) {
                            for cx in [1, 2] {
                                let mut cart = crate::cartridge::Cartridge::parse(
                                    &crate::cartridge::tests::rom(262144, 2, 0),
                                )
                                .unwrap();
                                cart.bytes.fill(0x0f);
                                b.ports[0xc2] = 0;
                                b.ports[0xc3] = 1;
                                for (address, value) in addresses.into_iter().zip([opcode, disp]) {
                                    if address == 0 {
                                        b.ram[0] = value;
                                    } else {
                                        let index = match address >> 16 {
                                            2 => address & 0xffff,
                                            3 => 0x10000 | (address & 0xffff),
                                            _ => 0x30000 | (address & 0xffff),
                                        };
                                        cart.bytes[index] = value;
                                    }
                                }
                                b.insert_cartridge(cart);
                                b.output(0xa0, control).unwrap();
                                c.segments[1] = cs;
                                c.ip = ip;
                                c.registers[1] = cx;
                                c.flags = flags;
                                let mut registers = c.registers;
                                registers[1] = cx - 1;
                                let ram = b.ram.clone();
                                let ports = b.ports;
                                let fetch = if addresses[1] == 0 {
                                    if control & 8 != 0 {
                                        3
                                    } else {
                                        2
                                    }
                                } else {
                                    fetch
                                };
                                let execution = if cx == 1 {
                                    fall
                                } else if target & 1 == 0 {
                                    even
                                } else {
                                    odd
                                };
                                assert_eq!(c.step(&mut b).unwrap(), fetch + execution);
                                assert_eq!(b.accesses.get(), fetch);
                                assert_eq!(
                                    (c.segments, c.ip, c.flags),
                                    (
                                        [0x10, cs, 0x20, 0x30],
                                        if cx == 1 { next } else { target },
                                        flags
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
fn loop_prefixes_and_corrected_jcxz_target_parity() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for (opcode, flags, cx, target, base) in [
            (0xe2, 0xf002, 2, 0x1000, 7),
            (0xe1, 0xf042, 2, 0x1000, 8),
            (0xe0, 0xf002, 2, 0x1000, 8),
            (0xe3, 0xf002, 0, 0x1000, 6),
            (0xe3, 0xf042, 1, 0x1002, 3),
        ] {
            for count in 0..=7 {
                let (mut c, mut b) = setup(model);
                let mut code = vec![0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0][..count].to_vec();
                code.extend_from_slice(&[opcode, 0xfe]);
                b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
                c.flags = flags;
                c.registers[1] = cx;
                assert_eq!(
                    c.step(&mut b).unwrap(),
                    base + count as u32 + u32::from((opcode != 0xe3 || cx == 0) && count % 2 == 1)
                );
                assert_eq!(b.accesses.get(), count as u32 + 2);
                assert_eq!(c.ip, target + count as u16);
                assert_eq!(c.flags, flags);
                assert_eq!(c.registers[1], if opcode == 0xe3 { cx } else { cx - 1 });
            }
            for prefix in [0xf2, 0xf3] {
                let (mut c, mut b) = setup(model);
                b.ram[0x1000..0x1003].copy_from_slice(&[prefix, opcode, 0xfe]);
                c.registers[1] = cx;
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!((c.ip, c.registers[1]), (0x1002, cx));
            }
        }
    }
}

#[test]
fn normal_run_frame_charges_loop_once_for_raster_timer_and_audio() {
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
            for (opcode, flags, even, odd, fall) in [
                (0xe2, 0xf002, 7, 8, 4),
                (0xe1, 0xf042, 8, 9, 5),
                (0xe0, 0xf002, 8, 9, 5),
            ] {
                for (disp, cx, ip, cost) in [
                    (2, 2, 0x1004, even),
                    (3, 2, 0x1005, odd),
                    (3, 1, 0x1002, fall),
                ] {
                    m.bus.ram[0x1000..0x1007]
                        .copy_from_slice(&[opcode, disp, 0x43, 0x43, 0x43, 0x43, 0x43]);
                    m.cpu.ip = 0x1000;
                    m.cpu.flags = flags;
                    m.cpu.registers[1] = cx;
                    let registers = m.cpu.registers;
                    let remaining =
                        crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                    m.advance((remaining - cost) as u32);
                    m.bus.output_word(0xa4, 2).unwrap();
                    m.bus.output(0xa2, 1).unwrap();
                    let before = m.bus.cycles;
                    let frame = m.frame_number();
                    m.audio_samples();
                    m.run_frame().unwrap();
                    assert_eq!(m.bus.cycles, before + cost);
                    assert_eq!((m.frame_number(), m.bus.raster_line), (frame + 1, 0));
                    assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                    assert_eq!(m.audio_samples(), [0.0, 0.0]);
                    let mut expected = registers;
                    expected[1] = cx - 1;
                    assert_eq!(
                        (m.cpu.ip, m.cpu.registers, m.cpu.flags),
                        (ip, expected, flags)
                    );
                }
            }
        }
    }
}

#[test]
fn loop_shadow_and_irq_tf_arbitration_add_only_the_selected_entry() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (opcode, zf, even, odd, fall) in [
                (0xe2, 0, 7, 8, 4),
                (0xe1, 0x40, 8, 9, 5),
                (0xe0, 0, 8, 9, 5),
            ] {
                for (disp, cx, next, cost) in [
                    (2, 2, 0x1004u16, even),
                    (3, 2, 0x1005, odd),
                    (3, 1, 0x1002, fall),
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
                            c.registers[1] = cx;
                            c.flags = 0xf202 | zf | if tf { TF } else { 0 };
                            c.interrupt_shadow = u8::from(shadow);
                            let flags = c.flags;
                            b.ram[0x1000..0x1002].copy_from_slice(&[opcode, disp]);
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
                            assert_eq!(
                                c.registers[1],
                                if before_instruction { cx } else { cx - 1 }
                            );
                            let saved_ip = if before_instruction { 0x1000 } else { next };
                            if entry_now {
                                assert_eq!(c.ip, if pending { 0x400 } else { 0x300 });
                                assert_eq!(c.registers[4], sp - 6);
                                let frame = (sp - 6) as usize;
                                assert_eq!(
                                    &b.ram[frame..frame + 6],
                                    &[
                                        saved_ip as u8,
                                        (saved_ip >> 8) as u8,
                                        0,
                                        0,
                                        flags as u8,
                                        (flags >> 8) as u8
                                    ]
                                );
                                assert_eq!(c.flags, flags & !0x300);
                                assert_eq!(
                                    b.accesses.get(),
                                    if before_instruction {
                                        entry_cost - 27
                                    } else if pending {
                                        entry_cost - 27 + 2
                                    } else {
                                        entry_cost - 20 + 2
                                    }
                                );
                                if pending {
                                    b.output(0xb6, 0x80).unwrap();
                                }
                                c.step(&mut b).unwrap(); // IRET restores the selected continuation.
                                assert_eq!((c.ip, c.flags, c.registers[4]), (saved_ip, flags, sp));
                            } else {
                                assert_eq!((c.ip, c.flags), (next, flags));
                                assert_eq!(b.accesses.get(), 2);
                                assert_eq!(c.step(&mut b).unwrap(), irq);
                                assert_eq!((c.ip, c.registers[1]), (0x400, cx - 1));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn loop_after_separate_dma_reservations_retains_model_and_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (opcode, flags, cost) in [(0xe2, 0xf002, 8), (0xe1, 0xf042, 9), (0xe0, 0xf002, 9)] {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.flags = flags;
                    c.registers[1] = 2;
                    b.ram[0x1000..0x1002].copy_from_slice(&[opcode, 3]);
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
                    assert_eq!(b.accesses.get(), 2);
                    assert_eq!((c.ip, c.registers[1], c.flags), (0x1005, 1, flags));
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
