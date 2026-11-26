//! From-scratch, browser-independent WonderSwan hardware subset.
#[cfg(test)]
mod a0_tests;
mod audio;
mod bus;
mod cartridge;
mod cpu;
#[cfg(test)]
mod deferred_tests;
mod eeprom;
#[cfg(test)]
mod gdma_tests;
mod internal_eeprom;
#[cfg(test)]
mod internal_eeprom_tests;
#[cfg(test)]
mod lcd_pipeline_tests;
#[cfg(test)]
mod lcd_tests;
#[cfg(test)]
mod raw_rom_tests;
mod rtc;
#[cfg(test)]
mod rtc_tests;
mod sdma;
mod snapshot;
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
pub use snapshot::{PreparedSnapshot, MAX_SNAPSHOT_BYTES};
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
    video: video::Pipeline,
    fault: Option<Error>,
}
impl Default for Machine {
    fn default() -> Self {
        Self::new(Model::default())
    }
}
impl Machine {
    pub fn new(model: Model) -> Self {
        Self {
            bus: bus::Bus::new(model),
            cpu: Cpu::default(),
            framebuffer: vec![255; FRAME_BYTES],
            video: video::Pipeline::default(),
            fault: None,
        }
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
    /// Leave the CPU at power-on until emulation advances, so callers can restore saves.
    pub fn load_rom(&mut self, bytes: &[u8]) -> Result<usize, Error> {
        let cartridge = cartridge::Cartridge::parse(bytes)?;
        if cartridge.header.model == HardwareModel::Color && self.model() == Model::Mono {
            return Err(Error::new("Color cartridges (model field 0x01) require a color model; WonderSwan mono was selected."));
        }
        let len = cartridge.bytes.len();
        self.reset(self.model());
        self.bus.insert_cartridge(cartridge);
        Ok(len)
    }
    /// Reset a loaded cartridge to start again through the built-in boot ROM.
    pub fn restart(&mut self) {
        let cartridge = self.bus.cartridge.take();
        self.reset(self.model());
        if let Some(c) = cartridge {
            self.bus.insert_cartridge(c);
        }
    }
    /// Switch the console and restart its volatile state, preserving the cartridge
    /// save and the independent console EEPROM record for every model.
    /// Loaded cartridges boot through the new physical model's built-in ROM.
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
            self.bus.insert_cartridge(c);
        }
        Ok(())
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
        self.bus.cancel_internal_eeprom_transfer();
        Ok(())
    }
    /// Override attachment without changing ROM identity or mapper capability.
    /// Intended for cartridge load configuration; detaching discards in-memory RTC.
    pub fn set_rtc_present(&mut self, present: bool) -> Result<(), Error> {
        let cart = self
            .bus
            .cartridge
            .as_mut()
            .ok_or_else(|| Error::new("Load a cartridge before configuring RTC."))?;
        if present && cart.header.mapper != Mapper::Bandai2003 {
            return Err(Error::new(
                "RTC requires the supported extended mapper interface.",
            ));
        }
        if present {
            cart.rtc.get_or_insert_with(rtc::Rtc::default);
        } else {
            cart.rtc = None;
        }
        self.bus.cancel_rtc();
        Ok(())
    }
    fn rtc(&self) -> Option<&rtc::Rtc> {
        self.bus.cartridge.as_ref().and_then(|c| c.rtc.as_ref())
    }
    fn rtc_at(&self, now_ms: u64) -> Option<rtc::Rtc> {
        let mut rtc = self.rtc()?.clone();
        rtc.sync(now_ms);
        Some(rtc)
    }
    pub fn rtc_data(&self) -> Vec<u8> {
        self.rtc().map_or_else(Vec::new, rtc::Rtc::export)
    }
    /// Sample an RTC record without changing the live clock or its anchor.
    pub fn rtc_data_at(&self, now_ms: u64) -> Vec<u8> {
        self.rtc_at(now_ms)
            .as_ref()
            .map_or_else(Vec::new, rtc::Rtc::export)
    }
    /// Check a native clock record without importing it or changing the bus.
    pub fn validate_rtc_data(data: &[u8]) -> Result<(), Error> {
        rtc::Rtc::import(data).map(|_| ())
    }
    /// Validate completely before mutation. Success cancels pending transport.
    pub fn load_rtc_data(&mut self, data: &[u8]) -> Result<(), Error> {
        let rtc = rtc::Rtc::import(data)?;
        let cart = self
            .bus
            .cartridge
            .as_mut()
            .filter(|c| c.rtc.is_some())
            .ok_or_else(|| Error::new("No attached RTC."))?;
        cart.rtc = Some(rtc);
        self.bus.cancel_rtc();
        Ok(())
    }
    /// Inject an absolute millisecond clock. First sample anchors, backward samples
    /// are ignored until the previous high-water mark is exceeded.
    pub fn sync_rtc(&mut self, now_ms: u64) {
        if let Some(rtc) = self.bus.cartridge.as_mut().and_then(|c| c.rtc.as_mut()) {
            rtc.sync(now_ms);
        }
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
    // Advance at line boundaries so translation, composition and events share time.
    fn advance(&mut self, cycles: u32) {
        let mut remaining = cycles;
        while remaining > 0 {
            let line = self.bus.raster_line as usize;
            let count = remaining.min(256 - (self.bus.cycles % 256) as u32);
            if self.bus.cycles % 256 + count as u64 == 256 {
                self.bus.sync_devices();
                self.video.line_end(&self.bus, &mut self.framebuffer, line);
            }
            let frame = self.frame_number();
            self.bus.tick(count);
            if self.frame_number() != frame {
                self.video.frame_wrap();
            }
            remaining -= count;
        }
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
                    // Cpu::step retains its single-clock HALT contract. Only
                    // the machine scheduler can skip idle CPU calls: it knows
                    // when raster/DMA events can next require CPU attention.
                    let cycles = if cycles == 1 && self.cpu.halted {
                        self.bus.halted_cycles()
                    } else {
                        cycles
                    };
                    if !self.bus.defer_tick(cycles) {
                        self.advance(cycles);
                    }
                }
                Err(error) => {
                    self.bus.sync_devices();
                    self.fault = Some(error.clone());
                    return Err(error);
                }
            }
        }
        self.bus.sync_devices();
        Ok(())
    }
}

