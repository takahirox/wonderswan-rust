// SPDX-License-Identifier: CC0-1.0
// Original deterministic vectors for the documented LCD compatibility policies.
use super::*;
use bus::Bus;

const MODELS: [Model; 3] = [Model::Mono, Model::Color, Model::Crystal];
fn out(b: &mut Bus, port: u16, value: u8) {
    b.output(port, value).unwrap();
}
fn machine(model: Model) -> Machine {
    let mut m = Machine::new(model);
    m.load_rom(&cartridge::tests::rom(131072, 2, 0)).unwrap();
    m.cpu.segments[1] = 0;
    m.cpu.ip = 0x800;
    m.cpu.flags &= !0x200;
    m
}

#[test]
fn independent_byte_registers_and_actual_cpu_word_io_on_every_model() {
    for model in MODELS {
        let mut b = Bus::new(model);
        assert_eq!(b.input(0x16).unwrap(), 158);
        assert_eq!(
            b.input(0x17).unwrap(),
            if model == Model::Crystal { 0 } else { 155 }
        );
        for total in [0, 1, 143, 144, 155, 156, 158, 180, 255] {
            out(&mut b, 0x16, total);
            out(&mut b, 0x17, total.wrapping_sub(3));
            assert_eq!(b.input(0x16).unwrap(), total);
            assert_eq!(
                b.input(0x17).unwrap(),
                if model == Model::Crystal {
                    0
                } else {
                    total.wrapping_sub(3)
                }
            );
            out(&mut b, 0x16, 158);
            assert_eq!(
                b.input(0x17).unwrap(),
                if model == Model::Crystal {
                    0
                } else {
                    total.wrapping_sub(3)
                }
            );
        }
        for code in [&[0xe7, 0x16, 0xe5, 0x16][..], &[0xef, 0xed][..]] {
            let mut cpu = Cpu::default();
            cpu.segments[1] = 0;
            cpu.ip = 0x800;
            cpu.registers[0] = 0xb1b4; // T=180, STN sync=T-3
            cpu.registers[2] = 0x16;
            b.ram[0x800..0x800 + code.len()].copy_from_slice(code);
            cpu.step(&mut b).unwrap();
            assert_eq!(b.input(0x16).unwrap(), 180);
            cpu.registers[0] = 0;
            cpu.step(&mut b).unwrap();
            assert_eq!(
                cpu.registers[0],
                if model == Model::Crystal { 180 } else { 0xb1b4 }
            );
        }
    }
}

