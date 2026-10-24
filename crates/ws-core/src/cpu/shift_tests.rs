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
        (Model::Crystal, 0),
        (Model::Crystal, 0x80),
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
                        assert_eq!(cost, 2 + u32::from(count & 31) + access);
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
            for mode in [0, 0x80, 0, 0xc0] {
                if model != Model::Mono {
                    m.bus.output(0x60, mode).unwrap();
                }
                m.cpu.ip = 0x1000;
                m.cpu.registers[0] = 0x40;
                m.cpu.flags = 0xffd7;
                for (cost, ip, access, value, flags) in [
                    (5, 0x1003, 3, 0x40, 0xff03),
                    (5, 0x1005, 2, 0x80, 0xff82),
                    (21, 0x1008, 3, 0x80, 0xf782),
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
