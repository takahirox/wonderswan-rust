use crate::{cartridge::Cartridge, Error, Model};

pub(crate) struct Bus {
    pub ram: Vec<u8>,
    pub ports: [u8; 256],
    pub cartridge: Option<Cartridge>,
    pub buttons: u16,
    pub cycles: u64,
    pub model: Model,
}

impl Bus {
    pub fn new(model: Model) -> Self {
        let mut ports = [0; 256];
        ports[0xc0] = 0x0f;
        ports[0xc2] = 0xff;
        ports[0xc3] = 0xff;
        Self {
            ram: vec![0; 65536],
            ports,
            cartridge: None,
            buttons: 0,
            cycles: 0,
            model,
        }
    }

    pub fn read(&self, address: u32) -> u8 {
        let a = address as usize & 0xfffff;
        if a < 65536 {
            return if self.model != Model::Mono || a < 0x4000 {
                self.ram[a]
            } else {
                0xff
            };
        }
        if a < 0x20000 {
            return 0xff;
        } // No SRAM in the supported cartridge subset.
        let Some(cart) = &self.cartridge else {
            return 0xff;
        };
        let bank = match a >> 16 {
            2 => self.ports[0xc2] as usize,
            3 => self.ports[0xc3] as usize,
            n => ((self.ports[0xc0] as usize & 15) << 4) | n,
        };
        cart.bytes[((bank << 16) | (a & 65535)) & (cart.bytes.len() - 1)]
    }

    pub fn write(&mut self, address: u32, value: u8) {
        let a = address as usize & 0xfffff;
        if a < 65536 && (self.model != Model::Mono || a < 0x4000) {
            self.ram[a] = value;
        }
    }

    pub fn input(&self, port: u16) -> Result<u8, Error> {
        match port {
            0x02 => Ok(((self.cycles / 256) % 159) as u8),
            0xb5 => {
                let select = self.ports[0xb5] & 0x70;
                let mut rows = 0;
                if select & 0x10 != 0 {
                    rows |= (self.buttons >> 8) as u8 & 15;
                }
                if select & 0x20 != 0 {
                    rows |= (self.buttons >> 4) as u8 & 15;
                }
                if select & 0x40 != 0 {
                    rows |= self.buttons as u8 & 15;
                }
                Ok(select | rows)
            }
            0xa0 => Ok(if self.model == Model::Mono {
                0x85
            } else {
                0x87
            }),
            0x00 | 0x01 | 0x07 | 0x10 | 0x11 | 0x14 | 0x1c..=0x3f | 0xc0..=0xc3 => {
                Ok(self.ports[port as usize])
            }
            _ => Err(Error::new(format!("Unsupported input port {port:#06x}."))),
        }
    }

    pub fn output(&mut self, port: u16, value: u8) -> Result<(), Error> {
        match port {
            0 if value & !1 != 0 => return Err(Error::new("Only Screen 1 video is implemented.")),
            0x07 if value & !7 != 0 => {
                return Err(Error::new("Only mono Screen 1 tile maps are implemented."))
            }
            0x00 | 0x01 | 0x07 | 0x10 | 0x11 | 0x14 | 0x1c..=0x3f | 0xb5 | 0xc0..=0xc3 => {}
            _ => return Err(Error::new(format!("Unsupported output port {port:#06x}."))),
        }
        self.ports[port as usize] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cartridge_banks_mirror_physical_size_and_rom_writes_are_ignored() {
        let mut bus = Bus::new(Model::Mono);
        let bytes = (0..4).flat_map(|b| vec![b; 65536]).collect();
        bus.cartridge = Some(Cartridge {
            bytes,
            color: false,
        });
        assert_eq!(bus.read(0xffff0), 3);
        assert_eq!(bus.read(0x20000), 3);
        bus.output(0xc2, 2).unwrap();
        bus.output(0xc3, 1).unwrap();
        bus.output(0xc0, 0).unwrap();
        assert_eq!(bus.read(0x20000), 2);
        assert_eq!(bus.read(0x30000), 1);
        assert_eq!(bus.read(0x40000), 0);
        assert_eq!(bus.read(0x50000), 1);
        bus.write(0x20000, 9);
        assert_eq!(bus.read(0x20000), 2);
        assert_eq!(bus.read(0x10000), 255);
        bus.write(0x1400, 42);
        assert_eq!(bus.read(0x101400), 42); // 20-bit bus wrap
        bus.write(0x4000, 7);
        assert_eq!(bus.read(0x4000), 255);
        let mut color = Bus::new(Model::Color);
        color.write(0x4000, 7);
        assert_eq!(color.read(0x4000), 7);
    }

    #[test]
    fn keypad_matrix_selection_combines_rows_and_scanline_wraps() {
        let mut bus = Bus::new(Model::Crystal);
        bus.buttons = 0x842; // Y4, X3, Start
        for (select, rows) in [(0, 0), (0x10, 8), (0x20, 4), (0x40, 2), (0x70, 14)] {
            bus.output(0xb5, select).unwrap();
            assert_eq!(bus.input(0xb5).unwrap(), select | rows);
        }
        for (cycles, line) in [
            (0, 0),
            (255, 0),
            (256, 1),
            (36864, 144),
            (40700, 158),
            (40704, 0),
        ] {
            bus.cycles = cycles;
            assert_eq!(bus.input(2).unwrap(), line);
        }
        assert!(bus.output(0, 2).is_err());
        assert!(bus.output(7, 8).is_err());
        assert!(bus.input(0x1234).is_err());
        assert!(bus.output(0x1234, 0).is_err());
    }
}
