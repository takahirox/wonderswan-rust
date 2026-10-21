//! Color sound DMA. Register bytes live in the shared port array; only writes
//! update the repeat shadow. Clock/stall scheduling belongs to the common bus.
#[derive(Default)]
pub(crate) struct SoundDma {
    shadow: [u8; 6],
    rate_clock: u8,
}

impl SoundDma {
    pub fn counter(ports: &[u8; 256], base: usize) -> u32 {
        u32::from_le_bytes([ports[base], ports[base + 1], ports[base + 2] & 15, 0])
    }

    pub fn write(&mut self, ports: &mut [u8; 256], port: usize, value: u8) {
        if port == 0x52 {
            ports[port] = value & 0xdf;
            if Self::counter(ports, 0x4e) == 0 {
                ports[port] &= 0x7f;
            }
        } else {
            let index = if port <= 0x4c {
                port - 0x4a
            } else {
                port - 0x4e + 3
            };
            let value = value & if index % 3 == 2 { 15 } else { 255 };
            ports[port] = value;
            self.shadow[index] = value;
        }
    }

    /// Called at 117 mod 128, including rate-divided and hold ticks. The bus
    /// reserves 6 + source access clocks on every enabled tick, not just samples.
    pub fn clock(&mut self, ports: &mut [u8; 256], value: u8) -> Option<(u8, bool)> {
        let control = ports[0x52];
        let length = Self::counter(ports, 0x4e);
        if length == 0 {
            ports[0x52] &= 0x7f;
            return None;
        }
        self.rate_clock += 1;
        if self.rate_clock < [6, 4, 2, 1][(control & 3) as usize] {
            return None;
        }
        self.rate_clock = 0;
        if control & 4 != 0 {
            return Some((0, control & 0x10 != 0));
        }
        let source =
            Self::counter(ports, 0x4a).wrapping_add(if control & 0x40 != 0 { u32::MAX } else { 1 })
                & 0xfffff;
        ports[0x4a..=0x4c].copy_from_slice(&source.to_le_bytes()[..3]);
        ports[0x4e..=0x50].copy_from_slice(&(length - 1).to_le_bytes()[..3]);
        if length == 1 {
            if control & 8 != 0 {
                ports[0x4a..=0x4c].copy_from_slice(&self.shadow[..3]);
                ports[0x4e..=0x50].copy_from_slice(&self.shadow[3..]);
                if Self::counter(ports, 0x4e) == 0 {
                    ports[0x52] &= 0x7f;
                }
            } else {
                ports[0x52] &= 0x7f;
            }
        }
        Some((value, control & 0x10 != 0))
    }
}
