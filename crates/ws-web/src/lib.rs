//! Thin WebAssembly adapter; all machine state lives in ws-core.

use wasm_bindgen::prelude::*;
use ws_core::{Machine, Model, FRAME_HEIGHT, FRAME_WIDTH};

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

    pub fn load_rom(&mut self, bytes: &[u8]) -> Result<usize, JsError> {
        self.machine
            .load_rom(bytes)
            .map_err(|error| JsError::new(&error.to_string()))
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
