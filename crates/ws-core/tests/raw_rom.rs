// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const PROBES: [&[u8]; 4] = [
    include_bytes!("../../../fixtures/generated/raw3-common.ws"),
    include_bytes!("../../../fixtures/generated/raw3-2003.ws"),
    include_bytes!("../../../fixtures/generated/raw6-common.ws"),
    include_bytes!("../../../fixtures/generated/raw6-2003.ws"),
];
fn word(m: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
}
fn run(m: &mut Machine, count: u8, size: usize) {
    m.run_frame().unwrap();
    m.run_frame().unwrap();
    assert!(m.cpu().halted);
    assert_eq!(word(m, 0x400), 0x600d, "CPU assertions on {:?}", m.model());
    assert_eq!(word(m, 0x408), 0xf000);
    assert_eq!(word(m, 0x40a), 0xa55a);
    assert_eq!(m.rom_len(), size);
    assert_eq!(m.cartridge_header().unwrap().rom_size, size);
    assert_eq!(m.read_memory(0x406), count);
    assert_eq!(m.save_data()[0], count);
    let color = m.model() != Model::Mono;
    assert_eq!(word(m, 0x402), u16::from(color));
    assert_eq!(word(m, 0x404), u16::from(color));
    assert_eq!(word(m, 0x1000), if color { 0xa55a } else { 0 });
    assert_eq!(word(m, 0x1002), if color { 0xa45b } else { 0 });
    assert_eq!(
        &m.framebuffer()[..4],
        if color {
            &[0, 255, 0, 255]
        } else {
            &[170, 170, 170, 255]
        }
    );
}
#[test]
fn raw_reset_entry_cpu_markers_dma_restart_models_and_independent_console_records() {
    for rom in PROBES {
        let mut m = Machine::default();
        for (index, model) in [Model::Mono, Model::Color, Model::Crystal]
            .into_iter()
            .enumerate()
        {
            m.reset(model);
            let mut console = m.console_data().to_vec();
            console[0] = 0x40 + index as u8;
            m.load_console_data(&console).unwrap();
            assert_eq!(m.load_rom(rom).unwrap(), rom.len());
            run(&mut m, 1, rom.len());
            m.restart();
            run(&mut m, 2, rom.len());
            assert_eq!(m.console_data(), console);
            // Bad replacements preserve execution, image, cartridge save and console.
            for invalid in [&rom[..rom.len() - 1], &[0][..]] {
                assert!(m.load_rom(invalid).is_err());
                assert!(m.cpu().halted);
                assert_eq!(m.save_data()[0], 2);
                assert_eq!(m.rom_len(), rom.len());
                assert_eq!(m.console_data(), console);
            }
        }
        for (index, model) in [Model::Mono, Model::Color, Model::Crystal]
            .into_iter()
            .enumerate()
        {
            m.change_model(model).unwrap();
            run(&mut m, 3 + index as u8, rom.len());
            assert_eq!(m.console_data()[0], 0x40 + index as u8);
        }
    }
}
