use crate::{bus::Bus, FRAME_HEIGHT, FRAME_WIDTH};

fn word(bus: &Bus, address: usize) -> u16 {
    u16::from_le_bytes([bus.ram[address & 65535], bus.ram[(address + 1) & 65535]])
}
#[cfg(test)]
fn tile_pixel(bus: &Bus, entry: u16, x: usize, y: usize, sprite: bool) -> Option<(usize, u8)> {
    let color = bus.color_mode();
    let four = color && bus.ports[0x60] & 64 != 0;
    let tile = (entry as usize & 511)
        + if color && !sprite && entry & 0x2000 != 0 {
            512
        } else {
            0
        };
    let x = if entry & 0x4000 != 0 { 7 - x } else { x };
    let y = if entry & 0x8000 != 0 { 7 - y } else { y };
    let planes = if four { 4 } else { 2 };
    let address = (if four { 0x4000 } else { 0x2000 }) + tile * planes * 8 + y * planes;
    let index = if four && bus.ports[0x60] & 32 != 0 {
        (bus.ram[address + x / 2] >> ((1 - (x & 1)) * 4)) & 15
    } else {
        (0..planes).fold(0, |value, p| {
            value | (((bus.ram[address + p] >> (7 - x)) & 1) << p)
        })
    };
    let palette = if sprite {
        8 + ((entry as usize >> 9) & 7)
    } else {
        (entry as usize >> 9) & 15
    };
    if index == 0 && (four || palette & 4 != 0) {
        None
    } else {
        Some((palette, index))
    }
}
// Identity type latches the composition mode; translation never reinterprets it.
#[derive(Clone, Copy)]
#[cfg_attr(test, derive(Debug, Eq, PartialEq))]
enum Pixel {
    Shade(u8),
    Color(u8),
}
#[cfg(test)]
fn identity(bus: &Bus, palette: usize, index: u8) -> Pixel {
    if bus.color_mode() {
        Pixel::Color((palette * 16 + index as usize) as u8)
    } else {
        let p = u16::from_le_bytes([bus.ports[0x20 + palette * 2], bus.ports[0x21 + palette * 2]]);
        Pixel::Shade(((p >> (index * 4)) & 7) as u8)
    }
}
fn translate(bus: &Bus, pixel: Pixel) -> [u8; 4] {
    if bus.ports[0x14] & 1 == 0 {
        return [255; 4];
    }
    match pixel {
        Pixel::Color(index) => {
            let c = word(bus, 0xfe00 + index as usize * 2);
            [
                (c >> 8 & 15) as u8 * 17,
                (c >> 4 & 15) as u8 * 17,
                (c & 15) as u8 * 17,
                255,
            ]
        }
        Pixel::Shade(index) => shade(bus, index),
    }
}
fn shade(bus: &Bus, index: u8) -> [u8; 4] {
    let lut = bus.ports[0x1c + index as usize / 2];
    let value = 255 - ((lut >> ((index & 1) * 4)) & 15) * 17;
    [value, value, value, 255]
}
fn window(bus: &Bus, base: usize, x: usize, y: usize) -> bool {
    x >= bus.ports[base] as usize
        && x <= bus.ports[base + 2] as usize
        && y >= bus.ports[base + 1] as usize
        && y <= bus.ports[base + 3] as usize
}
#[cfg(test)]
fn screen(bus: &Bus, layer: usize, x: usize, y: usize) -> Option<Pixel> {
    if bus.ports[0] & (1 << layer) == 0 {
        return None;
    }
    if layer == 1 && bus.ports[0] & 32 != 0 && window(bus, 8, x, y) == (bus.ports[0] & 16 != 0) {
        return None;
    }
    let sx = (x + bus.ports[0x10 + layer * 2] as usize) & 255;
    let sy = (y + bus.ports[0x11 + layer * 2] as usize) & 255;
    let base =
        ((bus.ports[7] >> (layer * 4)) & if bus.color_mode() { 15 } else { 7 }) as usize * 0x800;
    let entry = word(bus, base + (sy / 8 * 32 + sx / 8) * 2);
    tile_pixel(bus, entry, sx % 8, sy % 8, false).map(|(p, i)| identity(bus, p, i))
}
#[cfg(test)]
/// Compose identities, without final shade/RGB12 lookup. See docs/lcd-timing.md.
fn compose_line_reference(bus: &Bus, y: usize) -> [Pixel; FRAME_WIDTH] {
    let mut row = [Pixel::Shade(0); FRAME_WIDTH];
    let count = (bus.ports[6] as usize).min(128);
    let first = bus.ports[5] as usize & 127;
    let sprites: Vec<usize> = if bus.ports[0] & 4 != 0 {
        (0..count)
            .map(|i| (first + i) & 127)
            .filter(|i| (y as u8).wrapping_sub(bus.sprites[i * 4 + 2]) < 8)
            .take(32)
            .collect()
    } else {
        Vec::new()
    };
    for (x, output) in row.iter_mut().enumerate() {
        let background = if bus.color_mode() {
            Pixel::Color(bus.ports[1])
        } else {
            Pixel::Shade(bus.ports[1] & 7)
        };
        let mut pixel = screen(bus, 0, x, y).unwrap_or(background);
        // The first nontransparent sprite wins even when Screen 2 later obscures it.
        let mut sprite_pixel = None;
        for &i in &sprites {
            let base = i * 4;
            let entry = u16::from_le_bytes([bus.sprites[base], bus.sprites[base + 1]]);
            let sx = (x as u8).wrapping_sub(bus.sprites[base + 3]);
            if sx >= 8 {
                continue;
            }
            if bus.ports[0] & 8 != 0 && window(bus, 12, x, y) == (entry & 0x1000 != 0) {
                continue;
            }
            let sy = (y as u8).wrapping_sub(bus.sprites[base + 2]);
            if let Some((p, c)) = tile_pixel(bus, entry, sx as usize, sy as usize, true) {
                sprite_pixel = Some((identity(bus, p, c), entry & 0x2000 != 0));
                break;
            }
        }
        if let Some((p, false)) = sprite_pixel {
            pixel = p;
        }
        if let Some(p) = screen(bus, 1, x, y) {
            pixel = p;
        }
        if let Some((p, true)) = sprite_pixel {
            pixel = p;
        }
        *output = pixel;
    }
    row
}
/// Decode a tile row once, keeping palette identities until the later LCD stage.
fn tile_row(bus: &Bus, entry: u16, y: usize, sprite: bool) -> [Option<Pixel>; 8] {
    let color = bus.color_mode();
    let four = color && bus.ports[0x60] & 64 != 0;
    let tile = (entry as usize & 511)
        + if color && !sprite && entry & 0x2000 != 0 {
            512
        } else {
            0
        };
    let y = if entry & 0x8000 != 0 { 7 - y } else { y };
    let planes = if four { 4 } else { 2 };
    let address = (if four { 0x4000 } else { 0x2000 }) + tile * planes * 8 + y * planes;
    let palette = if sprite {
        8 + ((entry as usize >> 9) & 7)
    } else {
        (entry as usize >> 9) & 15
    };
    let transparent = four || palette & 4 != 0;
    let mono = u16::from_le_bytes([bus.ports[0x20 + palette * 2], bus.ports[0x21 + palette * 2]]);
    let packed = four && bus.ports[0x60] & 32 != 0;
    let mut row = [None; 8];
    for x in 0..8 {
        let index = if packed {
            (bus.ram[address + x / 2] >> ((1 - (x & 1)) * 4)) & 15
        } else {
            (0..planes).fold(0, |value, p| {
                value | (((bus.ram[address + p] >> (7 - x)) & 1) << p)
            })
        };
        let output = if entry & 0x4000 != 0 { 7 - x } else { x };
        if index != 0 || !transparent {
            row[output] = Some(if color {
                Pixel::Color((palette * 16 + index as usize) as u8)
            } else {
                Pixel::Shade(((mono >> (index * 4)) & 7) as u8)
            });
        }
    }
    row
}

