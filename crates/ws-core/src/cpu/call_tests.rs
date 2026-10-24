// SPDX-License-Identifier: CC0-1.0
use super::*;

pub(super) fn setup(model: Model) -> (Cpu, Bus) {
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

// Expected stack bytes are written into an independent RAM snapshot. This
// helper neither executes the CPU nor consults bus/address/timing helpers.
pub(super) fn frame(ram: &mut [u8], ss: u16, sp: u16, old_cs: u16, next: u16, far: bool) -> u16 {
    let mut sp = sp;
    for value in if far { vec![old_cs, next] } else { vec![next] } {
        sp = sp.wrapping_sub(2);
        for (offset, byte) in [(sp, value as u8), (sp.wrapping_add(1), (value >> 8) as u8)] {
            ram[(((u32::from(ss) << 4) + u32::from(offset)) & 0xfffff) as usize] = byte;
        }
    }
    sp
}

#[test]
fn literal_call_vectors_full_state_parities_and_live_modes() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode | 0x0a).unwrap();
            }
            for (disp, target) in [
                (0, 0x1003u16),
                (2, 0x1005),
                (3, 0x1006),
                (0xfffc, 0x0fff),
                (0xfffd, 0x1000),
                (0x8000, 0x9003),
                (0x7fff, 0x9002),
            ] {
                for (start, near_fetch, far_fetch) in [(0x1000, 3, 5), (0x1001, 2, 3)] {
                    for (sp, stack) in [(0x2800, 1), (0x2801, 2)] {
                        for flags in [0, 0xffff, 0xf002, 0xfcd7] {
                            for far in [false, true] {
                                for dest_cs in if far {
                                    &[0u16, 0x3456, 0xffff][..]
                                } else {
                                    &[0][..]
                                } {
                                    c.ip = start;
                                    c.segments[1] = 0;
                                    c.registers[4] = sp;
                                    c.flags = flags;
                                    let target = target + start - 0x1000;
                                    let operand = if far { target } else { disp };
                                    let mut code = vec![
                                        if far { 0x9a } else { 0xe8 },
                                        operand as u8,
                                        (operand >> 8) as u8,
                                    ];
                                    if far {
                                        code.extend_from_slice(&dest_cs.to_le_bytes());
                                    }
                                    b.ram[start as usize..start as usize + code.len()]
                                        .copy_from_slice(&code);
                                    let mut ram = b.ram.clone();
                                    let mut regs = c.registers;
                                    let ports = b.ports;
                                    regs[4] = frame(
                                        &mut ram,
                                        0x20,
                                        sp,
                                        0,
                                        start + if far { 5 } else { 3 },
                                        far,
                                    );
                                    let fetch = if far { far_fetch } else { near_fetch };
                                    let transfers = stack * if far { 2 } else { 1 };
                                    assert_eq!(
                                        c.instruction_step(&mut b).unwrap(),
                                        fetch
                                            + transfers
                                            + if far { 8 } else { 4 }
                                            + u32::from(target & 1)
                                    );
                                    assert_eq!(
                                        b.accesses.get(),
                                        fetch + transfers,
                                        "no destination pre-read"
                                    );
                                    assert_eq!(
                                        (c.registers, c.segments, c.ip, c.flags),
                                        (regs, [0x10, *dest_cs, 0x20, 0x30], target, flags)
                                    );
                                    assert!(b.ram == ram);
                                    assert_eq!(b.ports, ports);
                                    assert_eq!(b.cycles, 0);
                                    assert!(!c.halted);
                                    assert_eq!(c.interrupt_shadow, 0);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub(super) fn mapped_code(b: &mut Bus, addresses: &[u32], code: &[u8]) {
    if b.cartridge.is_none() {
        b.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(262144, 2, 0))
                .unwrap(),
        );
        b.cartridge.as_mut().unwrap().bytes.fill(0x0f);
    }
    b.ports[0xc2] = 0;
    b.ports[0xc3] = 1;
    for (&address, &byte) in addresses.iter().zip(code) {
        if address < 0x10000 {
            b.ram[address as usize] = byte;
        } else {
            let index = match address >> 16 {
                2 => address & 0xffff,
                3 => 0x10000 | (address & 0xffff),
                _ => 0x30000 | (address & 0xffff),
            };
            b.cartridge.as_mut().unwrap().bytes[index as usize] = byte;
        }
    }
}

#[test]
fn literal_internal_rom_prefixes_a0_waits_and_ss_stack() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for cs in [0, 0x2000] {
                for (control, lane, aligned) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                    for start in [0x1000u16, 0x1001] {
                        for count in 0..=7 {
                            for far in [false, true] {
                                for target in [0x1040u16, 0x1041] {
                                    for (sp, stack) in [(0x2800, 1), (0x2801, 2)] {
                                        let mut code = [0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0]
                                            [..count]
                                            .to_vec();
                                        let next = start + count as u16 + if far { 5 } else { 3 };
                                        let operand = if far { target } else { target - next };
                                        code.extend_from_slice(&[
                                            if far { 0x9a } else { 0xe8 },
                                            operand as u8,
                                            (operand >> 8) as u8,
                                        ]);
                                        if far {
                                            code.extend_from_slice(&[0xff, 0xff]);
                                        }
                                        let addresses: Vec<u32> = (0..code.len())
                                            .map(|i| {
                                                (if cs == 0 { 0 } else { 0x20000 })
                                                    + u32::from(start)
                                                    + i as u32
                                            })
                                            .collect();
                                        mapped_code(&mut b, &addresses, &code);
                                        if model != Model::Mono {
                                            b.output(0x60, mode | 0x0a).unwrap();
                                        }
                                        b.output(0xa0, control).unwrap();
                                        c.segments[1] = cs;
                                        c.ip = start;
                                        c.registers[4] = sp;
                                        c.flags = 0xfcd7;
                                        let mut regs = c.registers;
                                        let mut ram = b.ram.clone();
                                        let ports = b.ports;
                                        regs[4] = frame(&mut ram, 0x20, sp, cs, next, far);
                                        let byte = if cs == 0 { 1 } else { lane };
                                        let word = if (usize::from(start) + count) % 2 == 0 {
                                            2 * byte
                                        } else if cs == 0 {
                                            1
                                        } else {
                                            aligned
                                        };
                                        let fetch = (count as u32 + 1) * byte
                                            + word * if far { 2 } else { 1 };
                                        let writes = stack * if far { 2 } else { 1 };
                                        assert_eq!(c.instruction_step(&mut b).unwrap(),fetch+writes+if far {8} else {4}+u32::from(target&1),"{model:?}/{mode:x}/{cs:x}/{control:x}/{start:x}/{count}/{far}/{sp:x}");
                                        assert_eq!(b.accesses.get(), fetch + writes);
                                        assert_eq!(
                                            (c.registers, c.segments, c.ip, c.flags),
                                            (
                                                regs,
                                                [0x10, if far { 0xffff } else { cs }, 0x20, 0x30],
                                                target,
                                                0xfcd7
                                            )
                                        );
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
        for opcode in [0xe8, 0x9a] {
            for prefix in [0xf2, 0xf3] {
                c.ip = 0x1000;
                c.segments[1] = 0;
                b.ram[0x1000..0x1004].copy_from_slice(&[prefix, opcode, 0, 0]);
                let before = (c.registers, c.segments, c.flags);
                let ram = b.ram.clone();
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.segments, c.flags), before);
                assert!(b.ram == ram);
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn literal_call_ip_bank_and_twenty_bit_fetch_wraps() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (control, lane, aligned) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                // Explicit physical byte addresses and post-immediate IPs.
                for (cs, ip, addresses, near_next, far_next, near_words, far_words) in [
                    (
                        0x2000,
                        0xffff,
                        [0x2ffff, 0x20000, 0x20001, 0x20002, 0x20003],
                        2,
                        4,
                        aligned,
                        2 * aligned,
                    ),
                    (
                        0x2800,
                        0x7fff,
                        [0x2ffff, 0x30000, 0x30001, 0x30002, 0x30003],
                        0x8002,
                        0x8004,
                        aligned,
                        2 * aligned,
                    ),
                    (
                        0x2800,
                        0x7ffe,
                        [0x2fffe, 0x2ffff, 0x30000, 0x30001, 0x30002],
                        0x8001,
                        0x8003,
                        2 * lane,
                        4 * lane,
                    ),
                    (
                        0x2000,
                        0xfffe,
                        [0x2fffe, 0x2ffff, 0x20000, 0x20001, 0x20002],
                        1,
                        3,
                        2 * lane,
                        4 * lane,
                    ),
                    (0xffff, 0xf, [0xfffff, 0, 1, 2, 3], 0x12, 0x14, 1, 2),
                    (
                        0xffff,
                        0xe,
                        [0xffffe, 0xfffff, 0, 1, 2],
                        0x11,
                        0x13,
                        lane + 1,
                        lane + 3,
                    ),
                ] {
                    for far in [false, true] {
                        for operand in [0u16, 2, 0x7fff, 0x8000, 0xfffd, 0xfffe, 0xffff] {
                            let mut code = vec![
                                if far { 0x9a } else { 0xe8 },
                                operand as u8,
                                (operand >> 8) as u8,
                            ];
                            if far {
                                code.extend_from_slice(&[0xff, 0xff]);
                            }
                            mapped_code(&mut b, &addresses, &code);
                            if model != Model::Mono {
                                b.output(0x60, mode).unwrap();
                            }
                            b.output(0xa0, control).unwrap();
                            c.segments[1] = cs;
                            c.ip = ip;
                            c.registers[4] = 0x2800;
                            c.flags = 0xfcd7;
                            let next: u16 = if far { far_next } else { near_next };
                            let target = if far {
                                operand
                            } else {
                                next.wrapping_add(operand)
                            };
                            let mut regs = c.registers;
                            let mut ram = b.ram.clone();
                            let ports = b.ports;
                            regs[4] = frame(&mut ram, 0x20, 0x2800, cs, next, far);
                            let fetch = lane + if far { far_words } else { near_words };
                            let writes = if far { 2 } else { 1 };
                            assert_eq!(
                                c.instruction_step(&mut b).unwrap(),
                                fetch + writes + if far { 8 } else { 4 } + u32::from(target & 1)
                            );
                            assert_eq!(b.accesses.get(), fetch + writes);
                            assert_eq!(
                                (c.registers, c.segments, c.ip, c.flags),
                                (
                                    regs,
                                    [0x10, if far { 0xffff } else { cs }, 0x20, 0x30],
                                    target,
                                    0xfcd7
                                )
                            );
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
fn call_wrapping_stacks_mapped_sram_waits_and_real_returns() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for waits in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | waits).unwrap();
                }
                for far in [false, true] {
                    for ss in [0, 0x1000] {
                        for sp in [0u16, 1, 2, 3, 0x2800, 0x2801] {
                            c.segments[1] = 0;
                            c.segments[2] = ss;
                            c.ip = 0x1000;
                            c.registers[4] = sp;
                            c.flags = 0xfcd7;
                            let code = if far {
                                &[0x9a, 0, 0x12, 0, 0][..]
                            } else {
                                &[0xe8, 0xfd, 1][..]
                            };
                            b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                            b.ram[0x1200] = if far { 0xcb } else { 0xc3 };
                            let sram = ss == 0x1000;
                            let accessible = model != Model::Mono && mode != 0;
                            // All wrapping internal words here are wholly high or split
                            // FFFF->0000; only color mode consolidates high even pairs.
                            let per_word = if sram {
                                if model != Model::Mono && waits != 0 {
                                    4
                                } else {
                                    2
                                }
                            } else if sp & 1 == 0 && (sp >= 2 || accessible) {
                                1
                            } else {
                                2
                            };
                            let writes = if far && !sram && !accessible && sp == 2 {
                                3
                            } else {
                                per_word * if far { 2 } else { 1 }
                            };
                            let next = if far { 0x1005 } else { 0x1003 };
                            assert_eq!(
                                c.instruction_step(&mut b).unwrap(),
                                if far { 5 + 8 + writes } else { 3 + 4 + writes },
                                "{model:?}/{mode:x}/{waits}/{far}/{ss:x}/{sp:x}"
                            );
                            assert_eq!(b.accesses.get(), if far { 5 } else { 3 } + writes);
                            let saved_sp = sp.wrapping_sub(if far { 4 } else { 2 });
                            assert_eq!(c.registers[4], saved_sp);
                            // Compare each low/high lane, including ignored high mono RAM.
                            for (offset, value) in if far {
                                vec![(saved_sp, next), (sp.wrapping_sub(2), 0)]
                            } else {
                                vec![(saved_sp, next)]
                            } {
                                for (off, byte) in [
                                    (offset, value as u8),
                                    (offset.wrapping_add(1), (value >> 8) as u8),
                                ] {
                                    let address = (u32::from(ss) << 4) + u32::from(off);
                                    assert_eq!(
                                        b.read(address),
                                        if !sram && !accessible && off >= 0x4000 {
                                            0xff
                                        } else {
                                            byte
                                        }
                                    );
                                }
                            }
                            if sram || accessible || sp >= 4 {
                                c.instruction_step(&mut b).unwrap();
                                assert_eq!(
                                    (c.segments[1], c.ip, c.registers[4], c.flags),
                                    (0, next, sp, 0xfcd7)
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

fn install_call(b: &mut Bus, far: bool, odd: bool) {
    let code = if far {
        vec![0x9a, u8::from(odd), 0x12, 0, 0]
    } else {
        vec![0xe8, if odd { 0xfe } else { 0xfd }, 1]
    };
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
    b.ram[0x1200..0x1202].fill(0x43); // INC BX exposes an extra destination instruction
}

#[test]
fn normal_run_frame_call_advances_raster_timer_audio_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model);
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
            for (far, base, odd_stack) in [(false, 8, 1), (true, 15, 2)] {
                for odd in [false, true] {
                    for (sp, extra) in [(0x2800, 0), (0x2801, odd_stack)] {
                        install_call(&mut m.bus, far, odd);
                        m.cpu.segments[1] = 0;
                        m.cpu.ip = 0x1000;
                        m.cpu.flags = 0xf042;
                        m.cpu.registers[4] = sp;
                        let mut regs = m.cpu.registers;
                        regs[4] -= if far { 4 } else { 2 };
                        let cost = base + u64::from(odd) + extra;
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
                            (0x1200 + u16::from(odd), regs, 0xf042)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn call_irq_tf_priority_shadow_nested_frames_and_iret_return() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (far, base, stack_extra, fetch) in [(false, 8, 1, 3), (true, 15, 2, 5)] {
                for odd in [false, true] {
                    for (pending, tf, shadow) in [
                        (true, false, false),
                        (true, false, true),
                        (true, true, true),
                        (false, true, false),
                    ] {
                        for (sp, extra, irq, trap) in
                            [(0x2800, 0, 32, 25), (0x2801, stack_extra, 35, 28)]
                        {
                            let (mut c, mut b) = setup(model);
                            if model != Model::Mono {
                                b.output(0x60, mode).unwrap();
                            }
                            install_call(&mut b, far, odd);
                            c.segments[2] = 0;
                            c.registers[4] = sp;
                            c.flags = 0xf242 | if tf { TF } else { 0 };
                            c.interrupt_shadow = u8::from(shadow);
                            let flags = c.flags;
                            let registers = c.registers;
                            b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                            b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                            b.ram[0x300] = 0xcf;
                            b.ram[0x400] = 0xcf;
                            b.output(0xb2, 0x80).unwrap();
                            if pending {
                                b.request(7);
                            }
                            let before = pending && !shadow;
                            let entered = before || tf;
                            let cost = base + u32::from(odd) + extra;
                            let entry = if pending { irq } else { trap };
                            assert_eq!(
                                c.step(&mut b).unwrap(),
                                if before {
                                    irq
                                } else {
                                    cost + if tf { entry } else { 0 }
                                }
                            );
                            let call_words = if far { 2 } else { 1 };
                            let after_call = sp - if before { 0 } else { 2 * call_words };
                            let continuation = if before {
                                0x1000
                            } else {
                                0x1200 + u16::from(odd)
                            };
                            let mut regs = registers;
                            regs[4] = after_call - if entered { 6 } else { 0 };
                            assert_eq!(c.registers, regs);
                            if entered {
                                assert_eq!(
                                    (c.segments[1], c.ip, c.flags),
                                    (0, if pending { 0x400 } else { 0x300 }, flags & !0x300)
                                );
                                let frame = (after_call - 6) as usize;
                                assert_eq!(
                                    &b.ram[frame..frame + 6],
                                    &[
                                        continuation as u8,
                                        (continuation >> 8) as u8,
                                        0,
                                        0,
                                        flags as u8,
                                        (flags >> 8) as u8
                                    ]
                                );
                                assert_eq!(
                                    b.accesses.get(),
                                    if before {
                                        irq - 27
                                    } else {
                                        fetch
                                            + u32::from(call_words) * (1 + u32::from(sp & 1))
                                            + entry
                                            - if pending { 27 } else { 20 }
                                    }
                                );
                                if pending {
                                    b.output(0xb6, 0x80).unwrap();
                                }
                                c.step(&mut b).unwrap();
                                assert_eq!(
                                    (c.ip, c.flags, c.registers[4]),
                                    (continuation, flags, after_call)
                                );
                                if !before {
                                    b.ram[continuation as usize] = if far { 0xcb } else { 0xc3 };
                                    c.instruction_step(&mut b).unwrap();
                                    assert_eq!(
                                        (c.ip, c.segments[1], c.registers[4]),
                                        (if far { 0x1005 } else { 0x1003 }, 0, sp)
                                    );
                                }
                            } else {
                                assert_eq!((c.ip, c.flags), (continuation, flags));
                                assert_eq!(c.step(&mut b).unwrap(), irq);
                                assert_eq!(c.ip, 0x400);
                                assert_eq!(
                                    &b.ram[(after_call - 6) as usize..(after_call - 4) as usize],
                                    &continuation.to_le_bytes()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn call_after_separate_dma_reservations_retains_model_and_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (far, cost, fetch, sp) in [(false, 9, 4, 0x27fe), (true, 16, 7, 0x27fc)] {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.flags = 0xf042;
                    c.registers[1] = 0;
                    install_call(&mut b, far, true);
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
                    assert_eq!((c.ip, c.registers[1], c.flags), (0x1201, 0, 0xf042));
                    assert_eq!(c.registers[4], sp);
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

#[test]
fn call_stack_physical_twenty_bit_wrap_keeps_rom_write_policy() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for (control, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
            for far in [false, true] {
                b.output(0xa0, control).unwrap();
                let code = if far {
                    &[0x9a, 0x10, 0, 0xff, 0xff][..]
                } else {
                    &[0xe8, 0xfd, 1][..]
                };
                b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                c.ip = 0x1000;
                c.segments = [0x10, 0, 0xffff, 0x30];
                c.registers[4] = 0x11;
                c.flags = 0xfcd7;
                let mut regs = c.registers;
                regs[4] = if far { 0xd } else { 0xf };
                let mut ram = b.ram.clone();
                ram[0] = if far { 0 } else { 0x10 };
                // Near writes FFFFF,00000; far first writes CS there then
                // IP at FFFFD,FFFFE. ROM writes retain ignored-write behavior.
                let writes = if far { 3 * lane + 1 } else { lane + 1 };
                let fetch = if far { 5 } else { 3 };
                assert_eq!(
                    c.instruction_step(&mut b).unwrap(),
                    fetch + writes + if far { 8 } else { 4 }
                );
                assert_eq!(b.accesses.get(), fetch + writes);
                assert_eq!(
                    (c.registers, c.segments, c.ip, c.flags),
                    (
                        regs,
                        [0x10, if far { 0xffff } else { 0 }, 0xffff, 0x30],
                        if far { 0x10 } else { 0x1200 },
                        0xfcd7
                    )
                );
                assert!(b.ram == ram);
            }
        }
    }
}

#[test]
fn real_retf_restores_nonzero_mapped_caller_segment() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for (control, cost) in [(0, 15), (4, 13), (8, 20), (12, 16)] {
            mapped_code(
                &mut b,
                &[0x21001, 0x21002, 0x21003, 0x21004, 0x21005],
                &[0x9a, 0, 0x12, 0, 0],
            );
            b.output(0xa0, control).unwrap();
            b.ram[0x1200] = 0xcb;
            c.ip = 0x1001;
            c.segments = [0x10, 0x2000, 0x20, 0x30];
            c.registers[4] = 0x2800;
            c.flags = 0xfcd7;
            let registers = c.registers;
            assert_eq!(c.instruction_step(&mut b).unwrap(), cost);
            assert_eq!(&b.ram[0x29fc..0x2a00], &[6, 0x10, 0, 0x20]);
            assert_eq!(
                c.instruction_step(&mut b).unwrap(),
                7,
                "retained RETF approximation"
            );
            assert_eq!(
                (c.ip, c.segments, c.registers, c.flags),
                (0x1006, [0x10, 0x2000, 0x20, 0x30], registers, 0xfcd7)
            );
        }
    }
}
