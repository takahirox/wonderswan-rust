// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/rtc-calendar.ws");
#[test]
fn cpu_calendar_status_all_models_and_modes() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.load_rom(ROM).unwrap();
        m.sync_rtc(1000);
        let mut stale = vec![0x6d; 32768];
        stale[..2].copy_from_slice(&[0x0d, 0x63]);
        stale[0x100..0x102].fill(0);
        m.load_save(&stale).unwrap();
        for boot in 0..3 {
            if boot > 0 {
                m.restart();
                m.sync_rtc(1000 + boot * 61000);
            }
            m.run_frame().unwrap();
            let s = m.save_data();
            assert_eq!(
                u16::from_le_bytes([s[0x100], s[0x101]]),
                boot as u16 + 1,
                "fresh CPU boot counter"
            );
            assert_eq!(&s[..2], &[0x0d, 0x63], "fresh CPU completion");
            assert_eq!(
                &s[2..10],
                if boot == 0 {
                    &[0x82, 0, 1, 1, 0, 0, 0, 0]
                } else {
                    &[0x6a, 0x24, 2, 0x28, 3, 0x92, 0x35, 0x57]
                },
                "{model:?} boot {boot} retained calendar"
            );
            for (phase, mode) in (if model == Model::Mono {
                vec![0]
            } else {
                vec![0, 0x80, 0, 0xc0, 0, 0xe0, 0]
            })
            .into_iter()
            .enumerate()
            {
                let start = 10 + phase * 12;
                assert_eq!(
                    &s[start..start + 12],
                    &[mode, 0x6a, 0x24, 2, 0x28, 3, 0xa3, 0x59, 0x58, 0x92, 0x34, 0x56],
                    "{model:?} phase {phase}"
                );
            }
        }
    }
}
