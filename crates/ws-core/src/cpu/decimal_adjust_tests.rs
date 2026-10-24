// SPDX-License-Identifier: CC0-1.0
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

// Independent literals; derivation and table provenance: docs/decimal-adjust.md.
// opcode, incoming AX/FLAGS, expected AX/full FLAGS.
const VECTORS: &[(u8, u16, u16, u16, u16)] = &[
    (0x27, 0x007a, 0xf002, 0x0080, 0xf892),
    (0x2f, 0x0000, 0xf012, 0x00fa, 0xf096),
    (0x27, 0x0000, 0xf002, 0x0000, 0xf046),
    (0x27, 0x0009, 0xf002, 0x0009, 0xf006),
    (0x27, 0x000a, 0xf002, 0x0010, 0xf012),
    (0x27, 0x0079, 0xf002, 0x0079, 0xf002),
    (0x27, 0x007f, 0xf002, 0x0085, 0xf892),
    (0x27, 0x0080, 0xf002, 0x0080, 0xf082),
    (0x27, 0x0099, 0xf002, 0x0099, 0xf086),
    (0x27, 0x009a, 0xf002, 0x0000, 0xf057),
    (0x27, 0x00f9, 0xf002, 0x0059, 0xf007),
    (0x27, 0x00fa, 0xf002, 0x0060, 0xf017),
    (0x27, 0x00ff, 0xf002, 0x0065, 0xf017),
    (0x27, 0x0000, 0xf012, 0x0006, 0xf016),
    (0x27, 0x0000, 0xf003, 0x0060, 0xf007),
    (0x27, 0x0079, 0xf003, 0x00d9, 0xf883),
    (0x27, 0x0099, 0xf003, 0x00f9, 0xf087),
    (0x27, 0x0000, 0xf013, 0x0066, 0xf017),
    (0x27, 0x00ff, 0xf013, 0x0065, 0xf017),
    (0x2f, 0x0000, 0xf002, 0x0000, 0xf046),
    (0x2f, 0x0009, 0xf002, 0x0009, 0xf006),
    (0x2f, 0x000a, 0xf002, 0x0004, 0xf012),
    (0x2f, 0x0079, 0xf002, 0x0079, 0xf002),
    (0x2f, 0x007a, 0xf002, 0x0074, 0xf016),
    (0x2f, 0x007f, 0xf002, 0x0079, 0xf012),
    (0x2f, 0x0080, 0xf002, 0x0080, 0xf082),
    (0x2f, 0x0099, 0xf002, 0x0099, 0xf086),
    (0x2f, 0x009a, 0xf002, 0x0034, 0xf813),
    (0x2f, 0x00f9, 0xf002, 0x0099, 0xf087),
    (0x2f, 0x00fa, 0xf002, 0x0094, 0xf093),
    (0x2f, 0x00ff, 0xf002, 0x0099, 0xf097),
    (0x2f, 0x0000, 0xf003, 0x00a0, 0xf087),
    (0x2f, 0x0079, 0xf003, 0x0019, 0xf003),
    (0x2f, 0x0099, 0xf003, 0x0039, 0xf807),
    (0x2f, 0x0000, 0xf013, 0x009a, 0xf097),
    (0x2f, 0x00ff, 0xf013, 0x0099, 0xf097),
    (0x27, 0x0000, 0xf802, 0x0000, 0xf046),
];

