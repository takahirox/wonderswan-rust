// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/multiply-timing.ws");

#[test]
fn absolute_multiply_budgets_results_flags_counts_modes_and_fresh_restart() {
    let mut failures = Vec::new();
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
            for _ in 0..399 {
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
                // Independent literal absolute budgets and full result/FLAGS.
                let unsigned = if model == Model::Mono { 0xf002 } else { 0xf042 };
                let records = [
                    (6, 0x5aa5, unsigned, 152),
                    (6, 0x5aa5, unsigned, 156),
                    (6, 0x5aa5, unsigned, 156),
                    (6, 0, unsigned, 152),
                    (6, 0, unsigned, 156),
                    (6, 0, unsigned, 160),
                    (0xffeb, 0x5aa5, 0xf042, 152),
                    (0xffeb, 0x5aa5, 0xf042, 156),
                    (0xffeb, 0x5aa5, 0xf042, 156),
                    (0xffeb, 0xffff, 0xf042, 152),
                    (0xffeb, 0xffff, 0xf042, 156),
                    (0xffeb, 0xffff, 0xf042, 160),
                    (0x0303, 0x5aa5, 0xf042, 156),
                    (0x0303, 0x5aa5, 0xf042, 160),
                    (0x0303, 0x5aa5, 0xf042, 164),
                    (0x0180, 0x5aa5, 0xf042, 156),
                    (0x0180, 0x5aa5, 0xf042, 160),
                    (0x0180, 0x5aa5, 0xf042, 164),
                ];
                for (sample, (result, high_product, flags, low)) in records.into_iter().enumerate()
                {
                    let profile = sample % 3;
                    let high = low + 1;
                    let source = if profile == 0 {
                        [2, 2, 7, 7, 0xfffd, 0xfffd][sample / 3]
                    } else {
                        0x3200 + u16::from(profile == 2)
                    };
                    let i = 4 + u32::from(pass) * 144 + sample as u32 * 8;
                    observations.push(word(i));
                    if !(low..=high).contains(&word(i)) {
                        failures.push(format!("{model:?} boot {boot} pass {pass} sample {sample}: {} vs {low}..={high}",word(i)));
                    }
                    assert_eq!((word(i+1),word(i+2),word(i+3),word(i+4),word(i+5),word(i+6),word(i+7)),
                        (result,high_product,flags,1024,0,source,0x51a7),
                        "sample {sample}: CPU product, full FLAGS, execution/remaining/source and stack continuation");
                }
            }
            println!("{model:?} boot {boot}: multiply HBlank samples {observations:?}; execution count 1024, remaining=0 and continuation checked");
            for i in 8 + u32::from(passes) * 288..7168 {
                assert_eq!(m.read_memory(0x400 + i), 0, "unused observations cleared");
            }
            for i in 0..7168 {
                assert_eq!(m.read_memory(0x400 + i), m.save_data()[i as usize]);
            }
            assert!(m.cpu().halted);
            // Poison retained observations again before the actual restart.
            stale.fill(0x6d);
            stale[..4].copy_from_slice(&[0x0d, 0x60, boot as u8, 0]);
            m.load_save(&stale).unwrap();
            m.restart();
            assert_eq!(m.read_memory(0x400), 0);
        }
    }
    assert!(
        failures.is_empty(),
        "absolute multiply budgets failed:\n{}",
        failures.join("\n")
    );
}
