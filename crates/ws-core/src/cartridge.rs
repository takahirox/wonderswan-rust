use crate::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SaveKind {
    None,
    Sram(usize),
    Eeprom(usize),
}
impl SaveKind {
    pub fn len(self) -> usize {
        match self {
            Self::None => 0,
            Self::Sram(n) | Self::Eeprom(n) => n,
        }
    }
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }
}
/// Header 00 describes the common interface, not proof of a physical 2001 chip.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Mapper {
    Common,
    Bandai2003,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HardwareModel {
    Mono,
    Color,
}
#[derive(Clone, Debug)]
pub struct Header {
    pub entry_offset: u16,
    pub entry_segment: u16,
    pub maintenance: u8,
    pub publisher: u8,
    pub model: HardwareModel,
    pub game_id: u8,
    pub version: u8,
    pub internal_eeprom_write_protected: bool,
    pub rom_size: usize,
    pub save: SaveKind,
    pub orientation: Orientation,
    pub rom_word_bus: bool,
    pub rom_wait: bool,
    pub mapper: Mapper,
    pub checksum: u16,
}
#[derive(Clone, Debug)]
pub struct Cartridge {
    pub bytes: Vec<u8>,
    pub header: Header,
    pub save: Vec<u8>,
}
impl Cartridge {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() < 131_072 || bytes.len() > 67_108_864 || !bytes.len().is_power_of_two() {
            return Err(Error::new(format!("Expected a 128 KiB–64 MiB power-of-two WonderSwan ROM; file size is {} bytes. Non-power-of-two dumps require documented leading padding.", bytes.len())));
        }
        let h = &bytes[bytes.len() - 16..];
        if h[0] != 0xea {
            return Err(Error::new(format!(
                "Invalid entry opcode {:#04x}; expected far JMP EA.",
                h[0]
            )));
        }
        if h[5] & 15 != 0 {
            return Err(Error::new(format!(
                "Invalid maintenance field {:#04x}: low nibble must be zero.",
                h[5]
            )));
        }
        let model = match h[7] {
            0 => HardwareModel::Mono,
            1 => HardwareModel::Color,
            n => return Err(Error::new(format!("Unsupported model field {n:#04x}."))),
        };
        let sizes = [
            131072, 262144, 524288, 1048576, 2097152, 3145728, 4194304, 6291456, 8388608, 16777216,
            33554432, 67108864,
        ];
        let size = *sizes
            .get(h[10] as usize)
            .ok_or_else(|| Error::new(format!("Unsupported ROM size code {:#04x}.", h[10])))?;
        if size != bytes.len() {
            return Err(Error::new(format!(
                "ROM size code {:#04x} declares {size} bytes; file has {}.",
                h[10],
                bytes.len()
            )));
        }
        let save = match h[11] {
            0 => SaveKind::None,
            1 | 2 => SaveKind::Sram(32768),
            3 => SaveKind::Sram(131072),
            4 => SaveKind::Sram(262144),
            5 => SaveKind::Sram(524288),
            0x10 => SaveKind::Eeprom(128),
            0x20 => SaveKind::Eeprom(2048),
            0x50 => SaveKind::Eeprom(1024),
            n => {
                return Err(Error::new(format!(
                "Unsupported save type {n:#04x}; supported SRAM codes 01–05 and EEPROM 10/20/50."
            )))
            }
        };
        if h[12] & !0x0d != 0 {
            return Err(Error::new(format!("Unsupported cartridge flags {:#04x}; supported mask is 0x0d (orientation, ROM width and wait).",h[12])));
        }
        let mapper = match h[13] {
            0 => Mapper::Common,
            1 => Mapper::Bandai2003,
            n => {
                return Err(Error::new(format!(
                    "Unsupported mapper field {n:#04x}; expected 00 (common) or 01 (Bandai 2003)."
                )))
            }
        };
        if size > 16777216 && mapper == Mapper::Common {
            return Err(Error::new("Mapper 00 cannot address ROM sizes above 16 MiB; extended banking requires mapper 01."));
        }
        let checksum = u16::from_le_bytes([h[14], h[15]]);
        let actual = bytes[..bytes.len() - 2]
            .iter()
            .fold(0u16, |sum, b| sum.wrapping_add(*b as u16));
        if checksum != actual {
            return Err(Error::new(format!(
                "Cartridge checksum mismatch: header {checksum:04X}, calculated {actual:04X}."
            )));
        }
        Ok(Self {
            bytes: bytes.to_vec(),
            save: vec![0xff; save.len()],
            header: Header {
                entry_offset: u16::from_le_bytes([h[1], h[2]]),
                entry_segment: u16::from_le_bytes([h[3], h[4]]),
                maintenance: h[5],
                publisher: h[6],
                model,
                game_id: h[8],
                version: h[9] & 127,
                internal_eeprom_write_protected: h[9] & 128 == 0,
                rom_size: size,
                save,
                orientation: if h[12] & 1 == 0 {
                    Orientation::Horizontal
                } else {
                    Orientation::Vertical
                },
                rom_word_bus: h[12] & 4 != 0,
                rom_wait: h[12] & 8 != 0,
                mapper,
                checksum,
            },
        })
    }
    pub fn rom(&self, bank: usize, offset: usize) -> u8 {
        self.bytes[((bank << 16) | offset) & (self.bytes.len() - 1)]
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn rom(size: usize, save: u8, mapper: u8) -> Vec<u8> {
        let mut bytes = vec![0xff; size];
        let code = match size {
            131072 => 0,
            262144 => 1,
            524288 => 2,
            1048576 => 3,
            2097152 => 4,
            4194304 => 6,
            8388608 => 8,
            16777216 => 9,
            33554432 => 10,
            67108864 => 11,
            _ => panic!("fixture size"),
        };
        bytes[size - 16..].copy_from_slice(&[
            0xea, 0, 0, 0, 0xf0, 0, 0, 0, 0, 0x80, code, save, 0, mapper, 0, 0,
        ]);
        checksum(&mut bytes);
        bytes
    }
    pub fn checksum(bytes: &mut [u8]) {
        let len = bytes.len();
        let sum = bytes[..len - 2]
            .iter()
            .fold(0u16, |n, b| n.wrapping_add(*b as u16));
        bytes[len - 2..].copy_from_slice(&sum.to_le_bytes());
    }
    #[test]
    fn fields_have_independent_actionable_errors() {
        for (offset, value, diagnostic) in [
            (0, 0, "entry"),
            (5, 1, "maintenance"),
            (7, 2, "model"),
            (10, 1, "size"),
            (11, 0x11, "save type"),
            (12, 0x80, "flags"),
            (13, 2, "mapper"),
        ] {
            let mut r = rom(131072, 0, 0);
            r[131072 - 16 + offset] = value;
            checksum(&mut r);
            let error = Cartridge::parse(&r).unwrap_err().to_string();
            assert!(error.contains(diagnostic), "{error}");
        }
        let mut r = rom(131072, 0, 0);
        r[100] ^= 1;
        assert!(Cartridge::parse(&r)
            .unwrap_err()
            .to_string()
            .contains("checksum"));
        for code in [0, 1, 2, 3, 4, 5, 0x10, 0x20, 0x50] {
            let c = Cartridge::parse(&rom(131072, code, 0)).unwrap();
            assert_eq!(c.save.len(), c.header.save.len());
            assert!(c.save.iter().all(|b| *b == 255));
        }
        assert!(Cartridge::parse(&rom(33554432, 0, 0))
            .unwrap_err()
            .to_string()
            .contains("16 MiB"));
    }
}
