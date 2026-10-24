// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/ordinary-io.ws");

fn word(bytes: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
}

#[test]
fn ordinary_io_absolute_cpu_timer_observations_models_modes_and_fresh_restart() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        let mut stale = vec![0x6d; 32768];
        stale[0..4].copy_from_slice(&[0, 0, 0x0d, 0x60]);
        m.load_save(&stale).unwrap();
        for boot in 1..=2 {
            assert_eq!(m.read_memory(0x402), 0, "volatile status cleared");
            m.run_frame().unwrap();
            assert_eq!(word(m.save_data(), 0), boot);
            assert_eq!(
                word(m.save_data(), 2),
                0,
                "retained success invalidated while running"
            );
            for _ in 0..119 {
                if word(m.save_data(), 2) != 0 {
                    break;
                }
                m.run_frame().unwrap();
            }
            let ram: Vec<u8> = (0x400..0x500).map(|a| m.read_memory(a)).collect();
            eprintln!(
                "{model:?} boot {boot}: status={:04x}, ticks={:?}",
                word(&ram, 2),
                (8..32)
                    .step_by(2)
                    .map(|i| word(&ram, i))
                    .collect::<Vec<_>>()
            );
            // Check independent observations before the cartridge completion flag.
            let modes: &[u16] = if model == Model::Mono {
                &[0]
            } else {
                &[0, 0x80, 0, 0xc0, 0, 0xe0, 0]
            };
            assert_eq!(word(&ram, 0), boot);
            assert_eq!(word(&ram, 4), modes.len() as u16);
            for (phase, &mode) in modes.iter().enumerate() {
                assert_eq!(word(&ram, 6 + phase * 26), mode);
                // Unrolled 2048-transfer bursts, timer every 256 clocks. Initial
                // IN + MOV = 11 clocks; terminal IN samples before its charge.
                // floor/ceil bounds: 16395 ->64/65, 18443 ->72/73, 20491 ->80/81.
                for (index, low) in [64, 64, 72, 64, 64, 72, 72, 72, 80, 72, 72, 80]
                    .into_iter()
                    .enumerate()
                {
                    let ticks = word(&ram, 8 + phase * 26 + index * 2);
                    assert!((low..=low + 1).contains(&ticks), "{model:?} boot {boot} phase {phase} sample {index}: {ticks}, expected {low}..{}", low + 1);
                }
            }
            assert_eq!(word(&ram, 2), 0x600d);
            assert_eq!(&m.save_data()[..256], ram);
            assert!(ram[6 + modes.len() * 26..].iter().all(|&v| v == 0));
            assert!(m.cpu().halted);
            m.restart();
            assert_eq!(m.frame_number(), 0);
        }
    }
}
