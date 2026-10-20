//! Cartridge Microwire controller. Commands complete after a deterministic delay;
//! analogue programming/busy durations are approximated, data buffers are separate.
use crate::{
    cartridge::{Cartridge, Mapper, SaveKind},
    Error,
};
#[derive(Default)]
pub(crate) struct Eeprom {
    pub data: u16,
    pub command: u16,
    read_data: u16,
    enabled: bool,
    done: bool,
    pending: Option<(u32, u16, u16, u8)>,
}
impl Eeprom {
    pub fn read(&self, port: u16) -> u8 {
        match port {
            0xc4 => self.read_data as u8,
            0xc5 => (self.read_data >> 8) as u8,
            0xc6 => self.command as u8,
            0xc7 => (self.command >> 8) as u8,
            0xc8 => u8::from(self.done) | if self.pending.is_none() { 2 } else { 0 },
            _ => 0,
        }
    }
    pub fn write(&mut self, port: u16, value: u8, cart: &Cartridge) -> Result<(), Error> {
        if !matches!(cart.header.save, SaveKind::Eeprom(_)) {
            return Err(Error::new(
                "Cartridge EEPROM access requires an EEPROM save type.",
            ));
        }
        match port {
            0xc4 => self.data = (self.data & 0xff00) | value as u16,
            0xc5 => self.data = (self.data & 255) | ((value as u16) << 8),
            0xc6 => self.command = (self.command & 0xff00) | value as u16,
            0xc7 => self.command = (self.command & 255) | ((value as u16) << 8),
            0xc8 => {
                let mode = value & 0xf0;
                if mode & 0x80 != 0 || (mode & 0x70).count_ones() != 1 {
                    self.pending = None;
                    return Ok(());
                }
                if self.pending.is_some() {
                    return Err(Error::new("Cartridge EEPROM controller is busy."));
                }
                let bits = if cart.save.len() == 128 { 6 } else { 10 };
                let op = (self.command >> bits) & 3;
                let sub = (self.command >> (bits - 2)) & 3;
                let expected = match op {
                    2 => 0x10,
                    1 => 0x20,
                    3 => 0x40,
                    _ => {
                        if sub == 1 {
                            0x20
                        } else {
                            0x40
                        }
                    }
                };
                if self.command >> (bits + 2) != 1 || mode != expected {
                    return Err(Error::new(format!(
                        "Unsupported EEPROM command {:#06x} / control {value:#04x} for {} bytes.",
                        self.command,
                        cart.save.len()
                    )));
                }
                if (cart.header.mapper == Mapper::Common && mode != 0x10)
                    || (cart.header.mapper == Mapper::Bandai2003 && mode == 0x10)
                {
                    self.done = false;
                }
                self.pending = Some((
                    if mode == 0x10 { 32 } else { 3072 },
                    self.command,
                    self.data,
                    mode,
                ));
            }
            0xc9 => {}
            _ => unreachable!(),
        }
        Ok(())
    }
    pub fn tick(&mut self, cycles: u32, cart: &mut Cartridge) {
        let Some((remaining, command, data, mode)) = self.pending else {
            return;
        };
        if remaining > cycles {
            self.pending = Some((remaining - cycles, command, data, mode));
            return;
        }
        self.pending = None;
        let bits = if cart.save.len() == 128 { 6 } else { 10 };
        let op = (command >> bits) & 3;
        let offset = (command as usize & (cart.save.len() / 2 - 1)) * 2;
        let word = data.to_le_bytes();
        match op {
            2 => {
                self.read_data = u16::from_le_bytes([cart.save[offset], cart.save[offset + 1]]);
                self.done = true;
            }
            1 if self.enabled => cart.save[offset..offset + 2].copy_from_slice(&word),
            3 if self.enabled => cart.save[offset..offset + 2].fill(255),
            0 => match (command >> (bits - 2)) & 3 {
                0 => self.enabled = false,
                3 => self.enabled = true,
                1 if self.enabled => {
                    for w in cart.save.chunks_exact_mut(2) {
                        w.copy_from_slice(&word);
                    }
                }
                2 if self.enabled => cart.save.fill(255),
                _ => {}
            },
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cartridge::tests::rom;
    fn command(e: &mut Eeprom, c: &Cartridge, command: u16, control: u8) {
        e.write(0xc6, command as u8, c).unwrap();
        e.write(0xc7, (command >> 8) as u8, c).unwrap();
        e.write(0xc8, control, c).unwrap();
    }
    fn data(e: &mut Eeprom, c: &Cartridge, v: u16) {
        e.write(0xc4, v as u8, c).unwrap();
        e.write(0xc5, (v >> 8) as u8, c).unwrap();
    }
    fn read(e: &mut Eeprom, c: &mut Cartridge, cmd: u16) -> u16 {
        command(e, c, cmd, 0x10);
        assert_eq!(e.read(0xc8) & 2, 0);
        e.tick(32, c);
        assert_eq!(e.read(0xc8) & 3, 3);
        u16::from_le_bytes([e.read(0xc4), e.read(0xc5)])
    }
    #[test]
    fn all_documented_sizes_support_read_write_erase_enable_disable_and_all_commands() {
        for code in [0x10, 0x20, 0x50] {
            let mut c = Cartridge::parse(&rom(131072, code, 0)).unwrap();
            let mut e = Eeprom::default();
            let bits = if code == 0x10 { 6 } else { 10 };
            let start = 1 << (bits + 2);
            let last = (c.save.len() / 2 - 1) as u16;
            assert_eq!(read(&mut e, &mut c, start | (2 << bits) | last), 65535);
            data(&mut e, &c, 0x1234);
            command(&mut e, &c, start | (1 << bits) | last, 0x20);
            e.tick(3072, &mut c);
            assert!(c.save.iter().all(|b| *b == 255)); // write disabled
            command(&mut e, &c, start | (3 << (bits - 2)), 0x40);
            e.tick(3072, &mut c);
            data(&mut e, &c, 0x1234);
            command(&mut e, &c, start | (1 << bits) | last, 0x20);
            data(&mut e, &c, 0x9999);
            e.tick(3071, &mut c);
            assert_eq!(e.read(0xc8) & 2, 0);
            e.tick(1, &mut c);
            assert_eq!(read(&mut e, &mut c, start | (2 << bits) | last), 0x1234);
            assert_eq!(e.read(0xc6), (start | (2 << bits) | last) as u8);
            // Writes do not affect the separate read buffer.
            command(&mut e, &c, start | (3 << bits) | last, 0x40);
            e.tick(3072, &mut c);
            assert_eq!(e.read(0xc4), 0x34);
            assert_eq!(read(&mut e, &mut c, start | (2 << bits) | last), 65535);
            data(&mut e, &c, 0xabcd);
            command(&mut e, &c, start | (1 << (bits - 2)), 0x20);
            e.tick(3072, &mut c);
            assert!(c.save.chunks_exact(2).all(|w| w == [0xcd, 0xab]));
            command(&mut e, &c, start | (2 << (bits - 2)), 0x40);
            e.tick(3072, &mut c);
            assert!(c.save.iter().all(|b| *b == 255));
            command(&mut e, &c, start, 0x40);
            e.tick(3072, &mut c);
            data(&mut e, &c, 123);
            command(&mut e, &c, start | (1 << bits), 0x20);
            e.tick(3072, &mut c);
            assert_eq!(read(&mut e, &mut c, start | (2 << bits)), 65535);
            // Invalid controller combinations/abort do not change storage.
            e.write(0xc8, 0x30, &c).unwrap();
            assert_eq!(e.read(0xc8) & 2, 2);
            command(&mut e, &c, start | (2 << bits), 0x10);
            e.write(0xc8, 0x80, &c).unwrap();
            e.tick(3072, &mut c);
            assert!(c.save.iter().all(|b| *b == 255));
        }
    }
}

#[cfg(test)]
mod status_tests {
    use super::*;
    #[test]
    fn mapper2003_clears_read_done_on_read_but_preserves_it_on_other_commands() {
        let mut c = Cartridge::parse(&crate::cartridge::tests::rom(131072, 0x10, 1)).unwrap();
        let mut e = Eeprom {
            command: 0x180,
            ..Eeprom::default()
        };
        e.write(0xc8, 0x10, &c).unwrap();
        e.tick(32, &mut c);
        assert_eq!(e.read(0xc8), 3);
        e.write(0xc8, 0x10, &c).unwrap();
        assert_eq!(e.read(0xc8), 0);
        e.tick(32, &mut c);
        assert_eq!(e.read(0xc8), 3);
        e.command = 0x130;
        e.write(0xc8, 0x40, &c).unwrap();
        assert_eq!(e.read(0xc8), 1);
        e.tick(3072, &mut c);
        assert_eq!(e.read(0xc8), 3);
    }
}
