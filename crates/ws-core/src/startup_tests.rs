// SPDX-License-Identifier: CC0-1.0
// Literal cartridge-entry vectors for Issue #33, independent of the initializer.
use super::*;

const MODELS: [(Model, u16, u8); 3] = [
    (Model::Mono, 0xff00, 0x81),
    (Model::Color, 0xfe00, 0x83),
    (Model::Crystal, 0xfe00, 0x83),
];

fn dirty(m: &mut Machine) {
    m.cpu.registers[4] = 0x3456;
    m.cpu.segments[3] = 0x1234;
    m.cpu.halted = true;
    m.bus.output(0xa0, 0).unwrap();
}

fn entry(m: &mut Machine, ds: u16, a0: u8, mapper: u8, protected: bool) {
    assert_eq!(m.cpu.registers, [0, 0, 0, 0, 0x2000, 0, 0, 0]);
    assert_eq!(m.cpu.segments, [0, 0xffff, 0, ds]);
    assert_eq!(m.cpu.ip, 0);
    assert_eq!(m.cpu.flags, 0xf002);
    assert!(!m.cpu.halted);
    assert_eq!(m.bus.cycles, 0);
    assert_eq!(m.bus.input(0xa0).unwrap(), a0);
    assert_eq!(m.bus.input(0xbe).unwrap(), if protected { 0x82 } else { 2 });
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
