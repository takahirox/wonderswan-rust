// SPDX-License-Identifier: CC0-1.0
// Literal opcode checks use instruction_step to observe FLAGS before TF entry.
// Ordinary delivery and saved arithmetic FLAGS are covered in trap_tests.
use super::*;
include!("shift_vectors.rs");

fn setup(model: Model, mode: u8, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    if model != Model::Mono {
        b.output(0x60, mode).unwrap();
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

// Three separately executable regressions establish actual baseline failures.
fn regression(code: &[u8], value: u16, incoming: u16, result: u16, flags: u16) {
    let (mut c, mut b) = setup(Model::Mono, 0, code);
    c.registers[0] = value;
    c.flags = incoming;
    c.step(&mut b).unwrap();
    assert_eq!(c.registers[0], result);
    assert_eq!(c.flags, flags);
}
#[test]
fn zero_count_overflow_regression() {
    regression(&[0xc0, 0xe8, 0], 0x40, 0xf002, 0x40, 0xf802);
}
#[test]
fn shift_auxiliary_carry_regression() {
    regression(&[0xd0, 0xe0], 1, 0xf012, 2, 0xf002);
}
#[test]
fn multi_bit_overflow_regression() {
    regression(&[0xc0, 0xe0, 2], 0x20, 0xf002, 0x80, 0xf882);
}

#[test]
fn decoded_full_flags_results_memory_state_and_accounting() {
    for (model, mode) in [
        (Model::Mono, 0),
        (Model::Color, 0),
        (Model::Color, 0x80),
        (Model::Color, 0xc0),
        (Model::Color, 0xe0),
        (Model::Crystal, 0),
        (Model::Crystal, 0x80),
        (Model::Crystal, 0xc0),
        (Model::Crystal, 0xe0),
    ] {
        for &(op, wide, value, count, results, flags) in VECTORS {
            for (pattern, incoming) in [0xf002, 0xffd7].into_iter().enumerate() {
                // Implicit one, CL, immediate; implicit one only admits count=1.
                for form in 0..3 {
                    if form == 0 && count != 1 {
                        continue;
                    }
                    for location in 0..5 {
                        let address = 0x800u16 + u16::from(location % 2 == 0);
                        let opcode = [0xd0, 0xd2, 0xc0][form] | u8::from(wide);
                        let mut code = if location == 0 {
                            vec![opcode, 0xc0 | op << 3]
                        } else {
                            vec![
                                0x26,
                                opcode,
                                6 | op << 3,
                                address as u8,
                                (address >> 8) as u8,
                            ]
                        };
                        if form == 2 {
                            code.push(count);
                        }
                        let length = code.len();
                        code.extend([0xba, 0xef, 0xbe]); // exact next decode: MOV DX,BEEF
                        let (mut c, mut b) = setup(model, mode, &code);
                        c.flags = incoming;
                        c.registers[1] = 0x5a00 | u16::from(count);
                        let mut cart = crate::cartridge::Cartridge::parse(
                            &crate::cartridge::tests::rom(131072, 2, 0),
                        )
                        .unwrap();
                        if location == 0 {
                            c.registers[0] = if wide { value } else { 0xa500 | value };
                        } else if location <= 2 {
                            b.ram[address as usize..address as usize + 2]
                                .copy_from_slice(&value.to_le_bytes());
                        } else {
                            c.segments[0] = 0x1000;
                            cart.save[address as usize..address as usize + 2]
                                .copy_from_slice(&value.to_le_bytes());
                        }
                        b.insert_cartridge(cart);
                        let mut registers = c.registers;
                        let segments = c.segments;
                        let mut ram = b.ram.clone();
                        let ports = b.ports;
                        let mut save = b.cartridge.as_ref().unwrap().save.clone();
                        let expected = results[pattern];
                        if location == 0 {
                            registers[0] = if wide { expected } else { 0xa500 | expected };
                        } else {
                            let memory = if location <= 2 {
                                &mut ram[..]
                            } else {
                                &mut save[..]
                            };
                            memory[address as usize] = expected as u8;
                            if wide {
                                memory[address as usize + 1] = (expected >> 8) as u8;
                            }
                        }
                        let transfer = if location == 0 {
                            0
                        } else if wide && (location >= 3 || address & 1 != 0) {
                            2
                        } else {
                            1
                        };
                        let access = length as u32 + 2 * transfer;
                        let cost = c.instruction_step(&mut b).unwrap();
                        assert_eq!(
                            c.registers, registers,
                            "{model:?}/{mode:x} {code:x?} {value:x}/{count}"
                        );
                        assert_eq!(
                            c.flags, flags[pattern],
                            "{model:?}/{mode:x} {code:x?} incoming {incoming:x}"
                        );
                        assert_eq!(c.segments, segments);
                        assert!(!c.halted);
                        assert_eq!(c.interrupt_shadow, 0);
                        assert_eq!(c.ip, 0x1000 + length as u16);
                        assert_eq!(b.accesses.get(), access);
                        assert_eq!(cost, [1, 3, 3][form] + access);
                        assert_eq!(b.cycles, 0);
                        assert_eq!(b.ports, ports);
                        assert!(b.ram == ram);
                        assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                        c.instruction_step(&mut b).unwrap();
                        registers[2] = 0xbeef;
                        assert_eq!(c.registers, registers);
                        assert_eq!(c.flags, flags[pattern]);
                        assert_eq!(c.ip, 0x1003 + length as u16);
                    }
                }
            }
        }
    }
}

#[test]
fn cl_destination_alias_and_high_byte_decoding() {
    // The count must be sampled before writing CL/CX (and CH preserves CL).
    for (code, cx, ax, result_cx, result_ax, flags) in [
        (&[0xd2, 0xe1][..], 0x5a02, 0x1234, 0x5a08, 0x1234, 0xf002),
        (&[0xd3, 0xe9][..], 0x8001, 0x1234, 0x4000, 0x1234, 0xf807),
        (&[0xd2, 0xc5][..], 0x8102, 0x1234, 0x0602, 0x1234, 0xf002),
        (&[0xd2, 0xfc][..], 0x5a00, 0x4034, 0x5a00, 0x4034, 0xf802),
    ] {
        let (mut c, mut b) = setup(Model::Mono, 0, code);
        c.registers[0] = ax;
        c.registers[1] = cx;
        c.step(&mut b).unwrap();
        assert_eq!(
            (c.registers[0], c.registers[1], c.flags),
            (result_ax, result_cx, flags)
        );
    }
}

#[test]
fn runtime_video_transitions_advance_shared_devices_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, b) = setup(model, 0, &[0xc0, 0xe8, 0, 0xd0, 0xe0, 0xc1, 0xc0, 16]);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for _ in 0..4 {
            for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
                if model != Model::Mono {
                    m.bus.output(0x60, mode).unwrap();
                }
                m.cpu.ip = 0x1000;
                m.cpu.registers[0] = 0x40;
                m.cpu.flags = 0xffd7;
                for (cost, ip, access, value, flags) in [
                    (6, 0x1003, 3, 0x40, 0xff03),
                    (3, 0x1005, 2, 0x80, 0xff82),
                    (6, 0x1008, 3, 0x80, 0xf782),
                ] {
                    let before = m.bus.cycles;
                    let actual = m.cpu.instruction_step(&mut m.bus).unwrap();
                    assert_eq!(actual, cost);
                    assert_eq!(m.cpu.ip, ip);
                    assert_eq!(m.bus.accesses.get(), access);
                    assert_eq!((m.cpu.registers[0], m.cpu.flags), (value, flags));
                    assert_eq!(m.bus.cycles, before);
                    m.advance(actual);
                    assert_eq!(m.bus.cycles, before + u64::from(cost));
                    assert_eq!(u64::from(m.bus.raster_line), m.bus.cycles / 256);
                }
            }
        }
    }
}

