// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model, FRAME_WIDTH};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/lcd-output.ws");
fn pixel(m: &Machine, x: usize, y: usize) -> [u8; 4] {
    m.framebuffer()[(y * FRAME_WIDTH + x) * 4..(y * FRAME_WIDTH + x + 1) * 4]
        .try_into()
        .unwrap()
}
#[test]
fn original_output_probe_delays_literal_rows_and_restarts_with_fresh_cpu_markers() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        for boot in 1..=2 {
            for _ in 0..5 {
                m.run_frame().unwrap();
            }
            assert_eq!(m.frame_number(), 5);
            for (offset, expected) in [(0, boot), (4, 40), (6, 144), (8, 0x600d)] {
                let actual = u16::from_le_bytes([
                    m.read_memory(0x400 + offset),
                    m.read_memory(0x401 + offset),
                ]);
                assert_eq!(actual, expected, "{model:?} RAM {offset}");
                assert_eq!(
                    u16::from_le_bytes([
                        m.save_data()[offset as usize],
                        m.save_data()[offset as usize + 1]
                    ]),
                    expected
                );
            }
            assert!(u16::from_le_bytes([m.save_data()[2], m.save_data()[3]]) >= 3);
            // Independent literals: row39 uses line40 palettes with row39 tile
            // identity; row143 uses line144 palettes after the VBlank OAM latch.
            for (x, y, mono, color) in [
                (0, 0, 255, [0, 255, 0, 255]),
                (4, 0, 255, [255, 0, 0, 255]),
                (0, 38, 255, [0, 255, 0, 255]),
                (4, 38, 255, [255, 0, 0, 255]),
                (0, 39, 170, [255, 0, 255, 255]),
                (4, 39, 0, [255, 255, 0, 255]),
                (4, 40, 0, [255, 255, 0, 255]),
                (16, 40, 85, [0, 255, 255, 255]),
                (4, 46, 0, [255, 255, 0, 255]),
                (4, 47, 85, [0, 0, 255, 255]),
                (16, 47, 85, [0, 255, 255, 255]),
                (20, 48, 0, [255, 255, 0, 255]),
                (4, 142, 0, [255, 255, 0, 255]),
                (0, 143, 153, [68, 85, 102, 255]),
                (4, 143, 119, [170, 85, 51, 255]),
            ] {
                assert_eq!(
                    pixel(&m, x, y),
                    if model == Model::Mono {
                        [mono, mono, mono, 255]
                    } else {
                        color
                    },
                    "{model:?} boot{boot} ({x},{y})"
                );
            }
            // Compare complete adjacent output rows, including every tile and
            // all eight overlapping sprite pixels, against literal colors.
            for (y, fg, bg, mono_fg, mono_bg) in [
                (0, [0, 255, 0, 255], [255, 0, 0, 255], 255, 255),
                (38, [0, 255, 0, 255], [255, 0, 0, 255], 255, 255),
                (39, [255, 0, 255, 255], [255, 255, 0, 255], 170, 0),
                (40, [255, 0, 255, 255], [255, 255, 0, 255], 170, 0),
                (46, [255, 0, 255, 255], [255, 255, 0, 255], 170, 0),
                (47, [255, 0, 255, 255], [0, 0, 255, 255], 170, 85),
                (48, [255, 0, 255, 255], [255, 255, 0, 255], 170, 0),
                (142, [255, 0, 255, 255], [255, 255, 0, 255], 170, 0),
                (143, [68, 85, 102, 255], [170, 85, 51, 255], 153, 119),
            ] {
                for x in 0..FRAME_WIDTH {
                    let sprite = (40..48).contains(&y) && (16..24).contains(&x);
                    let expected = if model == Model::Mono {
                        let v = if sprite {
                            85
                        } else if x % 8 < 4 {
                            mono_fg
                        } else {
                            mono_bg
                        };
                        [v, v, v, 255]
                    } else if sprite {
                        [0, 255, 255, 255]
                    } else if x % 8 < 4 {
                        fg
                    } else {
                        bg
                    };
                    assert_eq!(
                        pixel(&m, x, y),
                        expected,
                        "{model:?} boot{boot} row {y} x{x}"
                    );
                }
            }
            m.restart();
            assert_eq!(m.frame_number(), 0);
            assert!(m.framebuffer().chunks_exact(4).all(|p| p == [255; 4]));
        }
    }
}
