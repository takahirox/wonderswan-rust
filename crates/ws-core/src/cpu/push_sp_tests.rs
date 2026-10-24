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

fn setup(model: Model, mode: u8, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    if model != Model::Mono {
        b.output(0x60, mode).unwrap();
    }
    b.ram.fill(0xa5);
    b.ram[0x3000..0x3000 + code.len()].copy_from_slice(code);
    let c = Cpu {
        registers: [0x1234, 0x5678, 0xabcd, 0x9876, 0x2000, 0x820, 0x840, 0x860],
        segments: [0x80, 0x100, 0x20, 0xc0],
        ip: 0x2000,
        ..Cpu::default()
    };
    (c, b)
}

fn vectors(encoding: &[u8]) {
    for (model, mode) in modes() {
        // Literal offsets, physical addresses and bytes: no CPU address/push helpers.
        for (sp, ss, result, address, bytes, word_cost) in [
            (0x2000, 0x20, 0x1ffe, 0x21fe, [0xfe, 0x1f], 1),
            (0x2001, 0x20, 0x1fff, 0x21ff, [0xff, 0x1f], 2),
            (0x0002, 0x20, 0x0000, 0x0200, [0x00, 0x00], 1),
            (0x0000, 0xf100, 0xfffe, 0x0ffe, [0xfe, 0xff], 1),
        ] {
            for flags in [0xf002, 0xffd7] {
                for prefix in [false, true] {
                    let mut code = Vec::new();
                    if prefix {
                        code.push(0x26); // ES differs from SS, DS and CS.
                    }
                    code.extend(encoding);
                    let length = code.len() as u16;
                    code.extend([0x58, 0xb9, 0xef, 0xbe]); // POP AX; MOV CX,BEEF
                    let (mut c, mut b) = setup(model, mode, &code);
                    c.registers[4] = sp;
                    c.segments[2] = ss;
                    c.flags = flags;
                    let mut registers = c.registers;
                    let segments = c.segments;
                    assert_eq!(
                        c.instruction_step(&mut b).unwrap(),
                        u32::from(length) + word_cost + 1
                    );
                    registers[4] = result;
                    assert_eq!(c.registers, registers, "{model:?}/{mode:x}/{encoding:x?}");
                    assert_eq!(c.segments, segments);
                    assert_eq!((c.flags, c.ip), (flags, 0x2000 + length));
                    assert_eq!(&b.ram[address..address + 2], &bytes);
                    assert_eq!(b.ram[address - 1], 0xa5);
                    assert_eq!(b.ram[address + 2], 0xa5);
                    // Every other RAM byte is preserved, including override destinations.
                    let mut expected = vec![0xa5; b.ram.len()];
                    expected[0x3000..0x3000 + code.len()].copy_from_slice(&code);
                    expected[address..address + 2].copy_from_slice(&bytes);
                    assert_eq!(&b.ram[..], &expected);
                    assert_eq!(c.instruction_step(&mut b).unwrap(), 2 + word_cost);
                    registers[0] = result;
                    registers[4] = sp;
                    assert_eq!(c.registers, registers);
                    assert_eq!((c.ip, c.flags), (0x2001 + length, flags));
                    c.instruction_step(&mut b).unwrap();
                    registers[1] = 0xbeef;
                    assert_eq!(c.registers, registers);
                    assert_eq!(c.segments, segments);
                    assert_eq!((c.ip, c.flags), (0x2004 + length, flags));
                    assert_eq!(&b.ram[..], &expected);
                }
            }
        }
    }
}

#[test]
fn short_push_sp_literal_vectors() {
    vectors(&[0x54]);
}

#[test]
fn modrm_push_sp_literal_vectors() {
    vectors(&[0xff, 0xf4]);
}

