//! Browser-independent state for the initial, non-playable prototype.

use std::fmt;

pub const FRAME_WIDTH: usize = 224;
pub const FRAME_HEIGHT: usize = 144;
pub const FRAME_BYTES: usize = FRAME_WIDTH * FRAME_HEIGHT * 4;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Model {
    Mono,
    Color,
    #[default]
    Crystal,
}

impl Model {
    pub fn name(self) -> &'static str {
        match self {
            Self::Mono => "WonderSwan (mono)",
            Self::Color => "WonderSwan Color",
            Self::Crystal => "SwanCrystal",
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct EmptyRom;

impl fmt::Display for EmptyRom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ROM data must not be empty.")
    }
}

impl std::error::Error for EmptyRom {}

pub struct Machine {
    model: Model,
    rom: Vec<u8>,
    framebuffer: Vec<u8>,
}

impl Default for Machine {
    fn default() -> Self {
        Self::new(Model::default())
    }
}

impl Machine {
    pub fn new(model: Model) -> Self {
        // An opaque black clear buffer, not simulated LCD output.
        let mut framebuffer = vec![0; FRAME_BYTES];
        for pixel in framebuffer.as_chunks_mut::<4>().0 {
            pixel[3] = 255;
        }
        Self {
            model,
            rom: Vec::new(),
            framebuffer,
        }
    }

    pub fn model(&self) -> Model {
        self.model
    }

    /// Recreate machine state, including when selecting the current model again.
    pub fn reset(&mut self, model: Model) {
        *self = Self::new(model);
    }

    /// Store raw cartridge bytes without header validation or execution.
    /// On error, any previously loaded bytes are preserved.
    pub fn load_rom(&mut self, bytes: &[u8]) -> Result<usize, EmptyRom> {
        if bytes.is_empty() {
            return Err(EmptyRom);
        }
        self.rom = bytes.to_vec();
        Ok(self.rom.len())
    }

    pub fn rom_len(&self) -> usize {
        self.rom.len()
    }

    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_swancrystal() {
        assert_eq!(Model::default(), Model::Crystal);
        assert_eq!(Machine::default().model(), Model::Crystal);
        assert_eq!(Machine::default().rom_len(), 0);
    }

    #[test]
    fn constructs_and_selects_all_three_models() {
        let mut machine = Machine::default();
        for (model, name) in [
            (Model::Mono, "WonderSwan (mono)"),
            (Model::Color, "WonderSwan Color"),
            (Model::Crystal, "SwanCrystal"),
        ] {
            assert_eq!(Machine::new(model).model(), model);
            machine.reset(model);
            assert_eq!(machine.model(), model);
            assert_eq!(machine.model().name(), name);
        }
    }

    #[test]
    fn reset_clears_rom_even_for_the_same_model() {
        let mut machine = Machine::default();
        for model in [Model::Crystal, Model::Mono, Model::Color, Model::Crystal] {
            machine.load_rom(&[1, 2, 3]).unwrap();
            machine.reset(model);
            assert_eq!(machine.model(), model);
            assert_eq!(machine.rom_len(), 0);
            assert_eq!(machine.framebuffer(), Machine::new(model).framebuffer());
        }
    }

    #[test]
    fn stores_actual_bytes_and_replaces_previous_data() {
        let mut machine = Machine::default();
        let mut bytes = [0, 255, 23, 42, 128];
        assert_eq!(machine.load_rom(&bytes), Ok(5));
        bytes.fill(0); // The core owns its copy of the original bytes.
        assert_eq!(machine.rom, [0, 255, 23, 42, 128]);
        assert_eq!(machine.rom_len(), 5);
        assert_eq!(machine.load_rom(&[7]), Ok(1));
        assert_eq!(machine.rom, [7]);
        assert_eq!(machine.rom_len(), 1);
    }

    #[test]
    fn rejects_empty_data_without_losing_previous_rom() {
        let mut machine = Machine::default();
        assert_eq!(machine.load_rom(&[]), Err(EmptyRom));
        assert_eq!(machine.rom_len(), 0);
        machine.load_rom(&[42]).unwrap();
        assert_eq!(machine.load_rom(&[]), Err(EmptyRom));
        assert_eq!(machine.rom, [42]);
        assert_eq!(EmptyRom.to_string(), "ROM data must not be empty.");
    }

    #[test]
    fn framebuffer_has_fixed_rgba_dimensions_and_deterministic_clear() {
        assert_eq!((FRAME_WIDTH, FRAME_HEIGHT), (224, 144));
        for model in [Model::Mono, Model::Color, Model::Crystal] {
            let mut machine = Machine::new(model);
            assert_eq!(machine.framebuffer().len(), 224 * 144 * 4);
            assert!(machine
                .framebuffer()
                .as_chunks::<4>()
                .0
                .iter()
                .all(|pixel| *pixel == [0, 0, 0, 255]));
            machine.load_rom(&[123]).unwrap();
            assert_eq!(machine.framebuffer(), Machine::new(model).framebuffer());
        }
    }
}
