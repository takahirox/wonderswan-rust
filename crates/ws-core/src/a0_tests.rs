// SPDX-License-Identifier: CC0-1.0
// Original runtime-control and synthetic-handoff vectors for Issue #23.
use super::*;
use bus::Bus;

fn rom(flags: u8) -> Vec<u8> {
    let mut bytes = cartridge::tests::rom(131072, 2, 0);
    let end = bytes.len();
    bytes[end - 4] = flags;
    cartridge::tests::checksum(&mut bytes);
    bytes
}

#[test]
fn a0_masks_writes_on_all_models_without_changing_cartridge_metadata() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for flags in [0, 4, 8, 12] {
            let mut b = Bus::new(model);
            b.insert_cartridge(cartridge::Cartridge::parse(&rom(flags)).unwrap());
            let fixed = if model == Model::Mono { 0x81 } else { 0x83 };
            assert_eq!(b.input(0xa0).unwrap(), fixed | flags);
            let header = format!("{:?}", b.cartridge.as_ref().unwrap().header);
            let bytes = b.cartridge.as_ref().unwrap().bytes.clone();
            for value in [0, 255, 255, 0, 0x55, 0xaa, 0x55, 0xaa, 4, 8, 12, 0x70] {
                b.output(0xa0, value).unwrap();
                assert_eq!(b.input(0xa0).unwrap(), fixed | (value & 12));
                assert_eq!(
                    format!("{:?}", b.cartridge.as_ref().unwrap().header),
                    header
                );
                assert_eq!(b.cartridge.as_ref().unwrap().bytes, bytes);
            }
            // Exercise sticky lock even outside the normal already-locked handoff.
            b.ports[0xa0] &= !1;
            b.output(0xa0, 0).unwrap();
            assert_eq!(b.input(0xa0).unwrap() & 1, 0);
            b.output(0xa0, 1).unwrap();
            b.output(0xa0, 0).unwrap();
            assert_eq!(b.input(0xa0).unwrap(), fixed);
        }
    }
}

#[test]
fn successful_handoffs_reseed_controls_and_failed_transitions_preserve_running_state() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for flags in [0, 4, 8, 12] {
            let mut m = Machine::new(model);
            m.load_rom(&rom(flags)).unwrap();
            let fixed = if model == Model::Mono { 0x81 } else { 0x83 };
            assert_eq!(m.bus.input(0xa0).unwrap(), fixed | flags);
            m.bus.output(0xa0, flags ^ 12).unwrap();
            m.bus.ram[123] = 99;
            m.bus.tick(123);
            let ports = m.bus.ports;
            let cpu = format!("{:?}", m.cpu);
            assert!(m.load_rom(&[0]).is_err());
            assert_eq!(m.bus.ports, ports);
            assert_eq!(m.bus.ram[123], 99);
            assert_eq!(m.bus.cycles, 123);
            assert_eq!(format!("{:?}", m.cpu), cpu);
            m.restart();
            assert_eq!(m.bus.input(0xa0).unwrap(), fixed | flags);
            for next in [Model::Color, Model::Crystal, Model::Mono] {
                m.bus.output(0xa0, flags ^ 12).unwrap();
                m.change_model(next).unwrap();
                assert_eq!(
                    m.bus.input(0xa0).unwrap(),
                    (if next == Model::Mono { 0x81 } else { 0x83 }) | flags
                );
            }
            // Replacement also uses the new header, not the last live controls.
            m.load_rom(&rom(flags ^ 12)).unwrap();
            assert_eq!(m.bus.input(0xa0).unwrap(), 0x81 | (flags ^ 12));
        }
    }
    let mut color = rom(4);
    let end = color.len();
    color[end - 9] = 1;
    cartridge::tests::checksum(&mut color);
    let mut m = Machine::new(Model::Color);
    m.load_rom(&color).unwrap();
    m.bus.output(0xa0, 8).unwrap();
    m.bus.ram[123] = 99;
    m.bus.tick(77);
    let ports = m.bus.ports;
    assert!(m.change_model(Model::Mono).is_err());
    assert_eq!(m.bus.ports, ports);
    assert_eq!(m.bus.ram[123], 99);
    assert_eq!(m.bus.cycles, 77);
    assert_eq!(m.model(), Model::Color);
    let mut mono = Machine::new(Model::Mono);
    mono.load_rom(&rom(0)).unwrap();
    mono.bus.output(0xa0, 12).unwrap();
    assert!(mono.load_rom(&color).is_err());
    assert_eq!(mono.bus.input(0xa0).unwrap(), 0x8d);
}
