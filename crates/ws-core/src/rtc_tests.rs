use crate::{cartridge::tests::rom, Machine, Model};
fn machine() -> Machine {
    let mut m = Machine::default();
    m.load_rom(&rom(131072, 2, 1)).unwrap();
    m
}
fn write(m: &mut Machine, command: u8, data: &[u8]) {
    m.bus.output(0xcb, data[0]).unwrap();
    m.bus.output(0xca, command).unwrap();
    m.bus.tick(64);
    for &b in &data[1..] {
        m.bus.output(0xcb, b).unwrap();
        m.bus.tick(64);
    }
    m.bus.output(0xcb, 0).unwrap();
}
fn read(m: &mut Machine, command: u8, length: usize) -> Vec<u8> {
    m.bus.output(0xca, command).unwrap();
    (0..length)
        .map(|_| {
            m.bus.tick(64);
            m.bus.input(0xcb).unwrap()
        })
        .collect()
}
fn elapsed(m: &mut Machine, milliseconds: u64) {
    m.bus
        .cartridge
        .as_mut()
        .unwrap()
        .rtc
        .as_mut()
        .unwrap()
        .advance(milliseconds);
}
#[test]
fn literal_rollovers_and_two_digit_year_policy() {
    for (before, after) in [
        ([0x24, 1, 1, 0, 0, 0, 0x59], [0x24, 1, 1, 0, 0, 1, 0]),
        ([0x24, 1, 1, 0, 0, 0x59, 0x59], [0x24, 1, 1, 0, 1, 0, 0]),
        (
            [0x24, 2, 0x28, 6, 0x23, 0x59, 0x59],
            [0x24, 2, 0x29, 0, 0, 0, 0],
        ),
        (
            [0x24, 2, 0x29, 0, 0x23, 0x59, 0x59],
            [0x24, 3, 1, 1, 0, 0, 0],
        ),
        (
            [0x23, 2, 0x28, 2, 0x23, 0x59, 0x59],
            [0x23, 3, 1, 3, 0, 0, 0],
        ),
        (
            [0x24, 4, 0x30, 4, 0x23, 0x59, 0x59],
            [0x24, 5, 1, 5, 0, 0, 0],
        ),
        (
            [0x24, 1, 0x31, 5, 0x23, 0x59, 0x59],
            [0x24, 2, 1, 6, 0, 0, 0],
        ),
        (
            [0x99, 0x12, 0x31, 6, 0x23, 0x59, 0x59],
            [0, 1, 1, 0, 0, 0, 0],
        ),
        ([0, 2, 0x28, 0, 0x23, 0x59, 0x59], [0, 2, 0x29, 1, 0, 0, 0]),
    ] {
        let mut m = machine();
        write(&mut m, 0x12, &[0x40]);
        write(&mut m, 0x14, &before);
        elapsed(&mut m, 999);
        assert_eq!(read(&mut m, 0x17, 3)[2], 0x59);
        elapsed(&mut m, 1);
        assert_eq!(read(&mut m, 0x15, 7), after);
    }
}
#[test]
fn twelve_hour_noon_midnight_and_twenty_four_readback() {
    let mut m = machine();
    write(&mut m, 0x10, &[0xff]);
    write(&mut m, 0x16, &[0x11, 0x59, 0x59]);
    elapsed(&mut m, 1000);
    assert_eq!(read(&mut m, 0x17, 3), [0x80, 0, 0]);
    write(&mut m, 0x16, &[0x91, 0x59, 0x59]);
    elapsed(&mut m, 1000);
    assert_eq!(read(&mut m, 0x17, 3), [0, 0, 0]);
    write(&mut m, 0x12, &[0x40]);
    write(&mut m, 0x16, &[0x23, 0x45, 0x56]);
    assert_eq!(read(&mut m, 0x17, 3), [0xa3, 0x45, 0x56]);
    write(&mut m, 0x12, &[0]);
    assert_eq!(read(&mut m, 0x17, 3), [0x91, 0x45, 0x56]);
}
#[test]
fn cold_reset_status_masks_and_write_only_alarm() {
    let mut m = machine();
    assert_eq!(read(&mut m, 0x13, 1), [0x82]);
    assert_eq!(&m.rtc_data()[12..14], &[0x80, 0]);
    write(&mut m, 0x12, &[0xff]);
    assert_eq!(read(&mut m, 0x13, 1), [0xea]);
    write(&mut m, 0x18, &[0x12, 0x34]);
    assert_eq!(&m.rtc_data()[12..14], &[0x12, 0x34]);
    assert_eq!(read(&mut m, 0x19, 2), [0xff, 0xff]);
    write(&mut m, 0x1a, &[0x56, 0x78]);
    assert_eq!(&m.rtc_data()[12..14], &[0x56, 0x78]);
    for cmd in [0x10, 0x11] {
        m.bus.output(0xcb, 0xff).unwrap();
        m.bus.output(0xca, cmd).unwrap();
        m.bus.tick(63);
        assert_eq!(m.rtc_data()[11], if cmd == 0x10 { 0xea } else { 0 });
        m.bus.tick(1);
        assert_eq!(read(&mut m, 0x13, 1), [0]);
        assert_eq!(read(&mut m, 0x15, 7), [0, 1, 1, 0, 0, 0, 0]);
        assert_eq!(&m.rtc_data()[12..14], &[0, 0]);
    }
}
#[test]
fn invalid_fields_and_delayed_second_carry() {
    let mut m = machine();
    write(&mut m, 0x12, &[0x40]);
    write(&mut m, 0x14, &[0xfa, 0, 0x39, 7, 0x29, 0x6a, 0x7a]);
    assert_eq!(read(&mut m, 0x15, 7), [0, 1, 1, 0, 0, 0, 0x7a]);
    elapsed(&mut m, 1000);
    assert_eq!(read(&mut m, 0x17, 3), [0, 1, 0]);
    for (year, month, day, expected) in [
        (0x23, 2, 0x29, [0x23, 3, 1]),
        (0x24, 2, 0x30, [0x24, 3, 1]),
        (0x24, 4, 0x31, [0x24, 5, 1]),
        (0x24, 0x19, 0, [0x24, 1, 1]),
    ] {
        write(&mut m, 0x14, &[year, month, day, 5, 0, 0, 0]);
        assert_eq!(
            &read(&mut m, 0x15, 7)[..4],
            &[expected[0], expected[1], expected[2], 5]
        );
    }
    write(&mut m, 0x10, &[0]);
    write(&mut m, 0x16, &[0x92, 0x60, 0x60]);
    assert_eq!(read(&mut m, 0x17, 3), [0x80, 0, 0x60]);
    elapsed(&mut m, 1000);
    assert_eq!(read(&mut m, 0x17, 3), [0x80, 1, 0]);
}
#[test]
fn injected_clock_partitioning_backward_highwater_and_bounded_offline() {
    let mut a = machine();
    let mut b = machine();
    a.sync_rtc(1000);
    b.sync_rtc(1000);
    let cycles = a.bus.cycles;
    let cpu = format!("{:?}", a.cpu());
    let framebuffer = a.framebuffer().to_vec();
    let console = a.console_data().to_vec();
    assert!(a.audio_samples().is_empty());
    let frame = a.frame_number();
    let ports = a.bus.ports;
    for t in [1200, 1999, 2001, 500, 2000, 2002, 999999999999] {
        a.sync_rtc(t);
    }
    b.sync_rtc(999999999999);
    assert_eq!(a.rtc_data(), b.rtc_data());
    assert_eq!(a.bus.cycles, cycles);
    assert_eq!(a.frame_number(), frame);
    assert_eq!(format!("{:?}", a.cpu()), cpu);
    assert_eq!(a.framebuffer(), framebuffer);
    assert_eq!(a.console_data(), console);
    assert!(a.audio_samples().is_empty());
    assert_eq!(a.bus.ports, ports);
    let before = a.rtc_data();
    a.sync_rtc(0);
    a.sync_rtc(999999999999);
    assert_eq!(a.rtc_data(), before);
    elapsed(&mut a, u64::MAX); // bounded even at maximum injected duration
    assert_eq!(a.rtc_data().len(), 25);
    a.sync_rtc(u64::MAX);
    let exported = a.rtc_data();
    a.load_rtc_data(&exported).unwrap();
    a.sync_rtc(u64::MAX);
    assert_eq!(a.rtc_data(), exported);
}
#[test]
fn snapshot_interruption_lifecycle_import_and_attachment_are_independent() {
    let mut m = machine();
    let raw = m.bus.cartridge.as_ref().unwrap().bytes.clone();
    m.sync_rtc(1000);
    write(&mut m, 0x12, &[0x40]);
    write(&mut m, 0x16, &[0x23, 0x59, 0x59]);
    m.bus.output(0xca, 0x15).unwrap();
    m.sync_rtc(2000);
    let snap: Vec<_> = (0..7)
        .map(|_| {
            m.bus.tick(64);
            m.bus.input(0xcb).unwrap()
        })
        .collect();
    assert_eq!(&snap[4..], &[0xa3, 0x59, 0x59]);
    assert_eq!(read(&mut m, 0x17, 3), [0, 0, 0]);
    let retained = m.rtc_data();
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        m.bus.output(0xcb, 0x99).unwrap();
        m.bus.output(0xca, 0x14).unwrap();
        m.bus.tick(64);
        m.change_model(model).unwrap();
        assert_eq!(m.bus.input(0xca).unwrap(), 0);
        assert_eq!(m.rtc_data(), retained);
        m.restart();
        assert_eq!(m.rtc_data(), retained);
    }
    for index in [0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 15] {
        let mut bad = retained.clone();
        bad[index] = 255;
        assert!(m.load_rtc_data(&bad).is_err());
        assert_eq!(m.rtc_data(), retained);
    }
    assert!(m.load_rtc_data(&[]).is_err());
    assert!(m.load_rom(&[]).is_err());
    assert_eq!(m.rtc_data(), retained);
    m.bus.output(0xca, 0x14).unwrap();
    m.load_rtc_data(&retained).unwrap();
    m.bus.tick(4096);
    assert_eq!(m.bus.input(0xca).unwrap(), 0);
    m.set_rtc_present(false).unwrap();
    assert!(m.rtc_data().is_empty());
    assert!(m.load_rtc_data(&retained).is_err());
    assert_eq!(read(&mut m, 0x15, 7), [255; 7]);
    assert_eq!(m.bus.cartridge.as_ref().unwrap().bytes, raw);
    m.load_rom(&rom(131072, 2, 0)).unwrap();
    assert!(m.rtc_data().is_empty());
    assert!(m.set_rtc_present(true).is_err());
    assert!(m.bus.output(0xca, 0x13).is_err());
}

