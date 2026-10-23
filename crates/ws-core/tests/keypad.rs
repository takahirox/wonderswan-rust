// SPDX-License-Identifier: CC0-1.0
use ws_core::{Button, Machine, Model, Orientation};

const HORIZONTAL: &[u8] = include_bytes!("../../../fixtures/generated/keypad-horizontal.ws");
const VERTICAL: &[u8] = include_bytes!("../../../fixtures/generated/keypad-vertical.ws");

fn sample(m: &mut Machine, mask: u16, y: u8, x: u8, actions: u8) {
    m.set_buttons(mask);
    for _ in 0..2 {
        m.run_frame().unwrap();
    }
    // Literal expectations for every select, independent of the core bit shifts.
    let expected = [
        0,
        0x10 | y,
        0x20 | x,
        0x30 | y | x,
        0x40 | actions,
        0x50 | y | actions,
        0x60 | x | actions,
        0x70 | y | x | actions,
    ];
    for (row, value) in expected.into_iter().enumerate() {
        assert_eq!(m.read_memory(0x400 + row as u32), value);
        assert_eq!(m.save_data()[row], value);
        for bit in 0..8 {
            let offset = ((16 + row * 8 + 3) * 224 + bit * 8 + 3) * 4;
            let shade = if value & (1 << bit) == 0 { 255 } else { 0 };
            assert_eq!(
                &m.framebuffer()[offset..offset + 4],
                &[shade, shade, shade, 255]
            );
        }
    }
}

#[test]
fn cpu_reads_each_button_mixed_rows_release_and_native_markers_on_every_model() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for (rom, orientation) in [
            (HORIZONTAL, Orientation::Horizontal),
            (VERTICAL, Orientation::Vertical),
        ] {
            let mut m = Machine::new(model);
            m.load_rom(rom).unwrap();
            assert_eq!(m.rom_len(), rom.len());
            assert_eq!(m.cartridge_header().unwrap().orientation, orientation);
            sample(&mut m, 0, 0, 0, 0);
            for (button, y, x, action) in [
                (Button::Y1, 1, 0, 0),
                (Button::Y2, 2, 0, 0),
                (Button::Y3, 4, 0, 0),
                (Button::Y4, 8, 0, 0),
                (Button::Up, 0, 1, 0),
                (Button::Right, 0, 2, 0),
                (Button::Down, 0, 4, 0),
                (Button::Left, 0, 8, 0),
                (Button::Start, 0, 0, 2),
                (Button::A, 0, 0, 4),
                (Button::B, 0, 0, 8),
            ] {
                sample(&mut m, button as u16, y, x, action);
                sample(&mut m, 0, 0, 0, 0);
            }
            sample(&mut m, 0xffe, 15, 15, 14);
            sample(&mut m, 0x596, 5, 9, 6); // X1+X4, Y1+Y3, A+Start
            sample(&mut m, 0, 0, 0, 0);
            for (x, y, shade) in [
                (3, 3, 0),
                (219, 3, 170),
                (3, 139, 85),
                (222, 143, 255),
                (223, 142, 255),
                (223, 143, 0),
            ] {
                let offset = (y * 224 + x) * 4;
                assert_eq!(
                    &m.framebuffer()[offset..offset + 4],
                    &[shade, shade, shade, 255]
                );
            }
            m.set_buttons(0xffe);
            m.restart();
            sample(&mut m, 0, 0, 0, 0);
            assert_eq!(m.cartridge_header().unwrap().orientation, orientation);
            assert!(m.load_rom(&[0]).is_err());
            assert_eq!(m.cartridge_header().unwrap().orientation, orientation);
        }
    }
}