fn literal_vectors(instruction: u8) {
    for (model, mode) in modes() {
        for &(opcode, ax, incoming, result, flags) in VECTORS {
            if opcode != instruction {
                continue;
            }
            for (profile, control) in [(incoming, 0), (0xf8c6, 0), (0xffc6, 0x700)] {
                for location in 0..4 {
                    let mut b = Bus::new(model);
                    if model != Model::Mono {
                        b.output(0x60, mode).unwrap();
                    }
                    let code = [opcode, 0xb9, 0xef, 0xbe]; // MOV CX,BEEF continuation
                    let mut cart = crate::cartridge::Cartridge::parse(
                        &crate::cartridge::tests::rom(131072, 2, 0),
                    )
                    .unwrap();
                    cart.bytes[65536..65540].copy_from_slice(&code);
                    b.insert_cartridge(cart);
                    // Both RAM and ROM with wait control enabled/disabled.
                    b.output(0xa0, if location % 2 == 0 { 4 } else { 12 })
                        .unwrap();
                    b.ram[0x1000..0x1004].copy_from_slice(&code);
                    let mut c = Cpu {
                        registers: [ax, 0x5678, 0xabcd, 0x9876, 0x2000, 0x820, 0x840, 0x860],
                        segments: [0x80, if location < 2 { 0 } else { 0xf000 }, 0x20, 0xc0],
                        ip: if location < 2 { 0x1000 } else { 0 },
                        flags: profile | (incoming & 0x11),
                        ..Cpu::default()
                    };
                    let start = c.ip;
                    let mut registers = c.registers;
                    let segments = c.segments;
                    let ram = b.ram.clone();
                    let ports = b.ports;
                    let save = b.cartridge.as_ref().unwrap().save.clone();
                    let access = if location == 3 { 2 } else { 1 };
                    let clocks = c.step(&mut b).unwrap();
                    registers[0] = result;
                    assert_eq!(
                        c.registers, registers,
                        "{model:?}/{mode:x} {opcode:x} AX={ax:04x}"
                    );
                    assert_eq!(c.flags, flags | control);
                    assert_eq!(clocks, 10 + access);
                    assert_eq!(c.ip, start + 1);
                    assert_eq!(c.segments, segments);
                    assert!(!c.halted);
                    assert_eq!(b.accesses.get(), access);
                    assert_eq!(b.cycles, 0); // scheduler alone advances devices
                    assert_eq!(b.ports, ports);
                    assert_eq!(b.ram, ram);
                    assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                    c.step(&mut b).unwrap();
                    registers[1] = 0xbeef;
                    assert_eq!(c.registers, registers);
                    assert_eq!(c.ip, start + 4);
                    assert_eq!(c.flags, flags | control);
                }
            }
        }
    }
}

#[test]
fn decimal_adjust_daa_literal_flags_state_continuation_and_fetch_costs() {
    literal_vectors(0x27);
}
#[test]
fn decimal_adjust_das_literal_flags_state_continuation_and_fetch_costs() {
    literal_vectors(0x2f);
}

#[test]
fn decimal_adjust_all_al_af_cf_ah_and_opposing_flags_from_literal_tables() {
    // Complete independent literal results, no production adjustment/ALU calls.
    let table: Vec<[u16; 5]> = include_str!("decimal_adjust_vectors.txt")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<u16> = line
                .split_whitespace()
                .map(|v| u16::from_str_radix(v, 16).unwrap())
                .collect();
            fields.try_into().unwrap()
        })
        .collect();
    assert_eq!(table.len(), 2 * 4 * 256);
    for (model, mode) in modes() {
        let mut b = Bus::new(model);
        if model != Model::Mono {
            b.output(0x60, mode).unwrap();
        }
        for (index, &[opcode, al, incoming, result, flags]) in table.iter().enumerate() {
            // Validate ordering so a duplicate/missing input cannot reduce coverage.
            assert_eq!(opcode, if index < 1024 { 0x27 } else { 0x2f });
            assert_eq!(al as usize, index % 256);
            assert_eq!(incoming, [0, 1, 16, 17][index / 256 % 4]);
            b.ram[0x1000] = opcode as u8;
            for ah in [0, 0xab00] {
                for (profile, control) in [(0xf002, 0), (0xffc6, 0x700)] {
                    let mut c = Cpu {
                        ip: 0x1000,
                        segments: [0; 4],
                        flags: profile | incoming,
                        ..Cpu::default()
                    };
                    c.registers[0] = ah | al;
                    assert_eq!(c.step(&mut b).unwrap(), 11);
                    assert_eq!(c.registers[0], ah | result);
                    assert_eq!(
                        c.flags,
                        flags | control,
                        "{model:?}/{mode:x} op={opcode:x} AL={al:x} AF/CF={incoming:x}"
                    );
                    assert_eq!(c.ip, 0x1001);
                }
            }
        }
    }
}

#[test]
fn decimal_adjust_runtime_modes_scheduler_advances_timer_raster_and_pcm_once() {
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
            for &(opcode, ax, incoming, result, flags) in VECTORS {
                m.cpu = Cpu {
                    ip: 0x1000,
                    segments: [0; 4],
                    flags: incoming,
                    ..Cpu::default()
                };
                m.cpu.registers[0] = ax;
                m.bus.ram[0x1000] = opcode;
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                m.advance((remaining - 11) as u32);
                // One HBlank at frame wrap must decrement this counter once.
                m.bus.output_word(0xa4, 2).unwrap();
                m.bus.output(0xa2, 1).unwrap();
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + 11);
                assert_eq!(m.frame_number(), frame + 1);
                assert_eq!(m.bus.raster_line, 0);
                assert_eq!(m.audio_samples().len(), 2);
                assert_eq!(m.bus.accesses.get(), 1);
                assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                assert_eq!(
                    (m.cpu.ip, m.cpu.registers[0], m.cpu.flags),
                    (0x1001, result, flags)
                );
            }
        }
    }
}

