// SPDX-License-Identifier: CC0-1.0
use super::*;

const MODES: [(Model, u8); 9] = [
    (Model::Mono, 0),
    (Model::Color, 0),
    (Model::Color, 0x80),
    (Model::Color, 0xc0),
    (Model::Color, 0xe0),
    (Model::Crystal, 0),
    (Model::Crystal, 0x80),
    (Model::Crystal, 0xc0),
    (Model::Crystal, 0xe0),
];
const OPCODES: [u8; 8] = [0xe4, 0xe5, 0xe6, 0xe7, 0xec, 0xed, 0xee, 0xef];

fn bus(model: Model, mode: u8) -> Bus {
    let mut b = Bus::new(model);
    b.insert_cartridge(
        crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0)).unwrap(),
    );
    if model != Model::Mono {
        b.output(0x60, mode).unwrap();
    }
    b
}

#[test]
fn ordinary_io_literal_all_encodings_state_lengths_masks_and_fetch_waits() {
    // Literal RAM totals: immediate byte/even/split = 8/8/9; DX = 7/7/8.
    // port, seeded low/high, input AX for byte/word, output lanes for byte/word.
    for (model, mode) in MODES {
        for (port, seed, input_byte, input_word, out_byte, out_word) in [
            (
                0x20,
                [0x12, 0x34],
                0xab12,
                0x3412,
                [0x77, 0x34],
                [0x77, 0x23],
            ),
            (
                0x21,
                [0x12, 0x34],
                0xab12,
                0x3412,
                [0x77, 0x34],
                [0x77, 0x23],
            ),
            (
                0x28,
                [0x10, 0x34],
                0xab10,
                0x3410,
                [0x70, 0x34],
                [0x70, 0x23],
            ),
            (
                0x27,
                [0x12, 0x30],
                0xab12,
                0x3012,
                [0x77, 0x30],
                [0x77, 0x20],
            ),
            (
                0x80,
                [0x12, 0x03],
                0xab12,
                0x0312,
                [0xff, 0x03],
                [0xff, 0x03],
            ),
            (
                0xa4,
                [0x12, 0x34],
                0xab12,
                0x3412,
                [0xff, 0x34],
                [0xff, 0xab],
            ),
            (
                0xa5,
                [0x12, 0x34],
                0xab12,
                0x3412,
                [0xff, 0x34],
                [0xff, 0xab],
            ),
        ] {
            for opcode in OPCODES {
                for control in [0, 4, 8, 12] {
                    for rom in [false, true] {
                        let mut b = bus(model, mode);
                        b.output(0xa0, control).unwrap();
                        b.output(port, seed[0]).unwrap();
                        b.output(port + 1, seed[1]).unwrap();
                        let immediate = opcode & 8 == 0;
                        let wide = opcode & 1 != 0;
                        let output = opcode & 2 != 0;
                        let mut code = vec![opcode];
                        if immediate {
                            code.push(port as u8);
                        }
                        code.extend([0xb9, 0xef, 0xbe]); // MOV CX,BEEF continuation
                        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
                        b.cartridge.as_mut().unwrap().bytes[65536..65536 + code.len()]
                            .copy_from_slice(&code);
                        let mut c = Cpu {
                            registers: [0xabff, 0x1234, port, 0x5678, 0x1800, 0x820, 0x840, 0x860],
                            segments: [0x80, if rom { 0xf000 } else { 0 }, 0x20, 0xc0],
                            ip: if rom { 0 } else { 0x1000 },
                            flags: 0xfed7, // status, IF and DF; TF clear
                            ..Cpu::default()
                        };
                        let mut registers = c.registers;
                        let segments = c.segments;
                        let ram = b.ram.clone();
                        let save = b.cartridge.as_ref().unwrap().save.clone();
                        let mut ports = b.ports;
                        let mut lanes = seed;
                        if output {
                            lanes = if wide { out_word } else { out_byte };
                            ports[port as usize..port as usize + 2].copy_from_slice(&lanes);
                        } else {
                            registers[0] = if wide { input_word } else { input_byte };
                        }
                        let split = wide && port & 1 != 0;
                        let total = match (immediate, split) {
                            (false, false) => 7,
                            (false, true) => 8,
                            (true, false) => 8,
                            (true, true) => 9,
                        };
                        let fetches = if immediate { 2 } else { 1 };
                        let wait = if rom && control & 8 != 0 { fetches } else { 0 };
                        let start = c.ip;
                        assert_eq!(
                            c.step(&mut b).unwrap(),
                            total + wait,
                            "{model:?}/{mode:x} {opcode:x}/{port:x} ROM={rom} A0={control:x}"
                        );
                        assert_eq!(b.accesses.get(), fetches + wait + if split { 2 } else { 1 });
                        assert_eq!(c.registers, registers);
                        assert_eq!((c.ip, c.flags), (start + fetches as u16, 0xfed7));
                        assert_eq!(c.segments, segments);
                        assert!(!c.halted);
                        assert_eq!(b.cycles, 0);
                        assert_eq!(b.ram, ram);
                        assert_eq!(b.ports, ports);
                        assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                        assert_eq!(b.input(port).unwrap(), lanes[0]);
                        assert_eq!(b.input(port + 1).unwrap(), lanes[1]);
                        if port == 0xa4 {
                            assert_eq!(b.input_word(0xa8).unwrap(), u16::from_le_bytes(lanes));
                        }
                        c.step(&mut b).unwrap();
                        registers[1] = 0xbeef;
                        assert_eq!(c.registers, registers);
                        assert_eq!((c.ip, c.flags), (start + fetches as u16 + 3, 0xfed7));
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_io_live_runtime_round_trips_cartridge_width_and_each_wait_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut b = bus(model, 0);
        let phases: &[u8] = if model == Model::Mono {
            &[0]
        } else {
            &[0, 0x80, 0, 0xc0, 0, 0xe0, 0]
        };
        for &mode in phases {
            for wait in [0, 8] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                    assert_eq!(b.input(0x60).unwrap(), mode | wait);
                } else {
                    // Existing Mono bus wait policy is internal, with no port 60 access.
                    b.ports[0x60] = wait;
                }
                for control in [0, 4, 8, 12] {
                    b.output(0xa0, control).unwrap();
                    for opcode in OPCODES {
                        for port in [0xc0u16, 0xc1, 0xbf] {
                            let wide = opcode & 1 != 0;
                            let immediate = opcode & 8 == 0;
                            let output = opcode & 2 != 0;
                            for rom in [false, true] {
                                let code = [opcode, port as u8];
                                b.ram[0x1000..0x1002].copy_from_slice(&code);
                                b.cartridge.as_mut().unwrap().bytes[65536..65538]
                                    .copy_from_slice(&code);
                                b.output(0xc0, 3).unwrap();
                                b.output(0xc1, 0x12).unwrap();
                                b.output(0xc2, 0x34).unwrap();
                                let mut c = Cpu {
                                    segments: [0, if rom { 0xf000 } else { 0 }, 0, 0],
                                    ip: if rom { 0 } else { 0x1000 },
                                    ..Cpu::default()
                                };
                                c.registers[0] = 0x0201;
                                c.registers[2] = port;
                                // Literal DX/immediate no-wait totals; one wait per external lane.
                                let base = match (immediate, wide) {
                                    (false, false) => 7,
                                    (false, true) => 8,
                                    (true, false) => 8,
                                    (true, true) => 9,
                                };
                                let lanes = if port == 0xbf {
                                    u32::from(wide)
                                } else if wide {
                                    2
                                } else {
                                    1
                                };
                                let fetch_wait = if rom && control & 8 != 0 {
                                    if immediate {
                                        2
                                    } else {
                                        1
                                    }
                                } else {
                                    0
                                };
                                assert_eq!(
                                    c.step(&mut b).unwrap(),
                                    base + if wait != 0 { lanes } else { 0 } + fetch_wait
                                );
                                assert_eq!(c.flags, 0xf002);
                                if output {
                                    if port == 0xbf && wide {
                                        assert_eq!(b.ports[0xc0], 2);
                                    } else if port != 0xbf {
                                        assert_eq!(b.ports[port as usize], 1);
                                    }
                                } else if port != 0xbf {
                                    assert_eq!(
                                        c.registers[0],
                                        match (port, wide) {
                                            (0xc0, false) => 0x0203,
                                            (0xc0, true) => 0x1203,
                                            (0xc1, false) => 0x0212,
                                            _ => 0x3412,
                                        }
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_io_prefixes_rep_rejection_errors_and_interrupt_halt_continuation() {
    for (model, mode) in MODES {
        for opcode in OPCODES {
            let mut b = bus(model, mode);
            let mut code = vec![0x26, 0xf0, opcode];
            if opcode & 8 == 0 {
                code.push(0x20);
            }
            code.extend([0xcc, 0xf4, 0xb9, 0xef, 0xbe]);
            b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
            b.ram[12..16].copy_from_slice(&[0, 3, 0, 0]);
            b.ram[0x300] = 0xcf;
            let mut c = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                flags: 0xf603,
                ..Cpu::default()
            };
            c.registers[0] = 0x1234;
            c.registers[2] = 0x20;
            c.registers[4] = 0x1800;
            assert_eq!(
                c.step(&mut b).unwrap(),
                if opcode & 8 == 0 { 10 } else { 9 }
            );
            let ax = c.registers[0];
            let next = c.ip;
            c.step(&mut b).unwrap(); // INT3 / IRET stacks and restores full FLAGS.
            assert_eq!(
                &b.ram[0x17fa..0x1800],
                &[(next + 1) as u8, 0x10, 0, 0, 3, 0xf6]
            );
            c.step(&mut b).unwrap();
            assert_eq!((c.ip, c.flags), (next + 1, 0xf603));
            c.step(&mut b).unwrap();
            assert!(c.halted);
            // Pending masked IRQ releases HALT; ordinary continuation still runs.
            c.flags &= !IF;
            b.ports[0xb2] = 0x80;
            b.ports[0xb4] = 0x80;
            c.step(&mut b).unwrap();
            assert!(!c.halted);
            assert_eq!((c.registers[0], c.registers[1]), (ax, 0xbeef));
            for prefix in [0xf2, 0xf3] {
                b.ports[0xb4] = 0;
                c.ip = 0x1000;
                b.ram[0x1000..0x1003].copy_from_slice(&[prefix, opcode, 0x20]);
                let ports = b.ports;
                let registers = c.registers;
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.registers, registers);
                assert_eq!(b.ports, ports);
                assert_eq!(b.accesses.get(), 2);
            }
        }
        for opcode in [0xed, 0xef] {
            let mut b = bus(model, mode);
            b.ram[0x1000] = opcode;
            let mut c = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                ..Cpu::default()
            };
            c.registers[0] = 0x1234;
            c.registers[2] = 0xffff;
            let ports = b.ports;
            assert!(c.step(&mut b).is_err());
            assert_eq!((c.ip, c.registers[0]), (0x1001, 0x1234));
            assert_eq!(b.accesses.get(), 2); // opcode + failed low lane; no port zero
            assert_eq!(b.ports, ports);
        }
        if model != Model::Mono {
            let mut b = bus(model, mode);
            b.ram[0x1000] = 0xef;
            let mut c = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                ..Cpu::default()
            };
            c.registers[0] = 0x0134;
            c.registers[2] = 0x6a;
            assert!(c.step(&mut b).is_err());
            assert_eq!((b.ports[0x6a], b.ports[0x6b]), (0x34, 0));
            assert_eq!(b.accesses.get(), 3);
        }
    }
}

#[test]
fn ordinary_io_normal_scheduler_burst_crosses_line_frame_timer_irq_and_pcm_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = crate::Machine::new(model);
        m.bus = bus(model, 0);
        let phases: &[u8] = if model == Model::Mono {
            &[0]
        } else {
            &[0, 0x80, 0, 0xc0, 0, 0xe0, 0]
        };
        for &mode in phases {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            // Forty consecutive DX byte INs = 280 clocks: cross one line and
            // the frame edge, with a literal three-clock overshoot.
            m.cpu = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                flags: 0xf002,
                ..Cpu::default()
            };
            m.cpu.registers[2] = 0xbc;
            m.bus.ram[0x1000..0x1028].fill(0xec);
            m.bus.ram[0x1028..0x102a].copy_from_slice(&[0xf4, 0x43]);
            let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
            m.advance((remaining - 277) as u32);
            m.bus.output_word(0xa4, 2).unwrap();
            m.bus.output(0xa2, 1).unwrap();
            m.bus.output(0xb2, 0x80).unwrap();
            m.bus.output(0xb6, 0xff).unwrap();
            let before = m.bus.cycles;
            let frame = m.frame_number();
            m.audio_samples();
            m.run_frame().unwrap();
            assert_eq!(m.bus.cycles, before + 280);
            assert_eq!(m.cpu.ip, 0x1028);
            assert_eq!(m.bus.accesses.get(), 2); // fetch + one raw I/O charge
            assert_eq!((m.frame_number(), m.bus.raster_line), (frame + 1, 0));
            assert_eq!(m.bus.input_word(0xa8).unwrap(), 0);
            assert_ne!(m.bus.ports[0xb4] & 0x80, 0);
            let pcm = 2 * (m.bus.cycles / 128 - before / 128) as usize;
            assert_eq!(pcm, 6); // boundaries at +21, +149, +277; stereo
            assert_eq!(m.audio_samples().len(), pcm);
            assert!(m.audio_samples().is_empty());
            // Ack the timer, then HALT. Deliver a real enabled interrupt and
            // return to the INC BX following HALT, without redoing the burst.
            m.bus.output(0xb6, 0xff).unwrap();
            assert_eq!(m.cpu.step(&mut m.bus).unwrap(), 2);
            assert!(m.cpu.halted);
            m.cpu.registers[4] = 0x1800;
            m.cpu.flags |= IF;
            m.bus.ports[0xb0] = 0;
            m.bus.ports[0xb4] = 0x80;
            m.bus.ram[28..32].copy_from_slice(&[0, 3, 0, 0]);
            m.bus.ram[0x300] = 0xcf;
            assert_eq!(m.cpu.step(&mut m.bus).unwrap(), 17);
            assert_eq!((m.cpu.ip, m.cpu.halted), (0x300, false));
            m.bus.output(0xb6, 0xff).unwrap();
            m.cpu.step(&mut m.bus).unwrap();
            assert_eq!(m.cpu.ip, 0x1029);
            m.cpu.step(&mut m.bus).unwrap();
            assert_eq!((m.cpu.ip, m.cpu.registers[3]), (0x102a, 1));
        }
    }
}

#[test]
fn ordinary_io_normal_scheduler_separates_gdma_sdma_and_cpu_clocks() {
    for (model, mode) in MODES {
        if model == Model::Mono {
            continue;
        }
        for sound in [false, true] {
            let mut m = crate::Machine::new(model);
            m.bus = bus(model, mode);
            m.cpu = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                ..Cpu::default()
            };
            m.cpu.registers[2] = 0xbc;
            m.bus.ram[0x1000..0x1002].copy_from_slice(&[0xec, 0xec]);
            m.bus.ram[0x800..0x802].copy_from_slice(&[0x12, 0x34]);
            if sound {
                // Enabled SDMA event at 117 mod 128, then a separately
                // scheduled seven-clock RAM-source stall. Fetch is untouched.
                m.advance((crate::CYCLES_PER_FRAME - 139) as u32);
                for (port, value) in [
                    (0x4a, 0),
                    (0x4b, 8),
                    (0x4c, 0),
                    (0x4e, 1),
                    (0x4f, 0),
                    (0x50, 0),
                    (0x52, 0x83),
                ] {
                    m.bus.output(port, value).unwrap();
                }
                // Start at frame end -15 (113 mod 128): the first IN crosses
                // the enabled event four clocks later at 117 mod 128.
                m.advance(124);
            } else {
                // One RAM GDMA word =7 separately, gated off in mode 00.
                let remaining = if mode == 0 { 7 } else { 14 };
                m.advance((crate::CYCLES_PER_FRAME - remaining) as u32);
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
            }
            let before = m.bus.cycles;
            m.audio_samples();
            m.run_frame().unwrap();
            if sound {
                // IN 7 + SDMA stall 7 + IN 7 crosses the frame from -15.
                assert_eq!(m.bus.cycles, before + 21);
                assert_eq!(m.cpu.ip, 0x1002);
                assert_eq!(m.bus.ports[0x89], 0x12);
                assert_eq!(m.bus.ports[0x52] & 0x80, 0);
            } else {
                assert_eq!(m.bus.cycles, before + if mode == 0 { 7 } else { 14 });
                assert_eq!(m.cpu.ip, 0x1001);
                assert_eq!(
                    &m.bus.ram[0x900..0x902],
                    if mode == 0 { &[0, 0] } else { &[0x12, 0x34] }
                );
            }
            assert_eq!(m.frame_number(), 1);
            assert_eq!(m.bus.raster_line, 0);
            assert_eq!(m.bus.accesses.get(), 2);
            assert_eq!(m.audio_samples().len(), 2);
            assert!(m.audio_samples().is_empty());
        }
    }
}
