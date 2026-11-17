use super::*;
use crate::{Model, FRAME_BYTES};

fn paint(
    bus: &Bus,
    cached: &mut Pipeline,
    reference: &mut Pipeline,
    actual: &mut [u8],
    expected: &mut [u8],
) {
    reference.invalidate_cache();
    for line in 0..=144 {
        cached.line_end(bus, actual, line);
        reference.line_end(bus, expected, line);
    }
    assert_eq!(actual, expected);
    cached.frame_wrap();
    reference.frame_wrap();
}

#[test]
fn cached_rows_follow_cpu_dma_sprite_palette_and_mode_changes() {
    for model in [Model::Mono, Model::Crystal] {
        let mut b = Bus::new(model);
        if model != Model::Mono {
            b.output(0x60, 0x80).unwrap();
        }
        for (p, v) in [
            (0, 7),
            (7, 0x21),
            (6, 1),
            (4, 0x18),
            (0x14, 1),
            (0x20, 0x10),
            (0x21, 0x32),
            (0x1c, 0xf0),
            (0x1d, 0x87),
        ] {
            b.output(p, v).unwrap();
        }
        b.ram[0x2000..0x2100].fill(0xaa);
        b.ram[0x4000..0x4200].fill(0x55);
        for i in 0..256 {
            b.ram[0xfe00 + i * 2..0xfe02 + i * 2].copy_from_slice(&((i * 17) as u16).to_le_bytes());
        }
        let mut cached = Pipeline::default();
        let mut reference = Pipeline::default();
        let mut actual = vec![42; FRAME_BYTES];
        let mut expected = actual.clone();
        paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        for a in [
            0x800, 0x801, 0x1000, 0x1001, 0x2000, 0x2001, 0x4000, 0x8000, 0xfe00, 0xfe1e,
        ] {
            b.write(a, b.read(a) ^ 0x80);
            paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        }
        for (p, v) in [
            (0, 1),
            (0x10, 7),
            (0x11, 255),
            (0x12, 3),
            (0x13, 7),
            (7, 0x11),
            (0, 0x37),
            (8, 7),
            (9, 2),
            (10, 200),
            (11, 120),
            (5, 127),
            (6, 128),
            (0x20, 0x32),
            (0x1c, 0x0f),
            (0x14, 0),
            (0x14, 1),
            (1, 7),
            (0, 7),
        ] {
            b.output(p, v).unwrap();
            paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        }
        if model != Model::Mono {
            for mode in [0xc0, 0xe0, 0x80] {
                b.output(0x60, mode).unwrap();
                b.write(0x4000, b.read(0x4000) ^ 0xff);
                paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
            }
            // Source setup is already cached before the DMA changes a tile.
            b.write(0x3000, 0xff);
            b.write(0x3001, 0);
            for (p, v) in [
                (0x40, 0),
                (0x41, 0x30),
                (0x44, 0),
                (0x45, 0x20),
                (0x46, 2),
                (0x48, 0x80),
            ] {
                b.output(p, v).unwrap();
            }
            paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
            assert_eq!(b.gdma_step(), Some(7));
            paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        }
        // OAM changes become visible at VBlank, independently of the source write.
        let source = 0x3000;
        b.write(source, 1);
        b.write(source + 1, 0x20);
        b.write(source + 2, 0);
        b.write(source + 3, 0);
        b.output(5, 0).unwrap();
        b.output(6, 1).unwrap();
        paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        b.tick(144 * 256);
        paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
        // A palette write AFTER composition must affect translation of that row.
        reference.invalidate_cache();
        cached.line_end(&b, &mut actual, 0);
        reference.line_end(&b, &mut expected, 0);
        b.output(0x1d, 0xf0).unwrap();
        b.write(0xfe1e, 0x0f);
        cached.line_end(&b, &mut actual, 1);
        reference.line_end(&b, &mut expected, 1);
        assert_eq!(&actual[..FRAME_WIDTH * 4], &expected[..FRAME_WIDTH * 4]);
    }
}

#[test]
fn map_row_invalidation_wraps_scroll_and_covers_overlapping_maps() {
    let mut b = Bus::new(Model::Mono);
    b.output(0, 1).unwrap();
    b.output(7, 1).unwrap();
    b.output(0x11, 255).unwrap();
    let before = b.video_row_generations;
    b.write(0x800 + 31 * 64, 1);
    let changed: Vec<_> = (0..144)
        .filter(|&y| before[y] != b.video_row_generations[y])
        .collect();
    assert_eq!(changed, [0]);
    let before = b.video_row_generations;
    b.write(0x800, 1);
    let changed: Vec<_> = (0..144)
        .filter(|&y| before[y] != b.video_row_generations[y])
        .collect();
    assert_eq!(changed, (1..=8).collect::<Vec<_>>());
    b.output(0, 3).unwrap();
    b.output(7, 0x11).unwrap();
    b.output(0x11, 0).unwrap();
    b.output(0x13, 255).unwrap();
    let before = b.video_row_generations;
    b.write(0x800, 2);
    let changed: Vec<_> = (0..144)
        .filter(|&y| before[y] != b.video_row_generations[y])
        .collect();
    assert_eq!(changed, (0..=8).collect::<Vec<_>>());
}

#[test]
fn unused_tile_ram_can_later_be_selected_and_remains_write_tracked() {
    let mut b = Bus::new(Model::Mono);
    for (p, v) in [(0, 1), (7, 1), (0x14, 1), (0x20, 0x10), (0x1c, 0xf0)] {
        b.output(p, v).unwrap();
    }
    let mut cached = Pipeline::default();
    let mut reference = Pipeline::default();
    let mut actual = vec![42; FRAME_BYTES];
    let mut expected = actual.clone();
    paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
    assert_eq!(&actual[..4], &[255; 4]);
    let generation = b.video_generation;
    b.write(0x3ef0, 0xff); // Unused tile 495, also a plausible stack location.
    assert_eq!(b.video_generation, generation);
    paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
    assert_eq!(&actual[..4], &[255; 4]);
    b.write(0x800, 0xef);
    b.write(0x801, 1);
    paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
    assert_eq!(&actual[..4], &[0, 0, 0, 255]);
    b.write(0x3ef0, 0);
    assert!(b.video_generation > generation);
    paint(&b, &mut cached, &mut reference, &mut actual, &mut expected);
    assert_eq!(&actual[..4], &[255; 4]);
}