#[test]
fn exact_full_height_vectors_share_line_irq_timers_vblank_and_sprite_latch() {
    for model in MODELS {
        for (total, clocks) in [(156, 40192), (158, 40704), (180, 46336)] {
            let mut b = Bus::new(model);
            out(&mut b, 0x16, total);
            out(&mut b, 0xb2, 0xf0);
            out(&mut b, 3, 144);
            out(&mut b, 0xa4, 1);
            out(&mut b, 0xa6, 2);
            out(&mut b, 0xa2, 0xf); // H repeat=1, V repeat=2
            out(&mut b, 4, 2);
            for frame in 0..3 {
                out(&mut b, 0xb2, 0xf0);
                b.ram[1024] = frame as u8 + 1;
                out(&mut b, 0xb6, 255);
                b.tick(256 * 144 - 1 - u32::from(frame > 0));
                assert_eq!(b.input(2).unwrap(), 143);
                assert_eq!(b.input(0xb4).unwrap(), 0x80);
                assert_eq!(b.sprites[0], frame as u8);
                assert_eq!(b.completed_frames, frame);
                b.tick(1);
                assert_eq!(b.input(2).unwrap(), 144);
                assert_eq!(b.input(0xb4).unwrap(), if frame == 1 { 0xf0 } else { 0xd0 });
                assert_eq!(b.input(0xaa).unwrap(), if frame == 1 { 2 } else { 1 });
                assert_eq!(b.sprites[0], frame as u8 + 1);
                b.tick(1);
                assert_eq!(b.input(2).unwrap(), 144);
                out(&mut b, 0xb2, 0); // existing latched IRQs survive disable
                assert_ne!(b.interrupt(), None);
                out(&mut b, 0xb6, 255);
                assert_eq!(b.interrupt(), None);
                b.tick(clocks - 256 * 144 - 2);
                assert_eq!(b.input(2).unwrap(), total);
                assert_eq!(b.completed_frames, frame);
                b.tick(1);
                assert_eq!(b.cycles, u64::from(clocks) * (frame + 1));
                assert_eq!(b.input(2).unwrap(), 0);
                assert_eq!(b.completed_frames, frame + 1);
                b.tick(1);
                assert_eq!(b.input(2).unwrap(), 0);
            }
            let mut repeated = Bus::new(model);
            out(&mut repeated, 0x16, total);
            out(&mut repeated, 0xa4, 0xe8);
            out(&mut repeated, 0xa5, 3); // H countdown from 1000
            out(&mut repeated, 0xa6, 3); // V countdown from 3
            out(&mut repeated, 0xa2, 5);
            for frame in 1..=4 {
                repeated.tick(255);
                assert_eq!(repeated.input(2).unwrap(), 0);
                repeated.tick(1);
                assert_eq!(repeated.input(2).unwrap(), 1);
                repeated.tick(1);
                assert_eq!(repeated.input(2).unwrap(), 1);
                repeated.tick(clocks - 257);
                let h = u16::from_le_bytes([
                    repeated.input(0xa8).unwrap(),
                    repeated.input(0xa9).unwrap(),
                ]);
                assert_eq!(u64::from(h), 1000 - frame * (u64::from(total) + 1));
                assert_eq!(
                    u64::from(repeated.input(0xaa).unwrap()),
                    3u64.saturating_sub(frame)
                );
                assert_eq!(repeated.completed_frames, frame);
                assert_eq!(repeated.input(2).unwrap(), 0);
                assert_eq!(repeated.cycles, frame * u64::from(clocks));
            }
        }
    }
}

#[test]
fn runtime_writes_preserve_history_and_take_effect_at_the_next_line_end() {
    for model in MODELS {
        for (line, old, new) in [
            (10, 156, 180),
            (150, 156, 180),
            (10, 180, 156),
            (170, 180, 156),
            (150, 180, 0),
        ] {
            let mut b = Bus::new(model);
            out(&mut b, 0x16, old);
            b.tick(256 * (u32::from(old) + 1) * 2 + line * 256 + 73);
            let history = (b.cycles, b.raster_line, b.completed_frames);
            out(&mut b, 0x16, new);
            assert_eq!((b.cycles, b.raster_line, b.completed_frames), history);
            b.tick(182);
            assert_eq!(b.raster_line, line as u8);
            assert_eq!(b.completed_frames, 2);
            b.tick(1);
            if line >= u32::from(new) {
                assert_eq!((b.raster_line, b.completed_frames), (0, 3));
            } else {
                assert_eq!((b.raster_line, b.completed_frames), (line as u8 + 1, 2));
                b.tick((u32::from(new) - line) * 256);
                assert_eq!((b.raster_line, b.completed_frames), (0, 3));
            }
            b.tick(256 * (u32::from(new) + 1));
            assert_eq!((b.raster_line, b.completed_frames), (0, 4));
        }
    }
}

