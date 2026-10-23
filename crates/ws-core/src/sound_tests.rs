// SPDX-License-Identifier: CC0-1.0
// Independent hardware vectors, not derived from another emulator.
use super::*;
use bus::Bus;
use sdma::SoundDma;

fn bus() -> Bus {
    let mut b = Bus::new(Model::Crystal);
    let mut c = cartridge::Cartridge::parse(&cartridge::tests::rom(262144, 3, 0)).unwrap();
    for bank in 0..4 {
        c.bytes[bank * 65536..(bank + 1) * 65536].fill(0x30 + bank as u8);
    }
    b.insert_cartridge(c);
    b.output(0x60, 0x80).unwrap();
    b.output(0x91, 8).unwrap();
    b
}
fn counter(b: &mut Bus, base: u16) -> u32 {
    u32::from_le_bytes([
        b.input(base).unwrap(),
        b.input(base + 1).unwrap(),
        b.input(base + 2).unwrap(),
        0,
    ])
}
fn program(b: &mut Bus, source: u32, length: u32, control: u8) {
    for (base, value) in [(0x4a, source), (0x4e, length)] {
        for (offset, byte) in value.to_le_bytes()[..3].iter().enumerate() {
            b.output(base + offset as u16, *byte).unwrap();
        }
    }
    b.output(0x52, control).unwrap();
}
fn output(b: &mut Bus) -> [i16; 2] {
    [0x64, 0x66]
        .map(|base| i16::from_le_bytes([b.input(base).unwrap(), b.input(base + 1).unwrap()]))
}
fn cadence(b: &mut Bus) {
    let until = (117 + 128 - b.cycles % 128 - 1) % 128 + 1;
    b.tick(until as u32);
}
fn pcm(b: &mut Bus, control: u8) {
    b.output(0x6a, control).unwrap();
    b.output(0x6b, 0x70).unwrap(); // reset, mono both
}

#[test]
fn sound_ports_masks_reserved_bytes_unknown_bits_and_model_scope() {
    for model in [Model::Color, Model::Crystal] {
        let mut b = Bus::new(model); // Color hardware also available in 2bpp mono video mode.
        program(&mut b, 0xffffff, 0xffffff, 0xff);
        assert_eq!(counter(&mut b, 0x4a), 0xfffff);
        assert_eq!(counter(&mut b, 0x4e), 0xfffff);
        assert_eq!(b.input(0x52).unwrap(), 0xdf);
        for port in [0x4d, 0x51] {
            let before = b.ports;
            b.output(port, 255).unwrap();
            assert_eq!(b.input(port).unwrap(), 0);
            assert_eq!(b.ports, before);
        }
        for (p, mask) in [
            (0x81, 7),
            (0x8d, 31),
            (0x8e, 0x17),
            (0x90, 0xef),
            (0x91, 0x8f),
            (0x94, 15),
            (0x9e, 3),
        ] {
            b.output(p, 255).unwrap();
            assert_eq!(b.input(p).unwrap(), mask);
        }
        b.output(0x6a, 255).unwrap();
        assert_eq!(b.input(0x6a).unwrap(), 255);
        b.output(0x6b, 0xf0).unwrap();
        assert_eq!(b.input(0x6b).unwrap(), 0x60); // reset is a strobe; bit 7 reserved
        assert!(b
            .output(0x6b, 1)
            .unwrap_err()
            .to_string()
            .contains("Unknown Hyper Voice"));
        assert_eq!(b.input(0x6b).unwrap(), 0x60);
        assert!(b.output(0x68, 1).is_err()); // No documented direct CPU input at 68.
        for p in [0x49, 0x53, 0x63, 0x6c, 0x95] {
            assert!(b.input(p).is_err());
            assert!(b.output(p, 0).is_err());
        }
    }
    let mut b = Bus::new(Model::Mono);
    for p in (0x4a..=0x52).chain(0x64..=0x6b) {
        assert!(b.input(p).is_err());
        assert!(b.output(p, 0).is_err());
    }
    b.tick(1024);
    assert!(b.audio.drain().iter().all(|v| *v == 0.0));
    assert_eq!(b.sound_dma_stall(), None);
}