fn screen_line(bus: &Bus, layer: usize, y: usize) -> [Option<Pixel>; FRAME_WIDTH] {
    let mut row = [None; FRAME_WIDTH];
    if bus.ports[0] & (1 << layer) == 0 {
        return row;
    }
    let sy = (y + bus.ports[0x11 + layer * 2] as usize) & 255;
    let scroll = bus.ports[0x10 + layer * 2] as usize;
    let base =
        ((bus.ports[7] >> (layer * 4)) & if bus.color_mode() { 15 } else { 7 }) as usize * 0x800;
    let mut x = 0;
    // Repeated map entries share the decoded row within this composition only.
    // No write invalidation is needed, including for CPU and DMA tile writes.
    let mut previous = None;
    let mut tile = [None; 8];
    while x < FRAME_WIDTH {
        let sx = (x + scroll) & 255;
        let entry = word(bus, base + (sy / 8 * 32 + sx / 8) * 2);
        if previous != Some(entry) {
            tile = tile_row(bus, entry, sy % 8, false);
            previous = Some(entry);
        }
        let count = (8 - sx % 8).min(FRAME_WIDTH - x);
        for offset in 0..count {
            let at = x + offset;
            if layer == 1
                && bus.ports[0] & 32 != 0
                && window(bus, 8, at, y) == (bus.ports[0] & 16 != 0)
            {
                continue;
            }
            row[at] = tile[sx % 8 + offset];
        }
        x += count;
    }
    row
}