// Independent literal totals, recorded before the production correction.
fn literal_timing(code: &[u8], count: u8, expected: u32) {
    let (mut c, mut b) = setup(Model::Mono, 0, code);
    c.registers[0] = 0x81;
    c.registers[1] = u16::from(count);
    assert_eq!(c.instruction_step(&mut b).unwrap(), expected);
}
#[test]
fn literal_implicit_one_timing() {
    literal_timing(&[0xd0, 0xe0], 1, 3);
}
#[test]
fn literal_cl_large_timing() {
    literal_timing(&[0xd2, 0xe0], 31, 5);
}
#[test]
fn literal_immediate_zero_timing() {
    literal_timing(&[0xc0, 0xe0, 32], 32, 6);
}

#[test]
fn shift_mapped_fetch_wait_prefix_and_addressing_accounting() {
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
                for (a0, rom_lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
                    b.output(0xa0, a0).unwrap();
                    let sram_lane = if model != Model::Mono && wait != 0 {
                        2
                    } else {
                        1
                    };
                    for cs in [0u16, 0x1000, 0x2000] {
                        for segment in [0x20u16, 0x1000] {
                            for count_prefix in 0..=7 {
                                for (form, opcode, residual) in
                                    [(0, 0xd0, 1), (1, 0xd2, 3), (2, 0xc0, 3)]
                                {
                                    for wide in [false, true] {
                                        for raw in if form == 0 {
                                            &[1][..]
                                        } else {
                                            &[0, 31, 32][..]
                                        } {
                                            // No prefix: BX defaults DS, BP+disp8 defaults SS.
                                            // Mixed prefixes: last one selects DS or ES, BUSLOCK adds no cost.
                                            for bp in [false, true] {
                                                let prefixes =
                                                    [0x26, 0xf0, 0x2e, 0x36, 0x3e, 0xf0, 0x26];
                                                let mut code = prefixes[..count_prefix].to_vec();
                                                code.extend([
                                                    opcode | u8::from(wide),
                                                    if bp { 0x66 } else { 0x27 },
                                                ]);
                                                if bp {
                                                    code.push(1);
                                                }
                                                if form == 2 {
                                                    code.push(*raw);
                                                }
                                                c.segments = [segment, cs, segment, segment];
                                                // If CS override is last, use a writable code bank for the operand.
                                                let selected =
                                                    if count_prefix == 3 { cs } else { segment };
                                                if selected == 0x2000 {
                                                    continue;
                                                }
                                                c.ip = 0x1000;
                                                c.registers[3] = 0x800;
                                                c.registers[5] = 0x800;
                                                c.registers[1] = 0x5a00 | u16::from(*raw);
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
                                                let address = u32::from(selected) * 16
                                                    + 0x800
                                                    + u32::from(bp);
                                                b.write(address, 0x81);
                                                b.write(address + 1, 0x80);
                                                let mut ram = b.ram.clone();
                                                let mut save =
                                                    b.cartridge.as_ref().unwrap().save.clone();
                                                let regs = c.registers;
                                                let segments = c.segments;
                                                let ports = b.ports;
                                                let value = if *raw & 31 == 0 {
                                                    if wide {
                                                        0x8081
                                                    } else {
                                                        0x81
                                                    }
                                                } else if *raw == 1 {
                                                    if wide {
                                                        0x0102
                                                    } else {
                                                        2
                                                    }
                                                } else {
                                                    0
                                                };
                                                let flags = if *raw & 31 == 0 {
                                                    0xf886
                                                } else if *raw == 1 {
                                                    0xf803
                                                } else {
                                                    0xf046
                                                };
                                                let expected = if address < 0x10000 {
                                                    &mut ram[..]
                                                } else {
                                                    &mut save[..]
                                                };
                                                let index = (address & 0xffff) as usize;
                                                expected[index] = value as u8;
                                                if wide {
                                                    expected[index + 1] = (value >> 8) as u8;
                                                }
                                                let fetch = code.len() as u32
                                                    * if cs == 0 {
                                                        1
                                                    } else if cs == 0x1000 {
                                                        sram_lane
                                                    } else {
                                                        rom_lane
                                                    };
                                                let transfer = if selected == 0x1000 {
                                                    sram_lane * if wide { 2 } else { 1 }
                                                } else if wide && bp {
                                                    2
                                                } else {
                                                    1
                                                };
                                                let cost = c.instruction_step(&mut b).unwrap();
                                                assert_eq!(b.accesses.get(),fetch+2*transfer,"each read and write exactly once, including masked zero");
                                                assert_eq!(cost,fetch+2*transfer+residual,"{model:?}/{mode:x}/{a0:x}/{cs:x}/{segment:x}/{code:x?}/{raw}");
                                                assert_eq!(
                                                    (c.registers, c.segments, c.ip, c.flags),
                                                    (
                                                        regs,
                                                        segments,
                                                        0x1000 + code.len() as u16,
                                                        flags
                                                    )
                                                );
                                                assert!(b.ram == ram);
                                                assert_eq!(
                                                    b.cartridge.as_ref().unwrap().save,
                                                    save
                                                );
                                                assert_eq!(b.ports, ports);
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
        for opcode in [0xc0, 0xc1, 0xd0, 0xd1, 0xd2, 0xd3] {
            for prefix in [0xf2, 0xf3] {
                c.segments[1] = 0;
                c.ip = 0x1000;
                b.ram[0x1000..0x1002].copy_from_slice(&[prefix, opcode]);
                let state = (c.registers, c.segments, c.flags);
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.segments, c.flags), state);
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn shift_operand_offset_bank_twenty_bit_and_available_ram_boundaries() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model, 0, &[]);
        mapped_code(&mut b, &[], &[]);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (segment, offset, first, second) in [
                (0u16, 0x3ffeu16, 0x3ffeu32, 0x3fff),
                (0, 0x3fff, 0x3fff, 0x4000),
                (0, 0x8000, 0x8000, 0x8001),
                (0, 0xffff, 0xffff, 0), // operand offset wraps
                (0x1000, 0xffff, 0x1ffff, 0x10000),
                (0x0fff, 0xf, 0xffff, 0x10000), // RAM/SRAM bank crossing
                (0xffff, 0xf, 0xfffff, 0),      // physical 20-bit wrap
            ] {
                for raw in [0, 1, 32] {
                    c.segments = [0, 0, 0, segment];
                    c.ip = 0x1000;
                    c.registers[3] = offset;
                    c.flags = 0xf002;
                    b.ram[0x1000..0x1003].copy_from_slice(&[0xc1, 0x27, raw]);
                    b.ram[first as usize & 0xffff] = 0x81;
                    b.ram[second as usize & 0xffff] = 0x80;
                    if (0x10000..0x20000).contains(&first) {
                        b.write(first, 0x81);
                    }
                    if (0x10000..0x20000).contains(&second) {
                        b.write(second, 0x80);
                    }
                    let mut ram = b.ram.clone();
                    let mut save = b.cartridge.as_ref().unwrap().save.clone();
                    let available = if model != Model::Mono && mode != 0 {
                        0x10000
                    } else {
                        0x4000
                    };
                    let read = |a: u32| {
                        if a < available {
                            b.ram[a as usize]
                        } else if a < 0x10000 {
                            0xff
                        } else if a < 0x20000 {
                            save[a as usize & 0x7fff]
                        } else {
                            0x0f
                        }
                    };
                    let input = u16::from_le_bytes([read(first), read(second)]);
                    let result = if raw & 31 == 0 {
                        input
                    } else {
                        input.wrapping_mul(2)
                    };
                    for (address, byte) in [(first, result as u8), (second, (result >> 8) as u8)] {
                        if address < available {
                            ram[address as usize] = byte;
                        } else if (0x10000..0x20000).contains(&address) {
                            save[address as usize & 0x7fff] = byte;
                        }
                    }
                    let transfer = if first & 1 == 0 && second == first + 1 && second < available {
                        1
                    } else {
                        2
                    };
                    assert_eq!(
                        c.instruction_step(&mut b).unwrap(),
                        6 + 2 * transfer,
                        "{model:?}/{mode:x}/{segment:x}/{offset:x}/{raw}"
                    );
                    assert_eq!(b.accesses.get(), 3 + 2 * transfer);
                    assert!(b.ram == ram, "only available writable lanes change");
                    assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                    assert_eq!(c.ip, 0x1003);
                }
            }
        }
    }
}

#[test]
fn shift_fetch_offset_and_twenty_bit_wraps() {
    use super::call_tests::mapped_code;
    for (cs, ip, addresses) in [
        (0u16, 0xffffu16, [0xffff, 0, 1]),
        (0x2000, 0xffff, [0x2ffff, 0x20000, 0x20001]),
        (0x2fff, 0xf, [0x2ffff, 0x30000, 0x30001]),
        (0xffff, 0xf, [0xfffff, 0, 1]),
    ] {
        let (mut c, mut b) = setup(Model::Color, 0x80, &[]);
        mapped_code(&mut b, &addresses, &[0xc1, 0xe0, 32]);
        for (a0, lane) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
            b.output(0xa0, a0).unwrap();
            c.segments[1] = cs;
            c.ip = ip;
            c.registers[0] = 0x4000;
            c.flags = 0xf002;
            let ram = b.ram.clone();
            let fetch = addresses
                .iter()
                .map(|a| if *a >= 0x20000 { lane } else { 1 })
                .sum::<u32>();
            assert_eq!(c.instruction_step(&mut b).unwrap(), fetch + 3);
            assert_eq!(b.accesses.get(), fetch);
            assert_eq!(
                (c.registers[0], c.flags, c.ip),
                (0x4000, 0xf006, ip.wrapping_add(3))
            );
            assert!(b.ram == ram);
        }
    }
}

#[test]
fn normal_run_frame_shifts_cross_frame_with_exact_timer_audio_and_continuation() {
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
                (&[0xd0, 0xe0][..], 1, 3, 0x8002, 0xf803),
                (&[0xd3, 0xe0][..], 31, 5, 0, 0xf046),
                (&[0xc1, 0xe0, 32][..], 32, 6, 0x8081, 0xf886),
                (&[0xc1, 0x27, 255][..], 255, 10, 0x8081, 0xf046), // odd word [BX]
            ] {
                m.cpu.ip = 0x1000;
                m.cpu.flags = 0xf002;
                m.cpu.halted = false;
                m.cpu.interrupt_shadow = 0;
                m.cpu.registers[0] = 0x8081;
                m.cpu.registers[1] = count;
                m.cpu.registers[3] = 0x801;
                m.cpu.segments = [0, 0, 0, 0];
                m.bus.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
                m.bus.ram[0x1000 + code.len()] = 0x43; // poison: INC BX must not execute
                m.bus.ram[0x801..0x803].copy_from_slice(&[0x81, 0x80]);
                let mut ram = m.bus.ram.clone();
                if code[1] == 0x27 {
                    ram[0x801..0x803].fill(0);
                }
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
                    (0x1000 + code.len() as u16, value, count, 0x801, flags)
                );
                assert!(m.bus.ram == ram);
            }
        }
    }
}