#[test]
fn every_sound_dma_rate_has_fixed_phase_and_bus_cost_even_between_samples() {
    for (rate, divisor) in [6, 4, 2, 1].into_iter().enumerate() {
        let mut b = bus();
        b.ram[0x100..0x103].copy_from_slice(&[10, 20, 30]);
        program(&mut b, 0x100, 3, 0x80 | rate as u8);
        b.accesses.set(0);
        b.tick(116);
        assert_eq!(b.sound_dma_stall(), None);
        for event in 1..=divisor * 3 {
            if event == 1 {
                b.tick(1);
            } else {
                b.tick(128);
            }
            assert_eq!(b.cycles, 117 + (event - 1) as u64 * 128);
            assert_eq!(b.sound_dma_stall(), Some(7));
            assert_eq!(b.accesses.get(), 0, "SDMA does not use CPU bus accounting");
            assert_eq!(SoundDma::counter(&b.ports, 0x4e), 3 - event / divisor);
            assert_eq!(b.ports[0x89], [0, 10, 20, 30][(event / divisor) as usize]);
        }
        assert_eq!(b.input(0x52).unwrap(), rate as u8);
        b.tick(1000);
        assert_eq!(b.sound_dma_stall(), None);
    }
}

#[test]
fn one_shot_direction_repeat_hold_zero_length_and_restart_are_real_transfers() {
    for reverse in [false, true] {
        let mut b = bus();
        b.ram[0x100..0x104].copy_from_slice(&[11, 22, 33, 44]);
        let start = if reverse { 0x103 } else { 0x100 };
        let control = if reverse { 0xc3 } else { 0x83 };
        program(&mut b, start, 4, control);
        cadence(&mut b);
        assert_eq!(b.ports[0x89], if reverse { 44 } else { 11 });
        b.output(0x52, control & 0x7f).unwrap();
        let paused = b.ports;
        b.tick(1024);
        assert_eq!(b.ports, paused);
        b.output(0x52, control | 4).unwrap();
        cadence(&mut b);
        assert_eq!(b.ports[0x89], 0);
        assert_eq!(counter(&mut b, 0x4e), 3);
        b.output(0x52, control).unwrap();
        for expected in if reverse { [33, 22, 11] } else { [22, 33, 44] } {
            cadence(&mut b);
            assert_eq!(b.ports[0x89], expected);
        }
        assert_eq!(counter(&mut b, 0x4e), 0);
        assert_eq!(counter(&mut b, 0x4a), if reverse { 0xff } else { 0x104 });
        b.output(0x52, control).unwrap();
        assert_eq!(
            b.input(0x52).unwrap() & 0x80,
            0,
            "zero length cannot enable"
        );
        program(&mut b, start, 4, control | 8);
        for _ in 0..8 {
            cadence(&mut b);
        }
        assert_eq!(counter(&mut b, 0x4a), start);
        assert_eq!(counter(&mut b, 0x4e), 4);
        assert_eq!(b.input(0x52).unwrap(), control | 8);
        // Changing live length to zero while running must stop without underflow.
        b.output(0x4e, 0).unwrap();
        cadence(&mut b);
        assert_eq!(counter(&mut b, 0x4e), 0);
        assert_eq!(b.input(0x52).unwrap() & 0x80, 0);
    }
}

#[test]
fn shadow_edits_copy_only_the_written_byte_not_other_live_counter_bytes() {
    let mut b = bus();
    program(&mut b, 0x1ff, 0x102, 0x8b);
    cadence(&mut b); // source 200, length 101; shadow still 1ff/102
    b.output(0x4a, 0x80).unwrap(); // live 280, shadow 180
    b.output(0x4e, 1).unwrap(); // live 101, shadow 101
    for _ in 0..0x101 {
        cadence(&mut b);
    }
    assert_eq!(counter(&mut b, 0x4a), 0x180);
    assert_eq!(counter(&mut b, 0x4e), 0x101);
    // Editing high byte also only changes that byte, including its shadow mask.
    b.output(0x4c, 0xf5).unwrap();
    assert_eq!(counter(&mut b, 0x4a), 0x50180);
    b.output(0x4f, 0).unwrap();
    cadence(&mut b);
    assert_eq!(counter(&mut b, 0x4a), 0x50180);
    assert_eq!(counter(&mut b, 0x4e), 1);
    // Length borrows across a byte boundary too: copying the whole live
    // counter instead of just the written byte would lose shadow byte 01.
    program(&mut b, 0x100, 0x100, 0x8b);
    cadence(&mut b);
    assert_eq!(counter(&mut b, 0x4e), 0xff);
    b.output(0x4e, 1).unwrap();
    cadence(&mut b);
    assert_eq!(counter(&mut b, 0x4e), 0x101);
    // Length really is twenty bits and borrows through all three bytes.
    program(&mut b, 0x100, 0x10000, 0x83);
    cadence(&mut b);
    assert_eq!(counter(&mut b, 0x4e), 0xffff);
}

