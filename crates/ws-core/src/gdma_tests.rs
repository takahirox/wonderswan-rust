// SPDX-License-Identifier: CC0-1.0
// Original hardware vectors, generated without third-party ROM/BIOS data.
use super::*;
use bus::Bus;

const REGISTERS: [u16; 8] = [0x40, 0x41, 0x42, 0x44, 0x45, 0x46, 0x47, 0x48];

fn bus() -> Bus {
    let mut b = Bus::new(Model::Crystal);
    b.output(0x60, 0x80).unwrap();
    let mut c = cartridge::Cartridge::parse(&cartridge::tests::rom(262144, 3, 0)).unwrap();
    c.header.rom_word_bus = true;
    for bank in 0..4 {
        c.bytes[bank * 65536..(bank + 1) * 65536].fill(0x30 + bank as u8);
    }
    b.insert_cartridge(c);
    b
}

fn program(b: &mut Bus, source: u32, destination: u16, length: u16, control: u8) {
    for (port, byte) in REGISTERS.into_iter().zip([
        source as u8,
        (source >> 8) as u8,
        (source >> 16) as u8,
        destination as u8,
        (destination >> 8) as u8,
        length as u8,
        (length >> 8) as u8,
        control,
    ]) {
        b.output(port, byte).unwrap();
    }
}

fn state(b: &mut Bus) -> (u32, u16, u16, u8) {
    let bytes = REGISTERS.map(|p| b.input(p).unwrap());
    (
        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]),
        u16::from_le_bytes([bytes[3], bytes[4]]),
        u16::from_le_bytes([bytes[5], bytes[6]]),
        bytes[7],
    )
}

fn finish(b: &mut Bus) -> u32 {
    let mut cycles = 0;
    for _ in 0..=32767 {
        match b.gdma_step() {
            Some(count) => cycles += count,
            None => return cycles,
        }
    }
    panic!("GDMA did not terminate within the maximum word count");
}

#[test]
fn byte_register_masks_readback_model_and_mode_gates() {
    for model in [Model::Color, Model::Crystal] {
        let mut b = Bus::new(model);
        program(&mut b, 0xffffff, 0xffff, 0xffff, 0x7f);
        assert_eq!(state(&mut b), (0xffffe, 0xfffe, 0xfffe, 0x40));
        for value in [0x01, 0x80, 0xff] {
            b.output(0x43, value).unwrap();
            assert_eq!(b.input(0x43).unwrap(), 0);
            assert_eq!(state(&mut b), (0xffffe, 0xfffe, 0xfffe, 0x40));
        }
        assert!(b.output(0x49, 255).is_err());
        assert!(b.input(0x49).is_err());
        // Register state is model-gated; the engine needs color enable at 60.7.
        b.ram[0x100..0x102].copy_from_slice(&[1, 2]);
        program(&mut b, 0x100, 0x200, 2, 0xff);
        assert_eq!(b.input(0x48).unwrap(), 0xc0);
        assert_eq!(finish(&mut b), 0);
        assert_eq!(state(&mut b), (0x100, 0x200, 2, 0x40));
        assert_eq!(&b.ram[0x200..0x202], &[0, 0]);
        b.output(0x60, 0x80).unwrap();
        b.output(0x48, 0x80).unwrap();
        assert_eq!(finish(&mut b), 7);
        assert_eq!(&b.ram[0x200..0x202], &[1, 2]);
        assert_eq!(state(&mut b), (0x102, 0x202, 0, 0));
    }
    let mut mono = Bus::new(Model::Mono);
    for port in REGISTERS.into_iter().chain([0x43]).chain(0x49..=0x52) {
        assert!(mono.output(port, 255).is_err());
        assert!(mono.input(port).is_err());
    }
    assert_eq!(finish(&mut mono), 0);
}