#[cfg(test)]
mod scheduling_tests {
    use super::*;

    #[test]
    fn sequential_wide_rom_opcodes_keep_hblank_handler_within_one_line() {
        let mut rom = cartridge::tests::rom(131072, 0, 0);
        let mut handler = vec![0x90; 102];
        handler.extend([0xb8, 1, 0, 0xe7, 0xa4]); // Reload the one-shot timer to one.
        handler.extend([0x90; 10]);
        handler.extend([0xb0, 0x80, 0xe6, 0xb6]); // Acknowledge this HBlank.
        handler.extend([0xfe, 0x06, 0, 4, 0xcf]); // INC byte [0400]; IRET.
        rom[65536..65536 + handler.len()].copy_from_slice(&handler);

        for (rom_flags, expected) in [(0, 1), (4, 158)] {
            let flags_offset = rom.len() - 4;
            rom[flags_offset] = rom_flags;
            cartridge::tests::checksum(&mut rom);
            let mut m = Machine::new(Model::Color);
            m.bus = bus::Bus::for_test(Model::Color);
            m.bus
                .insert_test_cartridge(cartridge::Cartridge::parse(&rom).unwrap());
            m.bus.ram[0x1c..0x20].copy_from_slice(&[0, 0, 0, 0xf0]);
            m.bus.ram[0x1000..0x1003].copy_from_slice(&[0xf4, 0xeb, 0xfd]); // HLT; JMP HLT.
            m.cpu.segments[1] = 0;
            m.cpu.segments[3] = 0;
            m.cpu.registers[4] = 0x2000;
            m.cpu.ip = 0x1000;
            m.cpu.flags |= 0x200;
            m.bus.output(0xb2, 0x80).unwrap();
            m.bus.output_word(0xa4, 1).unwrap();
            m.bus.output(0xa2, 1).unwrap();

            // At 8 bits the next timer edge arrives between reload and ACK.
            // At 16 bits the ISR completes on each of the 158 entered lines.
            m.run_frame().unwrap();
            assert_eq!(m.bus.ram[0x400], expected, "ROM flags {rom_flags:02x}");
        }
    }