#[test]
fn source_mapping_sram_slow_byte_rom_banks_and_twenty_bit_wrap() {
    for (source, reverse, expected) in [
        (0xfffe, false, [0xa1, 0xa2, 0xb1]),
        (0x1ffff, false, [0xb2, 0x31, 0x31]),
        (0x2ffff, false, [0x31, 0x32, 0x32]),
        (0xfffff, false, [0x33, 0xc1, 0xc2]),
        (0, true, [0xc1, 0x33, 0x33]),
    ] {
        for slow in [false, true] {
            let mut b = bus();
            b.output(0xc0, 0).unwrap();
            b.output(0xc2, 1).unwrap();
            b.output(0xc3, 2).unwrap();
            b.output(0x60, if slow { 0x82 } else { 0x80 }).unwrap();
            b.output(0xa0, u8::from(slow) << 3).unwrap();
            let c = b.cartridge.as_mut().unwrap();
            c.save[0] = 0xb1;
            c.save[65535] = 0xb2;
            b.ram[0xfffe..].copy_from_slice(&[0xa1, 0xa2]);
            b.ram[..2].copy_from_slice(&[0xc1, 0xc2]);
            program(&mut b, source, 3, if reverse { 0xc3 } else { 0x83 });
            for (i, byte) in expected.into_iter().enumerate() {
                let current = source.wrapping_add(if reverse {
                    0u32.wrapping_sub(i as u32)
                } else {
                    i as u32
                }) & 0xfffff;
                cadence(&mut b);
                assert_eq!(b.ports[0x89], byte, "source {current:x}");
                assert_eq!(
                    b.sound_dma_stall(),
                    Some(7 + u32::from(slow && current >= 0x10000))
                );
            }
            let next = source.wrapping_add(if reverse { u32::MAX - 2 } else { 3 }) & 0xfffff;
            assert_eq!(counter(&mut b, 0x4a), next);
        }
    }
    let mut b = bus();
    b.cartridge.as_mut().unwrap().header.mapper = Mapper::Bandai2003;
    b.output(0xd0, 1).unwrap();
    b.cartridge.as_mut().unwrap().save[65536] = 0xac;
    program(&mut b, 0x10000, 1, 0x83);
    cadence(&mut b);
    assert_eq!(b.ports[0x89], 0xac);
    b.output(0xce, 1).unwrap();
    program(&mut b, 0x10000, 1, 0x83);
    cadence(&mut b);
    assert_eq!(
        b.ports[0x89], 0x31,
        "mapper flash read window shared with CPU"
    );
    b.cartridge.as_mut().unwrap().bytes = vec![0; 512 * 65536];
    b.cartridge.as_mut().unwrap().bytes[257 * 65536] = 0x7c;
    b.output(0xd2, 1).unwrap();
    b.output(0xd3, 1).unwrap();
    program(&mut b, 0x20000, 1, 0x83);
    cadence(&mut b);
    assert_eq!(b.ports[0x89], 0x7c);
}

