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

// Independent literals, reconciled in docs/ascii-adjust.md. No ALU helpers.
// opcode, incoming AX/FLAGS, expected AX/full FLAGS.
const VECTORS: &[(u8, u16, u16, u16, u16)] = &[
    (0x37, 0x00fa, 0xf002, 0x0100, 0xf057),
    (0x3f, 0x0100, 0xf012, 0x000a, 0xf057),
    (0x37, 0xab00, 0xf002, 0xab00, 0xf086),
    (0x3f, 0xab00, 0xf002, 0xab00, 0xf086),
    (0x37, 0x120b, 0xf002, 0x1301, 0xf057),
    (0x3f, 0x120b, 0xf002, 0x1105, 0xf057),
    (0x37, 0x1201, 0xf012, 0x1307, 0xf057),
    (0x3f, 0x1201, 0xf012, 0x110b, 0xf057),
    (0x37, 0xff0a, 0xf002, 0x0000, 0xf057),
    (0x37, 0xffff, 0xf002, 0x0005, 0xf057),
    (0x3f, 0x0000, 0xf012, 0xff0a, 0xf057),
    (0x3f, 0x000f, 0xf002, 0xff09, 0xf057),
    (0x37, 0x1209, 0xf002, 0x1209, 0xf086),
    (0x3f, 0x1209, 0xf002, 0x1209, 0xf086),
    (0x37, 0x12f0, 0xf002, 0x1200, 0xf086),
    (0x3f, 0x12f0, 0xf002, 0x1200, 0xf086),
];

