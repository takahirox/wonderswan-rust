// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};

const ROM: &[u8] = include_bytes!("../../../fixtures/generated/gdma.ws");

fn word(m: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
}

#[test]
fn original_cartridge_programs_and_checks_forward_reverse_dma_via_cpu_byte_io() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        for _ in 0..2 {
            m.run_frame().unwrap();
        }
        assert!(m.cpu().halted);
        assert_eq!(m.frame_number(), 2);
        if model == Model::Mono {
            assert_eq!(word(&m, 0x400), 0x4d4f);
            assert_eq!(word(&m, 0x402), 0);
            assert_eq!(word(&m, 0x404), 0);
        } else {
            assert_eq!(word(&m, 0x400), 0x600d);
            assert_eq!(word(&m, 0x402), 1);
            assert_eq!(word(&m, 0x404), 1);
            for base in [0x1000, 0x1100, 0x1200] {
                assert_eq!(word(&m, base), 0x1122);
                assert_eq!(word(&m, base + 2), 0x3344);
                assert_eq!(word(&m, base + 4), 0x5566);
            }
            assert_eq!(&m.framebuffer()[..4], &[0, 255, 0, 255]);
        }
    }
}
