//! From-scratch, browser-independent WonderSwan hardware subset.
#[cfg(test)]
mod a0_tests;
mod audio;
mod bus;
mod cartridge;
mod cpu;
mod eeprom;
#[cfg(test)]
mod gdma_tests;
mod internal_eeprom;
#[cfg(test)]
mod internal_eeprom_tests;
#[cfg(test)]
mod lcd_tests;
#[cfg(test)]
mod raw_rom_tests;
mod sdma;
#[cfg(test)]
mod sound_tests;
#[cfg(test)]
mod startup_tests;
#[cfg(test)]
mod sweep_tests;
mod video;

pub use audio::SAMPLE_RATE;
pub use cartridge::{HardwareModel, Header, Mapper, Orientation, SaveKind};
pub use cpu::Cpu;
use std::fmt;

pub const FRAME_WIDTH: usize = 224;
pub const FRAME_HEIGHT: usize = 144;
pub const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;
/// Default cadence only; runtime frame completion follows the raster.
pub const CYCLES_PER_FRAME: u64 = 256 * 159;
pub const FRAME_RATE: f64 = 3_072_000.0 / CYCLES_PER_FRAME as f64;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Model {
    Mono,
    Color,
    #[default]
    Crystal,
}
impl Model {
    pub(crate) fn index(self) -> usize {
        match self {
            Self::Mono => 0,
            Self::Color => 1,
            Self::Crystal => 2,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Mono => "WonderSwan (mono)",
            Self::Color => "WonderSwan Color",
            Self::Crystal => "SwanCrystal",
        }
    }
}

