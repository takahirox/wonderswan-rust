// SPDX-License-Identifier: CC0-1.0
// Independent literal pixels for the documented line-boundary policies.
use super::*;

const MODELS: [Model; 3] = [Model::Mono, Model::Color, Model::Crystal];
fn out(m: &mut Machine, port: u16, value: u8) {
    m.bus.output(port, value).unwrap();
}
fn pixel(m: &Machine, x: usize, y: usize) -> [u8; 4] {
    m.framebuffer[(y * FRAME_WIDTH + x) * 4..(y * FRAME_WIDTH + x + 1) * 4]
        .try_into()
        .unwrap()
}
fn palette(m: &mut Machine, p: usize, i: usize, value: u16) {
    let a = 0xfe00 + p * 32 + i * 2;
    m.bus.ram[a..a + 2].copy_from_slice(&value.to_le_bytes());
}
fn entry(m: &mut Machine, a: usize, value: u16) {
    m.bus.ram[a..a + 2].copy_from_slice(&value.to_le_bytes());
}
fn tile(m: &mut Machine, mode: u8, n: usize, index: u8, mask: u8) {
    let four = mode & 0x40 != 0;
    let stride = if four { 4 } else { 2 };
    let base = if four { 0x4000 } else { 0x2000 };
    for y in 0..8 {
        for x in 0..8 {
            let i = if mask & (128 >> x) != 0 { index } else { 0 };
            if mode == 0xe0 {
                m.bus.ram[base + n * 32 + y * 4 + x / 2] |= i << ((1 - (x & 1)) * 4);
            } else {
                for p in 0..stride {
                    m.bus.ram[base + n * stride * 8 + y * stride + p] |= ((i >> p) & 1) << (7 - x);
                }
            }
        }
    }
}
fn scene(model: Model, mode: u8) -> Machine {
    let mut m = Machine::new(model);
    if model != Model::Mono {
        out(&mut m, 0x60, mode);
    }
    out(&mut m, 0x14, 1);
    out(&mut m, 0, 7);
    out(&mut m, 7, 0x21);
    out(&mut m, 1, if mode == 0 { 5 } else { 0x15 });
    tile(&mut m, mode, 0, 1, 255);
    tile(&mut m, mode, 1, 2, 0xf0); // Screen 2 transparent over right half.
    tile(&mut m, mode, 2, 3, 255);
    for n in 0..32 {
        entry(&mut m, 0x800 + n * 2, if n < 4 { 0 } else { 3 | (4 << 9) });
        entry(&mut m, 0x1000 + n * 2, 1 | (4 << 9));
    }
    out(&mut m, 6, 3);
    for (n, p, x, priority) in [(0, 4, 8, 0), (1, 5, 8, 0x2000), (2, 6, 16, 0x2000)] {
        let attr: u16 = 2 | (p << 9) | priority;
        m.bus.sprites[n * 4..n * 4 + 2].copy_from_slice(&attr.to_le_bytes());
        m.bus.sprites[n * 4 + 3] = x;
    }
    // Mono palette -> shade selection belongs to composition, not translation.
    out(&mut m, 0x20, 0x10);
    out(&mut m, 0x29, 0x02);
    out(&mut m, 0x39, 0x30);
    out(&mut m, 0x3b, 0x60);
    out(&mut m, 0x3d, 0x40);
    m.framebuffer.fill(42);
    m
}
fn final_palettes(m: &mut Machine, mode: u8) {
    if mode == 0 {
        for (p, v) in [(0x1c, 0xf0), (0x1d, 0x85), (0x1e, 0xca), (0x1f, 0)] {
            out(m, p, v);
        }
    } else {
        for (p, i, rgb) in [
            (0, 1, 0xf00),
            (4, 2, 0x0f0),
            (12, 3, 0x00f),
            (13, 3, 0xff0),
            (14, 3, 0xa53),
            (1, 5, 0x654),
        ] {
            palette(m, p, i, rgb);
        }
        palette(m, 6, 7, 0x789);
    }
}