#[test]
fn other_push_sources_and_pusha_keep_entry_values() {
    for (model, mode) in modes() {
        let mut encodings = vec![
            vec![0x68, 0, 0x20], // immediate equal to incoming SP
            vec![0x6a, 0x20],
            vec![0x06],
            vec![0x0e],
            vec![0x16],
            vec![0x1e],
            vec![0xff, 0x36, 0, 8],       // DS:0800
            vec![0x26, 0xff, 0x36, 0, 8], // ES:0800
            vec![0xff, 0x76, 0],          // SS:[BP], aliases destination
        ];
        for r in [0u8, 1, 2, 3, 5, 6, 7] {
            encodings.extend([vec![0x50 + r], vec![0xff, 0xf0 + r]]);
        }
        for code in encodings {
            let (mut c, mut b) = setup(model, mode, &code);
            c.registers.fill(0x2000);
            c.registers[5] = 0x1ffe;
            b.ram[0x1000..0x1002].copy_from_slice(&[0, 0x20]);
            b.ram[0x1400..0x1402].copy_from_slice(&[0, 0x20]);
            b.ram[0x21fe..0x2200].copy_from_slice(&[0, 0x20]);
            let value: u16 = match code.as_slice() {
                [0x6a, _] => 0x20,
                [0x06] => 0x80,
                [0x0e] => 0x100,
                [0x16] => 0x20,
                [0x1e] => 0xc0,
                [0x55] | [0xff, 0xf5] => 0x1ffe,
                _ => 0x2000,
            };
            c.flags = 0xffd7;
            let mut registers = c.registers;
            c.instruction_step(&mut b).unwrap();
            registers[4] = 0x1ffe;
            assert_eq!(c.registers, registers);
            assert_eq!(&b.ram[0x21fe..0x2200], &value.to_le_bytes(), "{code:x?}");
            assert_eq!((c.flags, c.ip), (0xffd7, 0x2000 + code.len() as u16));
        }
        let (mut c, mut b) = setup(model, mode, &[0x60, 0x61]);
        let registers = c.registers;
        c.instruction_step(&mut b).unwrap();
        assert_eq!(c.registers[4], 0x1ff0);
        assert_eq!(&b.ram[0x21f6..0x21f8], &[0, 0x20]);
        // POPA must skip the saved SP slot.
        b.ram[0x21f6..0x21f8].copy_from_slice(&[0x34, 0x12]);
        c.instruction_step(&mut b).unwrap();
        assert_eq!(c.registers, registers);
        assert_eq!(c.ip, 0x2002);
    }
}

#[test]
fn push_sp_runtime_modes_and_normal_machine_scheduler_advance_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for encoding in [&[0x54][..], &[0xff, 0xf4][..]] {
            for (sp, result, word_cost) in [(0x2000, 0x1ffe, 1), (0x2001, 0x1fff, 2)] {
                let (c, mut b) = setup(model, 0, encoding);
                b.insert_cartridge(
                    crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                        .unwrap(),
                );
                let mut m = crate::Machine::new(model);
                m.cpu = c;
                m.bus = b;
                super::trap_tests::install_handler(&mut m.bus);
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
                    m.cpu.ip = 0x2000;
                    m.cpu.registers[4] = sp;
                    m.cpu.flags = 0xffd7;
                    let mut cpu = m.cpu.clone();
                    let returned = cpu.step(&mut m.bus).unwrap();
                    assert_eq!(
                        returned,
                        encoding.len() as u32 + word_cost + 1 + 22 + 3 * word_cost
                    );
                    let remaining =
                        crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                    m.advance((remaining - u64::from(returned)) as u32);
                    let before = m.bus.cycles;
                    let frame = m.frame_number();
                    m.audio_samples();
                    m.run_frame().unwrap();
                    assert_eq!(m.bus.cycles, before + u64::from(returned));
                    assert_eq!(m.frame_number(), frame + 1);
                    assert_eq!(m.bus.raster_line, 0);
                    assert_eq!(m.audio_samples().len(), 2);
                    assert_eq!(m.bus.accesses.get(), returned - 21);
                    super::trap_tests::finish_trap_at(
                        &mut cpu,
                        &mut m.bus,
                        0x2000 + encoding.len() as u16,
                        0x100,
                        0xffd7,
                    );
                    let handler_cost = super::trap_tests::finish_trap_at(
                        &mut m.cpu,
                        &mut m.bus,
                        0x2000 + encoding.len() as u16,
                        0x100,
                        0xffd7,
                    );
                    m.advance(handler_cost);
                    assert_eq!(m.cpu.ip, 0x2000 + encoding.len() as u16);
                    assert_eq!((m.cpu.registers[4], m.cpu.flags), (result, 0xffd7));
                    assert_eq!(m.cpu.registers, cpu.registers);
                    let address = if sp == 0x2000 { 0x21fe } else { 0x21ff };
                    let bytes = if sp == 0x2000 {
                        [0xfe, 0x1f]
                    } else {
                        [0xff, 0x1f]
                    };
                    assert_eq!(&m.bus.ram[address..address + 2], &bytes);
                    assert_eq!(m.cpu.segments, cpu.segments);
                }
            }
        }
    }
}
