// SPDX-License-Identifier: CC0-1.0
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

// Independent literal AX/full-FLAGS vectors: quotient/remainder boundaries,
// modulo-256 product, final addition carry/auxiliary carry/signed overflow/SZP.
const VECTORS: &[(u8, u8, u16, u16, u16)] = &[
    (0xd4, 10, 0xa52a, 0x0402, 0xf002),
    (0xd5, 10, 0x0106, 0x0010, 0xf012),
    (0xd4, 1, 0xa500, 0x0000, 0xf046),
    (0xd4, 1, 0xa5ff, 0xff00, 0xf046),
    (0xd4, 2, 0xa501, 0x0001, 0xf002),
    (0xd4, 2, 0xa5ff, 0x7f01, 0xf002),
    (0xd4, 10, 0xa528, 0x0400, 0xf046),
    (0xd4, 10, 0xa509, 0x0009, 0xf006),
    (0xd4, 16, 0xa5ff, 0x0f0f, 0xf006),
    (0xd4, 255, 0xa57f, 0x007f, 0xf002),
    (0xd4, 255, 0xa580, 0x0080, 0xf082),
    (0xd4, 255, 0xa581, 0x0081, 0xf086),
    (0xd4, 255, 0xa5fe, 0x00fe, 0xf082),
    (0xd4, 255, 0xa5ff, 0x0100, 0xf046),
    (0xd5, 0, 0xff80, 0x0080, 0xf082),
    (0xd5, 1, 0x0000, 0x0000, 0xf046),
    (0xd5, 1, 0x010f, 0x0010, 0xf012),
    (0xd5, 1, 0x017f, 0x0080, 0xf892),
    (0xd5, 1, 0x01ff, 0x0000, 0xf057),
    (0xd5, 10, 0x1a00, 0x0004, 0xf002),
    (0xd5, 10, 0x1aff, 0x0003, 0xf017),
    (0xd5, 16, 0x10ff, 0x00ff, 0xf086),
    (0xd5, 16, 0x0870, 0x00f0, 0xf086),
    (0xd5, 128, 0x0180, 0x0000, 0xf847),
    (0xd5, 128, 0x0301, 0x0081, 0xf086),
    (0xd5, 255, 0x0101, 0x0000, 0xf057),
    (0xd5, 255, 0x027f, 0x007d, 0xf017),
];

fn vectors(conversion: u8) {
    for (model, mode) in modes() {
        for &(opcode, base, ax, result, flags) in VECTORS {
            if opcode != conversion {
                continue;
            }
            for (incoming, control) in [(0xf002, 0), (0xf8d7, 0), (0xffd7, 0x700)] {
                // Different incoming AH must have no effect on AAM.
                let original_ah = [ax >> 8];
                for ah in if opcode == 0xd4 {
                    &[0, 0xff][..]
                } else {
                    &original_ah[..]
                } {
                    for location in 0..3 {
                        let mut b = Bus::new(model);
                        if model != Model::Mono {
                            b.output(0x60, mode).unwrap();
                        }
                        let code = [opcode, base, 0xb9, 0xef, 0xbe];
                        let mut c = Cpu {
                            registers: [
                                (ax & 255) | ah << 8,
                                0x5678,
                                0xabcd,
                                0x9876,
                                0x2000,
                                0x820,
                                0x840,
                                0x860,
                            ],
                            segments: [0x80, 0, 0x20, 0xc0],
                            ip: 0x1000,
                            flags: incoming,
                            ..Cpu::default()
                        };
                        if location == 0 {
                            b.ram[0x1000..0x1005].copy_from_slice(&code);
                        } else {
                            let mut cart = crate::cartridge::Cartridge::parse(
                                &crate::cartridge::tests::rom(131072, 2, 0),
                            )
                            .unwrap();
                            cart.bytes[65536..65541].copy_from_slice(&code);
                            b.insert_cartridge(cart);
                            b.output(0xa0, if location == 1 { 4 } else { 12 }).unwrap();
                            c.segments[1] = 0xf000;
                            c.ip = 0;
                        }
                        let start = c.ip;
                        let mut registers = c.registers;
                        let segments = c.segments;
                        let ram = b.ram.clone();
                        let ports = b.ports;
                        let save = b.cartridge.as_ref().map(|cart| cart.save.clone());
                        let access = if location == 2 { 4 } else { 2 };
                        let execution = if opcode == 0xd4 { 16 } else { 1 };
                        assert_eq!(c.instruction_step(&mut b).unwrap(), execution + access);
                        registers[0] = result;
                        assert_eq!(
                            c.registers, registers,
                            "{model:?}/{mode:x} {opcode:x}/{base} AX={ax:04x}"
                        );
                        assert_eq!(c.flags, flags | control, "{model:?}/{mode:x} {opcode:x}/{base} AX={ax:04x} incoming={incoming:04x}");
                        assert_eq!(c.ip, start + 2);
                        assert_eq!(c.segments, segments);
                        assert!(!c.halted);
                        assert_eq!(b.accesses.get(), access);
                        assert_eq!(b.cycles, 0);
                        assert_eq!(b.ports, ports);
                        assert_eq!(b.ram, ram);
                        assert_eq!(b.cartridge.as_ref().map(|cart| &cart.save), save.as_ref());
                        c.instruction_step(&mut b).unwrap();
                        registers[1] = 0xbeef;
                        assert_eq!(c.registers, registers);
                        assert_eq!(c.ip, start + 5);
                        assert_eq!(c.flags, flags | control);
                    }
                }
            }
        }
    }
}