fn literal_vectors(instruction: u8) {
    for (model, mode) in modes() {
        for &(opcode, ax, incoming, result, flags) in VECTORS {
            if opcode != instruction {
                continue;
            }
            for (profile, control) in [(0xf002, 0), (0xf8c7, 0), (0xffc7, 0x700)] {
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
                        flags: profile | (incoming & 0x10),
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
                    assert_eq!(clocks, 9 + access);
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
fn ascii_adjust_aaa_literal_flags_state_continuation_and_fetch_costs() {
    literal_vectors(0x37);
}
#[test]
fn ascii_adjust_aas_literal_flags_state_continuation_and_fetch_costs() {
    literal_vectors(0x3f);
}

#[test]
fn ascii_adjust_all_al_af_ah_and_opposing_flags_from_literal_tables() {
    // Tables independently transcribed from the documented nibble truth table.
    // Each tuple is output AL, AH table row, complete arithmetic FLAGS.
    // Rows: original AH; adjusted AAA AH; adjusted AAS AH.
    const AH: [[u16; 4]; 3] = [
        [0x00, 0x01, 0xfe, 0xff],
        [0x01, 0x02, 0xff, 0x00],
        [0xff, 0x00, 0xfd, 0xfe],
    ];
    const AAA: [[(u16, usize, u16); 16]; 2] = [
        [
            (0, 0, 0x84),
            (1, 0, 0x84),
            (2, 0, 0x84),
            (3, 0, 0x84),
            (4, 0, 0x84),
            (5, 0, 0x84),
            (6, 0, 0x84),
            (7, 0, 0x84),
            (8, 0, 0x84),
            (9, 0, 0x84),
            (0, 1, 0x55),
            (1, 1, 0x55),
            (2, 1, 0x55),
            (3, 1, 0x55),
            (4, 1, 0x55),
            (5, 1, 0x55),
        ],
        [
            (6, 1, 0x55),
            (7, 1, 0x55),
            (8, 1, 0x55),
            (9, 1, 0x55),
            (10, 1, 0x55),
            (11, 1, 0x55),
            (12, 1, 0x55),
            (13, 1, 0x55),
            (14, 1, 0x55),
            (15, 1, 0x55),
            (0, 1, 0x55),
            (1, 1, 0x55),
            (2, 1, 0x55),
            (3, 1, 0x55),
            (4, 1, 0x55),
            (5, 1, 0x55),
        ],
    ];
    const AAS: [[(u16, usize, u16); 16]; 2] = [
        [
            (0, 0, 0x84),
            (1, 0, 0x84),
            (2, 0, 0x84),
            (3, 0, 0x84),
            (4, 0, 0x84),
            (5, 0, 0x84),
            (6, 0, 0x84),
            (7, 0, 0x84),
            (8, 0, 0x84),
            (9, 0, 0x84),
            (4, 2, 0x55),
            (5, 2, 0x55),
            (6, 2, 0x55),
            (7, 2, 0x55),
            (8, 2, 0x55),
            (9, 2, 0x55),
        ],
        [
            (10, 2, 0x55),
            (11, 2, 0x55),
            (12, 2, 0x55),
            (13, 2, 0x55),
            (14, 2, 0x55),
            (15, 2, 0x55),
            (0, 2, 0x55),
            (1, 2, 0x55),
            (2, 2, 0x55),
            (3, 2, 0x55),
            (4, 2, 0x55),
            (5, 2, 0x55),
            (6, 2, 0x55),
            (7, 2, 0x55),
            (8, 2, 0x55),
            (9, 2, 0x55),
        ],
    ];
    for (model, mode) in modes() {
        let mut b = Bus::new(model);
        if model != Model::Mono {
            b.output(0x60, mode).unwrap();
        }
        for (opcode, table) in [(0x37, AAA), (0x3f, AAS)] {
            b.ram[0x1000] = opcode;
            for al in 0..=255u16 {
                for (af, row) in table.iter().enumerate() {
                    let (result, ah_row, arithmetic) = row[usize::from(al & 15)];
                    for (&original_ah, result_ah) in AH[0].iter().zip(AH[ah_row]) {
                        for (profile, control) in [(0xf002, 0xf002), (0xffc7, 0xf702)] {
                            let mut c = Cpu {
                                ip: 0x1000,
                                segments: [0; 4],
                                flags: profile | ((af as u16) << 4),
                                ..Cpu::default()
                            };
                            c.registers[0] = original_ah << 8 | al;
                            assert_eq!(c.step(&mut b).unwrap(), 10);
                            assert_eq!(
                                c.registers[0],
                                result_ah << 8 | result,
                                "{model:?}/{mode:x} op={opcode:x} AL={al:x} AF={af} AH={:x}",
                                original_ah
                            );
                            assert_eq!(c.flags, control | arithmetic);
                            assert_eq!(c.ip, 0x1001);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn ascii_adjust_runtime_modes_scheduler_advances_timer_raster_and_pcm_once() {
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
                m.advance((remaining - 10) as u32);
                // One HBlank at frame wrap must decrement this counter once.
                m.bus.output_word(0xa4, 2).unwrap();
                m.bus.output(0xa2, 1).unwrap();
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + 10);
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
fn ascii_adjust_preserves_prefix_policy_and_interrupt_continuation() {
    for (model, mode) in modes() {
        for opcode in [0x37, 0x3f] {
            let mut b = Bus::new(model);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            // Segment overrides and ineffective BUSLOCK retain ordinary decode.
            b.ram[0x1000..0x1004].copy_from_slice(&[0x26, 0xf0, opcode, 0xcc]);
            b.ram[12..16].copy_from_slice(&[0, 3, 0, 0]);
            b.ram[0x300] = 0xcf;
            let mut c = Cpu {
                ip: 0x1000,
                segments: [0; 4],
                flags: 0xf602,
                ..Cpu::default()
            };
            c.registers[0] = 0xab00;
            c.registers[4] = 0x2000;
            assert_eq!(c.step(&mut b).unwrap(), 12);
            assert_eq!((c.ip, c.flags, c.registers[0]), (0x1003, 0xf686, 0xab00));
            c.step(&mut b).unwrap(); // INT3 stacks complete post-adjust FLAGS.
            assert_eq!((c.ip, c.flags), (0x300, 0xf486));
            assert_eq!(&b.ram[0x1ffa..0x2000], &[4, 0x10, 0, 0, 0x86, 0xf6]);
            c.step(&mut b).unwrap(); // IRET resumes after the one-byte INT3.
            assert_eq!((c.ip, c.flags, c.registers[4]), (0x1004, 0xf686, 0x2000));
            for repeat in [0xf2, 0xf3] {
                b.ram[0x1000..0x1002].copy_from_slice(&[repeat, opcode]);
                c.ip = 0x1000;
                let flags = c.flags;
                assert!(c
                    .step(&mut b)
                    .unwrap_err()
                    .to_string()
                    .contains("Unsupported REP"));
                assert_eq!((c.registers[0], c.flags), (0xab00, flags));
            }
        }
    }
}
