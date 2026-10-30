//! Internal console EEPROM, independent of cartridge controller/mapper quirks.
//! Word transfers and conservative word-zero ERASE; see docs/issue-91-validation.md.
//! Fixed delays are approximations, not measurements.
use crate::{Error, Model};

pub(crate) fn initial_data(model: Model) -> Vec<u8> {
    // Synthetic, deterministic settings: empty owner, unknown personal fields and
    // zero counters/options. No firmware image, clock, identity or factory dump.
    let mut bytes = vec![0xff; if model == Model::Mono { 128 } else { 2048 }];
    bytes[0x60..].fill(0);
    bytes
}

#[derive(Clone, Copy)]
enum Operation {
    Read(usize),
    Write(usize, u16),
    Erase(usize),
    Enable(bool),
    Locked,
}
#[derive(Default)]
pub(crate) struct InternalEeprom {
    data: u16,
    command: u16,
    read_data: u16,
    enabled: bool,
    protected: bool,
    done: bool,
    pending: Option<(u32, Operation)>,
}
impl InternalEeprom {
    /// Minimal firmware-free handoff: firmware normally unlocks ordinary writes
    /// and protects settings unless the cartridge header requests an exception.
    pub fn handoff(&mut self, protected: bool) {
        self.enabled = true;
        self.protected = protected;
    }
    pub fn read(&self, port: u16) -> u8 {
        match port {
            0xba => self.read_data as u8,
            0xbb => (self.read_data >> 8) as u8,
            0xbc => self.command as u8,
            0xbd => (self.command >> 8) as u8,
            0xbe => {
                u8::from(self.done)
                    | if self.pending.is_none() { 2 } else { 0 }
                    | if self.protected { 0x80 } else { 0 }
            }
            // Reserved high byte of word IN/OUT BE: zero reads, ignored writes.
            0xbf => 0,
            _ => unreachable!(),
        }
    }
    pub fn write(&mut self, port: u16, value: u8, color: bool) -> Result<(), Error> {
        match port {
            0xba => self.data = (self.data & 0xff00) | u16::from(value),
            0xbb => self.data = (self.data & 255) | (u16::from(value) << 8),
            0xbc => self.command = (self.command & 0xff00) | u16::from(value),
            0xbd => self.command = (self.command & 255) | (u16::from(value) << 8),
            0xbf => {}
            0xbe => {
                // Protection is sticky until a new handoff. It is never an abort.
                self.protected |= value & 0x80 != 0;
                let control = value & 0x70;
                if control == 0 {
                    return Ok(());
                }
                if self.pending.is_some() {
                    return Err(Error::new("Internal EEPROM controller is busy."));
                }
                let bits = if color { 10 } else { 6 };
                let op = (self.command >> bits) & 3;
                let sub = (self.command >> (bits - 2)) & 3;
                let offset = usize::from(self.command & ((1 << bits) - 1)) * 2;
                if self.command >> (bits + 2) != 1 {
                    return Err(Error::new(format!(
                        "Unsupported internal EEPROM command format {:#06x} in {} mode.",
                        self.command,
                        if color { "Color" } else { "mono" }
                    )));
                }
                let operation = match (op, sub, control) {
                    (2, _, 0x10) => Operation::Read(offset),
                    (1, _, 0x20) if self.enabled => Operation::Write(offset, self.data),
                    (1, _, 0x20) => Operation::Locked,
                    // Public libws defines this sequence, but the hardware
                    // experiment questions nonzero addresses. Do not generalize
                    // or redirect those requests to word zero.
                    (3, _, 0x40) if offset != 0 => return Err(Error::new(format!(
                        "Unsupported internal EEPROM ERASE at byte offset {offset:#05x}: nonzero-word effects are unverified; only word zero is supported."
                    ))),
                    (3, _, 0x40) if self.enabled => Operation::Erase(offset),
                    (3, _, 0x40) => Operation::Locked,
                    (0, 0, 0x40) => Operation::Enable(false),
                    (0, 3, 0x40) => Operation::Enable(true),
                    (0, sub @ (1 | 2), 0x20 | 0x40) => return Err(Error::new(format!(
                        "Unsupported internal EEPROM {}: internal-controller support and effects are unverified; storage is unchanged.",
                        if sub == 1 { "WRAL" } else { "ERAL" }
                    ))),
                    _ => return Err(Error::new(format!(
                        "Unsupported internal EEPROM command {:#06x} / control {value:#04x}.", self.command
                    ))),
                };
                // Internal DONE clears on reads (no cartridge 2001 stale-DONE quirk).
                if control == 0x10 {
                    self.done = false;
                }
                self.pending = Some((if control == 0x10 { 32 } else { 3072 }, operation));
            }
            _ => unreachable!(),
        }
        Ok(())
    }
    pub fn tick(&mut self, cycles: u32, bytes: &mut [u8]) {
        let Some((remaining, operation)) = self.pending else {
            return;
        };
        if remaining > cycles {
            self.pending = Some((remaining - cycles, operation));
            return;
        }
        self.pending = None;
        match operation {
            Operation::Read(offset) => {
                self.read_data = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
                self.done = true;
            }
            Operation::Write(offset, word) if !self.protected || offset < 0x60 => {
                bytes[offset..offset + 2].copy_from_slice(&word.to_le_bytes());
            }
            Operation::Erase(offset) if !self.protected || offset < 0x60 => {
                bytes[offset..offset + 2].fill(0xff);
            }
            Operation::Enable(enabled) => self.enabled = enabled,
            _ => {}
        }
    }
}
