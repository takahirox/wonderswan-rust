use crate::{
    audio::Audio,
    cartridge::{Cartridge, Mapper, SaveKind},
    eeprom::Eeprom,
    internal_eeprom::{initial_data, InternalEeprom},
    sdma::SoundDma,
    Error, Model,
};

pub(crate) struct Bus {
    pub ram: Vec<u8>,
    pub ports: [u8; 256],
    pub cartridge: Option<Cartridge>,
    pub buttons: u16,
    pub cycles: u64,
    pub model: Model,
    pub audio: Audio,
    pub sprites: [u8; 512],
    pub accesses: std::cell::Cell<u32>,
    eeprom: Eeprom,
    pub console_data: [Vec<u8>; 3],
    internal_eeprom: InternalEeprom,
    timer: [u16; 2],
    rtc_remaining: u8,
    rtc_command: u8,
    rtc_wait: u32,
    rtc_ready: bool,
    gdma_started: bool,
    sdma: SoundDma,
    sound_stall: u32,
    fault: Option<Error>,
}
impl Bus {
    pub fn new(model: Model) -> Self {
        let mut ports = [0; 256];
        ports[0xc0] = 15;
        ports[0xc2] = 255;
        ports[0xc3] = 255;
        ports[0xcf] = 15;
        ports[0xd2] = 255;
        ports[0xd4] = 255;
        ports[0x16] = 158;
        ports[0x91] = 0x80;
        ports[0x9e] = 3;
        Self {
            ram: vec![0; 65536],
            ports,
            cartridge: None,
            buttons: 0,
            cycles: 0,
            model,
            audio: Audio::default(),
            sprites: [0; 512],
            accesses: std::cell::Cell::new(0),
            eeprom: Eeprom::default(),
            console_data: [Model::Mono, Model::Color, Model::Crystal].map(initial_data),
            internal_eeprom: InternalEeprom::default(),
            timer: [0; 2],
            rtc_remaining: 0,
            rtc_command: 0,
            rtc_wait: 0,
            rtc_ready: false,
            gdma_started: false,
            sdma: SoundDma::default(),
            sound_stall: 0,
            fault: None,
        }
    }
    /// Firmware-free handoff maps the end of the complete ROM address space.
    pub fn insert_cartridge(&mut self, cartridge: Cartridge) {
        if cartridge.header.mapper == Mapper::Bandai2003 {
            self.ports[0xc0] = 63;
            self.ports[0xcf] = 63;
            self.ports[0xd3] = 3;
            self.ports[0xd5] = 3;
        }
        self.cartridge = Some(cartridge);
        self.reset_internal_eeprom();
    }
    pub fn reset_internal_eeprom(&mut self) {
        self.internal_eeprom = InternalEeprom::default();
        if let Some(c) = &self.cartridge {
            self.internal_eeprom
                .handoff(c.header.internal_eeprom_write_protected);
        }
    }
    pub fn color_mode(&self) -> bool {
        self.model != Model::Mono && self.ports[0x60] & 0x80 != 0
    }
    fn extended(&self) -> bool {
        self.cartridge
            .as_ref()
            .is_some_and(|c| c.header.mapper == Mapper::Bandai2003)
    }
    fn bank(&self, low: usize, high: usize) -> usize {
        self.ports[low] as usize
            | if self.extended() {
                ((self.ports[high] & 3) as usize) << 8
            } else {
                0
            }
    }
    fn access_time(&self, address: usize) -> u32 {
        // Byte bus accesses, including fetches. ROM header wait and SRAM wait add clocks.
        let wait = if address >= 0x20000 {
            self.cartridge.as_ref().is_some_and(|c| c.header.rom_wait) as u32
        } else if address >= 0x10000 {
            (self.ports[0x60] >> 1 & 1) as u32
        } else {
            0
        };
        1 + wait
    }
    fn account(&self, address: usize) {
        self.accesses
            .set(self.accesses.get() + self.access_time(address));
    }
    pub fn word_transfer(&self, first: u32, second: u32) {
        if first >= 0x20000 && first & 1 == 0 && second == first + 1 {
            if let Some(c) = self.cartridge.as_ref().filter(|c| c.header.rom_word_bus) {
                self.accesses.set(
                    self.accesses
                        .get()
                        .saturating_sub(1 + u32::from(c.header.rom_wait)),
                );
            }
        }
    }
    pub fn read(&self, address: u32) -> u8 {
        let a = address as usize & 0xfffff;
        self.account(a);
        self.read_mapped(a)
    }
    /// Shared mapping without CPU bus accounting (both DMAs have their own costs).
    fn read_mapped(&self, a: usize) -> u8 {
        if a < 65536 {
            return if a < 0x4000 || self.color_mode() {
                self.ram[a]
            } else {
                255
            };
        }
        let Some(cart) = &self.cartridge else {
            return 255;
        };
        if a < 0x20000 {
            let bank = self.bank(0xc1, 0xd1);
            if self.extended() && self.ports[0xce] & 1 != 0 {
                return cart.rom(bank, a & 65535);
            }
            return if matches!(cart.header.save, SaveKind::Sram(_)) {
                cart.save[((bank << 16) | (a & 65535)) & (cart.save.len() - 1)]
            } else {
                255
            };
        }
        let bank = match a >> 16 {
            2 => self.bank(0xc2, 0xd3),
            3 => self.bank(0xc3, 0xd5),
            n => ((self.ports[0xc0] as usize & if self.extended() { 63 } else { 15 }) << 4) | n,
        };
        cart.rom(bank, a & 65535)
    }
    /// One bounded DMA word, before any CPU instruction or interrupt delivery.
    /// The caller advances the same clock as for CPU execution.
    pub fn gdma_step(&mut self) -> Option<u32> {
        if self.ports[0x48] & 0x80 == 0 {
            return None;
        }
        let mut source =
            u32::from_le_bytes([self.ports[0x40], self.ports[0x41], self.ports[0x42], 0]);
        let mut destination = u16::from_le_bytes([self.ports[0x44], self.ports[0x45]]);
        let mut length = u16::from_le_bytes([self.ports[0x46], self.ports[0x47]]);
        // SRAM is always an 8-bit region, including the 2003 self-flash window.
        let valid_source = source < 0x10000
            || (source >= 0x20000
                && self
                    .cartridge
                    .as_ref()
                    .is_some_and(|c| c.header.rom_word_bus && !c.header.rom_wait));
        if !self.color_mode() || length == 0 || !valid_source {
            self.ports[0x48] &= 0x40;
            self.gdma_started = false;
            return None;
        }
        // Read the complete word before writing it, preserving directional overlap.
        let lo = self.read_mapped(source as usize);
        let hi = self.read_mapped(source as usize + 1);
        self.ram[destination as usize] = lo;
        self.ram[destination as usize + 1] = hi;
        let reverse = self.ports[0x48] & 0x40 != 0;
        source = if reverse {
            source.wrapping_sub(2)
        } else {
            source + 2
        } & 0xfffff;
        destination = if reverse {
            destination.wrapping_sub(2)
        } else {
            destination.wrapping_add(2)
        };
        length -= 2;
        self.ports[0x40..=0x42].copy_from_slice(&source.to_le_bytes()[..3]);
        self.ports[0x44..=0x45].copy_from_slice(&destination.to_le_bytes());
        self.ports[0x46..=0x47].copy_from_slice(&length.to_le_bytes());
        let cycles = if self.gdma_started { 2 } else { 7 };
        self.gdma_started = length != 0;
        if length == 0 {
            self.ports[0x48] &= 0x40;
        }
        Some(cycles)
    }
    pub fn write(&mut self, address: u32, value: u8) {
        let a = address as usize & 0xfffff;
        self.account(a);
        if a < 65536 && (a < 0x4000 || self.color_mode()) {
            self.ram[a] = value;
        }
        if (0x10000..0x20000).contains(&a) {
            let bank = self.bank(0xc1, 0xd1);
            if self.extended() && self.ports[0xce] & 1 != 0 {
                self.fault = Some(Error::new(
                    "Bandai 2003 self-flash write is unsupported; ROM remains read-only.",
                ));
                return;
            }
            if let Some(c) = &mut self.cartridge {
                if matches!(c.header.save, SaveKind::Sram(_)) {
                    let offset = ((bank << 16) | (a & 65535)) & (c.save.len() - 1);
                    c.save[offset] = value;
                }
            }
        }
    }
    pub fn take_fault(&mut self) -> Result<(), Error> {
        self.fault.take().map_or(Ok(()), Err)
    }
    pub fn request(&mut self, bit: u8) {
        self.ports[0xb4] |= self.ports[0xb2] & (1 << bit);
    }
    pub fn interrupt(&self) -> Option<u8> {
        // Enable controls request latching, not removal of previously latched requests.
        let pending = self.ports[0xb4];
        if pending == 0 {
            None
        } else {
            Some((self.ports[0xb0] & 0xf8) | (7 - pending.leading_zeros() as u8))
        }
    }
    pub fn set_buttons(&mut self, buttons: u16) {
        if buttons & !self.buttons != 0 {
            self.request(1);
        }
        self.buttons = buttons;
    }
    pub fn sound_dma_stall(&mut self) -> Option<u32> {
        let cycles = std::mem::take(&mut self.sound_stall);
        (cycles != 0).then_some(cycles)
    }
    pub fn tick(&mut self, mut cycles: u32) {
        // Split at SDMA events so an arbitrarily sized clock advance produces
        // exactly the same PCM and controller state as single-clock advances.
        while cycles != 0 {
            let until = (117 + 128 - self.cycles % 128 - 1) % 128 + 1;
            let count = cycles.min(until as u32);
            self.tick_devices(count);
            cycles -= count;
            if self.cycles % 128 == 117 && self.model != Model::Mono && self.ports[0x52] & 0x80 != 0
            {
                let source = SoundDma::counter(&self.ports, 0x4a) as usize;
                if SoundDma::counter(&self.ports, 0x4e) != 0 {
                    self.sound_stall += 6 + self.access_time(source);
                }
                let value = self.read_mapped(source);
                if let Some((value, hyper)) = self.sdma.clock(&mut self.ports, value) {
                    if hyper {
                        self.audio.hyper_input(value, true, &mut self.ports);
                    } else {
                        self.ports[0x89] = value;
                    }
                }
            }
        }
    }
    fn tick_devices(&mut self, cycles: u32) {
        if self.rtc_wait > 0 {
            self.rtc_wait = self.rtc_wait.saturating_sub(cycles);
            if self.rtc_wait == 0 {
                self.rtc_ready = true;
            }
        }
        let before = self.cycles / 256;
        self.cycles += cycles as u64;
        self.audio.tick(cycles, &self.ram, &mut self.ports);
        self.internal_eeprom
            .tick(cycles, &mut self.console_data[self.model.index()]);
        if let Some(cart) = &mut self.cartridge {
            self.eeprom.tick(cycles, cart);
        }
        for line in before + 1..=self.cycles / 256 {
            let line = (line % 159) as u8;
            self.timer_tick(0);
            if line == self.ports[3] {
                self.request(4);
            }
            if line == 144 {
                self.request(6);
                self.timer_tick(1);
                let base =
                    ((self.ports[4] & if self.color_mode() { 63 } else { 31 }) as usize) << 9;
                for i in 0..512 {
                    self.sprites[i] = self.ram[(base + i) & 65535];
                }
            }
        }
    }
    fn timer_tick(&mut self, index: usize) {
        let shift = index * 2;
        let control = self.ports[0xa2] >> shift;
        if self.timer[index] == 1 {
            self.request(if index == 0 { 7 } else { 5 });
        }
        if control & 1 != 0 && self.timer[index] != 0 {
            self.timer[index] -= 1;
            if self.timer[index] == 0 && control & 2 != 0 {
                self.timer[index] = u16::from_le_bytes([
                    self.ports[0xa4 + index * 2],
                    self.ports[0xa5 + index * 2],
                ]);
            }
        }
    }
    pub fn input(&mut self, port: u16) -> Result<u8, Error> {
        self.accesses
            .set(self.accesses.get() + 1 + u32::from(port >= 0xc0 && (self.ports[0x60] & 8 != 0)));
        match port {
            0x40..=0x42 | 0x44..=0x48 if self.model != Model::Mono => Ok(self.ports[port as usize]),
            0x4a..=0x4c | 0x4e..=0x50 | 0x52 | 0x64..=0x6b if self.model != Model::Mono => {
                Ok(self.ports[port as usize])
            }
            // Reserved source-segment high bytes: deterministic model readback.
            0x43 | 0x4d | 0x51 if self.model != Model::Mono => Ok(0),
            2 => Ok(((self.cycles / 256) % 159) as u8),
            0xb0 => Ok(self.interrupt().unwrap_or(self.ports[0xb0] & 0xf8)),
            0xb5 => {
                let select = self.ports[0xb5] & 0x70;
                let mut rows = 0;
                if select & 16 != 0 {
                    rows |= (self.buttons >> 8) as u8 & 15;
                }
                if select & 32 != 0 {
                    rows |= (self.buttons >> 4) as u8 & 15;
                }
                if select & 64 != 0 {
                    rows |= self.buttons as u8 & 15;
                }
                Ok(select | rows)
            }
            0xa0 => Ok(if self.model == Model::Mono {
                0x85
            } else {
                0x87
            }),
            0x62 if self.model != Model::Mono => {
                Ok(if self.model == Model::Crystal { 128 } else { 0 })
            }
            0xa8..=0xab => {
                let i = (port - 0xa8) as usize;
                Ok((self.timer[i / 2] >> ((i & 1) * 8)) as u8)
            }
            0xba..=0xbf => Ok(self.internal_eeprom.read(port)),
            0xc4..=0xc9 => Ok(self.eeprom.read(port)),
            0xca..=0xd5 if self.extended() => match port {
                0xca => Ok((if self.rtc_ready { 128 } else { 0 })
                    | (if self.rtc_remaining > 0 || self.rtc_wait > 0 {
                        16
                    } else {
                        0
                    })
                    | (self.rtc_command & 15)),
                0xcb => {
                    if self.rtc_ready && self.rtc_remaining > 0 && self.rtc_command & 1 != 0 {
                        self.rtc_remaining -= 1;
                        self.rtc_wait = 64;
                    }
                    self.rtc_ready = false;
                    Ok(255) // No external RTC is attached: mapper's weak pull-up.
                }
                0xcd => Ok(self.ports[0xcd] & self.ports[0xcc] & 15),
                _ => Ok(self.ports[port as usize]),
            },
            0x91 => Ok(self.ports[0x91] | 128),
            0x92..=0x93 => Ok((self.audio.lfsr >> ((port - 0x92) * 8)) as u8),
            0x96..=0x9b => {
                let pair = (port - 0x96) / 2;
                let value = match pair {
                    0 => self.audio.mix[1],
                    1 => self.audio.mix[0],
                    _ => self.audio.mix[0] + self.audio.mix[1],
                };
                Ok((value >> ((port & 1) * 8)) as u8)
            }
            0x00..=0x16
            | 0x1c..=0x3f
            | 0x80..=0x90
            | 0x94
            | 0x9e
            | 0xa2
            | 0xa4..=0xa7
            | 0xb2
            | 0xb4
            | 0xc0..=0xc3 => Ok(self.ports[port as usize]),
            0x60 if self.model != Model::Mono => Ok(self.ports[0x60]),
            _ => Err(Error::new(format!("Unsupported input port {port:#06x}."))),
        }
    }
    pub fn output(&mut self, port: u16, value: u8) -> Result<(), Error> {
        self.accesses
            .set(self.accesses.get() + 1 + u32::from(port >= 0xc0 && (self.ports[0x60] & 8 != 0)));
        match port {
            // Wide OUT at 42/4C/50 reaches an unused fourth counter byte.
            0x43 | 0x4d | 0x51 if self.model != Model::Mono => return Ok(()),
            0x4a..=0x4c | 0x4e..=0x50 | 0x52 if self.model != Model::Mono => {
                self.sdma.write(&mut self.ports, port as usize, value);
                return Ok(());
            }
            0x64..=0x67 if self.model != Model::Mono => {}
            0x69 if self.model != Model::Mono => {
                self.audio.hyper_input(value, false, &mut self.ports);
                return Ok(());
            }
            0x6a if self.model != Model::Mono => {}
            0x6b if self.model != Model::Mono => {
                if value & 15 != 0 {
                    return Err(Error::new(format!(
                        "Unknown Hyper Voice control bits {:#04x} at 0x6b; low nibble must be zero.", value & 15
                    )));
                }
                self.audio.hyper_control(value);
                self.ports[0x6b] = value & 0x60;
                return Ok(());
            }
            0x40..=0x42 | 0x44..=0x48 if self.model != Model::Mono => {
                let mask = match port {
                    0x40 | 0x44 | 0x46 => 0xfe,
                    0x42 => 0x0f,
                    0x48 => 0xc0,
                    _ => 0xff,
                };
                self.ports[port as usize] = value & mask;
                if port == 0x48 {
                    self.gdma_started = false;
                }
                return Ok(());
            }
            0x60 => {
                if self.model == Model::Mono {
                    return Err(Error::new(
                        "Color video register 0x60 is unavailable on WonderSwan mono.",
                    ));
                }
                if value & 0x60 != 0 && value & 0x80 == 0 {
                    return Err(Error::new(format!(
                        "Invalid color mode {value:#04x}: 4bpp requires color enable."
                    )));
                }
                if value & 0x20 != 0 && value & 0x40 == 0 {
                    return Err(Error::new("Packed mode requires 4bpp."));
                }
            }
            0x16 if value == 158 => {}
            0x16 => {
                return Err(Error::new(
                    "Programmable LCD total lines is unsupported; expected 158.",
                ))
            }
            0xa3 if value == 0 => return Ok(()),
            0xb0 => {
                self.ports[0xb0] = value & 0xf8;
                return Ok(());
            }
            0xb6 => {
                self.ports[0xb4] &= !value;
                return Ok(());
            }
            0xa4..=0xa7 => {
                self.ports[port as usize] = value;
                let i = (port as usize - 0xa4) / 2;
                self.timer[i] =
                    u16::from_le_bytes([self.ports[0xa4 + i * 2], self.ports[0xa5 + i * 2]]);
                return Ok(());
            }
            0xba..=0xbf => {
                let color = self.color_mode();
                return self.internal_eeprom.write(port, value, color);
            }
            0xc4..=0xc9 => {
                let c = self
                    .cartridge
                    .as_ref()
                    .ok_or_else(|| Error::new("No cartridge EEPROM."))?;
                return self.eeprom.write(port, value, c);
            }
            0xca..=0xd5 if self.extended() => match port {
                0xca => {
                    let length = match value {
                        0x10 | 0x11 => 0,
                        0x12 | 0x13 => 1,
                        0x14 | 0x15 => 7,
                        0x16 | 0x17 => 3,
                        0x18..=0x1b => 2,
                        _ => {
                            return Err(Error::new(format!(
                                "Unsupported RTC command {value:#04x}."
                            )))
                        }
                    };
                    self.rtc_command = value;
                    self.rtc_remaining = length;
                    self.rtc_wait = 64;
                    self.rtc_ready = false;
                    if value & 1 == 0 && self.rtc_remaining > 0 {
                        self.rtc_remaining -= 1;
                    }
                }
                0xcb => {
                    if self.rtc_ready && self.rtc_remaining > 0 && self.rtc_command & 1 == 0 {
                        self.rtc_remaining -= 1;
                        self.rtc_wait = 64;
                    }
                    self.rtc_ready = false;
                }
                0xcc | 0xcd => {
                    self.ports[port as usize] = value & 15;
                    return Ok(());
                }
                0xce => {
                    if value & !1 != 0 {
                        return Err(Error::new(format!(
                            "Unsupported self-flash control {value:#04x}."
                        )));
                    }
                }
                0xcf => {
                    self.ports[0xc0] = value & 63;
                    self.ports[0xcf] = value & 63;
                    return Ok(());
                }
                0xd0 | 0xd2 | 0xd4 => {
                    let legacy = 0xc1 + (port as usize - 0xd0) / 2;
                    self.ports[legacy] = value;
                }
                0xd1 | 0xd3 | 0xd5 => {
                    self.ports[port as usize] = value & 3;
                    return Ok(());
                }
                _ => unreachable!(),
            },
            0xc0 => {
                let v = value & if self.extended() { 63 } else { 15 };
                self.ports[0xc0] = v;
                self.ports[0xcf] = v;
                return Ok(());
            }
            0xc1..=0xc3 => {
                self.ports[0xd0 + (port as usize - 0xc1) * 2] = value;
            }
            0x81 | 0x83 | 0x85 | 0x87 | 0x8d | 0x8e | 0x90 | 0x91 | 0x94 | 0x9e => {
                let mask = match port {
                    0x81 | 0x83 | 0x85 | 0x87 => 7,
                    0x8d => 31,
                    0x8e => 0x17,
                    0x90 => 0xef,
                    0x91 | 0x94 => 15,
                    0x9e => 3,
                    _ => unreachable!(),
                };
                self.audio.write(port, value);
                self.ports[port as usize] = value & mask;
                return Ok(());
            }
            0x80 | 0x82 | 0x84 | 0x86 | 0x88..=0x8b | 0x8f => {}
            0x8c => {}
            0x20..=0x3f => {
                let mask = if port & 1 == 0 && ((port - 0x20) / 2) & 4 != 0 {
                    0x70
                } else {
                    0x77
                };
                self.ports[port as usize] = value & mask;
                return Ok(());
            }
            0x00..=0x01 | 0x03..=0x15 | 0x1c..=0x1f | 0xa2 | 0xb2 | 0xb5 => {}
            _ => return Err(Error::new(format!("Unsupported output port {port:#06x}."))),
        }
        self.ports[port as usize] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::tests::rom;
    fn bus(save: u8, mapper: u8) -> Bus {
        let mut b = Bus::new(Model::Crystal);
        b.cartridge = Some(Cartridge::parse(&rom(131072, save, mapper)).unwrap());
        b
    }
    fn out(b: &mut Bus, p: u16, v: u8) {
        b.output(p, v).unwrap();
    }
    #[test]
    fn all_sram_sizes_are_writable_mirrored_and_banked_separately_from_internal_ram() {
        for code in 1..=5 {
            let mut b = bus(code, 0);
            let size = b.cartridge.as_ref().unwrap().save.len();
            assert_eq!(b.read(0x10000), 255);
            for bank in 0..size.div_ceil(65536) {
                out(&mut b, 0xc1, bank as u8);
                b.write(0x10000, bank as u8 + 10);
                b.write(0x1ffff, bank as u8 + 20);
            }
            for bank in 0..size.div_ceil(65536) {
                out(&mut b, 0xc1, bank as u8);
                assert_eq!(b.read(0x10000), bank as u8 + 10);
                assert_eq!(b.read(0x1ffff), bank as u8 + 20);
            }
            out(&mut b, 0xc1, 0);
            b.write(0x10000, 0x99);
            if size == 32768 {
                assert_eq!(b.read(0x18000), 0x99);
            }
            out(&mut b, 0xc1, (size.div_ceil(65536)) as u8);
            assert_eq!(b.read(0x10000), 0x99);
            assert_eq!(b.read(0), 0);
            b.write(0, 42);
            assert_eq!(b.read(0x10000), 0x99);
        }
        let mut no_save = bus(0, 0);
        no_save.write(0x10000, 3);
        assert_eq!(no_save.read(0x10000), 255);
    }
    #[test]
    fn mapper_shared_banks_mirror_and_extended_registers_alias_at_boundaries() {
        for mapper in [0, 1] {
            let mut b = bus(0, mapper);
            b.cartridge.as_mut().unwrap().bytes =
                (0..256).flat_map(|n| vec![n as u8; 65536]).collect();
            assert_eq!(b.read(0xffff0), 255);
            out(&mut b, 0xc2, 254);
            out(&mut b, 0xc3, 255);
            out(&mut b, 0xc0, 0);
            assert_eq!(b.read(0x20000), 254);
            assert_eq!(b.read(0x2ffff), 254);
            assert_eq!(b.read(0x30000), 255);
            assert_eq!(b.read(0x40000), 4);
            b.write(0x20000, 0);
            assert_eq!(b.read(0x20000), 254);
        }
        let mut b = bus(3, 1);
        // Original generated bank markers distinguish all ten bank bits.
        let c = b.cartridge.as_mut().unwrap();
        c.bytes = vec![0; 64 * 1024 * 1024];
        for bank in 0..1024 {
            c.bytes[bank * 65536] = (bank & 255) as u8;
            c.bytes[bank * 65536 + 1] = (bank >> 8) as u8;
        }
        out(&mut b, 0xd2, 255);
        out(&mut b, 0xd3, 3);
        assert_eq!(b.read(0x20000), 255);
        assert_eq!(b.read(0x20001), 3);
        assert_eq!(b.input(0xc2).unwrap(), 255);
        out(&mut b, 0xc2, 0);
        assert_eq!(b.input(0xd2).unwrap(), 0);
        assert_eq!(b.read(0x20001), 3);
        out(&mut b, 0xd4, 1);
        out(&mut b, 0xd5, 2);
        assert_eq!(b.read(0x30000), 1);
        assert_eq!(b.read(0x30001), 2);
        out(&mut b, 0xcf, 63);
        assert_eq!(b.input(0xc0).unwrap(), 63);
        assert_eq!(b.read(0x40000), 244);
        assert_eq!(b.read(0x40001), 3);
        assert_eq!(b.read(0xffff1), 0);
        out(&mut b, 0xc0, 32);
        assert_eq!(b.input(0xcf).unwrap(), 32);
        assert_eq!(b.read(0x40001), 2);
        out(&mut b, 0xd0, 0);
        out(&mut b, 0xd1, 1);
        out(&mut b, 0xce, 1);
        assert_eq!(b.read(0x10001), 1);
        b.write(0x10000, 42);
        assert!(b
            .take_fault()
            .unwrap_err()
            .to_string()
            .contains("self-flash write"));
        assert_eq!(b.read(0x10001), 1);
        out(&mut b, 0xce, 0);
        b.write(0x10000, 77);
        assert_eq!(b.read(0x10000), 77);
        assert!(bus(0, 0).output(0xcf, 0).is_err());
    }
    #[test]
    fn bandai2003_gpo_and_absent_rtc_transactions_have_observable_status() {
        let mut b = bus(0, 1);
        out(&mut b, 0xcd, 15);
        assert_eq!(b.input(0xcd).unwrap(), 0);
        out(&mut b, 0xcc, 5);
        assert_eq!(b.input(0xcd).unwrap(), 5);
        for (cmd, length) in [(0x13, 1), (0x15, 7), (0x17, 3), (0x19, 2), (0x1b, 2)] {
            out(&mut b, 0xca, cmd);
            b.tick(64);
            assert_ne!(b.input(0xca).unwrap() & 16, 0);
            for _ in 0..length {
                assert_eq!(b.input(0xcb).unwrap(), 255);
                assert_eq!(b.input(0xca).unwrap() & 128, 0);
                b.tick(64);
            }
            assert_eq!(b.input(0xca).unwrap(), 0x80 | (cmd & 15));
        }
        for (cmd, length) in [
            (0x10, 0),
            (0x11, 0),
            (0x12, 1),
            (0x14, 7),
            (0x16, 3),
            (0x18, 2),
            (0x1a, 2),
        ] {
            out(&mut b, 0xcb, 0);
            out(&mut b, 0xca, cmd);
            b.tick(64);
            for _ in 1..length {
                out(&mut b, 0xcb, 0);
                b.tick(64);
            }
            assert_eq!(b.input(0xca).unwrap() & 16, 0);
        }
        assert!(b.output(0xca, 0x1f).is_err());
        assert!(b.output(0xce, 2).is_err());
    }
    #[test]
    fn interrupt_priority_enable_latching_ack_and_key_edges() {
        let mut b = bus(0, 0);
        out(&mut b, 0xb0, 0x27);
        b.request(7);
        assert_eq!(b.interrupt(), None);
        out(&mut b, 0xb2, 0xd2);
        for i in [1, 4, 6, 7] {
            b.request(i);
        }
        assert_eq!(b.input(0xb0).unwrap(), 0x27);
        assert_eq!(b.input(0xb4).unwrap(), 0xd2);
        out(&mut b, 0xb6, 0x80);
        assert_eq!(b.interrupt(), Some(0x26));
        out(&mut b, 0xb6, 0x50);
        assert_eq!(b.interrupt(), Some(0x21));
        out(&mut b, 0xb2, 0);
        assert_eq!(b.interrupt(), Some(0x21));
        out(&mut b, 0xb6, 2);
        assert_eq!(b.interrupt(), None);
        out(&mut b, 0xb2, 2);
        b.set_buttons(16);
        assert_eq!(b.interrupt(), Some(0x21));
        out(&mut b, 0xb6, 2);
        b.set_buttons(16);
        assert_eq!(b.interrupt(), None);
    }
    #[test]
    fn shared_timebase_drives_scanlines_vblank_line_match_and_both_timers() {
        let mut b = bus(0, 0);
        out(&mut b, 0xb2, 0xf0);
        out(&mut b, 3, 2);
        out(&mut b, 0xa4, 2);
        out(&mut b, 0xa6, 1);
        out(&mut b, 0xa2, 0x0f);
        b.tick(255);
        assert_eq!(b.input(2).unwrap(), 0);
        assert_eq!(b.input(0xa8).unwrap(), 2);
        b.tick(1);
        assert_eq!(b.input(0xa8).unwrap(), 1);
        b.tick(256);
        assert_eq!(b.input(0xa8).unwrap(), 2);
        assert_eq!(b.input(0xb4).unwrap(), 0x90);
        out(&mut b, 0xb6, 255);
        b.tick(256 * 142);
        assert_eq!(b.input(2).unwrap(), 144);
        assert_eq!(b.input(0xb4).unwrap() & 0x60, 0x60);
        assert_eq!(b.input(0xaa).unwrap(), 1);
        b.tick(256 * 15);
        assert_eq!(b.input(2).unwrap(), 0);
        // Countdown disabled still triggers at one (documented hardware quirk).
        out(&mut b, 0xa2, 0);
        out(&mut b, 0xb6, 255);
        out(&mut b, 0xa4, 1);
        b.tick(256);
        assert_eq!(b.input(0xb4).unwrap(), 0x80);
        // One-shot reaches zero and stays there.
        out(&mut b, 0xa2, 1);
        b.tick(256);
        assert_eq!(b.input(0xa8).unwrap(), 0);
        out(&mut b, 0xb6, 255);
        b.tick(256);
        assert_eq!(b.input(0xb4).unwrap(), 0);
    }
    #[test]
    fn keypad_model_gates_color_ram_and_ports() {
        let mut b = bus(0, 0);
        b.set_buttons(0x842);
        for (selection, rows) in [(0, 0), (16, 8), (32, 4), (64, 2), (0x70, 14)] {
            out(&mut b, 0xb5, selection);
            assert_eq!(b.input(0xb5).unwrap(), selection | rows);
        }
        b.write(0x4000, 7);
        assert_eq!(b.read(0x4000), 255);
        out(&mut b, 0x60, 0x80);
        b.write(0x4000, 7);
        assert_eq!(b.read(0x104000), 7);
        assert_eq!(b.input(0x62).unwrap(), 128);
        b.model = Model::Color;
        assert_eq!(b.input(0x62).unwrap(), 0);
        b.model = Model::Mono;
        assert!(b.output(0x60, 0x80).is_err());
        assert_eq!(b.read(0x4000), 255);
        assert!(b.input(0x1234).is_err());
    }
}