#[test]
fn delayed_identities_preserve_layers_transparency_sprites_backdrop_and_palette_stage() {
    for model in MODELS {
        for mode in if model == Model::Mono {
            &[0][..]
        } else {
            &[0, 0x80, 0xc0, 0xe0][..]
        } {
            let mut m = scene(model, *mode);
            m.advance(256); // compose row 0; no RGBA output yet
            assert_eq!(pixel(&m, 0, 0), [42; 4]);
            // Destroy all composition inputs, including mono palette mappings.
            // A previous-row refetch, RGBA shift or late palette-port lookup fails.
            m.bus.ram[..0xfe00].fill(0);
            m.bus.sprites.fill(0xff);
            m.bus.ports[0x20..0x40].fill(0);
            out(&mut m, 0, 0);
            out(&mut m, 1, if *mode == 0 { 6 } else { 0x67 });
            final_palettes(&mut m, *mode);
            m.advance(256); // translate row 0, compose row 1
            let mono = [
                [170, 170, 170, 255],
                [0, 0, 0, 255],
                [170, 170, 170, 255],
                [119, 119, 119, 255],
                [85, 85, 85, 255],
                [51, 51, 51, 255],
            ];
            let color = [
                [0, 255, 0, 255],
                [255, 0, 0, 255],
                [0, 255, 0, 255],
                [0, 0, 255, 255],
                [170, 85, 51, 255],
                [102, 85, 68, 255],
            ];
            for (i, x) in [0, 4, 8, 12, 16, 36].into_iter().enumerate() {
                let expected = if *mode == 0 { mono[i] } else { color[i] };
                assert_eq!(pixel(&m, x, 0), expected, "{model:?} mode {mode:02x} x={x}");
                assert_eq!(pixel(&m, x, 1), [42; 4]);
            }
            m.advance(256);
            assert_eq!(
                pixel(&m, 0, 1),
                if *mode == 0 {
                    [255; 4]
                } else {
                    [119, 136, 153, 255]
                }
            );
            assert_eq!(pixel(&m, 0, 2), [42; 4]);
            // Translation cannot revisit a completed row.
            m.bus.ports[0x1c..0x20].fill(0);
            m.bus.ram[0xfe00..].fill(0);
            m.advance(256);
            assert_eq!(
                pixel(&m, 4, 0),
                if *mode == 0 {
                    [0, 0, 0, 255]
                } else {
                    [255, 0, 0, 255]
                }
            );
        }
    }
}

#[test]
fn lcd_enable_and_runtime_mode_have_explicit_translation_policies() {
    for model in MODELS {
        let mut m = scene(model, 0);
        out(&mut m, 0x14, 0);
        m.advance(256); // composition continues with LCD disabled
        final_palettes(&mut m, 0);
        if model != Model::Mono {
            out(&mut m, 0x60, 0xe0);
        }
        out(&mut m, 0x14, 1);
        m.advance(256);
        assert_eq!(pixel(&m, 4, 0), [0, 0, 0, 255]); // latched mono identity
        out(&mut m, 0x14, 0);
        m.advance(256);
        assert_eq!(pixel(&m, 4, 1), [255; 4]); // translation-time gate
    }
    for model in [Model::Color, Model::Crystal] {
        for mode in [0x80, 0xc0, 0xe0] {
            let mut m = scene(model, mode);
            m.advance(256);
            out(&mut m, 0x60, 0);
            final_palettes(&mut m, mode); // retained internal palette RAM in mono mode
            m.advance(256);
            assert_eq!(pixel(&m, 4, 0), [255, 0, 0, 255]);
            m.bus.ports[0x1c..0x20].fill(0xff);
            m.advance(256);
            assert_eq!(pixel(&m, 4, 1), [0, 0, 0, 255]); // new mono composition
        }
    }
}