#[test]
fn conversion_aam_literal_full_flags_state_continuation_and_fetch_costs() {
    vectors(0xd4);
}

#[test]
fn conversion_aad_literal_full_flags_state_continuation_and_fetch_costs() {
    vectors(0xd5);
}

#[test]
fn conversions_runtime_modes_normal_scheduler_advances_shared_devices_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = crate::Machine::new(model);
        m.bus.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap(),
        );
        let modes: &[u8] = if model == Model::Mono {
            &[0]
        } else {
            &[0, 0x80, 0, 0xc0, 0, 0xe0, 0]
        };
        for &mode in modes {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
                assert_eq!(m.bus.input(0x60).unwrap(), mode);
            }
            for (opcode, ax, incoming, result, flags, cost) in [
                (0xd4, 0xa52a, 0xf8d7, 0x0402, 0xf002, 18),
                (0xd5, 0x0106, 0xf002, 0x0010, 0xf012, 3),
            ] {
                m.cpu = Cpu {
                    ip: 0x1000,
                    segments: [0; 4],
                    flags: incoming,
                    ..Cpu::default()
                };
                m.cpu.registers[0] = ax;
                m.bus.ram[0x1000..0x1002].copy_from_slice(&[opcode, 10]);
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                m.advance((remaining - cost) as u32);
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + cost);
                assert_eq!(m.frame_number(), frame + 1);
                assert_eq!(m.bus.raster_line, 0);
                assert_eq!(m.audio_samples().len(), 2);
                assert_eq!(m.bus.accesses.get(), 2);
                assert_eq!(
                    (m.cpu.ip, m.cpu.registers[0], m.cpu.flags),
                    (0x1002, result, flags)
                );
            }
        }
    }
}

#[test]
fn aam_zero_base_keeps_existing_fault_stack_and_return_behavior() {
    for (model, mode) in modes() {
        let mut b = Bus::new(model);
        if model != Model::Mono {
            b.output(0x60, mode).unwrap();
        }
        b.ram[0..4].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[0x1000..0x1003].copy_from_slice(&[0xd4, 0, 0xf4]);
        b.ram[0x300] = 0xcf; // IRET
        let mut c = Cpu {
            ip: 0x1000,
            segments: [0; 4],
            flags: 0xffd7,
            ..Cpu::default()
        };
        c.registers = [0xa52a, 1, 2, 3, 0x2000, 5, 6, 7];
        let mut registers = c.registers;
        let cost = c.step(&mut b).unwrap();
        registers[4] = 0x1ffa;
        assert_eq!(c.registers, registers);
        assert_eq!(c.ip, 0x300);
        assert_eq!(c.flags, 0xfcd7);
        assert_eq!(&b.ram[0x1ffa..0x2000], &[0, 0x10, 0, 0, 0xd7, 0xff]);
        assert_eq!(cost, 16 + b.accesses.get());
        assert_eq!(b.cycles, 0);
        // Model a handler advancing saved IP past both bytes, then IRET.
        b.ram[0x1ffa..0x1ffc].copy_from_slice(&[2, 0x10]);
        c.step(&mut b).unwrap();
        assert_eq!(
            (c.ip, c.flags, c.registers[0], c.registers[4]),
            (0x1002, 0xffd7, 0xa52a, 0x2000)
        );
        // HLT completes with TF restored: observe vector 1 rather than
        // assuming that TF remains inert during sustained standby.
        super::trap_tests::install_handler(&mut b);
        c.step(&mut b).unwrap();
        assert!(!c.halted);
        super::trap_tests::finish_trap(&mut c, &mut b, 0x1003, 0xffd7);
    }
}