#[test]
fn reserved_byte_preserves_pending_and_active_dma_state_and_io_cost() {
    for model in [Model::Color, Model::Crystal] {
        let mut b = Bus::new(model);
        b.output(0x60, 0x80).unwrap();
        b.ram[0x100..0x106].copy_from_slice(&[1, 2, 3, 4, 5, 6]);
        program(&mut b, 0x100, 0x200, 6, 0x80);
        for cycles in [7, 2, 2] {
            let ports = b.ports;
            let ram = b.ram.clone();
            let clock = b.cycles;
            b.accesses.set(0);
            for (index, value) in [0x01, 0x5a, 0x80, 0xff].into_iter().enumerate() {
                b.output(0x43, value).unwrap();
                assert_eq!(b.accesses.get(), index as u32 * 2 + 1);
                assert_eq!(b.input(0x43).unwrap(), 0);
                assert_eq!(b.accesses.get(), index as u32 * 2 + 2);
                assert_eq!(b.ports, ports);
                assert_eq!(b.ram, ram);
                assert_eq!(b.cycles, clock);
            }
            // Reserved I/O must neither start a transfer nor reset its setup cost.
            assert_eq!(b.gdma_step(), Some(cycles));
            b.tick(cycles);
        }
        assert_eq!(b.gdma_step(), None);
        assert_eq!(b.cycles, 11);
        assert_eq!(state(&mut b), (0x106, 0x206, 0, 0));
        assert_eq!(&b.ram[0x200..0x206], &[1, 2, 3, 4, 5, 6]);
        b.take_fault().unwrap();
    }
}

#[test]
fn cpu_byte_io_to_reserved_port_preserves_ax_high_byte_and_gdma_registers() {
    for model in [Model::Color, Model::Crystal] {
        for value in [0x01, 0x5a, 0x80, 0xff] {
            let mut b = Bus::new(model);
            program(&mut b, 0xabcde, 0x1234, 0x5678, 0x40);
            let ports = b.ports;
            let mut cpu = Cpu::default();
            cpu.segments[1] = 0;
            cpu.ip = 0x300;
            cpu.registers[0] = 0xa500 | value;
            cpu.registers[2] = 0x43;
            // Original code: OUT DX, AL; IN AL, DX; INC BX.
            b.ram[0x300..0x303].copy_from_slice(&[0xee, 0xec, 0x43]);
            assert_eq!(cpu.step(&mut b).unwrap(), 3);
            assert_eq!(cpu.ip, 0x301);
            assert_eq!(cpu.registers[0], 0xa500 | value);
            assert_eq!(cpu.step(&mut b).unwrap(), 3);
            assert_eq!(cpu.ip, 0x302);
            assert_eq!(cpu.registers[0], 0xa500);
            assert_eq!(cpu.registers[2], 0x43);
            assert_eq!(b.ports, ports);
            assert_eq!(cpu.step(&mut b).unwrap(), 2);
            assert_eq!(cpu.ip, 0x303);
            assert_eq!(cpu.registers[3], 1);
            assert!(!cpu.halted);
            b.take_fault().unwrap();
        }
    }
}

#[test]
fn cpu_word_io_masks_source_segment_and_continues_through_real_transfer() {
    for model in [Model::Color, Model::Crystal] {
        for (value, byte) in [(0xa5f2, 0x31), (0x5af4, 0x30), (0xff0f, 0x33)] {
            let mut b = bus();
            b.model = model;
            b.output(0xc0, 0).unwrap();
            b.output(0xc2, 1).unwrap();
            program(&mut b, 0x1234, 0x1400, 4, 0x40);
            let mut cpu = Cpu::default();
            cpu.segments[1] = 0;
            cpu.ip = 0x300;
            cpu.registers[0] = value;
            cpu.registers[2] = 0x42;
            // Original code: OUT DX, AX; IN AX, DX; INC BX; OUT 48, AL; INC BX.
            b.ram[0x300..0x306].copy_from_slice(&[0xef, 0xed, 0x43, 0xe6, 0x48, 0x43]);
            let source = 0x1234 | (u32::from(value & 0xf) << 16);
            assert_eq!(cpu.step(&mut b).unwrap(), 4);
            assert_eq!(cpu.ip, 0x301);
            assert_eq!(cpu.registers[0], value);
            assert_eq!(state(&mut b), (source, 0x1400, 4, 0x40));
            assert_eq!(cpu.step(&mut b).unwrap(), 4);
            assert_eq!(cpu.ip, 0x302);
            assert_eq!(cpu.registers[0], value & 0xf);
            assert_eq!(cpu.registers[2], 0x42);
            assert_eq!(state(&mut b), (source, 0x1400, 4, 0x40));
            assert_eq!(cpu.step(&mut b).unwrap(), 2);
            assert_eq!(cpu.ip, 0x303);
            assert_eq!(cpu.registers[3], 1);
            cpu.registers[0] = 0x80;
            assert_eq!(cpu.step(&mut b).unwrap(), 4);
            for cycles in [7, 2] {
                assert_eq!(cpu.step(&mut b).unwrap(), cycles);
                assert_eq!(cpu.ip, 0x305);
                assert_eq!(cpu.registers[3], 1);
            }
            assert_eq!(&b.ram[0x1400..0x1404], &[byte; 4]);
            assert_eq!(state(&mut b), (source + 4, 0x1404, 0, 0));
            assert_eq!(cpu.step(&mut b).unwrap(), 2);
            assert_eq!(cpu.ip, 0x306);
            assert_eq!(cpu.registers[3], 2);
            assert!(!cpu.halted);
            b.take_fault().unwrap();
        }
    }
}

