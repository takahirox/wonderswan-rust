// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/a0.ws");
fn word(m: &Machine, a: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(a), m.read_memory(a + 1)])
}
#[test]
fn cpu_probe_changes_a0_and_checks_matching_rom_and_both_dma_engines() {
    let mut m = Machine::default();
    m.load_rom(ROM).unwrap();
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.change_model(model).unwrap();
        for _ in 0..2 {
            m.run_frame().unwrap();
            m.run_frame().unwrap();
            assert!(m.cpu().halted);
            assert_eq!(word(&m, 0x400), 0x600d, "{model:?}");
            assert_eq!(word(&m, 0x402), u16::from(model != Model::Mono));
            assert_eq!(word(&m, 0x404), u16::from(model != Model::Mono));
            assert_eq!(
                m.read_memory(0x406),
                if model == Model::Mono { 0x85 } else { 0x87 }
            );
            assert_eq!(word(&m, 0x408), 0x3131);
            assert_eq!(
                word(&m, 0x1000),
                if model == Model::Mono { 0 } else { 0x3131 }
            );
            assert_eq!(word(&m, 0x1002), 0);
            assert_eq!(
                &m.framebuffer()[..4],
                if model == Model::Mono {
                    &[170, 170, 170, 255]
                } else {
                    &[0, 255, 0, 255]
                }
            );
            let header = m.cartridge_header().unwrap();
            assert!(header.rom_word_bus);
            assert!(!header.rom_wait);
            m.restart();
        }
    }
}
