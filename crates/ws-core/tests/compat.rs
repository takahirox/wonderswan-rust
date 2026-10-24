// SPDX-License-Identifier: CC0-1.0
use ws_core::{Button, Machine, Model};
const PROBES: [&[u8]; 5] = [
    include_bytes!("../../../fixtures/generated/sram32-common.ws"),
    include_bytes!("../../../fixtures/generated/sram128-2003.ws"),
    include_bytes!("../../../fixtures/generated/eeprom128.ws"),
    include_bytes!("../../../fixtures/generated/eeprom1024.ws"),
    include_bytes!("../../../fixtures/generated/eeprom2048-packed.ws"),
];
fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame().unwrap();
    }
}
fn word(m: &Machine, a: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(a), m.read_memory(a + 1)])
}
fn pixel(m: &Machine, x: usize, y: usize) -> &[u8] {
    &m.framebuffer()[(y * 224 + x) * 4..(y * 224 + x) * 4 + 4]
}
#[test]
fn original_cartridges_execute_mapper_save_cpu_interrupt_input_video_and_audio_paths() {
    for (i, rom) in PROBES.iter().enumerate() {
        for model in [Model::Mono, Model::Color, Model::Crystal] {
            let mut m = Machine::new(model);
            m.load_rom(rom).unwrap();
            m.set_rtc_present(false).unwrap(); // Preserve the original absent-chip probe.
            frames(&mut m, 4);
            assert_eq!(m.frame_number(), 4);
            assert_eq!(word(&m, 0x400), 1, "probe {i}");
            assert_eq!(word(&m, 0x406), 10);
            assert_eq!(word(&m, 0x40e), 0x1234);
            assert_eq!(m.read_memory(0x408), 0x31);
            assert!(word(&m, 0x402) > 10);
            if i == 1 || i >= 3 {
                assert_eq!(m.read_memory(0x40a), 5);
                assert_eq!(m.read_memory(0x40b), 0x31);
                assert_eq!(m.read_memory(0x40c), 255);
            }
            m.set_buttons(Button::Right as u16);
            frames(&mut m, 1);
            assert_eq!(m.read_memory(0x404), 0x22);
            if model != Model::Mono {
                assert_eq!(pixel(&m, 0, 0), [0, 255, 0, 255]);
                assert_eq!(pixel(&m, 9, 9), [255, 0, 0, 255]);
                assert_eq!(pixel(&m, 16, 16), [0, 0, 255, 255]);
            }
            let samples = m.audio_samples();
            assert!(!samples.is_empty());
            assert!(samples.iter().any(|s| *s > 0.01));
            let save = m.save_data().to_vec();
            assert_eq!(save[0], 1);
            if i == 1 {
                assert_eq!(save[65536], 65);
            }
            if i == 0 {
                assert_eq!(m.read_memory(0x40d), 1);
            }
            m.restart();
            assert_eq!(m.save_data(), save);
            frames(&mut m, 4);
            assert_eq!(m.save_data()[0], 2);
            let mut reopened = Machine::new(model);
            reopened.load_rom(rom).unwrap();
            reopened.set_rtc_present(false).unwrap();
            reopened.load_save(m.save_data()).unwrap();
            frames(&mut reopened, 4);
            assert_eq!(reopened.save_data()[0], 3);
            let saved = reopened.save_data().to_vec();
            assert!(reopened.load_save(&[1, 2, 3]).is_err());
            assert_eq!(reopened.save_data(), saved);
            reopened.change_model(Model::Crystal).unwrap();
            assert_eq!(reopened.save_data(), saved);
            frames(&mut reopened, 4);
            assert_eq!(reopened.save_data()[0], 4);
        }
    }
}