#[test]
fn all_short_odd_and_maximum_totals_terminate_and_chunking_matches_single_clocks() {
    for total in 0..=255 {
        let mut chunked = Bus::new(Model::Crystal);
        let mut single = Bus::new(Model::Crystal);
        for b in [&mut chunked, &mut single] {
            out(b, 0x16, total);
            out(b, 3, 0);
            out(b, 0xb2, 0xf0);
            out(b, 0xa4, 2);
            out(b, 0xa6, 1);
            out(b, 0xa2, 0xf);
        }
        for (new, clocks) in [
            (total, 256 * (u32::from(total) + 1) * 2 + 19),
            (0, 1000),
            (255, 70001),
        ] {
            out(&mut chunked, 0x16, new);
            out(&mut single, 0x16, new);
            chunked.tick(clocks);
            for _ in 0..clocks {
                single.tick(1);
            }
            assert_eq!(
                (
                    chunked.cycles,
                    chunked.raster_line,
                    chunked.completed_frames
                ),
                (single.cycles, single.raster_line, single.completed_frames)
            );
            assert_eq!(chunked.ports, single.ports);
            for port in [0xa8, 0xa9, 0xaa, 0xab] {
                assert_eq!(chunked.input(port).unwrap(), single.input(port).unwrap());
            }
            assert_eq!(chunked.audio.drain(), single.audio.drain());
        }
        let mut short = Bus::new(Model::Mono);
        out(&mut short, 0x16, total);
        out(&mut short, 0xb2, 0x40);
        short.tick(256 * (u32::from(total) + 1));
        assert_eq!(short.completed_frames, 1);
        assert_eq!(
            short.input(0xb4).unwrap(),
            if total < 144 { 0 } else { 0x40 }
        );
    }
}

#[test]
fn run_frame_follows_runtime_completion_and_preserves_instruction_and_dma_overshoot() {
    for model in MODELS {
        for (line, old, new) in [
            (10, 156, 180),
            (150, 156, 180),
            (10, 180, 156),
            (170, 180, 156),
            (150, 180, 0),
        ] {
            let mut m = machine(model);
            out(&mut m.bus, 0x16, old);
            m.bus.tick(line * 256 + 73);
            m.bus.ram[0x800..0x803].copy_from_slice(&[0xe6, 0x16, 0xf4]); // OUT 16,AL; HLT
            m.cpu.registers[0] = new as u16;
            m.run_frame().unwrap();
            assert_eq!(m.frame_number(), 1);
            let lines = if line >= new as u32 {
                line + 1
            } else {
                new as u32 + 1
            };
            assert_eq!(m.bus.cycles, u64::from(lines) * 256);
            assert_eq!(
                m.frame_rate(),
                3_072_000.0 / (256.0 * (f64::from(new) + 1.0))
            );
            for frame in 2..=4 {
                m.run_frame().unwrap();
                assert_eq!(m.frame_number(), frame);
            }
        }
        let mut m = machine(model);
        out(&mut m.bus, 0x16, 180);
        m.bus.tick(46335);
        m.bus.ram[0x800] = 0x43; // INC BX costs two clocks, crosses frame by one
        m.run_frame().unwrap();
        assert_eq!(
            (m.frame_number(), m.bus.cycles, m.bus.raster_line),
            (1, 46337, 0)
        );
        assert_eq!(m.cpu.registers[3], 1);
    }
    for model in [Model::Color, Model::Crystal] {
        let mut m = machine(model);
        out(&mut m.bus, 0x60, 0x80);
        out(&mut m.bus, 0x16, 156);
        m.bus.tick(40191);
        for (p, v) in [
            (0x40, 0),
            (0x41, 1),
            (0x44, 0),
            (0x45, 2),
            (0x46, 4),
            (0x48, 0x80),
        ] {
            out(&mut m.bus, p, v);
        }
        m.bus.ram[0x100..0x104].copy_from_slice(&[1, 2, 3, 4]);
        m.run_frame().unwrap();
        assert_eq!((m.frame_number(), m.bus.cycles), (1, 40198));
        assert_eq!(&m.bus.ram[0x200..0x204], &[1, 2, 0, 0]);
        assert_eq!(m.cpu.ip, 0x800);
        assert_eq!(m.bus.input(0x46).unwrap(), 2);
    }
}

