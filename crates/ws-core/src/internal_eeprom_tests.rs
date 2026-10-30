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
    for (model, color) in [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|m| [false, true].map(move |c| (m, c)))
        .filter(|(m, c)| *m != Model::Mono || !c)
    {
        let mut b = Bus::new(model);
        b.console_data[model.index()].fill(0x5a);
        if model != Model::Mono {
            b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
        }
        latch(&mut b, true);
        assert_eq!(read(&mut b, 0), 0x5a5a);
        let bytes = b.console_data.clone();
        let bits = if color { 10 } else { 6 };
        // Neither WDS nor settings protection hides unsupported diagnostics.
        for enabled in [false, true] {
            latch(&mut b, enabled);
            for protected in [false, true] {
                if protected {
                    b.output(0xbe, 0x80).unwrap();
                }
                for address in [1, 0x2f, 0x30, 63, (1 << bits) - 1] {
                    out_word(&mut b, 0xbc, (7 << bits) | address);
                    let error = b.output(0xbe, 0x40).unwrap_err().to_string();
                    assert!(error.contains("ERASE") && error.contains("unverified"));
                    assert!(error.contains(&format!("{:#05x}", address * 2)));
                    b.tick(10000);
                    assert_eq!(b.console_data, bytes);
                    assert_eq!(b.input(0xbe).unwrap() & 3, 3);
                }
                for (sub, name) in [(1, "WRAL"), (2, "ERAL")] {
                    for control in [0x20, 0x40] {
                        out_word(&mut b, 0xbc, (1 << (bits + 2)) | (sub << (bits - 2)));
                        let error = b.output(0xbe, control).unwrap_err().to_string();
                        assert!(error.contains(name) && error.contains("unverified"));
                        b.tick(10000);
                        assert_eq!(b.console_data, bytes);
                        assert_eq!(b.input(0xbe).unwrap() & 3, 3);
                    }
                }
            }
        }
        for cmd in [0, 3 << bits, 15 << bits, if color { 0x1c0 } else { 0x1c00 }] {
            out_word(&mut b, 0xbc, cmd);
            assert!(b
                .output(0xbe, 0x40)
                .unwrap_err()
                .to_string()
                .contains("format"));
            assert_eq!(in_word(&mut b, 0xbc), cmd);
        }
        for control in [0x10, 0x20, 0x30, 0x50, 0x60, 0x70] {
            out_word(&mut b, 0xbc, 7 << bits);
            assert!(b.output(0xbe, control).is_err());
        }
        b.tick(10000);
        assert_eq!(b.console_data, bytes);
        assert_eq!(in_word(&mut b, 0xba), 0x5a5a);
        assert_eq!(b.input(0xbe).unwrap(), 0x83);
    }
}