/// Keypad matrix convention: yyyy xxxx bas0 (active high).
#[derive(Clone, Copy)]
#[repr(u16)]
pub enum Button {
    Start = 2,
    A = 4,
    B = 8,
    Up = 16,
    Right = 32,
    Down = 64,
    Left = 128,
    Y1 = 256,
    Y2 = 512,
    Y3 = 1024,
    Y4 = 2048,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Error(String);
impl Error {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Error {}

pub struct Machine {
    bus: bus::Bus,
    cpu: Cpu,
    framebuffer: Vec<u8>,
    fault: Option<Error>,
}
impl Default for Machine {
    fn default() -> Self {
        Self::new(Model::default())
    }
}
impl Machine {
    pub fn new(model: Model) -> Self {
        let mut machine = Self {
            bus: bus::Bus::new(model),
            cpu: Cpu::default(),
            framebuffer: vec![0; FRAME_BYTES],
            fault: None,
        };
        video::render(&machine.bus, &mut machine.framebuffer);
        machine
    }
    pub fn model(&self) -> Model {
        self.bus.model
    }
    /// Clear cartridge, CPU, input, RAM, ports and display, even for the same model.
    /// Retain completed console EEPROM bytes for all three physical models.
    pub fn reset(&mut self, model: Model) {
        let console_data = std::mem::take(&mut self.bus.console_data);
        *self = Self::new(model);
        self.bus.console_data = console_data;
    }
    /// Validate before replacing the running machine. Failed loads preserve it.
    /// Successful cartridge entry uses FFFF:0000, SP=2000 and physical-model DS.
    pub fn load_rom(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        let cartridge = cartridge::Cartridge::parse(bytes)?;
        if cartridge.header.model == HardwareModel::Color && self.model() == Model::Mono {
            return Err(Error::new("Color cartridges (model field 0x01) require a color model; WonderSwan mono was selected."));
        }
        let len = cartridge.bytes.len();
        self.reset(self.model());
        self.cartridge_handoff(cartridge);
        Ok(len)
    }
    /// Restart a loaded cartridge at FFFF:0000 with SP=2000 and DS=FF00 (Mono)
    /// or FE00 (Color/Crystal). Without a cartridge, retain unloaded defaults.
    pub fn restart(&mut self) {
        let cartridge = self.bus.cartridge.take();
        self.reset(self.model());
        if let Some(c) = cartridge {
            self.cartridge_handoff(c);
        }
    }
    /// Switch the console and restart its volatile state, preserving the cartridge
    /// save and the independent console EEPROM record for every model.
    /// Loaded cartridges receive fresh physical-model SP/DS handoff values.
    /// Reject incompatible models before replacing any state.
    pub fn change_model(&mut self, model: Model) -> Result<(), Error> {
        if model == Model::Mono
            && self
                .bus
                .cartridge
                .as_ref()
                .is_some_and(|c| c.header.model == HardwareModel::Color)
        {
            return Err(Error::new("Color cartridges (model field 0x01) require a color model; WonderSwan mono was selected."));
        }
        let cartridge = self.bus.cartridge.take();
        self.reset(model);
        if let Some(c) = cartridge {
            self.cartridge_handoff(c);
        }
        Ok(())
    }
    fn cartridge_handoff(&mut self, cartridge: cartridge::Cartridge) {
        self.bus.insert_cartridge(cartridge);
        self.cpu = Cpu::cartridge_handoff(self.model());
    }
    pub fn cartridge_header(&self) -> Option<&Header> {
        self.bus.cartridge.as_ref().map(|c| &c.header)
    }
    /// Completed console EEPROM bytes for the active physical model, independent
    /// of the cartridge save and active mono/Color command mode.
    pub fn console_data(&self) -> &[u8] {
        &self.bus.console_data[self.model().index()]
    }
    /// Length-checked import for the active model. Failure preserves all state;
    /// success cancels pending transfers so they cannot overwrite imported bytes.
    pub fn load_console_data(&mut self, data: &[u8]) -> Result<(), Error> {
        let expected = self.console_data().len();
        if data.len() != expected {
            return Err(Error::new(format!(
                "Console EEPROM size mismatch: expected {expected} bytes, got {}.",
                data.len()
            )));
        }
        self.bus.console_data[self.model().index()].copy_from_slice(data);
        self.bus.reset_internal_eeprom();
        Ok(())
    }
    pub fn save_data(&self) -> &[u8] {
        self.bus
            .cartridge
            .as_ref()
            .map_or(&[], |c| c.save.as_slice())
    }
    /// A length mismatch cannot overwrite the currently loaded save.
    pub fn load_save(&mut self, data: &[u8]) -> Result<(), Error> {
        let c = self
            .bus
            .cartridge
            .as_mut()
            .ok_or_else(|| Error::new("Load a cartridge before importing save data."))?;
        if data.len() != c.save.len() {
            return Err(Error::new(format!(
                "Save size mismatch: expected {} bytes, got {}.",
                c.save.len(),
                data.len()
            )));
        }
        c.save.copy_from_slice(data);
        Ok(())
    }
    /// Interleaved stereo samples. Taking samples also bounds frontend latency.
    pub fn audio_samples(&mut self) -> Vec<f32> {
        self.bus.audio.drain()
    }
    pub fn rom_len(&self) -> usize {
        self.bus.cartridge.as_ref().map_or(0, |c| c.bytes.len())
    }
    pub fn set_buttons(&mut self, mask: u16) {
        self.bus.set_buttons(mask & 0xffe);
    }
    pub fn cpu(&self) -> &Cpu {
        &self.cpu
    }
    pub fn read_memory(&self, address: u32) -> u8 {
        self.bus.read(address)
    }
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
    pub fn frame_number(&self) -> u64 {
        self.bus.completed_frames
    }
    pub fn frame_rate(&self) -> f64 {
        3_072_000.0 / (256.0 * (f64::from(self.bus.ports[0x16]) + 1.0))
    }
    pub fn run_frame(&mut self) -> Result<(), Error> {
        if self.rom_len() == 0 {
            return Err(Error::new("Load a cartridge before running."));
        }
        if let Some(error) = &self.fault {
            return Err(error.clone());
        }
        let end = self.frame_number() + 1;
        while self.frame_number() < end {
            match self.cpu.step(&mut self.bus) {
                Ok(cycles) => {
                    // Advance at line boundaries so rendering and event delivery share time.
                    let mut remaining = cycles;
                    while remaining > 0 {
                        let line = self.bus.raster_line as usize;
                        let count = remaining.min(256 - (self.bus.cycles % 256) as u32);
                        if self.bus.cycles % 256 + count as u64 == 256 && line < FRAME_HEIGHT {
                            video::render_line(&self.bus, &mut self.framebuffer, line);
                        }
                        self.bus.tick(count);
                        remaining -= count;
                    }
                }
                Err(error) => {
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod extended_startup_tests {
    use super::*;
    #[test]
    fn a_64_mib_extended_cartridge_boots_from_its_last_bank_after_restart() {
        let mut rom = cartridge::tests::rom(67108864, 0, 1);
        let end = rom.len();
        rom[end - 65536] = 0xf4;
        cartridge::tests::checksum(&mut rom);
        let mut m = Machine::default();
        m.load_rom(&rom).unwrap();
        m.run_frame().unwrap();
        assert!(m.cpu().halted);
        assert_eq!(m.cpu().segments[1], 0xf000);
        m.restart();
        m.run_frame().unwrap();
        assert!(m.cpu().halted);
    }
}