#[test]
fn ram_words_overlap_in_transfer_order_in_both_directions() {
    for reverse in [false, true] {
        let mut b = bus();
        b.ram[0x100..0x108].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let (source, destination, expected, end) = if reverse {
            (
                0x106,
                0x104,
                [7, 8, 7, 8, 7, 8, 7, 8],
                (0x100, 0xfe, 0, 0x40),
            )
        } else {
            (0x100, 0x102, [1, 2, 1, 2, 1, 2, 1, 2], (0x106, 0x108, 0, 0))
        };
        program(
            &mut b,
            source,
            destination,
            6,
            if reverse { 0xc0 } else { 0x80 },
        );
        b.accesses.set(0);
        assert_eq!(finish(&mut b), 11);
        assert_eq!(b.accesses.get(), 0, "DMA must not charge CPU byte accesses");
        assert_eq!(&b.ram[0x100..0x108], &expected);
        assert_eq!(state(&mut b), end);
        let save = b.cartridge.as_ref().unwrap().save.clone();
        assert!(save.iter().all(|v| *v == 255));
        b.take_fault().unwrap();
    }
}

#[test]
fn rom_sources_use_both_banked_windows_and_linear_mapping_across_boundaries() {
    let mut b = bus();
    b.output(0xc2, 1).unwrap();
    b.output(0xc3, 2).unwrap();
    b.output(0xc0, 0).unwrap();
    program(&mut b, 0x2fffe, 0x100, 4, 0x80);
    assert_eq!(finish(&mut b), 9);
    assert_eq!(&b.ram[0x100..0x104], &[0x31, 0x31, 0x32, 0x32]);
    assert_eq!(state(&mut b), (0x30002, 0x104, 0, 0));
    program(&mut b, 0x40000, 0x104, 2, 0xc0);
    assert_eq!(finish(&mut b), 7);
    assert_eq!(&b.ram[0x104..0x106], &[0x30, 0x30]);
    assert_eq!(state(&mut b), (0x3fffe, 0x102, 0, 0x40));
    // The extended high bank bits must use the very same map as CPU reads.
    let c = b.cartridge.as_mut().unwrap();
    c.header.mapper = Mapper::Bandai2003;
    c.bytes = vec![0; 512 * 65536];
    c.bytes[257 * 65536..257 * 65536 + 2].copy_from_slice(&[0xab, 0xcd]);
    b.output(0xd2, 1).unwrap();
    b.output(0xd3, 1).unwrap();
    program(&mut b, 0x20000, 0x106, 2, 0x80);
    assert_eq!(finish(&mut b), 7);
    assert_eq!(&b.ram[0x106..0x108], &[0xab, 0xcd]);
}

#[test]
fn source_wraps_at_twenty_bits_and_destination_at_sixteen_bits() {
    for reverse in [false, true] {
        let mut b = bus();
        b.ram[0..2].copy_from_slice(&[0x12, 0x34]);
        let (source, destination) = if reverse { (0, 0) } else { (0xffffe, 0xfffe) };
        program(
            &mut b,
            source,
            destination,
            4,
            if reverse { 0xc0 } else { 0x80 },
        );
        assert_eq!(finish(&mut b), 9);
        if reverse {
            assert_eq!(&b.ram[0..2], &[0x12, 0x34]);
            assert_eq!(&b.ram[0xfffe..], &[0x33, 0x33]);
            assert_eq!(state(&mut b), (0xffffc, 0xfffc, 0, 0x40));
        } else {
            assert_eq!(&b.ram[0xfffe..], &[0x33, 0x33]);
            assert_eq!(&b.ram[0..2], &[0x12, 0x34]);
            assert_eq!(state(&mut b), (2, 2, 0, 0));
        }
    }
}

