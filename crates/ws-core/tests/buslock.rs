// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};

const ROM: &[u8] = include_bytes!("../../../fixtures/generated/buslock.ws");
// Independent of the cartridge's comparisons and the CPU implementation.
const RESULTS: [u16; 12] = [
    0x600d, 0xabce, 0x005a, 0xf046, 0x1234, 0x3210, 0x7654, 0xbeef, 0x1234, 0x5678, 0x2468, 0xf000,
];

#[test]
fn buslock_cartridge_executes_normal_footer_banking_and_restarts_on_all_models() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        for boot in 1..=2 {
            assert_eq!(m.frame_number(), 0);
            for _ in 0..2 {
                m.run_frame().unwrap();
            }
            assert_eq!(m.frame_number(), 2);
            for (i, expected) in RESULTS.into_iter().chain([boot]).enumerate() {
                let address = 0x400 + i as u32 * 2;
                let actual =
                    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)]);
                assert_eq!(actual, expected, "{model:?} boot {boot} RAM {address:04x}");
                assert_eq!(
                    u16::from_le_bytes([m.save_data()[i * 2], m.save_data()[i * 2 + 1]]),
                    expected
                );
            }
            assert!(m.cpu().halted);
            assert_eq!(m.cpu().segments[1], 0xf000);
            assert_eq!(m.cpu().registers[4], 0x3000);
            assert_eq!(
                &m.framebuffer()[..4],
                if model == Model::Mono {
                    &[170, 170, 170, 255]
                } else {
                    &[0, 255, 0, 255]
                }
            );
            assert_eq!(m.read_memory(0x904), 0xce);
            assert_eq!(m.read_memory(0x905), 0xab);
            m.restart();
        }
    }
}
