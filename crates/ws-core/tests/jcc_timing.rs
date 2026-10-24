// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/jcc-timing.ws");

#[test]
fn absolute_jcc_budget_counts_destinations_modes_and_fresh_restart() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        let mut stale = vec![0x6d; 32768];
        stale[..4].copy_from_slice(&[0x0d, 0x60, 0, 0]);
        m.load_save(&stale).unwrap();
        for boot in 1..=2 {
            m.run_frame().unwrap();
            assert_eq!(&m.save_data()[..2], &[0, 0], "stale completion invalidated");
            assert_eq!(m.read_memory(0x400), 0, "probe still running");
            for _ in 0..149 {
                m.run_frame().unwrap();
            }
            let word = |i: u32| {
                u16::from_le_bytes([m.read_memory(0x400 + i * 2), m.read_memory(0x401 + i * 2)])
            };
            assert_eq!(word(0), 0x600d);
            assert_eq!(word(1), boot);
            assert_eq!(word(2), if model == Model::Mono { 0 } else { 2 });
            let passes = if model == Model::Mono { 1 } else { 7 };
            assert_eq!(word(3), passes);
            let mut observations = Vec::new();
            for pass in 0..passes {
                // Literal absolute budgets, independent from CPU accounting helpers.
                for (sample, low, high) in [
                    (0, 416, 417),
                    (1, 432, 433),
                    (2, 368, 369),
                    (3, 416, 417),
                    (4, 432, 433),
                    (5, 368, 369),
                ] {
                    let i = 4 + u32::from(pass) * 30 + sample * 5;
                    observations.push(word(i));
                    assert!(
                        (low..=high).contains(&word(i)),
                        "{model:?} boot {boot} pass {pass} sample {sample}: {} ticks",
                        word(i)
                    );
                    assert_eq!(
                        (word(i + 1), word(i + 2), word(i + 3), word(i + 4)),
                        (4096, 4096, if sample % 3 == 2 { 1 } else { 0 }, 0x51a7),
                        "CPU execution/destination counts, CX and continuation"
                    );
                }
            }
            println!("{model:?} boot {boot}: Jcc/JCXZ HBlank samples {observations:?}; execution/destination counts 4096/4096, CX=0/0/1 and continuation checked");
            for i in 8 + u32::from(passes) * 60..512 {
                assert_eq!(m.read_memory(0x400 + i), 0, "unused observations cleared");
            }
            for i in 0..512 {
                assert_eq!(m.read_memory(0x400 + i), m.save_data()[i as usize]);
            }
            assert!(m.cpu().halted);
            m.restart();
            assert_eq!(m.read_memory(0x400), 0);
        }
    }
}