#[test]
fn decimal_adjust_preserves_prefix_policy_and_interrupt_continuation() {
    for (model, mode) in modes() {
        for (opcode, ax, incoming, result, flags, handler_flags) in [
            (0x27, 0xab7a, 0xf602, 0xab80, 0xfe92u16, 0xfc92),
            (0x2f, 0xab00, 0xf612, 0xabfa, 0xf696u16, 0xf496),
        ] {
            let mut b = Bus::new(model);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            // Segment overrides and ineffective BUSLOCK retain ordinary decode.
            b.ram[0x1000..0x1005].copy_from_slice(&[0x26, 0xf0, opcode, 0xcc, 0xf4]);
            b.ram[12..16].copy_from_slice(&[0, 3, 0, 0]);
            b.ram[0x300] = 0xcf;
            let mut c = Cpu {
                ip: 0x1000,
                segments: [0; 4],
                flags: incoming,
                ..Cpu::default()
            };
            c.registers[0] = ax;
            c.registers[4] = 0x2000;
            assert_eq!(c.step(&mut b).unwrap(), 13);
            assert_eq!((c.ip, c.flags, c.registers[0]), (0x1003, flags, result));
            c.step(&mut b).unwrap(); // INT3 stacks complete post-adjust FLAGS.
            assert_eq!((c.ip, c.flags), (0x300, handler_flags));
            assert_eq!(
                &b.ram[0x1ffa..0x2000],
                &[4, 0x10, 0, 0, flags as u8, (flags >> 8) as u8]
            );
            c.step(&mut b).unwrap(); // IRET resumes after the one-byte INT3.
            assert_eq!((c.ip, c.flags, c.registers[4]), (0x1004, flags, 0x2000));
            for repeat in [0xf2, 0xf3] {
                b.ram[0x1000..0x1002].copy_from_slice(&[repeat, opcode]);
                c.ip = 0x1000;
                let flags = c.flags;
                assert!(c
                    .step(&mut b)
                    .unwrap_err()
                    .to_string()
                    .contains("Unsupported REP"));
                assert_eq!((c.registers[0], c.flags), (result, flags));
            }
            c.ip = 0x1004;
            c.step(&mut b).unwrap();
            assert!(c.halted);
            assert_eq!((c.registers[0], c.flags), (result, flags));
        }
    }
}

#[test]
fn decimal_adjust_normal_scheduler_preserves_separate_gdma_accounting() {
    for (model, mode) in modes() {
        if model == Model::Mono {
            continue;
        }
        for opcode in [0x27, 0x2f] {
            let mut m = crate::Machine::new(model);
            m.bus.insert_cartridge(
                crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                    .unwrap(),
            );
            m.bus.output(0x60, mode).unwrap();
            m.cpu = Cpu {
                ip: 0x1000,
                segments: [0; 4],
                flags: 0xf802,
                ..Cpu::default()
            };
            m.bus.ram[0x1000] = opcode;
            m.bus.ram[0x800..0x802].copy_from_slice(&[0x12, 0x34]);
            let cost = if mode == 0 { 11 } else { 18 };
            m.advance((crate::CYCLES_PER_FRAME - cost) as u32);
            // GDMA is gated off in mode 00; color modes charge seven clocks
            // for one RAM word, separately from the eleven-clock adjustment.
            for (port, value) in [
                (0x40, 0),
                (0x41, 8),
                (0x42, 0),
                (0x44, 0),
                (0x45, 9),
                (0x46, 2),
                (0x47, 0),
                (0x48, 0x80),
            ] {
                m.bus.output(port, value).unwrap();
            }
            let before = m.bus.cycles;
            m.audio_samples();
            m.run_frame().unwrap();
            assert_eq!(m.bus.cycles, before + cost);
            assert_eq!(m.frame_number(), 1);
            assert_eq!(m.bus.raster_line, 0);
            assert_eq!(m.audio_samples().len(), 2);
            assert_eq!(
                &m.bus.ram[0x900..0x902],
                if mode == 0 { &[0, 0] } else { &[0x12, 0x34] }
            );
            assert_eq!(
                m.bus.input_word(0x46).unwrap(),
                if mode == 0 { 2 } else { 0 }
            );
            assert_eq!(
                (m.cpu.ip, m.cpu.registers[0], m.cpu.flags),
                (0x1001, 0, 0xf046)
            );
        }
    }
}
