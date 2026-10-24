// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const ROM: &[u8] = include_bytes!("../../../fixtures/generated/rtc.ws");

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
        for (command, count) in [
            (0x10, 1),
            (0x11, 1),
            (0x12, 1),
            (0x13, 1),
            (0x14, 7),
            (0x15, 7),
            (0x16, 3),
            (0x17, 3),
            (0x18, 2),
            (0x19, 2),
            (0x1a, 2),
            (0x1b, 2),
            (0x93, 1),
        ] {
            words.extend([
                count,
                0x80 | (command & 15),
                command & 15,
                if command & 1 != 0 { 255 } else { 0 },
                command & 0x7f,
            ]);
        }
    }
    words.resize(466, 0);
    words
}

fn check(model: Model) {
    let mut m = Machine::new(model);
    m.load_rom(ROM).unwrap();
    m.set_rtc_present(false).unwrap();
    let mut stale = vec![0x6d; 32768];
    stale[0..4].copy_from_slice(&[0x0d, 0x60, 0, 0]);
    m.load_save(&stale).unwrap();
    for boot in 1..=2 {
        for _ in 0..4 {
            m.run_frame().unwrap();
        }
        let actual: Vec<_> = (0..466)
            .map(|i| {
                let address = 0x400 + 2 * i;
                u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
            })
            .collect();
        eprintln!(
            "{model:?} boot {boot}: status={:04x}, phases={}, first read={:?}",
            actual[0],
            actual[3],
            &actual[10..15]
        );
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
fn rtc_cartridge_mono() {
    check(Model::Mono);
}
#[test]
fn rtc_cartridge_color() {
    check(Model::Color);
}
#[test]
fn rtc_cartridge_crystal() {
    check(Model::Crystal);
}
