// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/multiply.ws");

fn expected(model: Model, boot: u16) -> Vec<u16> {
    let mono = model == Model::Mono;
    let mut words = vec![
        0x600d,
        boot,
        if mono { 0 } else { 2 },
        if mono { 1 } else { 3 },
    ];
    // Independent literals: low product/destination, DX, full FLAGS, ZF/CF/OF branch bits.
    let results = [
        6,
        0x1357,
        if mono { 0xf002 } else { 0xf042 },
        if mono { 0 } else { 1 },
        0xfffe,
        1,
        if mono { 0xfc03 } else { 0xfc43 },
        if mono { 6 } else { 7 },
        0xffeb,
        0x1357,
        0xf042,
        1,
        0x8000,
        0,
        0xfc43,
        7,
        0xffeb,
        0x1357,
        0xf042,
        1,
        0x8000,
        0x1357,
        0xf442,
        1,
        0,
        0x1357,
        0xfc43,
        7,
        0x7f80,
        0x1357,
        0xf843,
        7,
    ];
    for _ in 0..if mono { 1 } else { 3 } {
        words.extend(results);
    }
    words.resize(100, 0);
    words
}

#[test]
fn multiplication_cartridge_full_results_branches_modes_and_fresh_restart() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        // Deliberately supply a stale pass marker with poisoned observations;
        // the CPU must replace every observation before comparisons can pass.
        let mut stale = vec![0x6d; 32768];
        stale[0..4].copy_from_slice(&[0x0d, 0x60, 0, 0]);
        m.load_save(&stale).unwrap();
        for boot in 1..=2 {
            for _ in 0..2 {
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
            m.restart();
            assert_eq!(m.frame_number(), 0);
        }
    }
}