#[test]
fn shifts_preserve_if_shadow_tf_irq_entry_and_dma_reservations() {
    use super::call_tests::mapped_code;
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            // Delivery follows the complete shift, with separate entry charge.
            for pending in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[0xd2, 0xe0]);
                c.segments = [0, 0, 0, 0];
                c.registers[4] = 0x2800;
                c.registers[0] = 0x81;
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
                    (if pending { 0x400 } else { 0x300 }, 0, 0x27fa, 0xf046)
                );
                assert_eq!(
                    &b.ram[0x27fa..0x2800],
                    &[2, 0x10, 0, 0, 0x46, if pending { 0xf3 } else { 0xf1 }]
                );
                if pending {
                    b.output(0xb6, 0x80).unwrap();
                }
                assert_eq!(c.step(&mut b).unwrap(), 11);
                assert_eq!(
                    (c.ip, c.registers[4], c.flags),
                    (0x1002, 0x2800, if pending { 0xf346 } else { 0xf146 })
                );
            }
            for sound in [false, true] {
                let (mut c, mut b) = setup(model, mode, &[0xc0, 0xe0, 32]);
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
                assert_eq!((c.ip, c.registers[0], c.flags), (0x1003, 0x40, 0xf002));
            }
        }
    }
}

#[test]
fn shift_distinct_default_and_override_segments_displacements_and_alias_costs() {
    // Different physical targets make incorrect DS/SS/last-override selection visible.
    for (code, physical, access, cost) in [
        (&[0xd1, 0x27][..], 0xb00, 4, 5),          // [BX], DS
        (&[0xd1, 0x66, 0][..], 0xa00, 5, 6),       // [BP+0], SS
        (&[0xd1, 0xa7, 0, 1][..], 0xc00, 5, 6),    // [BX+0100], DS; aligned disp16
        (&[0x26, 0xd1, 0x66, 0][..], 0x900, 6, 7), // ES overrides BP
        (&[0x36, 0xd1, 0x27][..], 0xa00, 5, 6),    // SS overrides BX
        (
            &[0x26, 0xf0, 0x36, 0x3e, 0xf0, 0x26, 0xf0, 0xc1, 0x27, 32][..],
            0x900,
            12,
            15,
        ),
    ] {
        let (mut c, mut b) = setup(Model::Mono, 0, code);
        c.segments = [0x10, 0, 0x20, 0x30];
        c.registers[3] = 0x800;
        c.registers[5] = 0x800;
        c.flags = 0xf002;
        b.ram[physical..physical + 2].copy_from_slice(&[0x81, 0x80]);
        let mut ram = b.ram.clone();
        let zero = code.last() == Some(&32);
        if !zero {
            ram[physical..physical + 2].copy_from_slice(&[2, 1]);
        }
        let state = (c.registers, c.segments, b.ports);
        assert_eq!(c.instruction_step(&mut b).unwrap(), cost, "{code:x?}");
        assert_eq!(b.accesses.get(), access);
        assert_eq!((c.registers, c.segments, b.ports), state);
        assert_eq!(
            (c.ip, c.flags),
            (
                0x1000 + code.len() as u16,
                if zero { 0xf886 } else { 0xf803 }
            )
        );
        assert!(b.ram == ram);
    }
    for (code, cx, ax, result_cx, result_ax, flags, cost) in [
        (&[0xd2, 0xe1][..], 0x5a02, 0x1234, 0x5a08, 0x1234, 0xf002, 5),
        (&[0xd3, 0xe9][..], 0x8001, 0x1234, 0x4000, 0x1234, 0xf807, 5),
        (&[0xd2, 0xc5][..], 0x8102, 0x1234, 0x0602, 0x1234, 0xf002, 5),
        (&[0xd2, 0xfc][..], 0x5a00, 0x4034, 0x5a00, 0x4034, 0xf802, 5),
    ] {
        let (mut c, mut b) = setup(Model::Mono, 0, code);
        c.registers[0] = ax;
        c.registers[1] = cx;
        assert_eq!(c.instruction_step(&mut b).unwrap(), cost);
        assert_eq!(
            (c.registers[0], c.registers[1], c.flags, c.ip),
            (result_ax, result_cx, flags, 0x1002)
        );
        assert_eq!(b.accesses.get(), 2);
    }
}