#[test]
fn hyper_extension_shift_and_ignored_mode_have_exact_independent_pcm_vectors() {
    // 0xe0 -> unsigned 57344; unsigned-negated/signed -8192.
    // Arithmetic shifts happen on the extended value before truncating to i16.
    let vectors = [
        [-8192, 28672, 14336, 7168],
        [-8192, -4096, -2048, -1024],
        [-8192, -4096, -2048, -1024],
        [-8192, -8192, -8192, -8192],
    ];
    for (extension, shifts) in vectors.into_iter().enumerate() {
        for (shift, expected) in shifts.into_iter().enumerate() {
            let mut b = bus();
            pcm(&mut b, 0x80 | (extension as u8) << 2 | shift as u8);
            b.audio.hyper_input(0xe0, true, &mut b.ports);
            b.tick(128);
            assert_eq!(output(&mut b), [expected; 2]);
            assert_eq!(b.audio.drain(), [expected as f32 / 32768.0; 2]);
        }
    }
    for (control, expected) in [(0x81, 4096), (0x85, -28672), (0x89, 4096)] {
        let mut b = bus();
        pcm(&mut b, control);
        b.audio.hyper_input(0x20, true, &mut b.ports);
        b.tick(128);
        assert_eq!(output(&mut b), [expected; 2]);
    }
}

#[test]
fn hyper_divisors_enable_reset_manual_alternation_and_all_dma_layouts() {
    for (rate, divisor) in [1, 2, 3, 4, 5, 6, 8, 12].into_iter().enumerate() {
        let mut b = bus();
        pcm(&mut b, 0x89 | (rate as u8) << 4);
        b.audio.hyper_input(0x20, true, &mut b.ports);
        b.tick(divisor * 128 - 1);
        assert_eq!(output(&mut b), [0; 2]);
        b.tick(1);
        assert_eq!(output(&mut b), [4096; 2]);
        b.output(0x6a, 0).unwrap();
        b.audio.hyper_input(0xe0, true, &mut b.ports);
        b.tick(2048);
        assert_eq!(
            output(&mut b),
            [4096; 2],
            "disable freezes the converter, not the direct output latch"
        );
    }
    for (layout, expected) in [
        (0, [4096, -4096]),
        (1, [-4096, 0]),
        (2, [0, -4096]),
        (3, [-4096, -4096]),
    ] {
        let mut b = bus();
        pcm(&mut b, 0x89);
        b.output(0x6b, 0x10 | layout << 5).unwrap();
        b.audio.hyper_input(0x20, true, &mut b.ports);
        b.audio.hyper_input(0xe0, true, &mut b.ports);
        b.tick(128);
        assert_eq!(output(&mut b), expected);
        // CPU input ignores mono mode and still alternates left, right.
        b.output(0x6b, 0x10 | layout << 5).unwrap();
        b.output(0x69, 0x20).unwrap();
        b.output(0x69, 0xe0).unwrap();
        b.tick(128);
        assert_eq!(output(&mut b), [4096, -4096]);
        b.output(0x69, 0x40).unwrap(); // next left; next would now be right
        b.output(0x6b, 0x10 | layout << 5).unwrap(); // reset selects left again
        b.output(0x69, 0x60).unwrap();
        b.tick(128);
        assert_eq!(output(&mut b), [12288, -4096]);
    }
}

#[test]
fn hyper_direct_outputs_headphone_wrap_speaker_isolation_and_legacy_readbacks() {
    let mut b = bus();
    for (p, v) in [(0x64, 0), (0x65, 0x70), (0x66, 0), (0x67, 0x80)] {
        b.output(p, v).unwrap();
    }
    // Three waves 225 and voice 255 -> L/R10 = 930, SUM11 = 1860.
    b.ram[..64].fill(255);
    for p in 0x88..=0x8b {
        b.output(p, 255).unwrap();
    }
    b.output(0x90, 0x2f).unwrap();
    b.output(0x94, 15).unwrap();
    b.tick(128);
    // Left: 0x7000 + 930*32 = 58432 wraps to -7104. Right: -3008.
    assert_eq!(b.audio.drain(), [-7104.0 / 32768.0, -3008.0 / 32768.0]);
    assert_eq!(b.input(0x96).unwrap(), 0xa2);
    assert_eq!(b.input(0x97).unwrap(), 3);
    assert_eq!(b.input(0x98).unwrap(), 0xa2);
    assert_eq!(b.input(0x99).unwrap(), 3);
    assert_eq!(b.input(0x9a).unwrap(), 0x44);
    assert_eq!(b.input(0x9b).unwrap(), 7);
    for shift in 0..4 {
        b.output(0x91, 1 | shift << 1).unwrap();
        b.tick(128);
        assert_eq!(
            b.audio.drain(),
            [(((1860 >> shift) & 255) as f32) / 256.0; 2]
        );
    }
    b.output(0x9e, 0).unwrap();
    b.tick(128);
    assert_eq!(b.audio.drain(), [0.0; 2]);
    b.output(0x91, 8).unwrap();
    b.tick(128);
    assert_eq!(
        b.audio.drain(),
        [-7104.0 / 32768.0, -3008.0 / 32768.0],
        "speaker volume does not scale headphones"
    );
    b.output(0x90, 0).unwrap();
    b.output(0x91, 1).unwrap();
    b.output(0x9e, 3).unwrap();
    b.tick(128);
    assert_eq!(
        b.audio.drain(),
        [0.0; 2],
        "Hyper Voice must never reach the internal speaker"
    );
    b.output(0x91, 0).unwrap();
    b.tick(128);
    assert_eq!(b.audio.drain(), [0.0; 2]);
}

