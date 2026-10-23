// SPDX-License-Identifier: CC0-1.0
// Original generated vectors for Issue #25; no third-party emulator code.
use super::*;
use bus::Bus;
use cartridge::{
    tests::{checksum, rom},
    Cartridge,
};

fn marked(size: usize, mapper: u8) -> Vec<u8> {
    let mut bytes = rom(size, 2, mapper);
    for (bank, chunk) in bytes.chunks_mut(65536).enumerate() {
        for pair in chunk.chunks_exact_mut(2) {
            pair.copy_from_slice(&[bank as u8 ^ 0x5a, bank as u8 ^ 0xa5]);
        }
    }
    let footer = rom(size, 2, mapper);
    bytes[size - 16..].copy_from_slice(&footer[size - 16..]);
    checksum(&mut bytes);
    bytes
}

#[test]
fn raw_sizes_validate_exact_headers_and_checksums_without_padding() {
    for size in [3145728, 6291456] {
        for mapper in [0, 1] {
            let bytes = marked(size, mapper);
            let c = Cartridge::parse(&bytes).unwrap();
            assert_eq!(c.bytes, bytes);
            assert_eq!(c.header.rom_size, size);
            assert_eq!(
                c.header.checksum,
                u16::from_le_bytes([bytes[size - 2], bytes[size - 1]])
            );
            for offset in [0, 5, 7, 11, 12, 13] {
                let mut bad = bytes.clone();
                bad[size - 16 + offset] = 0xff;
                checksum(&mut bad);
                assert!(Cartridge::parse(&bad).is_err(), "field {offset}");
            }
            let mut corrupt = bytes.clone();
            corrupt[123] ^= 1;
            assert!(Cartridge::parse(&corrupt)
                .unwrap_err()
                .to_string()
                .contains("checksum"));
            // Keep a valid footer at the physical end so truncation/extra bytes
            // reach exact declared-size validation, rather than opcode failure.
            for length in [
                size - 1,
                size + 1,
                size.next_power_of_two(),
                5 * 1024 * 1024,
            ] {
                let mut bad = vec![0; length];
                bad[length - 16..].copy_from_slice(&bytes[size - 16..]);
                let error = Cartridge::parse(&bad).unwrap_err().to_string();
                assert!(error.contains("declares"), "{error}");
                assert!(!error.contains("padding"));
            }
            let mut bad = bytes.clone();
            bad[size - 6] = if size == 3145728 { 7 } else { 5 };
            checksum(&mut bad);
            assert!(Cartridge::parse(&bad)
                .unwrap_err()
                .to_string()
                .contains("declares"));
            bad[size - 6] = 0xff;
            assert!(Cartridge::parse(&bad)
                .unwrap_err()
                .to_string()
                .contains("size code"));
        }
    }
    for size in [0, 131071, 67108865] {
        assert!(Cartridge::parse(&vec![0; size])
            .unwrap_err()
            .to_string()
            .contains("128 KiB–64 MiB"));
    }
}