#[test]
fn erase_zero_captures_authorization_preserves_buffers_and_completes_on_shared_clock() {
    for (model, color) in [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|m| [false, true, false].map(move |c| (m, c)))
        .filter(|(m, c)| *m != Model::Mono || !c)
    {
        for done in [false, true] {
            let mut b = Bus::new(model);
            if model != Model::Mono {
                b.output(0x60, if color { 0x80 } else { 0 }).unwrap();
            }
            b.console_data[model.index()].fill(0x5a);
            if done {
                assert_eq!(read(&mut b, 1), 0x5a5a);
            }
            let read_buffer = in_word(&mut b, 0xba);
            // Reset WDS: accepted erase consumes time but cannot mutate bytes.
            command(&mut b, 3, 0, 0x40);
            assert_eq!(b.input(0xbe).unwrap(), u8::from(done));
            b.tick(3071);
            assert_eq!(b.console_data[model.index()][0], 0x5a);
            b.tick(1);
            assert_eq!(b.input(0xbe).unwrap(), 2 | u8::from(done));
            assert_eq!(b.console_data[model.index()][0], 0x5a);
            latch(&mut b, true);
            let mut expected = b.console_data.clone();
            expected[model.index()][..2].fill(255);
            // Reserved low bits and BF are harmless; erase never uses BA data.
            out_word(&mut b, 0xba, 0x1234);
            command(&mut b, 3, 0, 0x4f);
            let bits = if color { 10 } else { 6 };
            assert_eq!(in_word(&mut b, 0xbc), 7 << bits);
            out_word(&mut b, 0xba, 0x9999);
            out_word(&mut b, 0xbc, 0xffff);
            assert_eq!(in_word(&mut b, 0xbc), 0xffff);
            if model != Model::Mono {
                b.output(0x60, if color { 0 } else { 0x80 }).unwrap();
            }
            for control in [0x10, 0x20, 0x40, 0x70] {
                assert!(b
                    .output(0xbe, control)
                    .unwrap_err()
                    .to_string()
                    .contains("busy"));
            }
            b.output(0xbf, 0xff).unwrap();
            assert_eq!(b.input(0xbf).unwrap(), 0);
            b.output(0xbe, 0x80).unwrap(); // protects settings, does not abort zero
            b.output(0xbe, 0).unwrap();
            b.tick(3071);
            assert_eq!(b.input(0xbe).unwrap(), 0x80 | u8::from(done));
            assert_eq!(b.console_data[model.index()][..2], [0x5a; 2]);
            assert_eq!(in_word(&mut b, 0xba), read_buffer);
            b.tick(1);
            assert_eq!(b.input(0xbe).unwrap(), 0x82 | u8::from(done));
            assert_eq!(b.console_data, expected); // all settings, models unchanged
            assert_eq!(in_word(&mut b, 0xba), read_buffer);
            assert_eq!(read(&mut b, 0), 0xffff);
            // Register changes while erasing leave the independent write latch.
            command(&mut b, 1, 1, 0x20);
            b.tick(3072);
            assert_eq!(read(&mut b, 1), 0x9999);
            latch(&mut b, false);
            command(&mut b, 3, 0, 0x40);
            b.tick(10000); // excess clocks still leave WDS in force
            assert_eq!(b.console_data[model.index()][..2], [255; 2]);
        }
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
    for (model, rom_source, erase) in [Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|model| [false, true].map(|rom| (model, rom)))
        .flat_map(|(model, rom)| [false, true].map(move |erase| (model, rom, erase)))
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
        b.console_data[model.index()][..2].fill(0x5a);
        out_word(&mut b, 0xba, 0x1234);
        command(
            &mut b,
            if erase { 3 } else { 1 },
            0,
            if erase { 0x40 } else { 0x20 },
        );
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
        assert_eq!(
            &b.console_data[model.index()][..2],
            if erase { &[255, 255] } else { &[0x34, 0x12] }
        );
        assert!(
            b.input(0x02).unwrap() > 0,
            "scanlines share programming clocks"
        );
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

#[test]
fn erase_lifecycle_cancellation_retention_and_failed_transitions() {
    for (model, color) in [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|m| [false, true].map(move |c| (m, c)))
        .filter(|(m, c)| *m != Model::Mono || !c)
    {
        for lifecycle in 0..5 {
            let mut m = Machine::new(model);
            let rom = cartridge::tests::rom(131072, 2, 0);
            m.load_rom(&rom).unwrap();
            m.load_save(&vec![0x77; 32768]).unwrap();
            if model != Model::Mono {
                m.bus.output(0x60, if color { 0x80 } else { 0 }).unwrap();
            }
            write(&mut m.bus, 0, 0x1234);
            let before = m.bus.console_data.clone();
            command(&mut m.bus, 3, 0, 0x40);
            m.bus.tick(3071);
            assert_eq!(m.bus.console_data, before);
            match lifecycle {
                0 => m.restart(),
                1 => {
                    m.load_rom(&rom).unwrap();
                }
                2 => m.reset(model),
                3 => {
                    let other = if model == Model::Mono {
                        Model::Color
                    } else {
                        Model::Mono
                    };
                    m.change_model(other).unwrap();
                    m.bus.tick(10000);
                    m.change_model(model).unwrap();
                }
                _ => m.load_console_data(&before[model.index()]).unwrap(),
            }
            m.bus.tick(10000);
            assert_eq!(m.bus.console_data, before, "unfinished erase discarded");
            if matches!(lifecycle, 0 | 3 | 4) {
                assert!(m.save_data().iter().all(|b| *b == 0x77));
            }
            // Completed erase survives every lifecycle above, including a
            // successful administrative import of the same completed bytes.
            m.load_rom(&rom).unwrap();
            command(&mut m.bus, 3, 0, 0x40);
            assert!(m.load_console_data(&[0]).is_err());
            assert!(m.load_rom(&[0]).is_err());
            m.bus.tick(3072);
            let mut after = before.clone();
            after[model.index()][..2].fill(255);
            assert_eq!(m.bus.console_data, after);
            m.restart();
            m.load_rom(&rom).unwrap();
            m.reset(model);
            m.change_model(if model == Model::Mono {
                Model::Color
            } else {
                Model::Mono
            })
            .unwrap();
            m.change_model(model).unwrap();
            m.bus.tick(10000);
            assert_eq!(m.bus.console_data, after);
            // A Color-only cartridge rejects a switch before touching pending
            // state; the original model's erase must still finish.
            if model != Model::Mono {
                let mut color_rom = rom.clone();
                color_rom[131072 - 9] = 1;
                cartridge::tests::checksum(&mut color_rom);
                m.load_rom(&color_rom).unwrap();
                write(&mut m.bus, 0, 0xbeef);
                command(&mut m.bus, 3, 0, 0x40);
                assert!(m.change_model(Model::Mono).is_err());
                assert_eq!(&m.console_data()[..2], &[0xef, 0xbe]);
                m.bus.tick(3072);
                assert_eq!(m.bus.console_data, after);
            }
        }
    }
}
