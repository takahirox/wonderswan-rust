use crate::{bus::Bus, FRAME_HEIGHT, FRAME_WIDTH};

/// Mono, interleaved 2bpp Screen 1. All models run this same common subset.
pub(crate) fn render(bus: &Bus, pixels: &mut [u8]) {
    for y in 0..FRAME_HEIGHT {
        for x in 0..FRAME_WIDTH {
            let mut shade = bus.ports[1] & 7;
            if bus.ports[0x14] & 1 == 0 {
                shade = 0;
            } else if bus.ports[0] & 1 != 0 {
                let sx = (x + bus.ports[0x10] as usize) & 255;
                let sy = (y + bus.ports[0x11] as usize) & 255;
                let map = (bus.ports[7] as usize & 7) * 0x800;
                let entry = map + ((sy / 8) * 32 + sx / 8) * 2;
                let tile = u16::from_le_bytes([bus.ram[entry], bus.ram[entry + 1]]);
                let tx = if tile & 0x4000 != 0 {
                    7 - sx % 8
                } else {
                    sx % 8
                };
                let ty = if tile & 0x8000 != 0 {
                    7 - sy % 8
                } else {
                    sy % 8
                };
                let data = 0x2000 + (tile as usize & 511) * 16 + ty * 2;
                let color = ((bus.ram[data] >> (7 - tx)) & 1)
                    | (((bus.ram[data + 1] >> (7 - tx)) & 1) << 1);
                let palette = ((tile >> 9) & 15) as usize;
                let word = u16::from_le_bytes([
                    bus.ports[0x20 + palette * 2],
                    bus.ports[0x21 + palette * 2],
                ]);
                shade = ((word >> (color * 4)) & 7) as u8;
                if palette & 4 != 0 && color == 0 {
                    shade = 0;
                }
            }
            let lut = bus.ports[0x1c + shade as usize / 2];
            let intensity = if bus.ports[0x14] & 1 == 0 {
                255
            } else {
                255 - ((lut >> ((shade & 1) * 4)) & 15) * 17
            };
            let offset = (y * FRAME_WIDTH + x) * 4;
            pixels[offset..offset + 4].copy_from_slice(&[intensity, intensity, intensity, 255]);
        }
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
