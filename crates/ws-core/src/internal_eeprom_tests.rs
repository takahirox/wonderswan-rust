// SPDX-License-Identifier: CC0-1.0
use super::*;
use bus::Bus;

fn out_word(b: &mut Bus, port: u16, value: u16) {
    b.output(port, value as u8).unwrap();
    b.output(port + 1, (value >> 8) as u8).unwrap();
}
fn in_word(b: &mut Bus, port: u16) -> u16 {
    u16::from_le_bytes([b.input(port).unwrap(), b.input(port + 1).unwrap()])
}
fn command(b: &mut Bus, op: u16, address: u16, control: u8) {
    let bits = if b.color_mode() { 10 } else { 6 };
    out_word(b, 0xbc, (1 << (bits + 2)) | (op << bits) | address);
    out_word(b, 0xbe, u16::from(control));
}
fn latch(b: &mut Bus, enabled: bool) {
    let bits = if b.color_mode() { 10 } else { 6 };
    command(b, 0, if enabled { 3 << (bits - 2) } else { 0 }, 0x40);
    b.tick(3072);
}
fn read(b: &mut Bus, address: u16) -> u16 {
    command(b, 2, address, 0x10);
    assert_eq!(in_word(b, 0xbe) & 3, 0);
    b.tick(31);
    assert_eq!(b.input(0xbe).unwrap() & 3, 0);
    b.tick(1);
    assert_eq!(b.input(0xbe).unwrap() & 3, 3);
    in_word(b, 0xba)
}
fn write(b: &mut Bus, address: u16, value: u16) {
    out_word(b, 0xba, value);
    command(b, 1, address, 0x20);
    assert_eq!(b.input(0xbe).unwrap() & 2, 0);
    b.tick(3072);
    assert_eq!(b.input(0xbe).unwrap() & 2, 2);
}
#[test]
fn capacity_boundaries_endianness_buffers_and_mode_transitions() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut b = Bus::new(model);
        let len = if model == Model::Mono { 128 } else { 2048 };
        assert_eq!(b.console_data[model.index()].len(), len);
        assert_eq!(read(&mut b, 0), 0xffff);
        // Reset latch is disabled; firmware handoff is tested separately.
        write(&mut b, 0, 1);
        assert_eq!(read(&mut b, 0), 0xffff);
        latch(&mut b, true);
        for color in [false, true, false, true] {
            if model == Model::Mono && color {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
            }
            let last = if color { 1023 } else { 63 };
            write(&mut b, 0, 0x1234);
            write(&mut b, last, 0xabcd);
            assert_eq!(read(&mut b, 0), 0x1234);
            assert_eq!(read(&mut b, last), 0xabcd);
            assert_eq!(
                &b.console_data[model.index()][last as usize * 2..last as usize * 2 + 2],
                &[0xcd, 0xab]
            );
            assert_eq!(b.console_data[model.index()].len(), len);
            assert_eq!(
                b.console_data[(model.index() + 1) % 3],
                internal_eeprom::initial_data(
                    [Model::Mono, Model::Color, Model::Crystal][(model.index() + 1) % 3]
                )
            );
        }
        latch(&mut b, false);
        write(&mut b, 0, 0x9999);
        assert_eq!(read(&mut b, 0), 0x1234);
        latch(&mut b, true);
        write(&mut b, 0, 0x4567);
        assert_eq!(in_word(&mut b, 0xba), 0x1234, "separate read buffer");
        assert_eq!(read(&mut b, 0), 0x4567);
    }
}
#[test]
fn captured_operands_mode_and_sticky_protection() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut b = Bus::new(model);
        latch(&mut b, true);
        write(&mut b, 0x30, 0xbeef);
        assert_eq!(read(&mut b, 0x30), 0xbeef);
        out_word(&mut b, 0xba, 0x1234);
        command(&mut b, 1, 0x2f, 0x20);
        out_word(&mut b, 0xba, 0x9999);
        out_word(&mut b, 0xbc, 0xffff);
        if model != Model::Mono {
            b.output(0x60, 0x80).unwrap();
        }
        b.tick(3071);
        assert_eq!(b.console_data[model.index()][0x5e], 255);
        // BF and protection must not abort an operation, even while busy.
        b.output(0xbf, 255).unwrap();
        assert_eq!(b.input(0xbf).unwrap(), 0);
        b.output(0xbe, 0x80).unwrap();
        assert_eq!(b.input(0xbe).unwrap() & 0x82, 0x80);
        b.tick(1);
        assert_eq!(in_word(&mut b, 0xba), 0xbeef);
        assert_eq!(read(&mut b, 0x2f), 0x1234);
        for color in [false, true, false] {
            if model == Model::Mono && color {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
            }
            b.output(0xbe, 0x80).unwrap();
            b.output(0xbe, 0).unwrap();
            latch(&mut b, false);
            write(&mut b, 0x2f, 0);
            assert_eq!(read(&mut b, 0x2f), 0x1234);
            latch(&mut b, true);
            write(&mut b, 0x2f, 0x1234);
            write(&mut b, 0x30, 0);
            assert_eq!(read(&mut b, 0x30), 0xbeef);
            assert_eq!(b.input(0xbe).unwrap(), 0x83);
        }
    }
}
#[test]
fn unsupported_commands_do_not_inherit_external_erase_bulk_or_abort_behavior() {
    for color in [false, true] {
        let mut b = Bus::new(Model::Crystal);
        b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
        latch(&mut b, true);
        let bytes = b.console_data.clone();
        let bits = if color { 10 } else { 6 };
        for (op, address, control) in [
            (3, 0, 0x40),
            (0, 1 << (bits - 2), 0x20),
            (0, 2 << (bits - 2), 0x40),
            (2, 0, 0x30),
        ] {
            out_word(&mut b, 0xbc, (1 << (bits + 2)) | (op << bits) | address);
            assert!(b
                .output(0xbe, control)
                .unwrap_err()
                .to_string()
                .contains("Unsupported internal EEPROM"));
            b.tick(10000);
            assert_eq!(b.console_data, bytes);
            assert_eq!(b.input(0xbe).unwrap() & 2, 2);
        }
        out_word(&mut b, 0xbc, 0);
        assert!(b.output(0xbe, 0x10).is_err());
        assert_eq!(b.console_data, bytes);
    }
}
#[test]
fn machine_retention_import_failures_header_handoff_and_pending_cancellation() {
    let mut m = Machine::new(Model::Mono);
    let rom = cartridge::tests::rom(131072, 2, 0);
    for (i, model) in [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .enumerate()
    {
        m.reset(model);
        let mut bytes = m.console_data().to_vec();
        bytes[0] = i as u8;
        bytes[0x60] = 0x55;
        m.load_console_data(&bytes).unwrap();
        assert!(m.load_console_data(&[1, 2]).is_err());
        assert!(m.load_rom(&[1, 2]).is_err());
        assert_eq!(m.console_data(), bytes);
        m.load_rom(&rom).unwrap(); // header exception: settings writable
        assert_eq!(m.bus.input(0xbe).unwrap(), 2);
        write(&mut m.bus, 0x30, 0x1234);
        out_word(&mut m.bus, 0xba, 0x8888);
        command(&mut m.bus, 1, 0, 0x20);
        m.restart(); // discard unfinished write, keep completed settings word
        assert_eq!(m.console_data()[0], i as u8);
        assert_eq!(&m.console_data()[0x60..0x62], &[0x34, 0x12]);
        m.bus.output(0xbe, 0x80).unwrap();
        m.restart();
        assert_eq!(m.bus.input(0xbe).unwrap(), 2);
        let mut protected = rom.clone();
        protected[131072 - 7] = 0;
        cartridge::tests::checksum(&mut protected);
        m.load_rom(&protected).unwrap();
        assert_eq!(m.bus.input(0xbe).unwrap(), 0x82);
        write(&mut m.bus, 0x30, 0);
        assert_eq!(&m.console_data()[0x60..0x62], &[0x34, 0x12]);
        write(&mut m.bus, 0x2f, 0x4567);
        m.restart();
        assert_eq!(m.bus.input(0xbe).unwrap(), 0x82);
        m.reset(model); // unload
        assert_eq!(m.console_data()[0], i as u8);
        assert_eq!(&m.console_data()[0x5e..0x60], &[0x67, 0x45]);
    }
    for (i, model) in [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .enumerate()
    {
        m.change_model(model).unwrap();
        assert_eq!(m.console_data()[0], i as u8);
    }
    // A rejected incompatible switch preserves console, controller and cartridge.
    m.change_model(Model::Color).unwrap();
    let mut color_rom = rom.clone();
    color_rom[131072 - 9] = 1;
    cartridge::tests::checksum(&mut color_rom);
    m.load_rom(&color_rom).unwrap();
    let bytes = m.console_data().to_vec();
    assert!(m.change_model(Model::Mono).is_err());
    assert_eq!(m.model(), Model::Color);
    assert_eq!(m.console_data(), bytes);
    out_word(&mut m.bus, 0xba, 0x9999);
    command(&mut m.bus, 1, 0, 0x20);
    m.load_console_data(&bytes).unwrap();
    m.bus.tick(3072);
    assert_eq!(m.console_data(), bytes);
}
#[test]
fn shared_clock_advances_internal_transfer_timer_audio_and_both_dmas() {
    for (model, rom_source) in [Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|model| [false, true].map(|rom| (model, rom)))
    {
        let mut b = Bus::new(model);
        if rom_source {
            let mut c = cartridge::Cartridge::parse(&cartridge::tests::rom(131072, 0, 0)).unwrap();
            c.header.rom_word_bus = true;
            c.bytes[65536..65540].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
            b.insert_cartridge(c);
        }
        b.output(0x60, 0x80).unwrap();
        latch(&mut b, true);
        out_word(&mut b, 0xba, 0x1234);
        command(&mut b, 1, 0, 0x20);
        b.output(0xa2, 1).unwrap();
        out_word(&mut b, 0xa4, 4);
        b.output(0xb2, 0x80).unwrap();
        b.ram[0x1000..0x1004].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        out_word(&mut b, 0x4a, if rom_source { 0 } else { 0x1000 });
        b.output(0x4c, if rom_source { 2 } else { 0 }).unwrap();
        out_word(&mut b, 0x4e, 4);
        b.output(0x52, 0x83).unwrap();
        out_word(&mut b, 0x40, 0x1000);
        out_word(&mut b, 0x44, 0x2000);
        out_word(&mut b, 0x46, 4096);
        b.output(0x48, 0x80).unwrap();
        let start = b.cycles;
        let mut cpu = Cpu::default();
        while b.ports[0x48] & 0x80 != 0 {
            if rom_source {
                // Live ROM waits change while SDMA and RAM GDMA share the clock.
                b.output(0xa0, 4 | (((b.cycles / 128) as u8 & 1) << 3))
                    .unwrap();
            }
            let cycles = cpu.step(&mut b).unwrap();
            b.tick(cycles);
        }
        assert!(b.cycles - start >= 4101);
        assert_eq!(cpu.ip, 0);
        assert_eq!(b.input(0xbe).unwrap() & 2, 2);
        assert_eq!(&b.console_data[model.index()][..2], &[0x34, 0x12]);
        assert_eq!(&b.ram[0x2000..0x2004], &[0x12, 0x34, 0x56, 0x78]);
        assert_eq!(in_word(&mut b, 0xa8), 0);
        assert_eq!(b.input(0xb4).unwrap() & 0x80, 0x80);
        assert_eq!(b.input(0x89).unwrap(), 0x78);
        assert_eq!(in_word(&mut b, 0x4e), 0);
        assert!(!b.audio.drain().is_empty());
    }
}

#[test]
fn read_captures_address_and_failed_import_preserves_pending_controller_state() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(&cartridge::tests::rom(131072, 2, 0)).unwrap();
        write(&mut m.bus, 0, 0x1234);
        write(&mut m.bus, 1, 0xabcd);
        command(&mut m.bus, 2, 0, 0x10);
        out_word(&mut m.bus, 0xbc, 0xffff);
        out_word(&mut m.bus, 0xba, 0x9999);
        if model != Model::Mono {
            m.bus.output(0x60, 0x80).unwrap();
        }
        assert!(m.load_console_data(&[0]).is_err());
        assert!(m.load_rom(&[0]).is_err());
        assert!(m
            .bus
            .output(0xbe, 0x10)
            .unwrap_err()
            .to_string()
            .contains("busy"));
        assert_eq!(m.bus.input(0xbe).unwrap(), 0);
        m.bus.tick(32);
        assert_eq!(in_word(&mut m.bus, 0xba), 0x1234);
        assert_eq!(m.bus.input(0xbe).unwrap(), 3);
        // Applying protection during an accepted settings write prevents commit.
        out_word(&mut m.bus, 0xba, 0xdead);
        command(&mut m.bus, 1, 0x30, 0x20);
        m.bus.output(0xbe, 0x80).unwrap();
        m.bus.tick(3072);
        assert_eq!(&m.console_data()[0x60..0x62], &[0, 0]);
        assert_eq!(in_word(&mut m.bus, 0xba), 0x1234);
    }
}
