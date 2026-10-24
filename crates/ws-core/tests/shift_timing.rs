// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/shift-timing.ws");

#[test]
fn absolute_shift_budgets_results_flags_counts_modes_and_fresh_restart() {
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
            for _ in 0..1399 {
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
                let records = [
                    (0xa503, 0xf803),
                    (0xa540, 0xf002),
                    (0xa5c0, 0xf802),
                    (0xa540, 0xf002),
                    (0xa5ff, 0xf003),
                    (0x0003, 0xf803),
                    (0x4000, 0xf002),
                    (0xc000, 0xf802),
                    (0x4000, 0xf002),
                    (0xffff, 0xf003),
                    (0xa5c0, 0xf003),
                    (0xa540, 0xf802),
                    (0xa503, 0xf002),
                    (0xa540, 0xf802),
                    (0xa5ff, 0xf003),
                    (0xc000, 0xf003),
                    (0x4000, 0xf802),
                    (0x0003, 0xf002),
                    (0x4000, 0xf802),
                    (0xffff, 0xf003),
                    (0xa502, 0xf803),
                    (0xa540, 0xf002),
                    (0xa514, 0xf002),
                    (0xa540, 0xf002),
                    (0xa5f7, 0xf003),
                    (0x0002, 0xf803),
                    (0x4000, 0xf002),
                    (0x5000, 0xf002),
                    (0x4000, 0xf002),
                    (0xdfff, 0xf003),
                    (0xa540, 0xf803),
                    (0xa540, 0xf802),
                    (0xa528, 0xf002),
                    (0xa540, 0xf802),
                    (0xa5ef, 0xf003),
                    (0x4000, 0xf803),
                    (0x4000, 0xf802),
                    (0x000a, 0xf002),
                    (0x4000, 0xf802),
                    (0xfffb, 0xf003),
                    (0xa502, 0xf803),
                    (0xa540, 0xf002),
                    (0xa500, 0xf046),
                    (0xa540, 0xf002),
                    (0xa500, 0xf046),
                    (0x0002, 0xf803),
                    (0x4000, 0xf006),
                    (0x0000, 0xf046),
                    (0x4000, 0xf006),
                    (0x0000, 0xf046),
                    (0xa540, 0xf803),
                    (0xa540, 0xf802),
                    (0xa500, 0xf046),
                    (0xa540, 0xf802),
                    (0xa500, 0xf046),
                    (0x4000, 0xf807),
                    (0x4000, 0xf806),
                    (0x0000, 0xf046),
                    (0x4000, 0xf806),
                    (0x0000, 0xf046),
                    (0xa5c0, 0xf087),
                    (0xa540, 0xf802),
                    (0xa5ff, 0xf087),
                    (0xa540, 0xf802),
                    (0xa5ff, 0xf087),
                    (0xc000, 0xf087),
                    (0x4000, 0xf806),
                    (0xffff, 0xf087),
                    (0x4000, 0xf806),
                    (0xffff, 0xf087),
                ];
                for (sample, (result, flags)) in records.into_iter().enumerate() {
                    let profile = sample % 5;
                    let low = [128, 136, 136, 140, 140][profile];
                    let high = low + 1;
                    let i = 4 + u32::from(pass) * 490 + sample as u32 * 7;
                    observations.push(word(i));
                    if !(low..=high).contains(&word(i)) {
                        failures.push(format!("{model:?} boot {boot} pass {pass} sample {sample}: {} vs {low}..={high}",word(i)));
                    }
                    assert_eq!((word(i+1),word(i+2),word(i+3),word(i+4),word(i+5),word(i+6)),
                        (result,flags,1024,0,[1,0,31,32,255][profile],0x51a7),
                        "sample {sample}: CPU result, full FLAGS, execution/remaining/count and stack continuation");
                }
            }
            println!("{model:?} boot {boot}: shift HBlank samples {observations:?}; execution count 1024, remaining=0 and continuation checked");
            for i in 8 + u32::from(passes) * 980..7168 {
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
        "absolute shift budgets failed:\n{}",
        failures.join("\n")
    );
}
