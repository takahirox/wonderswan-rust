// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/push-sp.ws");

fn expected(model: Model, boot: u16) -> Vec<u16> {
    let mono = model == Model::Mono;
    let mut words = vec![
        0x600d,
        boot,
        if mono { 0 } else { 2 },
        if mono { 1 } else { 7 },
    ];
    let modes: &[u16] = if mono {
        &[0]
    } else {
        &[0, 0x80, 0, 0xc0, 0, 0xe0, 0]
    };
    for &mode in modes {
        words.push(mode);
        for _encoding in 0..2 {
            for flags in [0xf002, 0xfcd7] {
                // Independent literal stored / popped / pushed SP / restored SP / FLAGS.
                words.extend([
                    0x1ffe, 0x1ffe, 0x1ffe, 0x2000, flags, 0x1fff, 0x1fff, 0x1fff, 0x2001, flags,
                    0, 0, 0, 2, flags, 0xfffe, 0xfffe, 0xfffe, 0, flags,
                ]);
            }
        }
    }
    words.resize(571, 0);
    words
}

fn check(model: Model) {
    let mut m = Machine::new(model);
    m.load_rom(ROM).unwrap();
    let mut stale = vec![0x6d; 32768];
    stale[0..4].copy_from_slice(&[0x0d, 0x60, 0, 0]);
    m.load_save(&stale).unwrap();
    for boot in 1..=2 {
        for _ in 0..2 {
            m.run_frame().unwrap();
        }
        let actual: Vec<_> = (0..571)
            .map(|i| {
                let address = 0x400 + 2 * i;
                u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
            })
            .collect();
        // Bounded baseline diagnostics identify both encodings and all reached phases.
        eprintln!("{model:?} boot {boot}: status={:04x}, phases={}, short stored/pop={:04x?}, ModR/M stored/pop={:04x?}",
            actual[0], actual[3], &actual[5..7], &actual[45..47]);
        for (i, value) in expected(model, boot).into_iter().enumerate() {
            assert_eq!(actual[i], value, "{model:?} boot {boot} RAM word {i}");
            assert_eq!(
                u16::from_le_bytes([m.save_data()[2 * i], m.save_data()[2 * i + 1]]),
                value,
                "{model:?} boot {boot} SRAM word {i}"
            );
        }
        assert!(m.cpu().halted);
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

#[test]
fn push_sp_cartridge_mono() {
    check(Model::Mono);
}
#[test]
fn push_sp_cartridge_color() {
    check(Model::Color);
}
#[test]
fn push_sp_cartridge_crystal() {
    check(Model::Crystal);
}