#[test]
fn last_row_translates_during_144_without_moving_vblank_oam_irq_timers_or_clocks() {
    for model in MODELS {
        for total in [156, 158, 180] {
            let mut m = Machine::new(model);
            out(&mut m, 0x16, total);
            out(&mut m, 0x14, 1);
            out(&mut m, 0x1c, 15);
            out(&mut m, 0xb2, 0xf0);
            out(&mut m, 3, 144);
            out(&mut m, 4, 12);
            out(&mut m, 0xa4, 1);
            out(&mut m, 0xa6, 1);
            out(&mut m, 0xa2, 15);
            m.bus.ram[0x1800] = 0x71;
            m.advance(144 * 256);
            assert_eq!(
                (m.bus.cycles, m.bus.raster_line, m.frame_number()),
                (36864, 144, 0)
            );
            assert_eq!(m.bus.input(0xb4).unwrap(), 0xf0);
            assert_eq!(m.bus.sprites[0], 0x71);
            assert_eq!(pixel(&m, 0, 142), [0, 0, 0, 255]);
            assert_eq!(pixel(&m, 0, 143), [255; 4]);
            m.bus.ram[0x1800] = 0x99;
            out(&mut m, 0x1c, 5);
            m.advance(255);
            assert_eq!(pixel(&m, 0, 143), [255; 4]);
            m.advance(1);
            assert_eq!(pixel(&m, 0, 143), [170, 170, 170, 255]);
            assert_eq!(m.bus.sprites[0], 0x71);
            m.advance((u32::from(total) - 144) * 256);
            assert_eq!(
                (m.frame_number(), m.bus.raster_line, m.bus.cycles),
                (1, 0, (u64::from(total) + 1) * 256)
            );
            m.advance(256); // no last-frame row translated at line zero
            assert_eq!(pixel(&m, 0, 143), [170, 170, 170, 255]);
            m.advance(u32::from(total) * 256);
            assert_eq!(m.frame_number(), 2);
            assert!(m
                .framebuffer
                .chunks_exact(4)
                .all(|p| p == [170, 170, 170, 255]));
            assert_eq!(m.bus.sprites[0], 0x99);
        }
    }
}

#[test]
fn short_odd_and_runtime_totals_drop_only_rows_without_a_successor() {
    for model in MODELS {
        for total in [0, 1, 2, 142, 143, 144, 145, 155, 157, 255] {
            let mut m = Machine::new(model);
            out(&mut m, 0x14, 1);
            out(&mut m, 0x1c, 15);
            out(&mut m, 0x16, total);
            for frame in 1..=2 {
                m.advance((u32::from(total) + 1) * 256);
                assert_eq!(m.frame_number(), frame);
                for y in 0..144 {
                    assert_eq!(
                        pixel(&m, 0, y),
                        if y < usize::from(total) {
                            [0, 0, 0, 255]
                        } else {
                            [255; 4]
                        },
                        "T={total} y={y}"
                    );
                }
            }
        }
        let mut m = Machine::new(model);
        out(&mut m, 0x14, 1);
        out(&mut m, 0x1c, 15);
        m.advance(11 * 256); // pending row 10
        out(&mut m, 0x16, 1); // shortening below current line
        m.advance(256); // translate 10; drop composed 11 on actual wrap
        assert_eq!(m.frame_number(), 1);
        assert_eq!(pixel(&m, 0, 10), [0, 0, 0, 255]);
        assert_eq!(pixel(&m, 0, 11), [255; 4]);
        out(&mut m, 0x1c, 5);
        out(&mut m, 0x16, 0);
        m.advance(256); // must not translate dropped 11 or current 0
        assert_eq!(pixel(&m, 0, 11), [255; 4]);
        assert_eq!(pixel(&m, 0, 0), [0, 0, 0, 255]);
        out(&mut m, 0x16, 1);
        m.advance(256); // pending row 0
        out(&mut m, 0x16, 144); // extension lets it translate
        m.advance(144 * 256);
        assert_eq!(m.frame_number(), 3);
        assert!(m
            .framebuffer
            .chunks_exact(4)
            .all(|p| p == [170, 170, 170, 255]));
    }
}

