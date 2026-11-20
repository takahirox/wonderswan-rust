use super::*;
use bus::Bus;

fn advance(bus: &mut Bus, cycles: u32) {
    if !bus.defer_tick(cycles) {
        bus.tick(cycles);
    }
}

#[test]
fn deferred_devices_match_eager_io_wave_writes_and_dma_boundaries() {
    let mut fast = Bus::new(Model::Crystal);
    let mut reference = Bus::new(Model::Crystal);
    for b in [&mut fast, &mut reference] {
        for (p, v) in [
            (0x60, 0x80),
            (0x90, 0xcf),
            (0x91, 8),
            (0x8e, 0x10),
            (0x84, 0xff),
            (0x85, 7),
            (0x8c, 3),
            (0x8d, 0),
        ] {
            b.output(p, v).unwrap();
        }
        for a in 0..64 {
            b.write(a, a as u8);
        }
    }
    for n in 0..400 {
        let count = [1, 7, 31, 127, 129][n % 5];
        advance(&mut fast, count);
        reference.tick(count);
        // Raster readback needs current CPU time without forcing devices.
        assert_eq!(fast.input(2).unwrap(), reference.input(2).unwrap());
        if n % 3 == 0 {
            let selection = (n as u8) & 0x70;
            fast.output(0xb5, selection).unwrap();
            reference.output(0xb5, selection).unwrap();
            assert_eq!(fast.input(0xb5).unwrap(), reference.input(0xb5).unwrap());
            fast.write(0x0800 + n as u32, n as u8);
            reference.write(0x0800 + n as u32, n as u8);
        }
        if n % 7 == 0 {
            let a = 64 * u32::from(fast.ports[0x8f]) + (n % 64) as u32;
            fast.write(a, n as u8);
            reference.write(a, n as u8);
        }
        if n % 23 == 0 {
            let wave_page = if n % 2 == 0 { 0xff } else { 0 };
            fast.output(0x8f, wave_page).unwrap();
            reference.output(0x8f, wave_page).unwrap();
        }
        if n % 11 == 0 {
            for p in [0x84, 0x85, 0x92, 0x93, 0x96, 0x99] {
                assert_eq!(fast.input(p).unwrap(), reference.input(p).unwrap());
            }
            assert_eq!(fast.audio, reference.audio);
        }
        if n % 29 == 0 {
            for b in [&mut fast, &mut reference] {
                b.output(0x95, if n & 1 == 0 { 2 } else { 0 }).unwrap();
            }
        }
    }
    fast.sync_devices();
    assert_eq!(fast.cycles, reference.cycles);
    assert_eq!(fast.ports, reference.ports);
    assert_eq!(fast.audio, reference.audio);
    // Enabling SDMA flushes the preceding interval and disables deferral.
    for b in [&mut fast, &mut reference] {
        for (p, v) in [(0x4a, 0), (0x4b, 0), (0x4e, 2), (0x52, 0x83)] {
            b.output(p, v).unwrap();
        }
    }
    assert!(!fast.defer_tick(1));
    fast.tick(256);
    reference.tick(256);
    assert_eq!(fast.ports, reference.ports);
    assert_eq!(fast.sound_dma_stall(), reference.sound_dma_stall());
    assert_eq!(fast.audio, reference.audio);
}

#[test]
fn deferred_eeprom_completion_and_general_dma_observe_the_previous_interval() {
    let mut b = Bus::new(Model::Crystal);
    // Original mono-command-mode internal EEPROM read: exactly 32 clocks.
    b.output_word(0xbc, 0x180).unwrap();
    b.output(0xbe, 0x10).unwrap();
    assert!(b.defer_tick(31));
    assert_eq!(b.input(0xbe).unwrap() & 3, 0);
    assert!(b.defer_tick(1));
    assert_eq!(b.input(0xbe).unwrap() & 3, 3);
    assert_eq!(b.input_word(0xba).unwrap(), 0xffff);
    // A waveform-producing interval must finish before GDMA replaces wave RAM.
    b.output(0x60, 0x80).unwrap();
    b.output(0x91, 8).unwrap();
    b.output(0x90, 1).unwrap();
    b.output(0x88, 0xff).unwrap();
    b.write(0, 0xff);
    assert!(b.defer_tick(128));
    for (p, v) in [(0x40, 2), (0x44, 0), (0x46, 2), (0x48, 0x80)] {
        b.output(p, v).unwrap();
    }
    let before = b.audio.drain();
    assert!(before.iter().any(|s| *s > 0.0));
    assert_eq!(b.gdma_step(), Some(7));
    assert_eq!(&b.ram[..2], &[0, 0]);
}