    #[test]
    fn halted_event_scheduling_matches_single_clock_machine_execution() {
        let keypad: &[u8] = include_bytes!("../../../fixtures/generated/keypad-horizontal.ws");
        let compat: &[u8] = include_bytes!("../../../fixtures/generated/sram128-2003.ws");
        let hyper: &[u8] = include_bytes!("../../../fixtures/generated/sound-hyper.ws");
        for (model, rom) in [
            (Model::Mono, keypad),
            (Model::Color, compat),
            (Model::Crystal, compat),
            (Model::Crystal, hyper),
        ] {
            let mut fast = Machine::new(model);
            let mut reference = Machine::new(model);
            fast.load_rom(rom).unwrap();
            reference.load_rom(rom).unwrap();
            for frame in 1..=6 {
                let buttons = if frame == 4 { Button::Right as u16 } else { 0 };
                fast.set_buttons(buttons);
                reference.set_buttons(buttons);
                fast.run_frame().unwrap();
                reference.video.invalidate_cache();
                let end = reference.frame_number() + 1;
                while reference.frame_number() < end {
                    let cycles = reference.cpu.step(&mut reference.bus).unwrap();
                    reference.advance(cycles);
                }
                assert_eq!(fast.bus.cycles, reference.bus.cycles);
                assert_eq!(fast.bus.raster_line, reference.bus.raster_line);
                assert_eq!(fast.cpu.registers, reference.cpu.registers);
                assert_eq!(fast.cpu.segments, reference.cpu.segments);
                assert_eq!(fast.cpu.ip, reference.cpu.ip);
                assert_eq!(fast.cpu.flags, reference.cpu.flags);
                assert_eq!(fast.cpu.halted, reference.cpu.halted);
                assert_eq!(fast.bus.ram, reference.bus.ram);
                assert_eq!(fast.bus.ports, reference.bus.ports);
                assert_eq!(fast.bus.sprites, reference.bus.sprites);
                assert_eq!(fast.bus.sprite_first, reference.bus.sprite_first);
                assert_eq!(fast.bus.sprite_count, reference.bus.sprite_count);
                for port in 0xa8..=0xab {
                    assert_eq!(
                        fast.bus.input(port).unwrap(),
                        reference.bus.input(port).unwrap()
                    );
                }
                assert_eq!(fast.framebuffer(), reference.framebuffer());
                assert_eq!(fast.audio_samples(), reference.audio_samples());
                assert_eq!(fast.save_data(), reference.save_data());
            }
        }
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

#[cfg(test)]
mod peripheral_startup_tests {
    use super::*;

    #[test]
    fn cartridge_initializes_nmi_control_and_reads_idle_serial_status() {
        let mut rom = cartridge::tests::rom(131072, 0, 0);
        let program = [
            0x31, 0xc0, // XOR AX, AX.
            0x8e, 0xd8, // MOV DS, AX.
            0xe6, 0xb7, // Clear low-battery NMI control.
            0xe4, 0xb3, // Read serial status before enabling the UART.
            0xa2, 0x00, 0x04, // Store AL at 0400.
            0xb0, 0xa4, // Enable the UART at 9600 baud and clear overrun.
            0xe6, 0xb3, 0xe4, 0xb3, // Read enabled, idle UART status.
            0xa2, 0x01, 0x04, 0xf4, // HLT.
        ];
        rom[65536..65536 + program.len()].copy_from_slice(&program);
        cartridge::tests::checksum(&mut rom);

        let mut m = Machine::new(Model::Color);
        m.load_rom(&rom).unwrap();
        m.bus.output(0xb7, 0xff).unwrap();
        assert_eq!(m.bus.input(0xb7).unwrap(), 0x10);
        m.run_frame().unwrap();
        assert!(m.cpu().halted);
        assert_eq!(&m.bus.ram[0x400..0x402], &[0, 0x84]);
        assert_eq!(m.bus.input(0xb7).unwrap(), 0);

        m.bus.output(0xb3, 0xff).unwrap();
        assert_eq!(m.bus.input(0xb3).unwrap(), 0xc4);
        m.bus.output(0xb3, 0x40).unwrap();
        assert_eq!(m.bus.input(0xb3).unwrap(), 0x40);
    }
}
