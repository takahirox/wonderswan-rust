// SPDX-License-Identifier: CC0-1.0
// Literal cartridge-entry vectors for Issues #33 and #140.
use super::*;

const MODELS: [(Model, u16, u8); 3] = [
    (Model::Mono, 0xff00, 0x81),
    (Model::Color, 0xfe00, 0x83),
    (Model::Crystal, 0xfe00, 0x83),
];

impl Machine {
    /// Run the real IPL and stop immediately before the first cartridge fetch.
    pub(crate) fn boot_to_cartridge(&mut self) {
        assert_eq!(self.bus.ports[0xa0] & 1, 0);
        while self.bus.cycles < 256 {
            let clocks = self.cpu.step(&mut self.bus).unwrap();
            self.advance(clocks);
            if self.bus.ports[0xa0] & 1 != 0 && self.cpu.segments[1] == 0xffff && self.cpu.ip == 0 {
                assert!(
                    self.bus.cycles < 256,
                    "IPL must finish within scanline zero"
                );
                return;
            }
        }
        panic!("IPL did not reach the cartridge footer in scanline zero");
    }
}

fn dirty(m: &mut Machine) {
    m.cpu.registers[4] = 0x3456;
    m.cpu.segments[3] = 0x1234;
    m.cpu.halted = true;
    m.bus.output(0xa0, 0).unwrap();
}

fn entry(m: &mut Machine, ds: u16, a0: u8, mapper: u8, protected: bool) {
    assert_eq!(m.bus.cycles, 0);
    assert_eq!(m.cpu.registers, [0; 8]);
    assert_eq!(m.cpu.segments, [0, 0xffff, 0, 0]);
    m.boot_to_cartridge();
    let entry_dot = match m.model() {
        Model::Mono => 219,
        Model::Color => 226,
        Model::Crystal => 218,
    } + if protected { 3 } else { 0 };
    assert_eq!(m.bus.cycles, entry_dot);
    assert_eq!(m.cpu.registers, [0, 0, 0, 0, 0x2000, 0, 0, 0]);
    assert_eq!(m.cpu.segments, [0, 0xffff, 0, ds]);
    assert_eq!(m.cpu.ip, 0);
    assert_eq!(m.cpu.flags, 0xf002);
    assert!(!m.cpu.halted);
    assert_eq!(m.bus.raster_line, 0);
    assert_eq!(m.bus.completed_frames, 0);
    assert_eq!(m.bus.input(0xa0).unwrap(), a0);
    assert_eq!(m.bus.input(0xbe).unwrap(), if protected { 0x82 } else { 2 });
    assert_eq!(m.bus.input_word(0xbc).unwrap(), 0);
    assert_eq!(m.bus.ports[0x60], 0);
    assert_eq!(m.bus.ports[0x14], 0);
    assert_eq!(m.bus.ports[0x16], 158);
    assert_eq!(
        m.bus.ports[0x17],
        if m.model() == Model::Crystal { 0 } else { 155 }
    );
    assert_eq!(m.bus.ports[0x90], 0);
    assert_eq!(m.bus.ports[0x9e], 3);
    assert_eq!(m.bus.ports[0xc0], if mapper == 1 { 63 } else { 15 });
    assert_eq!(m.bus.ports[0xcf], if mapper == 1 { 63 } else { 15 });
    assert_eq!(m.bus.ports[0xc2], 255);
    assert_eq!(m.bus.ports[0xc3], 255);
    assert_eq!(m.bus.ports[0xd3], if mapper == 1 { 3 } else { 0 });
    assert_eq!(m.bus.ports[0xd5], if mapper == 1 { 3 } else { 0 });
}

#[test]
fn unloaded_new_reset_restart_and_model_change_keep_zero_sp_ds() {
    for (model, _, _) in MODELS {
        let mut m = Machine::new(model);
        for next in [model, Model::Mono, Model::Color, Model::Crystal] {
            assert_eq!(m.cpu.registers[4], 0);
            assert_eq!(m.cpu.segments[3], 0);
            assert_eq!(m.read_memory(0xffff0), 0xea);
            let a0 = if m.model() == Model::Mono { 4 } else { 6 };
            assert_eq!(m.bus.input(0xa0).unwrap(), a0);
            assert!(m.run_frame().is_err());
            assert_eq!(m.bus.cycles, 0);
            dirty(&mut m);
            m.restart();
            assert_eq!(m.cpu.registers[4], 0);
            assert_eq!(m.cpu.segments[3], 0);
            dirty(&mut m);
            m.change_model(next).unwrap();
        }
        m.load_rom(include_bytes!(
            "../../../fixtures/generated/startup-mono-common.ws"
        ))
        .unwrap();
        dirty(&mut m);
        m.reset(model);
        assert_eq!(m.rom_len(), 0);
        assert_eq!(m.cpu.registers[4], 0);
        assert_eq!(m.cpu.segments[3], 0);
        assert_eq!(m.bus.input(0xa0).unwrap() & 1, 0);
    }
}