fn pending_machine(model: Model) -> Machine {
    let mut m = Machine::new(model);
    m.load_rom(&cartridge::tests::rom(131072, 2, 0)).unwrap();
    out(&mut m, 0x14, 1);
    out(&mut m, 0x1c, 15);
    m.advance(256);
    m
}
fn cleared(m: &mut Machine) {
    assert_eq!((m.frame_number(), m.bus.cycles), (0, 0));
    m.advance(256);
    assert!(m.framebuffer.chunks_exact(4).all(|p| p == [255; 4]));
}
#[test]
fn lifecycle_clears_pending_rows_and_rejections_preserve_them() {
    for model in MODELS {
        let rom = cartridge::tests::rom(131072, 2, 0);
        let mut m = pending_machine(model);
        m.reset(model);
        cleared(&mut m);
        let mut m = pending_machine(model);
        m.restart();
        cleared(&mut m);
        let mut m = pending_machine(model);
        m.load_rom(&rom).unwrap();
        cleared(&mut m);
        let mut m = pending_machine(model);
        m.change_model(if model == Model::Crystal {
            Model::Mono
        } else {
            Model::Crystal
        })
        .unwrap();
        cleared(&mut m);
        let mut m = pending_machine(model);
        assert!(m.load_rom(&[1, 2, 3]).is_err());
        assert_eq!(m.bus.cycles, 256);
        m.advance(256);
        assert_eq!(pixel(&m, 0, 0), [0, 0, 0, 255]);
    }
    let mut m = Machine::new(Model::Color);
    let mut rom = cartridge::tests::rom(131072, 2, 0);
    let end = rom.len();
    rom[end - 9] = 1;
    cartridge::tests::checksum(&mut rom);
    m.load_rom(&rom).unwrap();
    out(&mut m, 0x14, 1);
    out(&mut m, 0x1c, 15);
    m.advance(256);
    assert!(m.change_model(Model::Mono).is_err());
    m.advance(256);
    assert_eq!(pixel(&m, 0, 0), [0, 0, 0, 255]);
    let mut m = pending_machine(Model::Mono);
    assert!(m.load_rom(&rom).is_err());
    m.advance(256);
    assert_eq!(pixel(&m, 0, 0), [0, 0, 0, 255]);
}

#[test]
fn vblank_oam_latch_cannot_replace_pending_last_row_sprite_identity() {
    for model in MODELS {
        for mode in if model == Model::Mono {
            &[0][..]
        } else {
            &[0, 0x80, 0xc0, 0xe0][..]
        } {
            let mut m = scene(model, *mode);
            // Move the previously latched high-priority sprite to the last row.
            for i in 0..3 {
                m.bus.sprites[i * 4 + 2] = 143;
            }
            out(&mut m, 4, 12);
            let replacement: u16 = 2 | (5 << 9) | 0x2000;
            m.bus.ram[0x1800..0x1802].copy_from_slice(&replacement.to_le_bytes());
            m.bus.ram[0x1802..0x1804].copy_from_slice(&[143, 16]);
            m.advance(144 * 256);
            assert_eq!(&m.bus.sprites[..4], &m.bus.ram[0x1800..0x1804]);
            assert_eq!(pixel(&m, 16, 143), [42; 4]);
            final_palettes(&mut m, *mode);
            m.advance(256);
            // Old high sprite selected palette14/index3, not replacement13/index3.
            assert_eq!(
                pixel(&m, 16, 143),
                if *mode == 0 {
                    [85, 85, 85, 255]
                } else {
                    [170, 85, 51, 255]
                }
            );
            m.advance(14 * 256); // finish default frame
            m.advance(145 * 256); // next frame uses the newly latched palette13
            assert_eq!(
                pixel(&m, 16, 143),
                if *mode == 0 {
                    [255; 4]
                } else {
                    [255, 255, 0, 255]
                }
            );
        }
    }
}