/// Compose identities, without final shade/RGB12 lookup. See docs/lcd-timing.md.
fn compose_line(bus: &Bus, y: usize) -> [Pixel; FRAME_WIDTH] {
    let background = if bus.color_mode() {
        Pixel::Color(bus.ports[1])
    } else {
        Pixel::Shade(bus.ports[1] & 7)
    };
    let mut row = [background; FRAME_WIDTH];
    let back = screen_line(bus, 0, y);
    let front = screen_line(bus, 1, y);
    let mut sprites = [None; FRAME_WIDTH];
    if bus.ports[0] & 4 != 0 {
        let count = (bus.ports[6] as usize).min(128);
        let first = bus.ports[5] as usize & 127;
        let mut selected = 0;
        for n in 0..count {
            let base = ((first + n) & 127) * 4;
            let sy = (y as u8).wrapping_sub(bus.sprites[base + 2]);
            if sy >= 8 {
                continue;
            }
            selected += 1;
            let entry = u16::from_le_bytes([bus.sprites[base], bus.sprites[base + 1]]);
            for (dx, pixel) in tile_row(bus, entry, sy as usize, true)
                .into_iter()
                .enumerate()
            {
                let x = bus.sprites[base + 3].wrapping_add(dx as u8) as usize;
                if x >= FRAME_WIDTH || sprites[x].is_some() {
                    continue;
                }
                if bus.ports[0] & 8 != 0 && window(bus, 12, x, y) == (entry & 0x1000 != 0) {
                    continue;
                }
                // The first opaque sprite claims this location even if Screen 2
                // will later cover it. List priority precedes layer priority.
                if let Some(pixel) = pixel {
                    sprites[x] = Some((pixel, entry & 0x2000 != 0));
                }
            }
            if selected == 32 {
                break;
            }
        }
    }
    for x in 0..FRAME_WIDTH {
        if let Some(pixel) = back[x] {
            row[x] = pixel;
        }
        if let Some((pixel, false)) = sprites[x] {
            row[x] = pixel;
        }
        if let Some(pixel) = front[x] {
            row[x] = pixel;
        }
        if let Some((pixel, true)) = sprites[x] {
            row[x] = pixel;
        }
    }
    row
}

