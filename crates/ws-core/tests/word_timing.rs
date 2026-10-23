// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};

const ROM: &[u8] = include_bytes!("../../../fixtures/generated/word-timing.ws");

#[test]
fn original_relative_word_timing_probe_all_models_and_fresh_restart() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        for boot in 1..=2 {
            for _ in 0..20 {
                m.run_frame().unwrap();
            }
            let ram = |offset: u32| {
                u16::from_le_bytes([m.read_memory(0x400 + offset), m.read_memory(0x401 + offset)])
            };
            assert_eq!(ram(0), boot, "{model:?} fresh boot");
            assert_eq!(ram(2), 0x600d, "{model:?} cartridge self-check");
            for group in [4, 10, 16, 22] {
                let byte = ram(group) as i32;
                let even = ram(group + 2) as i32;
                let odd = ram(group + 4) as i32;
                assert!(even > 0, "calibrated loop actually ran");
                assert!(
                    (byte - even).abs() <= 2,
                    "{model:?} group{group}: byte {byte}, even {even}"
                );
                assert!(
                    (14..=18).contains(&(odd - even)),
                    "{model:?} group{group}: even {even}, odd {odd}"
                );
            }
            for offset in (0..28).step_by(2) {
                assert_eq!(
                    ram(offset),
                    u16::from_le_bytes([
                        m.save_data()[offset as usize],
                        m.save_data()[offset as usize + 1]
                    ])
                );
            }
            m.restart();
            assert_eq!(m.read_memory(0x400), 0, "volatile markers cleared");
        }
    }
}