#[test]
fn nondefault_frames_keep_audio_sdma_hyper_and_eeprom_on_elapsed_machine_clocks() {
    for total in [156, 180] {
        for hyper in [false, true] {
            let mut b = Bus::new(Model::Crystal);
            let cart =
                cartridge::Cartridge::parse(&cartridge::tests::rom(131072, 0x10, 0)).unwrap();
            b.insert_cartridge(cart);
            out(&mut b, 0x16, total);
            let clocks = 256 * (u32::from(total) + 1);
            for (p, v) in [(0xc6, 0x30), (0xc7, 1), (0xc8, 0x40)] {
                out(&mut b, p, v);
            }
            b.tick(3072); // WEN
            b.tick(clocks - 200 - 3072);
            // WRITE completes after 3072 machine clocks even across a raster wrap.
            for (p, v) in [(0xc4, 0x34), (0xc5, 0x12), (0xc6, 0x40), (0xc8, 0x20)] {
                out(&mut b, p, v);
            }
            for (p, v) in [
                (0x4a, 0),
                (0x4b, 0x10),
                (0x4e, 4),
                (0x52, if hyper { 0x93 } else { 0x83 }),
            ] {
                out(&mut b, p, v);
            }
            if hyper {
                out(&mut b, 0x6a, 0x89);
                out(&mut b, 0x6b, 0x70);
            }
            b.ram[0x1000..0x1004].copy_from_slice(&[10, 20, 30, 40]);
            let before = b.cycles;
            let events = (before + 512 + 11) / 128 - (before + 11) / 128;
            b.tick(512);
            assert_eq!(b.completed_frames, 1);
            assert_eq!(b.sound_dma_stall(), Some(events as u32 * 7));
            assert_eq!(b.input(0x4a).unwrap(), 4);
            assert_eq!(b.input(0x4e).unwrap(), 0);
            assert_eq!(b.input(0x52).unwrap() & 0x80, 0);
            assert_eq!(b.input(if hyper { 0x69 } else { 0x89 }).unwrap(), 40);
            assert_eq!(b.input(0xc8).unwrap() & 2, 0);
            b.tick(3072 - 512 - 1);
            assert_eq!(b.input(0xc8).unwrap() & 2, 0);
            b.tick(1);
            assert_eq!(b.input(0xc8).unwrap() & 2, 2);
            let samples = b.audio.drain();
            assert_eq!(samples.len() as u64, b.cycles / 128 * 2);
            assert_eq!(&b.cartridge.as_ref().unwrap().save[..2], &[0x34, 0x12]);
            assert!(b.cartridge.as_ref().unwrap().save[2..]
                .iter()
                .all(|v| *v == 255));
            b.tick(clocks);
            assert_eq!(b.completed_frames, 2);
            assert_eq!(b.audio.drain().len(), clocks as usize / 128 * 2);
        }
    }
}

#[test]
fn rendering_uses_the_same_runtime_raster_and_short_frames_retain_unvisited_rows() {
    for model in MODELS {
        for total in [156, 158, 180] {
            let mut m = machine(model);
            m.cpu.halted = true;
            out(&mut m.bus, 0x16, total);
            out(&mut m.bus, 0x14, 1);
            out(&mut m.bus, 0x1c, 15); // black backdrop in mono video mode
            m.run_frame().unwrap();
            assert!(m.framebuffer.chunks_exact(4).all(|p| p == [0, 0, 0, 255]));
            // Use the actual raster after full-height frames, including nondefault
            // totals. A shade LUT write during line 50 must target row 49.
            m.advance(50 * 256 + 128);
            out(&mut m.bus, 0x1c, 0);
            out(&mut m.bus, 0x16, if total == 156 { 180 } else { 156 });
            m.run_frame().unwrap();
            for (row, pixels) in m.framebuffer.chunks_exact(FRAME_WIDTH * 4).enumerate() {
                let expected = if row < 49 { [0, 0, 0, 255] } else { [255; 4] };
                assert!(
                    pixels.chunks_exact(4).all(|p| p == expected),
                    "{model:?} T={total} row={row}"
                );
            }
            // Short frames translate only rows with a visited successor and still finish.
            out(&mut m.bus, 0x16, 1);
            out(&mut m.bus, 0x1c, 5); // gray 170
            m.run_frame().unwrap();
            assert_eq!(m.frame_number(), 3);
            for row in 0..FRAME_HEIGHT {
                let expected = if row == 0 {
                    [170, 170, 170, 255]
                } else if row < 49 {
                    [0, 0, 0, 255]
                } else {
                    [255; 4]
                };
                assert_eq!(
                    &m.framebuffer[row * FRAME_WIDTH * 4..row * FRAME_WIDTH * 4 + 4],
                    expected
                );
            }
        }
    }
}