#[test]
fn overlay_reset_vector_word_access_and_one_way_lockout() {
    let mut rom = cartridge::tests::rom(131072, 0, 0);
    for address in [0xfe000, 0xff000] {
        let offset = address - 0xe0000;
        rom[offset - 1..offset + 2].copy_from_slice(&[0x5a, 0x62, 0x63]);
    }
    cartridge::tests::checksum(&mut rom);

    for (model, segment, fixed) in MODELS {
        let start = u32::from(segment) << 4;
        let mut m = Machine::new(model);
        m.load_rom(&rom).unwrap();
        let b = &mut m.bus;
        // Slow byte cartridge controls must not affect IPL reads.
        b.output(0xa0, 0x78).unwrap();
        assert_eq!(b.input(0xa0).unwrap(), (fixed & !1) | 8);
        assert_eq!(b.read(start - 1), 0x5a);
        assert_eq!(b.read(start - 0xd0000), 0x62); // ROM0 sees the covered bank.
        b.accesses.set(0);
        assert_eq!(b.read_word(start, start + 1), 0xfcfa); // CLI; CLD
        assert_eq!(b.accesses.get(), 1);
        b.accesses.set(0);
        assert_eq!(b.read_word(start - 1, start), 0xfa5a);
        assert_eq!(b.accesses.get(), 3);
        b.accesses.set(0);
        assert_eq!(b.read_word(start + 1, start + 2), 0x31fc);
        assert_eq!(b.accesses.get(), 2);
        b.accesses.set(0);
        assert_eq!(b.read_word(0xfffff, 0), 0x00ff);
        assert_eq!(b.accesses.get(), 2);
        b.write(start, 0x90);
        assert_eq!(b.read(start), 0xfa);

        // An aligned IPL fetch leaves both a high lane and prefetch credits.
        b.begin_instruction(start);
        assert_eq!(b.fetch_instruction_byte(start), 0xfa);
        b.finish_instruction(start + 1, 4);
        b.output(0xa0, 5).unwrap();
        b.begin_instruction(start + 1);
        b.accesses.set(0);
        assert_eq!(b.fetch_instruction_byte(start + 1), 0x63);
        assert_eq!(
            b.accesses.get(),
            1,
            "lockout must discard both prefetch paths"
        );
        assert_eq!(b.read_word(start, start + 1), 0x6362);

        b.output(0xa0, 0).unwrap();
        assert_eq!(b.input(0xa0).unwrap(), fixed);
        assert_eq!(b.read(start), 0x62);
        m.restart();
        assert_eq!(m.read_memory(start), 0xfa);
        assert_eq!(m.read_memory(0xffff1), 0);
        assert_eq!(m.read_memory(0xffff4), (segment >> 8) as u8);
    }
}

#[test]
fn locked_out_from_ram_before_executing_the_first_footer_instruction() {
    let cases: [(Model, u16, &[u8]); 3] = [
        (
            Model::Mono,
            0xff00,
            include_bytes!("../../../fixtures/generated/startup-mono-common.ws"),
        ),
        (
            Model::Color,
            0xfe00,
            include_bytes!("../../../fixtures/generated/raw3-common.ws"),
        ),
        (
            Model::Crystal,
            0xfe00,
            include_bytes!("../../../fixtures/generated/raw6-2003.ws"),
        ),
    ];
    for (model, segment, rom) in cases {
        let mut m = Machine::new(model);
        m.load_rom(rom).unwrap();
        let records = m.bus.console_data.clone();

        let clocks = m.cpu.step(&mut m.bus).unwrap();
        m.advance(clocks);
        assert_eq!((m.cpu.segments[1], m.cpu.ip), (segment, 0));

        while m.cpu.segments[1] != 0 && m.bus.cycles < 256 {
            assert_eq!(m.bus.ports[0xa0] & 1, 0);
            let clocks = m.cpu.step(&mut m.bus).unwrap();
            m.advance(clocks);
        }
        assert_eq!((m.cpu.segments[1], m.cpu.ip), (0, 0x1ff0));
        assert_eq!(&m.bus.ram[0x1ff0..0x1ff2], &[0xe6, 0xa0]);
        assert_eq!(m.bus.ports[0xa0] & 1, 0);
        let clocks = m.cpu.step(&mut m.bus).unwrap();
        m.advance(clocks);
        assert_eq!((m.cpu.ip, m.bus.ports[0xa0] & 1), (0x1ff2, 1));
        assert_eq!(m.read_memory(0xffff4), 0xf0);

        while (m.cpu.segments[1], m.cpu.ip) != (0xffff, 0) && m.bus.cycles < 256 {
            let clocks = m.cpu.step(&mut m.bus).unwrap();
            m.advance(clocks);
        }
        assert_eq!((m.cpu.segments[1], m.cpu.ip), (0xffff, 0));
        assert!(m.bus.cycles < 256);
        assert_eq!(m.bus.console_data, records);
        assert!(m.bus.ram[..0x1ff0]
            .iter()
            .chain(&m.bus.ram[0x1ffb..])
            .all(|&byte| byte == 0));
        let clocks = m.cpu.step(&mut m.bus).unwrap();
        m.advance(clocks);
        assert_eq!((m.cpu.segments[1], m.cpu.ip), (0xf000, 0));
    }
}