#[test]
fn zero_invalid_sram_slow_and_byte_rom_sources_stop_without_setup_or_save_changes() {
    for source in [0x10000, 0x1fffe, 0x20000] {
        for (word, wait) in [(false, false), (true, true), (false, true)] {
            let mut b = bus();
            let c = b.cartridge.as_mut().unwrap();
            c.header.rom_word_bus = word;
            c.header.rom_wait = wait;
            let save = c.save.clone();
            let rom = c.bytes.clone();
            program(&mut b, source, 0x100, 4, 0xc0);
            assert_eq!(finish(&mut b), 0);
            assert_eq!(state(&mut b), (source, 0x100, 4, 0x40));
            assert_eq!(&b.ram[0x100..0x104], &[0; 4]);
            assert_eq!(b.cartridge.as_ref().unwrap().save, save);
            assert_eq!(b.cartridge.as_ref().unwrap().bytes, rom);
        }
    }
    let mut b = bus();
    for length in [0, 1] {
        program(&mut b, 0x20000, 0x100, length, 0x80);
        assert_eq!(finish(&mut b), 0);
        assert_eq!(state(&mut b), (0x20000, 0x100, 0, 0));
    }
    b.cartridge = None;
    program(&mut b, 0x20000, 0x100, 2, 0x80);
    assert_eq!(finish(&mut b), 0);
    // Flash mapped into the SRAM region is still not a 16-bit source window.
    b = bus();
    b.cartridge.as_mut().unwrap().header.mapper = Mapper::Bandai2003;
    b.output(0xce, 1).unwrap();
    program(&mut b, 0x10000, 0x100, 2, 0x80);
    assert_eq!(finish(&mut b), 0);
    b.take_fault().unwrap();
}

#[test]
fn invalid_source_mid_transfer_preserves_remaining_length_and_next_addresses() {
    for (source, control, next) in [(0xfffe, 0x80, 0x10000), (0x20000, 0xc0, 0x1fffe)] {
        let mut b = bus();
        b.ram[0xfffe..].copy_from_slice(&[0x56, 0x78]);
        program(&mut b, source, 0x200, 6, control);
        assert_eq!(finish(&mut b), 7);
        let dest = if control & 0x40 != 0 { 0x1fe } else { 0x202 };
        assert_eq!(state(&mut b), (next, dest, 4, control & 0x40));
        let expected = if source == 0xfffe {
            [0x56, 0x78]
        } else {
            [0x33, 0x33]
        };
        assert_eq!(&b.ram[0x200..0x202], &expected);
        assert!(b.ram[0x202..0x206].iter().all(|v| *v == 0));
    }
    // Re-evaluate the ROM access restrictions for every word.
    let mut b = bus();
    program(&mut b, 0x20000, 0x100, 4, 0x80);
    assert_eq!(b.gdma_step(), Some(7));
    b.cartridge.as_mut().unwrap().header.rom_wait = true;
    assert_eq!(b.gdma_step(), None);
    assert_eq!(state(&mut b), (0x20002, 0x102, 2, 0));
}

#[test]
fn cpu_stalls_before_fetch_and_interrupt_delivery_without_double_charging() {
    let mut b = bus();
    let mut cpu = Cpu::default();
    cpu.flags |= 0x200;
    b.output(0xb2, 0x80).unwrap();
    b.request(7);
    program(&mut b, 0x20000, 0x100, 6, 0x80);
    let before = (cpu.registers, cpu.segments, cpu.ip, cpu.flags);
    for expected in [7, 2, 2] {
        assert_eq!(cpu.step(&mut b).unwrap(), expected);
        assert_eq!(b.accesses.get(), 0);
        assert_eq!((cpu.registers, cpu.segments, cpu.ip, cpu.flags), before);
    }
    assert!(b.interrupt().is_some(), "IRQ stays pending during DMA");
    cpu.step(&mut b).unwrap();
    assert_ne!(cpu.segments, before.1, "IRQ delivered after completion");
    // Invalid/zero transfers must execute the next instruction without delay.
    for (source, length) in [(0x10000, 2), (0, 0)] {
        b.ram[0x300] = 0x90; // NOP
        cpu.segments[1] = 0;
        cpu.ip = 0x300;
        program(&mut b, source, 0x100, length, 0x80);
        assert_eq!(cpu.step(&mut b).unwrap(), 2); // NOP + fetch
        assert_eq!(cpu.ip, 0x301);
        assert_eq!(b.input(0x48).unwrap(), 0);
    }
}

