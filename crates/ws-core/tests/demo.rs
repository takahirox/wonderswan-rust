use ws_core::{Button, Machine, Model, FRAME_BYTES};

const ROM: &[u8] = include_bytes!("../../../web/public/demo/beacon-trail.ws");
fn frames(machine: &mut Machine, count: usize) {
    for _ in 0..count {
        machine.run_frame().unwrap();
    }
}
fn tap(machine: &mut Machine, button: Button) {
    machine.set_buttons(button as u16);
    frames(machine, 2);
    machine.set_buttons(0);
    frames(machine, 2);
}
fn word(machine: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([
        machine.read_memory(address),
        machine.read_memory(address + 1),
    ])
}

#[test]
fn cartridge_game_plays_to_completion_and_restarts_on_all_models() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        assert_eq!(m.cpu().segments[1], 0xffff);
        assert_eq!(m.cpu().ip, 0);
        let blank = m.framebuffer().to_vec();
        frames(&mut m, 12);
        assert_eq!(m.frame_number(), 12);
        assert_eq!(
            (word(&m, 0x400), word(&m, 0x402), word(&m, 0x404)),
            (3, 8, 0)
        );
        assert_ne!(m.framebuffer(), blank);
        assert_eq!(m.framebuffer().len(), FRAME_BYTES);
        let initial = m.framebuffer().to_vec();
        tap(&mut m, Button::Right);
        assert_eq!(word(&m, 0x400), 4);
        assert_ne!(m.framebuffer(), initial);
        for _ in 0..5 {
            tap(&mut m, Button::Right);
        }
        assert_eq!(word(&m, 0x404), 1);
        assert_eq!(word(&m, 0x800 + 64 + 42), b'1' as u16);
        for _ in 0..3 {
            tap(&mut m, Button::Up);
        }
        assert_eq!(word(&m, 0x404), 2);
        for _ in 0..9 {
            tap(&mut m, Button::Right);
        }
        assert_eq!(word(&m, 0x404), 3);
        assert_eq!(word(&m, 0x800 + 10 * 64 + 18), b'A' as u16); // ALL FOUND!
        let won = m.framebuffer().to_vec();
        tap(&mut m, Button::Left);
        assert_eq!(word(&m, 0x400), 18); // completed game stops movement
        assert_eq!(m.framebuffer(), won);
        tap(&mut m, Button::Start);
        assert_eq!(
            (word(&m, 0x400), word(&m, 0x402), word(&m, 0x404)),
            (3, 8, 0)
        );
        assert_eq!(m.framebuffer(), initial);
        m.set_buttons(Button::Right as u16);
        m.restart();
        frames(&mut m, 12);
        assert_eq!(m.framebuffer(), initial); // restart clears held input
        assert_eq!(word(&m, 0x400), 3);
    }
}

#[test]
fn game_bounds_repeat_timing_and_model_reset_are_deterministic() {
    let mut a = Machine::default();
    let mut b = Machine::default();
    for m in [&mut a, &mut b] {
        m.load_rom(ROM).unwrap();
        frames(m, 12);
    }
    for _ in 0..3 {
        tap(&mut a, Button::Left);
        tap(&mut b, Button::Left);
    }
    assert_eq!(word(&a, 0x400), 2);
    assert_eq!(a.framebuffer(), b.framebuffer());
    a.set_buttons(Button::Down as u16);
    frames(&mut a, 80);
    assert_eq!(word(&a, 0x402), 14);
    for model in [Model::Mono, Model::Color, Model::Crystal, Model::Crystal] {
        a.reset(model);
        assert_eq!(a.model(), model);
        assert_eq!(a.rom_len(), 0);
        assert_eq!(a.frame_number(), 0);
        assert!(a.run_frame().is_err());
        a.load_rom(ROM).unwrap();
        frames(&mut a, 12);
        assert_eq!(word(&a, 0x402), 8);
    }
}

#[test]
fn bad_cartridges_preserve_the_previous_machine_and_unsupported_code_errors() {
    let mut m = Machine::default();
    m.load_rom(ROM).unwrap();
    frames(&mut m, 12);
    for bytes in [
        vec![],
        vec![0; 16],
        vec![0; 131072],
        ROM[..ROM.len() - 1].to_vec(),
    ] {
        assert!(m.load_rom(&bytes).is_err());
        assert_eq!(m.frame_number(), 12);
        assert_eq!(m.rom_len(), ROM.len());
    }
    let mut rom = ROM.to_vec();
    rom[65536] = 0x0f; // unsupported, not a magic demo identifier
    let sum = rom[..rom.len() - 2]
        .iter()
        .fold(0u16, |n, b| n.wrapping_add(*b as u16));
    let len = rom.len();
    rom[len - 2..].copy_from_slice(&sum.to_le_bytes());
    m.load_rom(&rom).unwrap();
    let error = m.run_frame().unwrap_err().to_string();
    assert!(
        error.contains("Unsupported opcode 0F at F000:0000"),
        "{error}"
    );
    assert_eq!(m.run_frame().unwrap_err().to_string(), error); // faults stay latched
    m.load_rom(ROM).unwrap();
    frames(&mut m, 12);
    assert_eq!(word(&m, 0x400), 3);
}

#[test]
fn default_model_and_color_header_compatibility_are_explicit() {
    assert_eq!(Machine::default().model(), Model::Crystal);
    let mut rom = ROM.to_vec();
    let len = rom.len();
    rom[len - 9] = 1; // header model flag
    let sum = rom[..len - 2]
        .iter()
        .fold(0u16, |n, b| n.wrapping_add(*b as u16));
    rom[len - 2..].copy_from_slice(&sum.to_le_bytes());
    let mut mono = Machine::new(Model::Mono);
    assert!(mono
        .load_rom(&rom)
        .unwrap_err()
        .to_string()
        .contains("Color cartridges"));
    assert_eq!(mono.rom_len(), 0);
    let mut color = Machine::new(Model::Color);
    assert!(color.load_rom(&rom).is_ok());
}