#[test]
fn color_dma_uses_the_overlay_and_internal_access_time_until_lockout() {
    let mut m = Machine::new(Model::Color);
    let mut rom = cartridge::tests::rom(131072, 0, 0);
    rom[0x1e000] = 0x52;
    cartridge::tests::checksum(&mut rom);
    m.load_rom(&rom).unwrap();
    let b = &mut m.bus;
    b.output(0x60, 0x80).unwrap();
    for (a0, byte, stall, gdma) in [(8, 0xfa, 7, Some(7)), (9, 0x52, 8, None)] {
        b.output(0xa0, a0).unwrap(); // Slow byte transfers only on the cartridge.
        b.output_word(0x40, 0xe000).unwrap();
        b.output(0x42, 0x0f).unwrap();
        b.output_word(0x44, 0x0800).unwrap();
        b.output_word(0x46, 2).unwrap();
        b.output(0x48, 0x80).unwrap();
        assert_eq!(b.gdma_step(), gdma);
        assert_eq!(&b.ram[0x800..0x802], &[0xfa, 0xfc]);

        b.output_word(0x4a, 0xe000).unwrap();
        b.output(0x4c, 0x0f).unwrap();
        b.output_word(0x4e, 1).unwrap();
        b.output(0x52, 0x83).unwrap();
        b.tick(if a0 == 8 { 117 } else { 128 });
        assert_eq!(b.sound_dma_stall(), Some(stall));
        assert_eq!(b.input(0x89).unwrap(), byte);
    }
}

#[test]
fn every_header_and_physical_model_handoff_replaces_sp_ds_and_keeps_bus_policies() {
    for header in [0, 1] {
        for mapper in [0, 1] {
            for flags in [0, 4, 8, 12] {
                for protected in [false, true] {
                    let mut rom = cartridge::tests::rom(131072, 2, mapper);
                    rom[131072 - 9] = header;
                    rom[131072 - 7] = if protected { 0 } else { 0x80 };
                    rom[131072 - 4] = flags;
                    cartridge::tests::checksum(&mut rom);
                    for (model, ds, fixed) in MODELS {
                        if header == 1 && model == Model::Mono {
                            continue;
                        }
                        let mut m = Machine::new(model);
                        // Independent records must survive every handoff.
                        for (i, record) in m.bus.console_data.iter_mut().enumerate() {
                            record[0] = 0x40 + i as u8;
                        }
                        let records = m.bus.console_data.clone();
                        dirty(&mut m);
                        m.load_rom(&rom).unwrap();
                        entry(&mut m, ds, fixed | flags, mapper, protected);
                        m.load_save(&vec![0x77; 32768]).unwrap();
                        dirty(&mut m);
                        m.restart();
                        entry(&mut m, ds, fixed | flags, mapper, protected);
                        dirty(&mut m);
                        m.change_model(model).unwrap(); // same-model restart
                        entry(&mut m, ds, fixed | flags, mapper, protected);
                        let mut replacement = rom.clone();
                        replacement[131072 - 4] = flags ^ 12;
                        cartridge::tests::checksum(&mut replacement);
                        dirty(&mut m);
                        m.load_rom(&replacement).unwrap();
                        entry(&mut m, ds, fixed | (flags ^ 12), mapper, protected);
                        assert!(m.save_data().iter().all(|v| *v == 255));
                        assert_eq!(m.bus.console_data, records);
                        dirty(&mut m);
                        m.load_rom(&rom).unwrap();
                        entry(&mut m, ds, fixed | flags, mapper, protected);
                        m.load_save(&vec![0x77; 32768]).unwrap();
                        for (next, next_ds, next_fixed) in MODELS {
                            if header == 1 && next == Model::Mono {
                                continue;
                            }
                            // Return to each source before each directed transition.
                            m.change_model(model).unwrap();
                            dirty(&mut m);
                            m.change_model(next).unwrap();
                            entry(&mut m, next_ds, next_fixed | flags, mapper, protected);
                            assert!(m.save_data().iter().all(|v| *v == 0x77));
                            assert_eq!(m.bus.console_data, records);
                        }
                    }
                }
            }
        }
    }
}
