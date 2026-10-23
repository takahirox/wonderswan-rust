// SPDX-License-Identifier: CC0-1.0
use ws_core::{Button, Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/lcd.ws");
fn word(m: &Machine, a: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(a), m.read_memory(a + 1)])
}
fn frames(m: &mut Machine, n: usize) {
    for _ in 0..n {
        m.run_frame().unwrap();
    }
}
#[test]
fn original_lcd_cartridge_checks_word_byte_io_raster_irqs_runtime_and_restart() {
    let mut m = Machine::default();
    m.load_rom(ROM).unwrap();
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.change_model(model).unwrap();
        for _ in 0..2 {
            frames(&mut m, 7);
            assert_eq!(word(&m, 0x400), 0x600d, "{model:?}");
            assert_eq!(word(&m, 0x402), 157);
            assert_eq!(word(&m, 0x404), 181);
            assert_eq!(m.save_data()[10], 0xf0);
            assert_eq!(m.frame_rate(), 3_072_000.0 / 46336.0);
            assert_eq!(
                &m.framebuffer()[..4],
                if model == Model::Mono {
                    &[170, 170, 170, 255]
                } else {
                    &[0, 255, 0, 255]
                }
            );
            let frame = m.frame_number();
            m.set_buttons(Button::Right as u16);
            frames(&mut m, 2);
            assert_eq!(m.frame_number(), frame + 2);
            assert_eq!(word(&m, 0x406), 156);
            assert_eq!(word(&m, 0x408), 1);
            assert_eq!(m.frame_rate(), 3_072_000.0 / 40192.0);
            m.set_buttons(0);
            frames(&mut m, 2);
            assert_eq!(word(&m, 0x406), 180);
            assert_eq!(word(&m, 0x408), 2);
            assert_eq!(&m.save_data()[..8], &[0x0d, 0x60, 157, 0, 181, 0, 180, 0]);
            m.restart();
            assert_eq!(m.frame_number(), 0);
            assert_eq!(m.frame_rate(), ws_core::FRAME_RATE);
        }
    }
}