fn translate_line(bus: &Bus, pixels: &mut [u8], y: usize, row: [Pixel; FRAME_WIDTH]) {
    let output = &mut pixels[y * FRAME_WIDTH * 4..(y + 1) * FRAME_WIDTH * 4];
    if bus.ports[0x14] & 1 == 0 {
        output.fill(255);
        return;
    }
    let shades: [[u8; 4]; 8] = std::array::from_fn(|i| shade(bus, i as u8));
    // Translation reads the palette at this boundary, one line after composition.
    // Alpha=255 distinguishes cached colors from the zero-initialized sentinel.
    let mut colors = [0u32; 256];
    for (output, pixel) in output.chunks_exact_mut(4).zip(row) {
        let rgba = match pixel {
            Pixel::Shade(i) => shades[i as usize],
            Pixel::Color(i) => {
                let cached = &mut colors[i as usize];
                if *cached == 0 {
                    *cached = u32::from_le_bytes(translate(bus, pixel));
                }
                cached.to_le_bytes()
            }
        };
        output.copy_from_slice(&rgba);
    }
}

#[cfg(test)]
mod row_tests {
    use super::*;
    use crate::Model;

    #[test]
    fn row_composition_matches_pixel_reference_with_wrapping_windows_and_sprite_limits() {
        let mut seed = 0x9e3779b9u32;
        let mut next = || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 16) as u8
        };
        for scene in 0..96 {
            let model = [Model::Mono, Model::Color, Model::Crystal][scene % 3];
            let mut bus = Bus::new(model);
            for byte in &mut bus.ram {
                *byte = next();
            }
            for byte in &mut bus.ports {
                *byte = next();
            }
            bus.ports[0x60] = [0, 0x80, 0xc0, 0xe0][scene / 3 % 4];
            bus.ports[0] = [0, 1, 2, 3, 7, 15, 0x27, 0x37][scene % 8];
            bus.ports[5] = next();
            bus.ports[6] = [0, 1, 32, 33, 128, 255][scene % 6];
            bus.ports[0x10] = [0, 1, 7, 255][scene % 4];
            for y in [0, 7, 128, 143] {
                for sprite in bus.sprites.chunks_exact_mut(4) {
                    sprite[0] = next();
                    sprite[1] = next();
                    sprite[2] = (y as u8).wrapping_sub(next() % 12);
                    sprite[3] = [0, 7, 220, 248, 255, next()][scene % 6];
                }
                let row = compose_line(&bus, y);
                assert_eq!(row, compose_line_reference(&bus, y), "scene {scene}, y {y}");
                let mut pixels = vec![0; crate::FRAME_BYTES];
                // Palette changes belong to translation, including mixed
                // latched Shade/Color identities after a video-mode change.
                bus.ram[0xfe00..].fill(next());
                bus.ports[0x60] ^= 0x80;
                translate_line(&bus, &mut pixels, y, row);
                for (x, pixel) in row.into_iter().enumerate() {
                    assert_eq!(
                        &pixels[(y * FRAME_WIDTH + x) * 4..(y * FRAME_WIDTH + x + 1) * 4],
                        &translate(&bus, pixel)
                    );
                }
                // Changed tile/palette inputs must be observed on every line.
                bus.ram[0x2000..].fill(next());
            }
        }
    }
}

#[derive(Default)]
pub(crate) struct Pipeline {
    pending: Option<(usize, [Pixel; FRAME_WIDTH])>,
}
impl Pipeline {
    pub(crate) fn line_end(&mut self, bus: &Bus, pixels: &mut [u8], line: usize) {
        if let Some((y, row)) = self.pending.take() {
            if line == y + 1 {
                translate_line(bus, pixels, y, row);
            }
        }
        if line < FRAME_HEIGHT {
            self.pending = Some((line, compose_line(bus, line)));
        }
    }
    pub(crate) fn frame_wrap(&mut self) {
        self.pending = None;
    }
}