#[test]
fn complete_write_stage_cancellation_and_failed_import_preserve_transport() {
    let mut m = machine();
    assert!(m.cartridge_header().unwrap().rtc_equipped);
    assert_eq!(
        m.cartridge_header().unwrap().mapper,
        crate::Mapper::Bandai2003
    );
    write(&mut m, 0x18, &[0x12, 0x34]);
    let retained = m.rtc_data();
    m.bus.output(0xcb, 0xff).unwrap();
    m.bus.output(0xca, 0x12).unwrap();
    m.bus.tick(63);
    assert_eq!(m.rtc_data(), retained);
    assert!(m.load_rtc_data(&[]).is_err());
    m.bus.tick(1);
    assert_eq!(read(&mut m, 0x13, 1), [0xea]);
    // Complete payload, but no final scheduled acknowledgment: discard it.
    write(&mut m, 0x12, &[0x40]);
    m.bus.output(0xcb, 0x23).unwrap();
    m.bus.output(0xca, 0x16).unwrap();
    m.bus.tick(64);
    m.bus.output(0xcb, 0x59).unwrap();
    m.bus.tick(64);
    m.bus.output(0xcb, 0x59).unwrap();
    m.bus.tick(63);
    let before = m.rtc_data();
    m.restart();
    assert_eq!(m.rtc_data(), before);
    assert_eq!(&m.rtc_data()[12..14], &[0x12, 0x34]);
    assert_eq!(read(&mut m, 0x17, 3), [0, 0, 0]);
    // Replacing an incomplete write with a read also discards it.
    m.bus.output(0xcb, 0x23).unwrap();
    m.bus.output(0xca, 0x16).unwrap();
    m.bus.tick(64);
    assert_eq!(read(&mut m, 0x17, 3), [0, 0, 0]);
    write(&mut m, 0x14, &[0x24, 2, 0x28, 6, 0x23, 0x59, 0x7a]);
    elapsed(&mut m, 1000);
    assert_eq!(read(&mut m, 0x15, 7), [0x24, 2, 0x29, 0, 0, 0, 0]);
}
