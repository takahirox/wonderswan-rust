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

// Bit index C + 2P + 4Z + 8S + 16O. These literal truth sets do not call
// Cpu::condition, and contain both outcomes for every one of the 16 conditions.
const TRUTH: [u32; 16] = [
    0xffff0000, 0x0000ffff, 0xaaaaaaaa, 0x55555555, 0xf0f0f0f0, 0x0f0f0f0f, 0xfafafafa, 0x05050505,
    0xff00ff00, 0x00ff00ff, 0xcccccccc, 0x33333333, 0x00ffff00, 0xff0000ff, 0xf0fffff0, 0x0f00000f,
];

fn flag_vector(index: u32, other: u16) -> u16 {
    other
        | ((index as u16 & 1)
            | ((index as u16 & 2) << 1)
            | ((index as u16 & 4) << 4)
            | ((index as u16 & 8) << 4)
            | ((index as u16 & 16) << 7))
}

#[test]
fn literal_all_condition_truth_sets_and_jcxz_preserve_full_state() {
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
                for opcode in (0x70..=0x7f).chain([0xe3]) {
                    for bits in 0..32 {
                        for other in [0, 0xf73a] {
                            // every non-condition FLAGS bit
                            for cx in if opcode == 0xe3 {
                                &[0, 1, 2, 0xffff][..]
                            } else {
                                &[0x4567][..]
                            } {
                                let take = if opcode == 0xe3 {
                                    *cx == 0
                                } else {
                                    TRUTH[usize::from(opcode - 0x70)] & (1 << bits) != 0
                                };
                                for (disp, even_target, odd_target) in [
                                    (0, 0x1002, 0x1003),
                                    (2, 0x1004, 0x1005),
                                    (3, 0x1005, 0x1006),
                                    (0xfc, 0x0ffe, 0x0fff),
                                    (0xfd, 0x0fff, 0x1000),
                                    (0x80, 0x0f82, 0x0f83),
                                    (0x7f, 0x1081, 0x1082),
                                ] {
                                    for (start, target, fall) in [
                                        (0x1000, even_target, 0x1002),
                                        (0x1001, odd_target, 0x1003),
                                    ] {
                                        c.ip = start;
                                        c.flags = flag_vector(bits, other);
                                        c.registers[1] = *cx;
                                        b.ram[start as usize..start as usize + 2]
                                            .copy_from_slice(&[opcode, disp]);
                                        let ram = b.ram.clone();
                                        let ports = b.ports;
                                        let registers = c.registers;
                                        let flags = c.flags;
                                        let cost = if !take {
                                            3
                                        } else if target & 1 == 0 {
                                            6
                                        } else {
                                            7
                                        };
                                        assert_eq!(c.instruction_step(&mut b).unwrap(), cost,
                                            "{model:?}/{mode:x}/{waits:x}/{opcode:x}/{bits}/{cx:x}/{start:x}/{disp:x}");
                                        assert_eq!(b.accesses.get(), 2, "no destination read");
                                        assert_eq!(
                                            (c.segments, c.ip, c.flags),
                                            (
                                                [0x10, 0, 0x20, 0x30],
                                                if take { target } else { fall },
                                                flags
                                            )
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
}

// Independently specified physical fetch locations, never Cpu::physical.
fn mapped_code(b: &mut Bus, addresses: &[u32], code: &[u8]) {
    if b.cartridge.is_none() {
        let cart = crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(262144, 2, 0))
            .unwrap();
        b.insert_cartridge(cart);
    }
    let cart = b.cartridge.as_mut().unwrap();
    cart.bytes.fill(0x0f); // poison destinations: reading one adds a fetch
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
}

#[test]
fn literal_mapped_bank_ip_and_twenty_bit_wrap_fetches() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode | 0x0a).unwrap();
            }
            for (control, fetch, split_fetch) in [(0, 2, 2), (4, 2, 2), (8, 4, 3), (12, 4, 3)] {
                for (cs, ip, addresses, next, targets) in [
                    (
                        0x2000,
                        0x1000,
                        [0x21000, 0x21001],
                        0x1002,
                        [0x1002, 0x1000, 0x1001, 0x1004, 0x1005, 0x0f82, 0x1081],
                    ),
                    (
                        0x2000,
                        0x1001,
                        [0x21001, 0x21002],
                        0x1003,
                        [0x1003, 0x1001, 0x1002, 0x1005, 0x1006, 0x0f83, 0x1082],
                    ),
                    (
                        0x2800,
                        0x7fff,
                        [0x2ffff, 0x30000],
                        0x8001,
                        [0x8001, 0x7fff, 0x8000, 0x8003, 0x8004, 0x7f81, 0x8080],
                    ),
                    (
                        0x2000,
                        0xfffe,
                        [0x2fffe, 0x2ffff],
                        0,
                        [0, 0xfffe, 0xffff, 2, 3, 0xff80, 0x7f],
                    ),
                    (
                        0x2000,
                        0xffff,
                        [0x2ffff, 0x20000],
                        1,
                        [1, 0xffff, 0, 3, 4, 0xff81, 0x80],
                    ),
                    (
                        0xffff,
                        0xf,
                        [0xfffff, 0],
                        0x11,
                        [0x11, 0xf, 0x10, 0x13, 0x14, 0xff91, 0x90],
                    ),
                ] {
                    for (disp, target) in [0, 0xfe, 0xff, 2, 3, 0x80, 0x7f].into_iter().zip(targets)
                    {
                        for opcode in (0x70..=0x7f).chain([0xe3]) {
                            for take in [false, true] {
                                // Fixed truth sets select contrasting condition inputs.
                                let bits = (0..32)
                                    .find(|bits| {
                                        (TRUTH[usize::from(opcode & 15)] & (1 << bits) != 0) == take
                                    })
                                    .unwrap();
                                mapped_code(&mut b, &addresses, &[opcode, disp]);
                                b.output(0xa0, control).unwrap(); // cartridge insertion resets A0
                                c.segments[1] = cs;
                                c.ip = ip;
                                c.flags = flag_vector(bits, 0xf432); // no TF/IRQ delivery
                                c.registers[1] = if take { 0 } else { 0xffff };
                                let registers = c.registers;
                                let flags = c.flags;
                                let ram = b.ram.clone();
                                let ports = b.ports;
                                let fetch = if addresses[1] == 0 {
                                    split_fetch
                                } else {
                                    fetch
                                };
                                let execution = if !take {
                                    1
                                } else if target & 1 == 0 {
                                    4
                                } else {
                                    5
                                };
                                assert_eq!(c.step(&mut b).unwrap(), fetch + execution, "{model:?}/{mode:x}/{control:x}/{cs:x}:{ip:x}/{opcode:x}/{disp:x}/{take}");
                                assert_eq!(b.accesses.get(), fetch);
                                assert_eq!(
                                    (c.segments, c.ip, c.flags),
                                    (
                                        [0x10, cs, 0x20, 0x30],
                                        if take { target } else { next },
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
fn literal_internal_and_rom_prefix_fetches_and_rep_rejection() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for cs in [0, 0x2000] {
                for (control, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
                    for start in [0x1000u16, 0x1001] {
                        for count in 0..=7 {
                            for opcode in (0x70..=0x7f).chain([0xe3]) {
                                for take in [false, true] {
                                    for target in [0x1040, 0x1041] {
                                        let bits = (0..32)
                                            .find(|bits| {
                                                (TRUTH[usize::from(opcode & 15)] & (1 << bits) != 0)
                                                    == take
                                            })
                                            .unwrap();
                                        let mut code = [0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0]
                                            [..count]
                                            .to_vec();
                                        code.extend_from_slice(&[
                                            opcode,
                                            (target - start - count as u16 - 2) as u8,
                                        ]);
                                        let addresses: Vec<u32> = (0..code.len()).map(|i| if cs == 0 { 0 } else { 0x20000 } + u32::from(start) + i as u32).collect();
                                        mapped_code(&mut b, &addresses, &code);
                                        b.output(0xa0, control).unwrap();
                                        c.segments[1] = cs;
                                        c.ip = start;
                                        c.flags = flag_vector(bits, 0xf432);
                                        c.registers[1] = if take { 0 } else { 2 };
                                        let registers = c.registers;
                                        let flags = c.flags;
                                        let ram = b.ram.clone();
                                        let ports = b.ports;
                                        let fetch =
                                            (count as u32 + 2) * if cs == 0 { 1 } else { lane };
                                        let execution = if !take {
                                            1
                                        } else if target & 1 == 0 {
                                            4
                                        } else {
                                            5
                                        };
                                        assert_eq!(c.step(&mut b).unwrap(), fetch + execution);
                                        assert_eq!(b.accesses.get(), fetch);
                                        assert_eq!(
                                            (c.segments, c.ip, c.flags),
                                            (
                                                [0x10, cs, 0x20, 0x30],
                                                if take {
                                                    target
                                                } else {
                                                    start + count as u16 + 2
                                                },
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
        for opcode in (0x70..=0x7f).chain([0xe3]) {
            for prefix in [0xf2, 0xf3] {
                c.segments[1] = 0;
                c.ip = 0x1000;
                let before = (c.registers, c.flags);
                b.ram[0x1000..0x1003].copy_from_slice(&[prefix, opcode, 3]);
                let ram = b.ram.clone();
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.flags), before);
                assert!(b.ram == ram);
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn normal_run_frame_charges_branch_once_for_raster_timer_and_audio() {
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
            for (opcode, flags, even, odd, fall) in
                [(0x74, 0xf042, 6, 7, 3), (0xe3, 0xf042, 6, 7, 3)]
            {
                for (disp, cx, ip, cost) in [
                    (2, 0, 0x1004, even),
                    (3, 0, 0x1005, odd),
                    (3, 1, 0x1002, fall),
                ] {
                    m.bus.ram[0x1000..0x1007]
                        .copy_from_slice(&[opcode, disp, 0x43, 0x43, 0x43, 0x43, 0x43]);
                    m.cpu.ip = 0x1000;
                    m.cpu.flags = if opcode == 0x74 && cx == 1 {
                        flags & !0x40
                    } else {
                        flags
                    };
                    let flags = m.cpu.flags;
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
                    assert_eq!(
                        (m.cpu.ip, m.cpu.registers, m.cpu.flags),
                        (ip, registers, flags)
                    );
                }
            }
        }
    }
}

#[test]
fn branch_shadow_and_irq_tf_arbitration_add_only_the_selected_entry() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (opcode, zf, even, odd, fall) in [(0x74, 0x40, 6, 7, 3), (0xe3, 0x40, 6, 7, 3)] {
                for (disp, cx, next, cost) in [
                    (2, 0, 0x1004u16, even),
                    (3, 0, 0x1005, odd),
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
                            c.flags = 0xf202
                                | if opcode == 0x74 && cx == 1 { 0 } else { zf }
                                | if tf { TF } else { 0 };
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
                            assert_eq!(c.registers[1], cx);
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
                                assert_eq!((c.ip, c.registers[1]), (0x400, cx));
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn branch_after_separate_dma_reservations_retains_model_and_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (opcode, flags, cost) in [(0x74, 0xf042, 7), (0xe3, 0xf002, 7)] {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.flags = flags;
                    c.registers[1] = 0;
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
                    assert_eq!((c.ip, c.registers[1], c.flags), (0x1005, 0, flags));
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