// Static rendering helpers for composition tests only; machines use Pipeline.
#[cfg(test)]
fn render_line(bus: &Bus, pixels: &mut [u8], y: usize) {
    translate_line(bus, pixels, y, compose_line(bus, y));
}
#[cfg(test)]
fn render(bus: &Bus, pixels: &mut [u8]) {
    for y in 0..FRAME_HEIGHT {
        render_line(bus, pixels, y);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Model, FRAME_BYTES};

    #[test]
    fn tile_planes_palette_lut_flips_scroll_and_display_enable() {
        let mut bus = Bus::new(Model::Mono);
        let mut pixels = vec![0; FRAME_BYTES];
        bus.ports[0] = 1;
        bus.ports[0x14] = 1;
        bus.ports[7] = 1;
        bus.ports[0x1c] = 0xf0;
        bus.ports[0x1d] = 0x87;
        bus.ports[0x20] = 0x10;
        bus.ports[0x21] = 0x32;
        bus.ram[0x800] = 1;
        bus.ram[0x2010] = 0x80;
        bus.ram[0x2011] = 0x40;
        render(&bus, &mut pixels);
        assert_eq!(&pixels[0..4], &[0, 0, 0, 255]); // plane 0 -> shade 1 -> black
        assert_eq!(&pixels[4..8], &[136, 136, 136, 255]); // plane 1 -> shade 2 -> 7
        assert_eq!(&pixels[8..12], &[255, 255, 255, 255]);
        bus.ram[0x801] = 0x40; // horizontal flip
        render(&bus, &mut pixels);
        assert_eq!(&pixels[28..32], &[0, 0, 0, 255]);
        bus.ram[0x801] = 0x80; // vertical flip -> row 7 is blank
        render(&bus, &mut pixels);
        assert_eq!(&pixels[..4], &[255, 255, 255, 255]);
        bus.ram[0x801] = 0;
        bus.ports[0x10] = 1;
        render(&bus, &mut pixels);
        assert_eq!(&pixels[..4], &[136, 136, 136, 255]);
        bus.ports[0] = 0;
        bus.ports[1] = 1;
        render(&bus, &mut pixels);
        assert_eq!(&pixels[..4], &[0, 0, 0, 255]);
        bus.ports[0x14] = 0;
        render(&bus, &mut pixels);
        assert!(pixels.chunks_exact(4).all(|p| p == [255, 255, 255, 255]));
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;
    use crate::{Model, FRAME_BYTES};
    fn setup(mode: u8) -> Bus {
        let mut b = Bus::new(Model::Crystal);
        b.output(0x60, mode).unwrap();
        b.ports[0x14] = 1;
        b.ports[7] = 0x21;
        b.ports[0] = 3;
        b
    }
    fn palette(b: &mut Bus, p: usize, index: usize, value: u16) {
        let a = 0xfe00 + p * 32 + index * 2;
        b.ram[a..a + 2].copy_from_slice(&value.to_le_bytes());
    }
    fn map(b: &mut Bus, base: usize, entry: u16) {
        for w in b.ram[base..base + 2048].chunks_exact_mut(2) {
            w.copy_from_slice(&entry.to_le_bytes());
        }
    }
    fn pixel(b: &Bus, x: usize, y: usize) -> [u8; 4] {
        let mut pixels = vec![0; FRAME_BYTES];
        render_line(b, &mut pixels, y);
        pixels[(y * FRAME_WIDTH + x) * 4..(y * FRAME_WIDTH + x) * 4 + 4]
            .try_into()
            .unwrap()
    }
    fn tile(b: &mut Bus, index: usize, color: u8) {
        let four = b.ports[0x60] & 64 != 0;
        let planes = if four { 4 } else { 2 };
        let base = if four { 0x4000 } else { 0x2000 };
        for y in 0..8 {
            for p in 0..planes {
                b.ram[base + index * planes * 8 + y * planes + p] =
                    if color & (1 << p) != 0 { 255 } else { 0 };
            }
        }
    }
    #[test]
    fn planar_2_and_4_bpp_banks_rgb12_and_palette_zero_transparency() {
        for mode in [0x80, 0xc0] {
            let mut b = setup(mode);
            b.ports[0] = 1;
            tile(&mut b, 512, 3);
            map(&mut b, 0x800, 0x2000);
            palette(&mut b, 0, 3, 0xa53);
            assert_eq!(pixel(&b, 0, 0), [170, 85, 51, 255]);
            tile(&mut b, 0, 0);
            map(&mut b, 0x800, 0);
            b.ports[1] = 0x12;
            palette(&mut b, 0, 0, 0xf00);
            palette(&mut b, 1, 2, 0x00f);
            assert_eq!(
                pixel(&b, 0, 0),
                if mode == 0x80 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                }
            );
            map(&mut b, 0x800, 4 << 9);
            assert_eq!(pixel(&b, 0, 0), [0, 0, 255, 255]);
        }
    }
    #[test]
    fn packed_layout_flips_scroll_and_independent_screen2_window() {
        let mut b = setup(0xe0);
        b.ports[0] = 1;
        b.ram[0x4000..0x4004].copy_from_slice(&[0x12, 0x34, 0x56, 0x78]);
        for i in 1..=8 {
            palette(&mut b, 0, i, (i as u16) << 8);
        }
        assert_eq!(pixel(&b, 0, 0), [17, 0, 0, 255]);
        assert_eq!(pixel(&b, 7, 0), [136, 0, 0, 255]);
        map(&mut b, 0x800, 0x4000);
        assert_eq!(pixel(&b, 0, 0), [136, 0, 0, 255]);
        map(&mut b, 0x800, 0x8000);
        assert_eq!(pixel(&b, 0, 0), [0, 0, 0, 255]);
        map(&mut b, 0x800, 0);
        b.ports[0x10] = 1;
        assert_eq!(pixel(&b, 0, 0), [34, 0, 0, 255]);
        b.ports[0x11] = 1;
        assert_eq!(pixel(&b, 0, 0), [0, 0, 0, 255]);
        b = setup(0xc0);
        tile(&mut b, 0, 1);
        tile(&mut b, 1, 2);
        map(&mut b, 0x1000, 1 | (1 << 9));
        palette(&mut b, 0, 1, 0x0f0);
        palette(&mut b, 1, 2, 0xf00);
        b.ports[0] = 0x23;
        b.ports[8] = 1;
        b.ports[9] = 1;
        b.ports[10] = 2;
        b.ports[11] = 2;
        assert_eq!(pixel(&b, 0, 0), [0, 255, 0, 255]);
        assert_eq!(pixel(&b, 1, 1), [255, 0, 0, 255]);
        assert_eq!(pixel(&b, 2, 2), [255, 0, 0, 255]);
        assert_eq!(pixel(&b, 3, 2), [0, 255, 0, 255]);
        b.ports[0] |= 16;
        assert_eq!(pixel(&b, 1, 1), [0, 255, 0, 255]);
        assert_eq!(pixel(&b, 0, 0), [255, 0, 0, 255]);
        b.ports[0] = 3;
        b.ports[0x12] = 8;
        b.ram[0x1002..0x1004].copy_from_slice(&(2u16 | (4u16 << 9)).to_le_bytes());
        assert_eq!(pixel(&b, 0, 0), [0, 255, 0, 255]);
    }
    fn sprite(b: &mut Bus, index: usize, entry: u16, x: u8, y: u8) {
        let a = index * 4;
        b.sprites[a..a + 2].copy_from_slice(&entry.to_le_bytes());
        b.sprites[a + 2] = y;
        b.sprites[a + 3] = x;
    }
    #[test]
    fn sprites_obey_priority_list_order_windows_flips_wrap_and_32_sprite_limit() {
        let mut b = setup(0xc0);
        b.ports[0] = 7;
        b.ports[6] = 2;
        tile(&mut b, 0, 1);
        tile(&mut b, 1, 2);
        tile(&mut b, 2, 3);
        map(&mut b, 0x1000, 1 | (1 << 9));
        palette(&mut b, 0, 1, 0x0f0);
        palette(&mut b, 1, 2, 0xf00);
        palette(&mut b, 12, 3, 0x00f);
        palette(&mut b, 13, 3, 0xff0);
        sprite(&mut b, 0, 2 | (4 << 9), 0, 0);
        sprite(&mut b, 1, 2 | (5 << 9) | 0x2000, 0, 0);
        assert_eq!(pixel(&b, 0, 0), [255, 0, 0, 255]); // Earlier low-priority sprite masks later high-priority sprite.
        sprite(&mut b, 0, 2 | (4 << 9) | 0x2000, 0, 0);
        assert_eq!(pixel(&b, 0, 0), [0, 0, 255, 255]);
        b.ports[0] |= 8;
        b.ports[12] = 2;
        b.ports[13] = 2;
        b.ports[14] = 4;
        b.ports[15] = 4;
        assert_eq!(pixel(&b, 0, 0), [255, 0, 0, 255]);
        assert_eq!(pixel(&b, 2, 2), [0, 0, 255, 255]);
        sprite(&mut b, 0, 2 | (4 << 9) | 0x3000, 0, 0);
        assert_eq!(pixel(&b, 0, 0), [0, 0, 255, 255]);
        assert_eq!(pixel(&b, 2, 2), [255, 255, 0, 255]);
        b.ports[0] = 5;
        b.ports[6] = 33;
        for i in 0..32 {
            sprite(&mut b, i, 2 | (4 << 9) | 0x2000, 240, 0);
        }
        sprite(&mut b, 32, 2 | (4 << 9) | 0x2000, 0, 0);
        assert_eq!(pixel(&b, 0, 0), [0, 255, 0, 255]);
        b.ports[5] = 1;
        assert_eq!(pixel(&b, 0, 0), [0, 0, 255, 255]);
        b.ports[5] = 0;
        b.ports[6] = 1;
        sprite(&mut b, 0, 2 | (4 << 9) | 0x2000, 255, 255);
        assert_eq!(pixel(&b, 0, 0), [0, 0, 255, 255]);
        // A single planar pixel makes both sprite flips observable.
        b.ram[0x4040..0x4060].fill(0);
        b.ram[0x4040] = 128;
        palette(&mut b, 12, 1, 0x00f);
        sprite(&mut b, 0, 2 | (4 << 9) | 0xe000, 0, 0);
        assert_eq!(pixel(&b, 7, 7), [0, 0, 255, 255]);
        assert_eq!(pixel(&b, 0, 0), [0, 255, 0, 255]);
    }
    #[test]
    fn sprite_table_is_latched_at_vblank_and_mono_rejects_color_modes() {
        let mut b = setup(0x80);
        b.ports[4] = 12;
        b.ram[0x1800] = 42;
        assert_eq!(b.sprites[0], 0);
        b.tick(256 * 144);
        assert_eq!(b.sprites[0], 42);
        b.ram[0x1800] = 99;
        assert_eq!(b.sprites[0], 42);
        b.tick(256 * 159);
        assert_eq!(b.sprites[0], 99);
        let mut mono = Bus::new(Model::Mono);
        assert!(mono.output(0x60, 0x80).is_err());
        assert!(!mono.color_mode());
        assert!(b.output(0x60, 0x40).is_err());
        assert!(b.output(0x60, 0xa0).is_err());
    }
}
