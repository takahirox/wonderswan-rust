// SPDX-License-Identifier: CC0-1.0
// Execute the original probe through the real footer/mapper/CPU paths.
use ws_core::{Machine, Model};

const PROBES: [(&[u8], bool); 4] = [
    (
        include_bytes!("../../../fixtures/generated/startup-mono-common.ws"),
        false,
    ),
    (
        include_bytes!("../../../fixtures/generated/startup-mono-2003.ws"),
        false,
    ),
    (
        include_bytes!("../../../fixtures/generated/startup-color-common.ws"),
        true,
    ),
    (
        include_bytes!("../../../fixtures/generated/startup-color-2003.ws"),
        true,
    ),
];
const MODELS: [(Model, u16, u16); 3] = [
    (Model::Mono, 0xff00, 0x8d),
    (Model::Color, 0xfe00, 0x8f),
    (Model::Crystal, 0xfe00, 0x8f),
];

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}
fn run(m: &mut Machine, ds: u16, a0: u16, count: u16) {
    m.run_frame().unwrap();
    assert!(m.cpu().halted);
    assert_eq!(word(m.save_data(), 0), count);
    for (save, ram, expected) in [
        (2, 0x400, 0x2000),
        (4, 0x402, ds),
        (6, 0x408, 0x600d),
        (8, 0x404, a0),
        (10, 0x406, 0xf000),
    ] {
        assert_eq!(word(m.save_data(), save), expected);
        assert_eq!(
            u16::from_le_bytes([m.read_memory(ram), m.read_memory(ram + 1)]),
            expected
        );
    }
    assert_eq!(m.cpu().registers[4], 0x3456);
    assert_eq!(m.cpu().segments[3], 0x1234);
}

#[test]
fn incoming_register_probe_load_restart_replacement_and_all_compatible_transitions() {
    for (rom, color_only) in PROBES {
        for (model, ds, a0) in MODELS {
            if color_only && model == Model::Mono {
                continue;
            }
            let mut m = Machine::new(model);
            m.load_rom(rom).unwrap();
            run(&mut m, ds, a0, 1);
            m.restart();
            run(&mut m, ds, a0, 2);
            m.change_model(model).unwrap();
            run(&mut m, ds, a0, 3);
            // Replace a running cartridge with each compatible header/mapper
            // on every source model, after the CPU dirtied its incoming values.
            for (replacement, color_only) in PROBES {
                if color_only && model == Model::Mono {
                    continue;
                }
                m.load_rom(replacement).unwrap();
                run(&mut m, ds, a0, 1);
            }
            m.load_rom(rom).unwrap();
            run(&mut m, ds, a0, 1);
            let mut count = 1;
            for (next, next_ds, next_a0) in MODELS {
                if color_only && next == Model::Mono {
                    continue;
                }
                m.change_model(model).unwrap();
                count += 1;
                run(&mut m, ds, a0, count);
                m.change_model(next).unwrap();
                count += 1;
                run(&mut m, next_ds, next_a0, count);
            }
        }
    }
}
