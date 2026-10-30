// SPDX-License-Identifier: CC0-1.0
use ws_core::{Machine, Model};
const PROBES: [&[u8]; 2] = [
    include_bytes!("../../../fixtures/generated/internal-eeprom.ws"),
    include_bytes!("../../../fixtures/generated/internal-eeprom-protected.ws"),
];
fn word(m: &Machine, address: u32) -> u16 {
    u16::from_le_bytes([m.read_memory(address), m.read_memory(address + 1)])
}
fn run(m: &mut Machine, count: u16) {
    m.run_frame().unwrap();
    m.run_frame().unwrap();
    m.run_frame().unwrap(); // render a complete frame after the longer probe
    assert_eq!(word(m, 0x400), 0x600d, "CPU assertions on {:?}", m.model());
    assert_eq!(word(m, 0x402), count);
    assert_eq!(
        &m.console_data()[..2],
        &[255, 255],
        "completed direct ERASE"
    );
    assert_eq!(word(m, 0x40a), 0xffff);
    assert_eq!(&m.console_data()[2..4], &count.to_le_bytes());
    assert_eq!(&m.console_data()[0x5e..0x60], &[0xaa, 0x55]);
    assert_eq!(word(m, 0x406), 0x1234);
    if m.model() == Model::Mono {
        assert_eq!(&m.framebuffer()[..4], &[170, 170, 170, 255]);
    } else {
        assert_eq!(&m.framebuffer()[..4], &[0, 255, 0, 255]);
        assert_eq!(word(m, 0x408), 0xabcd);
    }
}
#[test]
fn cpu_settings_shared_area_status_locks_protection_cross_cartridge_and_reopen() {
    let mut m = Machine::default();
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.reset(model);
        let mut bytes = m.console_data().to_vec();
        bytes[0x60..0x62].copy_from_slice(&[0x34, 0x12]);
        if model != Model::Mono {
            bytes[2046..].copy_from_slice(&[0xcd, 0xab]);
        }
        m.load_console_data(&bytes).unwrap();
        m.load_rom(PROBES[0]).unwrap();
        run(&mut m, 1);
        m.restart();
        run(&mut m, 2);
        m.load_rom(PROBES[1]).unwrap();
        run(&mut m, 3);
        assert!(m.save_data().iter().all(|b| *b == 255));
        let data = m.console_data().to_vec();
        let mut reopened = Machine::new(model);
        reopened.load_console_data(&data).unwrap();
        reopened.load_rom(PROBES[0]).unwrap();
        assert_eq!(
            reopened.console_data(),
            data,
            "retained erase before CPU execution"
        );
        run(&mut reopened, 4);
        assert_eq!(&reopened.console_data()[0x60..0x62], &[0x34, 0x12]);
    }
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.reset(model);
        assert_eq!(m.console_data()[2], 3, "per-model isolation");
        assert_eq!(&m.console_data()[..2], &[255, 255]);
    }
}

#[test]
fn cpu_rejected_erase_bulk_malformed_and_busy_commands_preserve_completed_storage() {
    let probes: [&[u8]; 8] = [
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-1.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-2.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-3.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-4.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-5.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-6.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-7.ws"),
        include_bytes!("../../../fixtures/generated/internal-eeprom-rejected-8.ws"),
    ];
    let messages = [
        "ERASE", "ERASE", "ERASE", "WRAL", "ERAL", "command", "format", "busy",
    ];
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for (probe, message) in probes.into_iter().zip(messages) {
            let mut m = Machine::new(model);
            let mut expected = vec![0x5a; m.console_data().len()];
            m.load_console_data(&expected).unwrap();
            m.load_rom(probe).unwrap();
            m.load_save(&vec![0x77; 32768]).unwrap();
            let error = m.run_frame().unwrap_err().to_string();
            assert!(error.contains(message), "{error}");
            assert_eq!(word(&m, 0x400), 0x91ee);
            expected[..2].copy_from_slice(&[0x34, 0x12]);
            assert_eq!(m.console_data(), expected);
            assert!(m.save_data().iter().all(|b| *b == 0x77));
            m.restart(); // including cancellation of the busy probe's erase
            assert_eq!(m.console_data(), expected);
        }
    }
}