#[test]
fn cpu_stalls_before_instructions_gdma_and_interrupts_without_charging_memory_twice() {
    let mut b = bus();
    let mut cpu = Cpu::default();
    cpu.segments[1] = 0;
    cpu.ip = 0x300;
    b.ram[0x300] = 0x43; // INC BX
    b.ram[0x100] = 0x20;
    program(&mut b, 0x100, 32, 0x8b);
    b.tick(117);
    let before = (cpu.registers, cpu.ip, cpu.flags);
    assert_eq!(cpu.step(&mut b).unwrap(), 7);
    b.tick(7);
    assert_eq!((cpu.registers, cpu.ip, cpu.flags), before);
    assert_eq!(b.accesses.get(), 0);
    assert_eq!(cpu.step(&mut b).unwrap(), 2);
    assert_eq!(cpu.registers[3], 1);
    // GDMA waits behind the sound bus reservation, then gets its ordinary cost.
    for (p, v) in [
        (0x40, 0),
        (0x41, 1),
        (0x42, 0),
        (0x44, 0),
        (0x45, 2),
        (0x46, 4),
        (0x47, 0),
        (0x48, 0x80),
    ] {
        b.output(p, v).unwrap();
    }
    cadence(&mut b);
    assert_eq!(cpu.step(&mut b).unwrap(), 7);
    b.tick(7);
    assert_eq!(b.ram[0x200], 0);
    assert_eq!(cpu.step(&mut b).unwrap(), 7);
    b.tick(7);
    assert_eq!(b.ram[0x200], 0x20);
    assert_eq!(cpu.step(&mut b).unwrap(), 2);
    b.tick(2);
    cpu.flags |= 0x200;
    b.output(0xb2, 0x80).unwrap();
    b.request(7);
    cadence(&mut b);
    let ip = cpu.ip;
    assert_eq!(cpu.step(&mut b).unwrap(), 7);
    b.tick(7);
    assert_eq!(cpu.ip, ip);
    assert!(b.interrupt().is_some());
    cpu.step(&mut b).unwrap();
    assert_ne!(cpu.ip, ip);
}

#[test]
fn chunked_clock_matches_single_cycles_and_frame_scheduler_does_not_starve_cpu() {
    fn prepared() -> Bus {
        let mut b = bus();
        pcm(&mut b, 0x89);
        b.ram[0x100..0x104].copy_from_slice(&[0x20, 0xe0, 0x40, 0xc0]);
        program(&mut b, 0x100, 4, 0x9b);
        b
    }
    let mut a = prepared();
    let mut b = prepared();
    a.tick(90000);
    for _ in 0..90000 {
        b.tick(1);
    }
    assert_eq!(a.ports, b.ports);
    assert_eq!(a.cycles, b.cycles);
    assert_eq!(a.audio.drain(), b.audio.drain());
    assert_eq!(a.sound_dma_stall(), b.sound_dma_stall());
    let mut m = Machine::new(Model::Crystal);
    m.bus = prepared();
    m.cpu.segments[1] = 0;
    m.cpu.ip = 0x300;
    m.bus.ram[0x300..0x303].copy_from_slice(&[0x43, 0xeb, 0xfd]); // INC BX; JMP back
    m.bus.output(0xa4, 2).unwrap();
    m.bus.output(0xa5, 0).unwrap();
    m.bus.output(0xa2, 3).unwrap();
    m.bus.output(0xb2, 0xd0).unwrap();
    m.bus.output(3, 1).unwrap();
    for frame in 1..=3 {
        m.run_frame().unwrap();
        assert_eq!(m.frame_number(), frame);
        assert!(m.bus.cycles < frame * CYCLES_PER_FRAME + 16);
        assert!(m.cpu.registers[3] > 1000 * frame as u16);
        assert!(m.audio_samples().iter().any(|v| *v != 0.0));
    }
    assert_eq!(m.bus.input(0xb4).unwrap() & 0xd0, 0xd0);
    assert_eq!(m.bus.input(0x52).unwrap(), 0x9b);
}

