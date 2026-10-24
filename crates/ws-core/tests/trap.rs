// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/trap.ws");

fn expected(model: Model, boot: u16) -> Vec<u16> {
    let mono = model == Model::Mono;
    let mut words = vec![
        0x600d,
        boot,
        if mono { 0 } else { 2 },
        if mono { 1 } else { 7 },
    ];
    // Per case: count, final BX, then IP/CS/saved FLAGS/live FLAGS/SP/CX/SI/DI/BX.
    let cases: &[&[[u16; 9]]] = &[
        &[[0x1005, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[[0x1005, 0, 0xf302, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[
            [0x1004, 0, 0xf102, 0xf002, 0x2ff4, 1, 0x1801, 0x1901, 0],
            [0x1008, 0, 0xf102, 0xf002, 0x2ff4, 0, 0x1802, 0x1902, 0],
        ],
        &[[0x1008, 0, 0xf102, 0xf002, 0x2ff4, 0, 0x1800, 0x1900, 0]],
        &[[0x1005, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[[0x1007, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[
            [0x1005, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0],
            [0x1006, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0],
        ],
        &[[0x1009, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[[0x1008, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
        &[[0x1006, 0, 0xf102, 0xf002, 0x2ff4, 2, 0x1800, 0x1900, 0]],
    ];
    for _ in 0..if mono { 1 } else { 7 } {
        for frames in cases {
            words.extend([frames.len() as u16, 0x1234]);
            for frame in *frames {
                words.extend(frame);
            }
        }
    }
    words.resize(900, 0);
    words
}

#[test]
fn trap_cartridge_frames_continuation_modes_and_fresh_restart() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        let mut stale = vec![0x6d; 32768];
        stale[..4].copy_from_slice(&[0x0d, 0x60, 0, 0]);
        m.load_save(&stale).unwrap();
        for boot in 1..=2 {
            for _ in 0..12 {
                m.run_frame().unwrap();
            }
            for (i, value) in expected(model, boot).into_iter().enumerate() {
                let address = 0x400 + i as u32 * 2;
                assert_eq!(
                    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)]),
                    value,
                    "{model:?} boot {boot} RAM word {i}"
                );
                assert_eq!(
                    u16::from_le_bytes([m.save_data()[2 * i], m.save_data()[2 * i + 1]]),
                    value,
                    "{model:?} boot {boot} SRAM word {i}"
                );
            }
            assert_eq!((m.read_memory(0x1900), m.read_memory(0x1901)), (0xa5, 0x5a));
            assert!(m.cpu().halted);
            assert_eq!(m.cpu().registers[4], 0x3000);
            m.restart();
            assert_eq!(m.frame_number(), 0);
        }
    }
}