#[test]
fn maximum_transfer_crosses_frames_and_advances_timers_interrupts_audio_and_preserves_sram() {
    for model in [Model::Color, Model::Crystal] {
        let mut m = Machine::new(model);
        m.bus = bus();
        m.bus.model = model;
        // Do not let the first CPU instruction run until the second frame.
        m.cpu.halted = true;
        m.bus.output(0xa4, 200).unwrap();
        m.bus.output(0xa5, 0).unwrap();
        m.bus.output(0xa2, 1).unwrap();
        m.bus.output(0xb2, 0xd0).unwrap(); // line match, VBlank, HBlank timer
        m.bus.output(3, 1).unwrap();
        let save = m.save_data().to_vec();
        program(&mut m.bus, 0x20000, 0, 0xffff, 0x80);
        m.run_frame().unwrap();
        assert_eq!(m.frame_number(), 1);
        assert_eq!(m.cpu.ip, 0);
        assert_eq!(m.bus.cycles, CYCLES_PER_FRAME + 1); // word boundary rounding
        assert_eq!(m.bus.input(0x48).unwrap(), 0x80);
        let mut remaining = 0;
        while let Some(count) = m.bus.gdma_step() {
            m.bus.tick(count);
            remaining += count;
        }
        assert_eq!(m.bus.cycles, 5 + 0xfffe);
        assert_eq!(remaining as u64, 5 + 0xfffe - CYCLES_PER_FRAME - 1);
        assert_eq!(m.bus.input(2).unwrap(), 97);
        assert_eq!(m.bus.input(0xa8).unwrap(), 0);
        assert_eq!(m.bus.input(0xb4).unwrap() & 0xd0, 0xd0);
        assert_eq!(m.bus.audio.drain().len(), (65539 / 128) * 2);
        assert_eq!(m.save_data(), save);
        assert_eq!(state(&mut m.bus), (0x2fffe, 0xfffe, 0, 0));
        assert!(m.bus.ram[..0xfffe].iter().all(|v| *v == 0x33));
        m.run_frame().unwrap();
        assert_eq!(m.frame_number(), 2);
    }
}

#[test]
fn enabling_out_keeps_its_normal_cost_and_dma_finishes_before_the_next_instruction() {
    let mut b = bus();
    let mut cpu = Cpu::default();
    cpu.segments[1] = 0;
    cpu.ip = 0x300;
    cpu.registers[0] = 0x80;
    b.ram[0x300..0x303].copy_from_slice(&[0xe6, 0x48, 0x43]); // OUT 48, AL; INC BX
    program(&mut b, 0x20000, 0x100, 4, 0);
    assert_eq!(cpu.step(&mut b).unwrap(), 4); // instruction + 2 fetches + OUT
    assert_eq!(cpu.ip, 0x302);
    assert_eq!(cpu.step(&mut b).unwrap(), 7);
    assert_eq!(&b.ram[0x100..0x104], &[0x33, 0x33, 0, 0]);
    assert_eq!(cpu.ip, 0x302);
    assert_eq!(cpu.registers[3], 0);
    assert_eq!(cpu.step(&mut b).unwrap(), 2);
    assert_eq!(&b.ram[0x100..0x104], &[0x33; 4]);
    assert_eq!(cpu.ip, 0x302);
    assert_eq!(cpu.step(&mut b).unwrap(), 2); // INC BX + fetch
    assert_eq!(cpu.registers[3], 1);
}

#[test]
fn dma_stall_clock_completes_pending_eeprom_programming_without_changing_its_buffers() {
    let mut b = bus();
    let c = b.cartridge.as_mut().unwrap();
    c.header.save = SaveKind::Eeprom(128);
    c.save = vec![255; 128];
    for (p, v) in [(0xc6, 0x30), (0xc7, 1), (0xc8, 0x40)] {
        b.output(p, v).unwrap(); // WEN
    }
    b.tick(3072);
    for (p, v) in [(0xc4, 0x34), (0xc5, 0x12), (0xc6, 0x40), (0xc8, 0x20)] {
        b.output(p, v).unwrap(); // WRITE word 0
    }
    assert_eq!(b.input(0xc8).unwrap() & 2, 0);
    program(&mut b, 0x20000, 0x1000, 4096, 0x80);
    let mut cpu = Cpu::default();
    let mut elapsed = 0;
    while b.ports[0x48] & 0x80 != 0 {
        let cycles = cpu.step(&mut b).unwrap();
        b.tick(cycles);
        elapsed += cycles;
    }
    assert_eq!(elapsed, 4101);
    assert_eq!(b.input(0xc8).unwrap() & 2, 2);
    assert_eq!(b.input(0xc6).unwrap(), 0x40);
    assert_eq!(b.input(0xc7).unwrap(), 1);
    assert_eq!(&b.cartridge.as_ref().unwrap().save[..2], &[0x34, 0x12]);
    assert!(b.cartridge.as_ref().unwrap().save[2..]
        .iter()
        .all(|v| *v == 255));
    assert_eq!(cpu.ip, 0);
}
