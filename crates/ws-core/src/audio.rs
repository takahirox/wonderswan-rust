//! Four-channel sound clocked by the same 3.072 MHz timebase as display/timers.
use std::collections::VecDeque;
pub const SAMPLE_RATE: u32 = 24000;
const MAX_SAMPLES: usize = 4800 * 2;
#[derive(Default)]
pub(crate) struct Audio {
    phases: [usize; 4],
    clocks: [u32; 4],
    sample_clock: u32,
    sweep_clock: u32,
    pub lfsr: u16,
    pub mix: [u16; 2],
    samples: VecDeque<f32>,
}
impl Audio {
    pub fn write(&mut self, port: u16, value: u8) {
        if port == 0x8d {
            self.sweep_clock = 0;
        }
        if port == 0x8e && value & 8 != 0 {
            self.lfsr = 0;
        }
    }
    pub fn tick(&mut self, cycles: u32, ram: &[u8], ports: &mut [u8; 256]) {
        for _ in 0..cycles {
            let control = ports[0x90];
            for ch in 0..4 {
                if control & (1 << ch) == 0 {
                    continue;
                }
                self.clocks[ch] += 1;
                let freq = u16::from_le_bytes([ports[0x80 + ch * 2], ports[0x81 + ch * 2]]) & 2047;
                if self.clocks[ch] >= 2048 - freq as u32 {
                    self.clocks[ch] = 0;
                    self.phases[ch] = (self.phases[ch] + 1) & 31;
                    if ch == 3 && ports[0x8e] & 16 != 0 {
                        let tap = [14, 10, 13, 4, 8, 6, 9, 11][(ports[0x8e] & 7) as usize];
                        let bit = (!(self.lfsr >> 7 ^ self.lfsr >> tap)) & 1;
                        self.lfsr = ((self.lfsr << 1) | bit) & 0x7fff;
                    }
                }
            }
            if control & 0x44 == 0x44 {
                self.sweep_clock += 1;
                if self.sweep_clock >= 8192 * (u32::from(ports[0x8d] & 31) + 1) {
                    self.sweep_clock = 0;
                    let frequency = u16::from_le_bytes([ports[0x84], ports[0x85]])
                        .wrapping_add(ports[0x8c] as i8 as u16)
                        & 2047;
                    [ports[0x84], ports[0x85]] = frequency.to_le_bytes();
                }
            }
            self.sample_clock += 1;
            if self.sample_clock == 128 {
                self.sample_clock = 0;
                self.sample(ram, ports);
            }
        }
    }
    fn sample(&mut self, ram: &[u8], ports: &[u8; 256]) {
        let mut mix = [0u16; 2];
        for ch in 0..4 {
            if ch == 1 && ports[0x90] & 0x20 != 0 {
                for (side, shift) in [2, 0].into_iter().enumerate() {
                    let volume = (ports[0x94] >> shift) & 3;
                    mix[side] += match volume {
                        1 | 3 => ports[0x89] as u16,
                        2 => (ports[0x89] as u16) / 2,
                        _ => 0,
                    };
                }
                continue;
            }
            if ports[0x90] & (1 << ch) == 0 {
                continue;
            }
            let sample = if ch == 3 && ports[0x90] & 0x80 != 0 {
                if self.lfsr & 1 != 0 {
                    15
                } else {
                    0
                }
            } else {
                let address = (ports[0x8f] as usize * 64 + ch * 16 + self.phases[ch] / 2) & 0x3fff;
                (ram[address] >> ((self.phases[ch] & 1) * 4)) & 15
            } as u16;
            mix[0] += sample * (ports[0x88 + ch] >> 4) as u16;
            mix[1] += sample * (ports[0x88 + ch] & 15) as u16;
        }
        self.mix = mix;
        let output = if ports[0x91] & 8 != 0 {
            [mix[0] as f32 / 1024.0, mix[1] as f32 / 1024.0]
        } else if ports[0x91] & 1 != 0 {
            let mono = (((mix[0] + mix[1]) >> ((ports[0x91] >> 1) & 3)) & 255) as f32 / 256.0
                * (ports[0x9e] & 3) as f32
                / 3.0;
            [mono; 2]
        } else {
            [0.0; 2]
        };
        if self.samples.len() >= MAX_SAMPLES {
            self.samples.pop_front();
            self.samples.pop_front();
        }
        self.samples.extend(output);
    }
    pub fn drain(&mut self) -> Vec<f32> {
        self.samples.drain(..).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (Audio, Vec<u8>, [u8; 256]) {
        let mut ram = vec![0; 65536];
        for ch in 0..4 {
            ram[ch * 16 + 8..ch * 16 + 16].fill(255);
        }
        let mut ports = [0; 256];
        ports[0x91] = 8;
        for ch in 0..4 {
            ports[0x80 + ch * 2] = 0xc0;
            ports[0x81 + ch * 2] = 7;
            ports[0x88 + ch] = 0xff;
        }
        (Audio::default(), ram, ports)
    }
    #[test]
    fn disabled_is_silent_and_each_wave_channel_produces_deterministic_stereo() {
        let (mut a, ram, mut p) = setup();
        a.tick(3072, &ram, &mut p);
        assert!(a.drain().iter().all(|s| *s == 0.0));
        for ch in 0..4 {
            let (mut a, ram, mut p) = setup();
            p[0x90] = 1 << ch;
            p[0x88 + ch] = 0xf0;
            a.tick(4096, &ram, &mut p);
            let first = a.drain();
            assert!(first.chunks_exact(2).any(|v| v[0] > 0.0));
            assert!(first.chunks_exact(2).all(|v| v[1] == 0.0));
            let (mut b, _, _) = setup();
            b.tick(4096, &ram, &mut p);
            assert_eq!(b.drain(), first);
        }
    }
    #[test]
    fn voice_without_wave_enable_sweep_wrap_noise_taps_and_bounded_sample_clock() {
        let (mut a, ram, mut p) = setup();
        p[0x90] = 0x20;
        p[0x89] = 128;
        p[0x94] = 6;
        a.tick(128, &ram, &mut p);
        assert_eq!(a.mix, [128, 64]);
        assert_eq!(a.drain(), [0.125, 0.0625]);
        p[0x90] = 0x44;
        p[0x84] = 0xff;
        p[0x85] = 7;
        p[0x8c] = 1;
        p[0x8d] = 0;
        a.tick(8192, &ram, &mut p);
        assert_eq!([p[0x84], p[0x85]], [0, 0]);
        p[0x8c] = 255;
        a.write(0x8d, 0);
        a.tick(8192, &ram, &mut p);
        assert_eq!([p[0x84], p[0x85]], [255, 7]);
        // Independent states after 8, 16 and 50 sample clocks, calculated from
        // the hardware tap specification (zero seed), not the generator under test.
        let states = [
            [255, 32513, 3099],
            [255, 32543, 956],
            [255, 32515, 15603],
            [248, 30913, 20775],
            [255, 32639, 15836],
            [254, 32508, 15154],
            [255, 32575, 255],
            [255, 32527, 17344],
        ];
        for (mode, expected) in states.into_iter().enumerate() {
            let (mut a, ram, mut p) = setup();
            p[0x90] = 0x88;
            p[0x8e] = 0x10 | mode as u8;
            for (cycles, state) in [8 * 64, 8 * 64, 34 * 64].into_iter().zip(expected) {
                a.tick(cycles, &ram, &mut p);
                assert_eq!(a.lfsr, state);
            }
            a.write(0x8e, 8);
            assert_eq!(a.lfsr, 0);
        }
        let (mut a, ram, mut p) = setup();
        a.tick(127, &ram, &mut p);
        assert!(a.drain().is_empty());
        a.tick(1, &ram, &mut p);
        assert_eq!(a.drain().len(), 2);
        a.tick(700000, &ram, &mut p);
        assert_eq!(a.drain().len(), MAX_SAMPLES);
        assert!(a.drain().is_empty());
    }
}