#[test]
fn top_aligned_bank_markers_cover_both_interfaces_and_rounded_aliases() {
    for size in [3145728usize, 6291456] {
        for mapper in [0, 1] {
            let bytes = marked(size, mapper);
            let capacity = size.next_power_of_two();
            let gap = capacity - size;
            let c = Cartridge::parse(&bytes).unwrap();
            // Every bank, both edges, interior offsets, and every mapper alias.
            // Expected values index raw bytes without masking or modulo by L.
            for bank in 0..1024 {
                for offset in [0, 1, 0x1234, 0x8000, 0xffef, 0xfffe, 0xffff] {
                    let address = (bank * 65536 + offset) % capacity;
                    let expected = if address < gap {
                        255
                    } else {
                        bytes[address - gap]
                    };
                    assert_eq!(
                        c.rom(bank, offset),
                        expected,
                        "size {size} bank {bank:x}:{offset:x}"
                    );
                }
            }
            assert_eq!(c.rom(gap / 65536, 0), bytes[0]);
            assert_eq!(c.rom(capacity / 65536 - 1, 65535), bytes[size - 1]);
            let mut b = Bus::new(Model::Crystal);
            b.insert_cartridge(c);
            for bank in 0..if mapper == 1 { 1024 } else { 256 } {
                for (low, high, base) in [(0xc2, 0xd3, 0x20000), (0xc3, 0xd5, 0x30000)] {
                    b.output(low, bank as u8).unwrap();
                    if mapper == 1 {
                        b.output(high, (bank >> 8) as u8 | 0xfc).unwrap();
                        assert_eq!(
                            b.input(low + if low == 0xc2 { 0x10 } else { 0x11 })
                                .unwrap(),
                            bank as u8
                        );
                    }
                    for offset in [0, 1, 0x8000, 65535] {
                        assert_eq!(
                            b.read(base + offset),
                            b.cartridge.as_ref().unwrap().rom(bank, offset as usize)
                        );
                    }
                }
                if mapper == 1 {
                    b.output(0xd0, bank as u8).unwrap();
                    b.output(0xd1, (bank >> 8) as u8 | 0xfc).unwrap();
                    b.output(0xce, 1).unwrap();
                    assert_eq!(b.read(0x10000), b.cartridge.as_ref().unwrap().rom(bank, 0));
                    assert_eq!(b.input(0xc1).unwrap(), bank as u8);
                }
            }
            for linear in 0..=255u8 {
                b.output(if mapper == 1 { 0xcf } else { 0xc0 }, linear)
                    .unwrap();
                if mapper == 1 {
                    assert_eq!(b.input(0xc0).unwrap(), linear & 63);
                }
                for window in 4..16 {
                    let bank =
                        ((linear as usize & if mapper == 1 { 63 } else { 15 }) << 4) | window;
                    for offset in [0, 1, 65535] {
                        assert_eq!(
                            b.read((window * 65536 + offset) as u32),
                            b.cartridge.as_ref().unwrap().rom(bank, offset)
                        );
                    }
                }
            }
            // All mappings are read-only; the original checksum and bytes survive.
            b.write(0x20000, 0);
            assert_eq!(b.cartridge.as_ref().unwrap().bytes, bytes);
        }
    }
}

#[test]
fn raw_dma_crosses_mapped_banks_with_existing_a0_costs_and_counters() {
    for size in [3145728usize, 6291456] {
        for mapper in [0, 1] {
            for model in [Model::Mono, Model::Color, Model::Crystal] {
                let mut bytes = marked(size, mapper);
                bytes[size - 4] = 4; // fast physical 16-bit ROM
                checksum(&mut bytes);
                let mut b = Bus::new(model);
                b.insert_cartridge(Cartridge::parse(&bytes).unwrap());
                if model != Model::Mono {
                    b.output(0x60, 0x80).unwrap();
                }
                let first = ((size.next_power_of_two() - size) / 65536) as u8;
                b.output(0xc2, first).unwrap();
                b.output(0xc3, first + 1).unwrap();
                if model == Model::Mono {
                    assert!(b.output(0x48, 0x80).is_err());
                    assert!(b.output(0x52, 0x83).is_err());
                    assert_eq!(b.gdma_step(), None);
                    b.tick(1024);
                    assert_eq!(b.sound_dma_stall(), None);
                    continue;
                }
                for (port, value) in [
                    (0x40, 0xfe),
                    (0x41, 0xff),
                    (0x42, 2),
                    (0x44, 0),
                    (0x45, 0x10),
                    (0x46, 4),
                    (0x47, 0),
                    (0x48, 0x80),
                ] {
                    b.output(port, value).unwrap();
                }
                assert_eq!(b.gdma_step(), Some(7));
                assert_eq!(b.gdma_step(), Some(2));
                assert_eq!(b.gdma_step(), None);
                assert_eq!(&b.ram[0x1000..0x1004], &[0x5a, 0xa5, 0x5b, 0xa4]);
                assert_eq!(&b.ports[0x40..0x43], &[2, 0, 3]);
                assert_eq!(&b.ports[0x44..0x48], &[4, 0x10, 0, 0]);
                for a0 in [0, 8, 12] {
                    b.output(0xa0, a0).unwrap();
                    b.output(0x46, 2).unwrap();
                    b.output(0x48, 0x80).unwrap();
                    assert_eq!(b.gdma_step(), None);
                    assert_eq!(b.ports[0x46], 2);
                }
                for (a0, cost) in [(4, 7), (12, 8)] {
                    b.output(0xa0, a0).unwrap();
                    for (port, value) in [
                        (0x4a, 0xfe),
                        (0x4b, 0xff),
                        (0x4c, 2),
                        (0x4e, 4),
                        (0x4f, 0),
                        (0x50, 0),
                        (0x52, 0x83),
                    ] {
                        b.output(port, value).unwrap();
                    }
                    for expected in [0x5a, 0xa5, 0x5b, 0xa4] {
                        let until = (117 + 128 - b.cycles % 128 - 1) % 128 + 1;
                        b.tick(until as u32);
                        assert_eq!(b.ports[0x89], expected);
                        assert_eq!(b.sound_dma_stall(), Some(cost));
                    }
                    assert_eq!(&b.ports[0x4a..0x4d], &[2, 0, 3]);
                    assert_eq!(&b.ports[0x4e..0x51], &[0, 0, 0]);
                    assert_eq!(b.ports[0x52] & 128, 0);
                }
            }
        }
    }
}