#[test]
fn sound_stall_clocks_complete_eeprom_without_corrupting_samples_or_save_buffers() {
    let mut b = bus();
    let c = b.cartridge.as_mut().unwrap();
    c.header.save = SaveKind::Eeprom(128);
    c.save = vec![255; 128];
    for (p, v) in [(0xc6, 0x30), (0xc7, 1), (0xc8, 0x40)] {
        b.output(p, v).unwrap();
    }
    b.tick(3072);
    for (p, v) in [(0xc4, 0x34), (0xc5, 0x12), (0xc6, 0x40), (0xc8, 0x20)] {
        b.output(p, v).unwrap();
    }
    b.ram[0x100] = 128;
    b.output(0x90, 0x20).unwrap();
    b.output(0x94, 5).unwrap();
    program(&mut b, 0x100, 1, 0x8b);
    let mut cpu = Cpu::default();
    cpu.halted = true;
    for _ in 0..4000 {
        let cycles = cpu.step(&mut b).unwrap();
        b.tick(cycles);
    }
    assert_eq!(b.input(0xc8).unwrap() & 2, 2);
    assert_eq!(&b.cartridge.as_ref().unwrap().save[..2], &[0x34, 0x12]);
    assert!(b.cartridge.as_ref().unwrap().save[2..]
        .iter()
        .all(|v| *v == 255));
    assert_eq!(b.input(0xc6).unwrap(), 0x40);
    assert!(b.audio.drain().chunks_exact(2).any(|v| v == [0.125, 0.125]));
    assert_eq!(cpu.ip, 0);
}

#[test]
fn active_rom_sdma_samples_live_a0_wait_at_each_event_for_both_targets() {
    for model in [Model::Color, Model::Crystal] {
        for hyper in [false, true] {
            let mut b = bus();
            b.model = model;
            b.output(0xc2, 1).unwrap();
            // Matching physical 16-bit ROM; both wait settings are supported.
            b.cartridge.as_mut().unwrap().header.rom_word_bus = true;
            b.cartridge.as_mut().unwrap().bytes[65536..65540]
                .copy_from_slice(&[0x20, 0xe0, 0x40, 0xc0]);
            pcm(&mut b, 0x89);
            program(&mut b, 0x20000, 4, if hyper { 0x93 } else { 0x83 });
            for (i, (control, byte)) in [(4, 0x20), (12, 0xe0), (12, 0x40), (4, 0xc0)]
                .into_iter()
                .enumerate()
            {
                b.output(0xa0, control).unwrap();
                b.accesses.set(0);
                cadence(&mut b);
                assert_eq!(b.cycles, 117 + i as u64 * 128);
                assert_eq!(
                    b.sound_dma_stall(),
                    Some(if control & 8 == 0 { 7 } else { 8 })
                );
                assert_eq!(b.accesses.get(), 0);
                assert_eq!(counter(&mut b, 0x4a), 0x20001 + i as u32);
                assert_eq!(counter(&mut b, 0x4e), 3 - i as u32);
                if hyper {
                    assert_eq!(b.input(0x68).unwrap(), byte);
                    assert_eq!(b.input(0x69).unwrap(), byte);
                } else {
                    assert_eq!(b.input(0x89).unwrap(), byte);
                }
                assert!(!b.cartridge.as_ref().unwrap().header.rom_wait);
            }
            assert_eq!(b.input(0x52).unwrap() & 0x80, 0);
            cadence(&mut b);
            assert_eq!(b.sound_dma_stall(), None);
        }
    }
}
