//! Thin WebAssembly adapter; all machine state lives in ws-core.

use wasm_bindgen::prelude::*;
use ws_core::{Machine, Model, Orientation, FRAME_HEIGHT, FRAME_WIDTH};

#[wasm_bindgen]
#[derive(Default)]
pub struct WebMachine {
    machine: Machine,
}

#[wasm_bindgen]
impl WebMachine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(&self) -> String {
        match self.machine.model() {
            Model::Mono => "mono",
            Model::Color => "color",
            Model::Crystal => "crystal",
        }
        .into()
    }

    pub fn model_name(&self) -> String {
        self.machine.model().name().into()
    }

    /// Cartridge presentation only; native pixels and button identities are unchanged.
    pub fn orientation(&self) -> String {
        match self.machine.cartridge_header().map(|h| h.orientation) {
            Some(Orientation::Vertical) => "vertical",
            _ => "horizontal",
        }
        .into()
    }

    pub fn reset(&mut self, model: &str) -> Result<(), JsError> {
        let model = match model {
            "mono" => Model::Mono,
            "color" => Model::Color,
            "crystal" => Model::Crystal,
            _ => return Err(JsError::new("Unknown console model.")),
        };
        self.machine.reset(model);
        Ok(())
    }

    pub fn change_model(&mut self, model: &str) -> Result<(), JsError> {
        let model = match model {
            "mono" => Model::Mono,
            "color" => Model::Color,
            "crystal" => Model::Crystal,
            _ => return Err(JsError::new("Unknown console model.")),
        };
        self.machine
            .change_model(model)
            .map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn console_data(&self) -> Vec<u8> {
        self.machine.console_data().to_vec()
    }
    pub fn load_console_data(&mut self, data: &[u8]) -> Result<(), JsError> {
        self.machine
            .load_console_data(data)
            .map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn save_data(&self) -> Vec<u8> {
        self.machine.save_data().to_vec()
    }
    pub fn load_save(&mut self, data: &[u8]) -> Result<(), JsError> {
        self.machine
            .load_save(data)
            .map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn configure_rtc(&mut self, mode: &str) -> Result<(), JsError> {
        let present = match mode {
            "auto" => self
                .machine
                .cartridge_header()
                .is_some_and(|h| h.rtc_equipped),
            "absent" => false,
            _ => return Err(JsError::new("Unknown RTC attachment mode.")),
        };
        self.machine
            .set_rtc_present(present)
            .map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn rtc_data(&self) -> Vec<u8> {
        self.machine.rtc_data()
    }
    pub fn load_rtc_data(&mut self, data: &[u8]) -> Result<(), JsError> {
        self.machine
            .load_rtc_data(data)
            .map_err(|e| JsError::new(&e.to_string()))
    }
    pub fn sync_rtc(&mut self, now_ms: f64) -> Result<(), JsError> {
        if !now_ms.is_finite()
            || !(0.0..=9_007_199_254_740_991.0).contains(&now_ms)
            || now_ms.fract() != 0.0
        {
            return Err(JsError::new(
                "RTC time must be a nonnegative safe integer of milliseconds.",
            ));
        }
        self.machine.sync_rtc(now_ms as u64);
        Ok(())
    }
    pub fn audio_samples(&mut self) -> Vec<f32> {
        self.machine.audio_samples()
    }
    pub fn sample_rate(&self) -> u32 {
        ws_core::SAMPLE_RATE
    }

    pub fn load_rom(&mut self, bytes: &[u8]) -> Result<usize, JsError> {
        self.machine
            .load_rom(bytes)
            .map_err(|error| JsError::new(&error.to_string()))
    }

    pub fn restart(&mut self) {
        self.machine.restart();
    }

    pub fn set_buttons(&mut self, mask: u16) {
        self.machine.set_buttons(mask);
    }

    pub fn run_frame(&mut self) -> Result<(), JsError> {
        self.machine
            .run_frame()
            .map_err(|error| JsError::new(&error.to_string()))
    }

    pub fn frame_number(&self) -> u32 {
        self.machine.frame_number() as u32
    }

    pub fn frame_rate(&self) -> f64 {
        self.machine.frame_rate()
    }

    pub fn rom_len(&self) -> usize {
        self.machine.rom_len()
    }

    pub fn frame_width(&self) -> usize {
        FRAME_WIDTH
    }

    pub fn frame_height(&self) -> usize {
        FRAME_HEIGHT
    }

    /// Copy the core-owned RGBA buffer across the boundary for canvas display.
    pub fn framebuffer(&self) -> Vec<u8> {
        self.machine.framebuffer().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orientation_is_header_metadata_through_load_restart_model_change_and_reset() {
        let mut m = WebMachine::new();
        assert_eq!(m.orientation(), "horizontal");
        for (bytes, orientation) in [
            (
                include_bytes!("../../../fixtures/generated/keypad-vertical.ws"),
                "vertical",
            ),
            (
                include_bytes!("../../../fixtures/generated/keypad-horizontal.ws"),
                "horizontal",
            ),
        ] {
            m.load_rom(bytes).unwrap();
            for model in ["mono", "color", "crystal"] {
                m.change_model(model).unwrap();
                m.restart();
                assert_eq!(m.orientation(), orientation);
                assert_eq!(m.frame_width(), 224);
                assert_eq!(m.frame_height(), 144);
                assert_eq!(m.rom_len(), bytes.len());
            }
        }
        m.reset("mono").unwrap();
        assert_eq!(m.orientation(), "horizontal");
    }
}