#[test]
fn failed_raw_replacements_and_model_changes_preserve_live_machine_and_all_records() {
    for size in [3145728usize, 6291456] {
        for mapper in [0, 1] {
            let mut bytes = marked(size, mapper);
            bytes[size - 9] = 1; // Color-only image
            checksum(&mut bytes);
            let mut m = Machine::new(Model::Color);
            // Distinct records for every console, in addition to cartridge SRAM.
            for model in [Model::Mono, Model::Color, Model::Crystal] {
                m.reset(model);
                let data = vec![model.index() as u8 + 20; m.console_data().len()];
                m.load_console_data(&data).unwrap();
            }
            m.change_model(Model::Color).unwrap();
            m.load_rom(&bytes).unwrap();
            m.load_save(&vec![0x77; 32768]).unwrap();
            m.bus.output(0xa0, 12).unwrap();
            m.bus.ram[0x123] = 0x99;
            m.bus.tick(77);
            m.cpu.registers[4] = 0x3456;
            m.cpu.segments[3] = 0x1234;
            let ports = m.bus.ports;
            let ram = m.bus.ram.clone();
            let records = m.bus.console_data.clone();
            let cpu = format!("{:?}", m.cpu);
            let mut corrupt = bytes.clone();
            corrupt[0] ^= 1;
            for invalid in [&corrupt[..], &bytes[..size - 1], &[0][..]] {
                assert!(m.load_rom(invalid).is_err());
                assert_eq!(m.bus.ports, ports);
                assert_eq!(m.bus.ram, ram);
                assert_eq!(m.bus.cycles, 77);
                assert_eq!(m.bus.console_data, records);
                assert_eq!(format!("{:?}", m.cpu), cpu);
                assert_eq!(m.bus.cartridge.as_ref().unwrap().bytes, bytes);
                assert!(m.save_data().iter().all(|b| *b == 0x77));
            }
            assert!(m.change_model(Model::Mono).is_err());
            assert_eq!(m.model(), Model::Color);
            assert_eq!(m.bus.ports, ports);
            assert_eq!(m.bus.ram, ram);
            assert_eq!(m.bus.cycles, 77);
            assert_eq!(m.bus.console_data, records);
            assert_eq!(format!("{:?}", m.cpu), cpu);
            assert_eq!(m.bus.cartridge.as_ref().unwrap().bytes, bytes);
            assert!(m.save_data().iter().all(|b| *b == 0x77));
            // Mono rejects a valid incompatible replacement before any mutation.
            let mut mono = Machine::new(Model::Mono);
            let running = marked(size, mapper);
            mono.load_rom(&running).unwrap();
            mono.load_save(&vec![0x88; 32768]).unwrap();
            mono.bus.output(0xa0, 12).unwrap();
            mono.cpu.registers[4] = 0x5678;
            mono.cpu.segments[3] = 0x2345;
            let cpu = format!("{:?}", mono.cpu);
            let ports = mono.bus.ports;
            let records = mono.bus.console_data.clone();
            assert!(mono.load_rom(&bytes).is_err());
            assert_eq!(format!("{:?}", mono.cpu), cpu);
            assert_eq!(mono.bus.ports, ports);
            assert_eq!(mono.bus.console_data, records);
            assert_eq!(mono.bus.cartridge.as_ref().unwrap().bytes, running);
            assert!(mono.save_data().iter().all(|b| *b == 0x88));
        }
    }
}
