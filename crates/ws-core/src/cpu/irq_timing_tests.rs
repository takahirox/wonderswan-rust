// SPDX-License-Identifier: CC0-1.0
use super::*;

fn setup(model: Model) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    b.insert_cartridge(
        crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0)).unwrap(),
    );
    b.ram[0x1100..0x1103].copy_from_slice(&[0x90, 0x43, 0x90]); // NOP; INC BX; NOP
    b.ram[0x9c..0xa0].copy_from_slice(&[0, 4, 0, 0]);
    b.ram[0x400..0x405].copy_from_slice(&[0xb0, 0x80, 0xe6, 0xb6, 0xcf]);
    b.output(0xb0, 0x20).unwrap();
    b.output(0xb2, 0x90).unwrap();
    (
        Cpu {
            segments: [0, 0x10, 0, 0],
            ip: 0x1000,
            flags: 0xffff, // Reserved bits must normalize in the saved frame.
            ..Cpu::default()
        },
        b,
    )
}

#[test]
fn literal_external_irq_totals_frames_priority_and_continuation_across_live_modes() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for wait in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                }
                let waiting = model != Model::Mono && wait != 0;
                for (ss, sp, entry, boundary_total) in [
                    (0, 0x2000, 32, 34),
                    (0, 0x2001, 35, 37),
                    (
                        0x1000,
                        0x2000,
                        if waiting { 41 } else { 35 },
                        if waiting { 43 } else { 37 },
                    ),
                    (
                        0x1000,
                        0x2001,
                        if waiting { 41 } else { 35 },
                        if waiting { 43 } else { 37 },
                    ),
                ] {
                    for boundary in [false, true] {
                        c.segments[1] = 0x10;
                        c.segments[2] = ss;
                        c.ip = 0x1000;
                        c.registers[3] = 0;
                        c.registers[4] = sp;
                        c.flags = 0xffff;
                        c.interrupt_shadow = u8::from(boundary);
                        b.request(4);
                        b.request(7);
                        assert_eq!(
                            c.step(&mut b).unwrap(),
                            if boundary { boundary_total } else { entry },
                            "{model:?}/{mode:x}/{wait}/{ss:x}/{sp:x}/{boundary}"
                        );
                        assert_eq!((c.segments[1], c.ip, c.flags), (0, 0x400, 0xfcd7));
                        assert_eq!((c.registers[3], c.registers[4]), (0, sp - 6));
                        assert_eq!(c.interrupt_shadow, 0);
                        assert_eq!(b.interrupt(), Some(0x27), "entry does not acknowledge");
                        let ip = if boundary { 0x1001u16 } else { 0x1000 };
                        // Independently addressed frame bytes, low IP first, then CS, FLAGS.
                        let base = u32::from(ss) * 16 + u32::from(sp - 6);
                        for (i, value) in [ip as u8, 0x10, 0x10, 0, 0xd7, 0xff]
                            .into_iter()
                            .enumerate()
                        {
                            assert_eq!(b.read(base + i as u32), value);
                        }
                        for _ in 0..3 {
                            c.step(&mut b).unwrap();
                        } // MOV AL; ACK; IRET
                        assert_eq!(
                            (c.segments[1], c.ip, c.flags, c.registers[4]),
                            (0x10, ip, 0xffd7, sp)
                        );
                        assert_eq!(b.interrupt(), Some(0x24), "lower priority stays latched");
                        b.output(0xb6, 0x10).unwrap();
                        c.flags = 0xf002; // Continue without creating a new TF frame.
                        c.step(&mut b).unwrap();
                        assert_eq!(c.ip, ip + 1);
                        assert_eq!(c.registers[3], u16::from(boundary));
                    }
                }
            }
        }
    }
}

#[test]
fn literal_irq_segment_offset_and_twenty_bit_wrap_frames() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            // SS=FFFF wraps into low internal RAM. SS=0100 wraps the IP high
            // byte at offset FFFF into RAM 1000; its low byte is in SRAM 10FFF.
            for (ss, sp, addresses) in [
                (0xffff, 0x17, [1, 2, 3, 4, 5, 6]),
                (0x0100, 5, [0x10fff, 0x1000, 0x1001, 0x1002, 0x1003, 0x1004]),
            ] {
                let (mut c, mut b) = setup(model);
                if model != Model::Mono {
                    b.output(0x60, mode).unwrap();
                }
                c.segments[2] = ss;
                c.registers[4] = sp;
                b.request(7);
                assert_eq!(c.step(&mut b).unwrap(), 35);
                assert_eq!(c.registers[4], sp.wrapping_sub(6));
                for (address, value) in addresses.into_iter().zip([0, 0x10, 0x10, 0, 0xd7, 0xff]) {
                    assert_eq!(
                        b.read(address),
                        value,
                        "{model:?}/{mode:x}/{ss:x}/{address:x}"
                    );
                }
                b.output(0xb6, 0x80).unwrap();
                b.ram[0x400] = 0xcf;
                c.step(&mut b).unwrap();
                assert_eq!(
                    (c.segments[1], c.ip, c.flags, c.registers[4]),
                    (0x10, 0x1000, 0xffd7, sp)
                );
            }
        }
    }
}

#[test]
fn normal_run_frame_charges_irq_once_for_raster_timer_and_audio() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, b) = setup(model);
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
            for (boundary, sp, cost) in [
                (false, 0x2000, 32),
                (false, 0x2001, 35),
                (true, 0x2000, 34),
                (true, 0x2001, 37),
            ] {
                m.cpu.segments[1] = 0x10;
                m.cpu.ip = 0x1000;
                m.cpu.flags = 0xf302;
                m.cpu.registers[3] = 0;
                m.cpu.registers[4] = sp;
                m.cpu.interrupt_shadow = u8::from(boundary);
                m.bus.request(7);
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
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
                    (m.cpu.ip, m.cpu.registers[3], m.cpu.registers[4]),
                    (0x400, 0, sp - 6)
                );
                m.bus.output(0xb6, 0x80).unwrap();
            }
        }
    }
}

#[test]
fn dma_before_pending_irq_keeps_each_charge_and_model_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for sound in [false, true] {
                let (mut c, mut b) = setup(model);
                c.registers[4] = 0x2000;
                if model == Model::Mono {
                    assert!(b.output(if sound { 0x52 } else { 0x48 }, 0x80).is_err());
                    continue;
                }
                b.output(0x60, mode).unwrap();
                b.ram[0x800..0x802].copy_from_slice(&[0x51, 0xa2]);
                for (p, v) in if sound {
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
                    b.output(p, v).unwrap();
                }
                if sound {
                    b.tick(117);
                }
                b.request(7);
                if !sound && mode == 0 {
                    assert_eq!(
                        c.step(&mut b).unwrap(),
                        32,
                        "GDMA engine needs color enable"
                    );
                    assert_eq!(&b.ram[0x900..0x902], &[0, 0]);
                    continue;
                }
                let before = (c.ip, c.registers, c.flags);
                assert_eq!(c.step(&mut b).unwrap(), 7);
                assert_eq!((c.ip, c.registers, c.flags), before);
                assert_eq!(
                    b.accesses.get(),
                    0,
                    "DMA accesses are not CPU entry transfers"
                );
                assert_eq!(b.interrupt(), Some(0x27));
                if !sound {
                    assert_eq!(&b.ram[0x900..0x902], &[0x51, 0xa2]);
                }
                assert_eq!(c.step(&mut b).unwrap(), 32);
                assert_eq!((c.ip, c.registers[4]), (0x400, 0x1ffa));
            }
        }
    }
}
