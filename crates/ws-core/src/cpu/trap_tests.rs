// SPDX-License-Identifier: CC0-1.0
use super::*;

fn setup(model: Model, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
    b.ram[0x300] = 0xcf;
    (
        Cpu {
            segments: [0; 4],
            registers: [0, 0, 0, 0, 0x2000, 0, 0, 0],
            ip: 0x1000,
            ..Cpu::default()
        },
        b,
    )
}

fn word(b: &Bus, address: usize) -> u16 {
    u16::from_le_bytes([b.ram[address], b.ram[address + 1]])
}

#[test]
fn issue_67_popf_nop_reproduction() {
    let (mut c, mut b) = setup(Model::Mono, &[0x9d, 0x90, 0xbb, 0x34, 0x12]);
    c.registers[4] = 0x1ffe;
    b.ram[0x1ffe..0x2000].copy_from_slice(&[2, 0xf1]);
    c.step(&mut b).unwrap();
    assert_eq!((c.ip, c.flags, c.registers[4]), (0x1001, 0xf102, 0x2000));
    assert_eq!(c.step(&mut b).unwrap(), 27);
    assert_eq!((c.segments[1], c.ip, c.flags), (0, 0x300, 0xf002));
    assert_eq!(c.registers[3], 0);
    assert_eq!(c.registers[4], 0x1ffa);
    assert_eq!(
        (word(&b, 0x1ffa), word(&b, 0x1ffc), word(&b, 0x1ffe)),
        (0x1002, 0, 0xf102)
    );
}

fn configurations() -> impl Iterator<Item = (Model, u8)> {
    [Model::Mono, Model::Color, Model::Crystal]
        .into_iter()
        .flat_map(|m| {
            [0, 0x80, 0xc0, 0xe0]
                .into_iter()
                .filter(move |mode| m != Model::Mono || *mode == 0)
                .map(move |mode| (m, mode))
        })
}

pub(super) fn install_handler(b: &mut Bus) {
    b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
    b.ram[0x300] = 0xcf;
}

// Inspect the architectural frame, then execute the real IRET. Used by older
// scheduler regressions to retain their full FLAGS observations and continuation.
pub(super) fn finish_trap(c: &mut Cpu, b: &mut Bus, ip: u16, flags: u16) -> u32 {
    finish_trap_at(c, b, ip, 0, flags)
}

pub(super) fn finish_trap_at(c: &mut Cpu, b: &mut Bus, ip: u16, cs: u16, flags: u16) -> u32 {
    assert_eq!((c.segments[1], c.ip, c.flags), (0, 0x300, flags & !0x300));
    let address = Cpu::physical(c.segments[2], c.registers[4]) as usize;
    assert_eq!(
        (word(b, address), word(b, address + 2), word(b, address + 4)),
        (ip, cs, flags)
    );
    let cycles = c.step(b).unwrap();
    assert_eq!((c.ip, c.flags), (ip, flags));
    cycles
}

#[test]
fn activation_iret_handler_stop_and_if_are_independent_on_every_configuration() {
    for (model, mode) in configurations() {
        for flags in [0xf102u16, 0xf302, 0xffd7] {
            let (mut c, mut b) = setup(model, &[0x9d, 0x90, 0x90, 0xbb, 0x34, 0x12, 0x90]);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            c.registers[4] = 0x1ffe;
            b.ram[0x1ffe..0x2000].copy_from_slice(&flags.to_le_bytes());
            c.step(&mut b).unwrap(); // enable without trapping POPF
            for ip in [0x1002, 0x1003] {
                c.step(&mut b).unwrap();
                assert_eq!(c.registers[4], 0x1ffa);
                assert_eq!(c.registers[3], 0);
                assert_eq!(
                    finish_trap(&mut c, &mut b, ip, flags),
                    11 + u32::from(ip & 1)
                );
                assert_eq!(c.registers[4], 0x2000);
            }
            // Real handler clears stacked TF, preserving the other flag bits.
            b.ram[0x300..0x30a]
                .copy_from_slice(&[0x55, 0x89, 0xe5, 0x81, 0x66, 6, 0xff, 0xfe, 0x5d, 0xcf]);
            c.step(&mut b).unwrap(); // MOV BX,1234 traps once
            assert_eq!(word(&b, 0x1ffa), 0x1006);
            for _ in 0..5 {
                c.step(&mut b).unwrap();
            }
            assert_eq!(
                (c.ip, c.flags, c.registers[4], c.registers[3]),
                (0x1006, flags & !0x100, 0x2000, 0x1234)
            );
            c.step(&mut b).unwrap();
            assert_eq!((c.ip, c.registers[4]), (0x1007, 0x2000));
        }
        // IRET activation without a preceding vector-1 entry; reserved bits normalize.
        let (mut c, mut b) = setup(model, &[0xcf]);
        b.ram[0x1ffa..0x2000].copy_from_slice(&[0, 0x11, 0, 0, 0xff, 0xff]);
        b.ram[0x1100] = 0x90;
        c.registers[4] = 0x1ffa;
        c.step(&mut b).unwrap();
        assert_eq!((c.ip, c.flags, c.registers[4]), (0x1100, 0xffd7, 0x2000));
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1101, 0xffd7);
    }
}

#[test]
fn ss_loads_suppress_only_their_boundary_and_prefixes_are_indivisible() {
    let cases: &[(&[u8], bool)] = &[
        (&[0x8e, 0xd0], true),       // MOV SS,AX
        (&[0x8e, 0x16, 0, 8], true), // MOV SS,[0800]
        (&[0x17], true),             // POP SS
        (&[0x8e, 0xc0], false),      // MOV ES,AX
        (&[0x8e, 0xd8], false),      // MOV DS,AX
        (&[0x8c, 0xd0], false),      // MOV AX,SS
        (&[0x07], false),
        (&[0x1f], false),
        (&[0x16], false),
        (&[0x26, 0xf0, 0x8e, 0xd0], true),
        (&[0xf0, 0x26, 0x90], false),
        (&[0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0, 0x90], false),
    ];
    for (model, mode) in configurations() {
        for &(code, suppressed) in cases {
            let mut program = code.to_vec();
            program.extend([0x90, 0xbb, 0x34, 0x12]);
            let (mut c, mut b) = setup(model, &program);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            c.flags = 0xf102;
            c.registers[0] = 0x10;
            b.ram[0x800..0x802].copy_from_slice(&[0x10, 0]);
            b.ram[0x2000..0x2002].copy_from_slice(&[0x10, 0]);
            c.step(&mut b).unwrap();
            if suppressed {
                assert_eq!(c.ip, 0x1000 + code.len() as u16);
                assert_eq!(c.interrupt_shadow, 1);
                c.step(&mut b).unwrap();
            }
            let saved_ip = 0x1000 + code.len() as u16 + u16::from(suppressed);
            assert_eq!(c.registers[3], 0);
            finish_trap(&mut c, &mut b, saved_ip, 0xf102);
        }
        // The independent POPF+STI vector: STI must not delay single-step.
        let (mut c, mut b) = setup(model, &[0x9d, 0xfb, 0x90]);
        c.registers[4] = 0x1ffe;
        b.ram[0x1ffe..0x2000].copy_from_slice(&[2, 0xf1]);
        c.step(&mut b).unwrap();
        c.step(&mut b).unwrap();
        assert_eq!(word(&b, 0x1ffa), 0x1002);
        assert_eq!(word(&b, 0x1ffe), 0xf302);
        assert_eq!(c.interrupt_shadow, 0); // entry consumes the application shadow
    }
}

fn irq(b: &mut Bus) {
    b.output(0xb0, 0x20).unwrap();
    b.output(0xb2, 0x80).unwrap();
    b.request(7);
    b.ram[0x9c..0xa0].copy_from_slice(&[0, 4, 0, 0]);
    b.ram[0x400..0x405].copy_from_slice(&[0xb0, 0x80, 0xe6, 0xb6, 0xcf]);
}

#[test]
fn irq_priority_before_execution_and_at_trap_boundary_has_one_frame() {
    for (model, mode) in configurations() {
        for boundary in [false, true] {
            let (mut c, mut b) = setup(
                model,
                if boundary {
                    &[0x9d, 0x90, 0x90]
                } else {
                    &[0x90, 0x90, 0x90]
                },
            );
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            if boundary {
                c.registers[4] = 0x1ffe;
                b.ram[0x1ffe..0x2000].copy_from_slice(&[2, 0xf3]);
                c.step(&mut b).unwrap(); // POPF external-INT delay
            } else {
                c.flags = 0xf302;
            }
            irq(&mut b);
            c.step(&mut b).unwrap();
            let ip = if boundary { 0x1002 } else { 0x1000 };
            assert_eq!((c.ip, c.flags, c.registers[4]), (0x400, 0xf002, 0x1ffa));
            assert_eq!(
                (word(&b, 0x1ffa), word(&b, 0x1ffc), word(&b, 0x1ffe)),
                (ip, 0, 0xf302)
            );
            for _ in 0..3 {
                c.step(&mut b).unwrap();
            } // acknowledge and IRET
            assert_eq!((c.ip, c.registers[4], c.flags), (ip, 0x2000, 0xf302));
            assert_eq!(b.interrupt(), None);
            c.step(&mut b).unwrap(); // one ordinary application instruction
            finish_trap(&mut c, &mut b, ip + 1, 0xf302);
        }
        // IF clear: a pending IRQ cannot prevent vector 1.
        let (mut c, mut b) = setup(model, &[0x90]);
        c.flags = 0xf102;
        irq(&mut b);
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1001, 0xf102);
        assert_eq!(b.interrupt(), Some(0x27));
        // STI protects against external INT but still delivers TF immediately.
        let (mut c, mut b) = setup(model, &[0xfb, 0x90]);
        c.flags = 0xf102;
        irq(&mut b);
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1001, 0xf302);
    }
}

#[test]
fn synchronous_entries_consume_tf_boundary_without_nested_traps() {
    let cases: &[(&[u8], u8, u16, u16)] = &[
        (&[0xcc], 3, 0x1001, 0xf302),
        (&[0xcd, 0x30], 0x30, 0x1002, 0xf302),
        (&[0xce], 4, 0x1001, 0xfb02),
        (&[0x62, 0x06, 0, 8], 5, 0x1000, 0xf302),
        (&[0xf6, 0xf3], 0, 0x1000, 0xf302),
        (&[0xf7, 0xf3], 0, 0x1000, 0xf302),
        (&[0xd4, 0], 0, 0x1000, 0xf302),
    ];
    for (model, _) in configurations() {
        for &(code, vector, ip, flags) in cases {
            let (mut c, mut b) = setup(model, code);
            c.flags = flags;
            c.registers[0] = 5;
            b.ram[vector as usize * 4..vector as usize * 4 + 4].copy_from_slice(&[0, 5, 0, 0]);
            b.ram[0x500] = 0xcf;
            // A pending IRQ delayed by SS must not outrank the synchronous result.
            irq(&mut b);
            c.interrupt_shadow = 1;
            c.step(&mut b).unwrap();
            assert_eq!(
                (c.ip, c.flags, c.registers[4]),
                (0x500, flags & !0x300, 0x1ffa)
            );
            assert_eq!(
                (word(&b, 0x1ffa), word(&b, 0x1ffc), word(&b, 0x1ffe)),
                (ip, 0, flags)
            );
            c.step(&mut b).unwrap();
            assert_eq!((c.ip, c.flags, c.registers[4]), (ip, flags, 0x2000));
            // No vector 1 frame or handler instruction was manufactured.
        }
        let (mut c, mut b) = setup(model, &[0xce]);
        c.flags = 0xf102;
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1001, 0xf102);
    }
}

#[test]
fn rep_element_final_zero_comparison_and_irq_restart_traces() {
    for (model, mode) in configurations() {
        for count in [0, 1, 3] {
            let (mut c, mut b) = setup(model, &[0xf0, 0x26, 0xf3, 0xa4, 0xbb, 0x34, 0x12]);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            c.flags = 0xf102;
            c.registers[1] = count;
            c.registers[6] = 0x800;
            c.registers[7] = 0x900;
            b.ram[0x800..0x803].copy_from_slice(&[0x35, 0x67, 0x89]);
            for done in 1..=count.max(1) {
                let cycles = c.step(&mut b).unwrap();
                assert_eq!(cycles, if count == 0 { 30 } else { 34 });
                let progress = if count == 0 { 0 } else { done };
                assert_eq!(
                    (c.registers[1], c.registers[6], c.registers[7]),
                    (count - progress, 0x800 + progress, 0x900 + progress)
                );
                assert_eq!(
                    &b.ram[0x900..0x900 + progress as usize],
                    &[0x35, 0x67, 0x89][..progress as usize]
                );
                assert_eq!(b.ram[0x900 + progress as usize], 0);
                finish_trap(
                    &mut c,
                    &mut b,
                    if done < count { 0x1000 } else { 0x1004 },
                    0xf102,
                );
            }
        }
        // REPE and REPNE terminate early on opposite comparison results.
        for (prefix, src, dst, flags) in [(0xf3, 1, 2, 0xf197), (0xf2, 2, 2, 0xf146)] {
            let (mut c, mut b) = setup(model, &[prefix, 0xa6]);
            c.flags = 0xf102;
            c.registers[1] = 3;
            c.registers[6] = 0x800;
            c.registers[7] = 0x900;
            b.ram[0x800] = src;
            b.ram[0x900] = dst;
            c.step(&mut b).unwrap();
            assert_eq!(
                (c.registers[1], c.registers[6], c.registers[7]),
                (2, 0x801, 0x901)
            );
            finish_trap(&mut c, &mut b, 0x1002, flags);
        }
        // IRQ after the first IRET runs one protected element, then outranks
        // TF at the next boundary. Resume the remaining element exactly once.
        let (mut c, mut b) = setup(model, &[0xf0, 0x26, 0xf3, 0xa4]);
        c.flags = 0xf302;
        c.registers[1] = 3;
        c.registers[6] = 0x800;
        c.registers[7] = 0x900;
        b.ram[0x800..0x803].copy_from_slice(&[3, 7, 11]);
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1000, 0xf302);
        irq(&mut b);
        c.step(&mut b).unwrap();
        assert_eq!((c.ip, word(&b, 0x1ffa)), (0x400, 0x1000));
        assert_eq!(
            (c.registers[1], c.registers[6], c.registers[7]),
            (1, 0x802, 0x902)
        );
        assert_eq!(&b.ram[0x900..0x904], &[3, 7, 0, 0]);
        for _ in 0..3 {
            c.step(&mut b).unwrap();
        }
        c.step(&mut b).unwrap();
        finish_trap(&mut c, &mut b, 0x1004, 0xf302);
        assert_eq!(&b.ram[0x900..0x904], &[3, 7, 11, 0]);
    }
}

#[test]
fn hlt_completion_traps_but_sustained_idle_does_not_execute() {
    for (model, _) in configurations() {
        let (mut c, mut b) = setup(model, &[0xf4, 0xbb, 0x34, 0x12]);
        c.flags = 0xf102;
        assert_eq!(c.step(&mut b).unwrap(), 27);
        assert!(!c.halted);
        assert_eq!(c.registers[3], 0);
        finish_trap(&mut c, &mut b, 0x1001, 0xf102);
        c.step(&mut b).unwrap();
        assert_eq!(c.registers[3], 0x1234);
        let (mut c, mut b) = setup(model, &[0xf4, 0xbb, 0x34, 0x12]);
        c.step(&mut b).unwrap();
        assert!(c.halted);
        c.flags = 0xf102; // even a host-side edit must not turn idle into execution
        let state = format!("{c:?}");
        for _ in 0..8 {
            assert_eq!(c.step(&mut b).unwrap(), 1);
            assert_eq!(format!("{c:?}"), state);
        }
    }
}

#[test]
fn trap_entry_internal_odd_and_waiting_stack_costs() {
    for (model, mode) in configurations() {
        for (ss, sp, cost) in [(0, 0x2000, 27), (0, 0x2001, 30), (0x1000, 0x2000, 30)] {
            let (mut c, mut b) = setup(model, &[0x90]);
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            b.insert_cartridge(
                crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                    .unwrap(),
            );
            if model != Model::Mono {
                b.output(0x60, mode | 2).unwrap();
            } // SRAM wait
            c.flags = 0xf102;
            c.segments[2] = ss;
            c.registers[4] = sp;
            // SRAM has two byte accesses per word, plus one wait per lane.
            assert_eq!(
                c.step(&mut b).unwrap(),
                if ss == 0x1000 {
                    if model == Model::Mono {
                        30
                    } else {
                        36
                    }
                } else {
                    cost
                }
            );
        }
    }
}

#[test]
fn ordinary_traps_preserve_arithmetic_flags_and_multiplication_history() {
    for (model, mode) in configurations() {
        // IMUL AL; PUSH FFD7; POPF; NOP; MOV AX,6; MOV BX,2; DIV BL.
        let (mut c, mut b) = setup(
            model,
            &[
                0xf6, 0xe8, 0x68, 0xd7, 0xff, 0x9d, 0x90, 0xb8, 6, 0, 0xbb, 2, 0, 0xf6, 0xf3,
            ],
        );
        if model != Model::Mono {
            b.output(0x60, mode).unwrap();
        }
        c.registers[0] = 0x10;
        for _ in 0..3 {
            c.step(&mut b).unwrap();
        }
        assert_eq!((c.ip, c.flags, c.registers[0]), (0x1006, 0xffd7, 0x100));
        for (ip, ax, bx, flags) in [
            (0x1007, 0x100, 0, 0xffd7),
            (0x100a, 6, 0, 0xffd7),
            (0x100d, 6, 2, 0xffd7),
            (0x100f, 3, 2, 0xff43),
        ] {
            c.step(&mut b).unwrap();
            assert_eq!(
                (c.registers[0], c.registers[3], c.registers[4]),
                (ax, bx, 0x1ffa)
            );
            finish_trap(&mut c, &mut b, ip, flags);
        }
        // Ordinary delivery uses the completed operation's FLAGS, including
        // physical-model multiplication behavior, rather than input FLAGS.
        for (code, ax, bx, result, flags) in [
            (&[0xf7, 0xf3][..], 6, 2, 3, 0xf742),
            (&[0xf6, 0xfb][..], 0xfffa, 2, 0x00fd, 0xf782),
            (
                &[0xf6, 0xe3][..],
                3,
                2,
                6,
                if model == Model::Mono { 0xf702 } else { 0xf742 },
            ),
            (&[0xd0, 0xe0][..], 0x40, 2, 0x80, 0xff82),
        ] {
            let (mut c, mut b) = setup(model, code);
            c.flags = 0xffd7;
            c.registers[0] = ax;
            c.registers[3] = bx;
            c.step(&mut b).unwrap();
            assert_eq!(c.registers[0], result);
            finish_trap(&mut c, &mut b, 0x1002, flags);
        }
    }
}

#[test]
fn cloned_ss_suppression_and_lifecycle_operations_isolate_trap_state() {
    let rom = crate::cartridge::tests::rom(131072, 2, 0);
    let mut color_rom = rom.clone();
    let model_field = color_rom.len() - 9;
    color_rom[model_field] = 1;
    crate::cartridge::tests::checksum(&mut color_rom);
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model, &[0x8e, 0xd0, 0x90]);
        c.flags = 0xf102;
        c.step(&mut b).unwrap();
        for mut copy in [c.clone(), c] {
            copy.step(&mut b).unwrap();
            finish_trap(&mut copy, &mut b, 0x1003, 0xf102);
        }
        let mut m = crate::Machine::new(model);
        for loaded in [false, true] {
            if loaded {
                m.load_rom(&rom).unwrap();
            }
            m.cpu.flags = 0xf302;
            m.cpu.interrupt_shadow = 1;
            m.restart();
            assert_eq!(m.cpu.flags & 0x300, 0);
            assert_eq!(m.cpu.interrupt_shadow, 0);
        }
        m.cpu.flags = 0xf302;
        m.cpu.interrupt_shadow = 1;
        m.reset(model);
        assert_eq!(m.cpu.flags & 0x300, 0);
        assert_eq!(m.cpu.interrupt_shadow, 0);
        for _ in 0..2 {
            m.cpu.flags = 0xf302;
            m.cpu.interrupt_shadow = 1;
            m.load_rom(&rom).unwrap();
            assert_eq!(m.cpu.flags & 0x300, 0);
            assert_eq!(m.cpu.interrupt_shadow, 0);
        }
        for next in [Model::Mono, Model::Color, Model::Crystal, Model::Crystal] {
            m.cpu.flags = 0xf302;
            m.cpu.interrupt_shadow = 1;
            m.change_model(next).unwrap();
            assert_eq!(m.cpu.flags & 0x300, 0);
            assert_eq!(m.cpu.interrupt_shadow, 0);
        }
        m.change_model(Model::Mono).unwrap();
        let (mut c, b) = setup(Model::Mono, &[0x8e, 0xd0, 0x90]);
        c.flags = 0xf102;
        m.cpu = c;
        m.bus = b;
        m.cpu.step(&mut m.bus).unwrap();
        for data in [&[0][..], &color_rom] {
            let cpu = format!("{:?}", m.cpu);
            let ram = m.bus.ram.clone();
            let cycles = m.bus.cycles;
            assert!(m.load_rom(data).is_err());
            assert_eq!(format!("{:?}", m.cpu), cpu);
            assert_eq!(m.bus.ram, ram);
            assert_eq!(m.bus.cycles, cycles);
        }
        m.cpu.step(&mut m.bus).unwrap();
        finish_trap(&mut m.cpu, &mut m.bus, 0x1003, 0xf102);
        m.change_model(Model::Color).unwrap();
        m.load_rom(&color_rom).unwrap();
        m.cpu.flags = 0xf302;
        m.cpu.interrupt_shadow = 1;
        let cpu = format!("{:?}", m.cpu);
        let ram = m.bus.ram.clone();
        assert!(m.change_model(Model::Mono).is_err());
        assert_eq!(format!("{:?}", m.cpu), cpu);
        assert_eq!(m.bus.ram, ram);
        assert_eq!(m.model(), Model::Color);
    }
}

#[test]
fn normal_machine_schedules_instruction_trap_dma_and_idle_costs_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model, &[0x90, 0xbb, 0x34, 0x12]);
        b.insert_cartridge(
            crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
                .unwrap(),
        );
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            for kind in 0..4 {
                if kind >= 2 && (model == Model::Mono || mode == 0) {
                    continue;
                }
                m.cpu.ip = 0x1000;
                m.cpu.registers[3] = 0;
                m.cpu.registers[4] = 0x2000;
                m.cpu.flags = 0xf102;
                m.cpu.halted = kind == 1;
                // 0: NOP + trap; 1: sustained HALT; 2: GDMA only; 3: SDMA stall only.
                let cost = match kind {
                    0 => 27,
                    1 => 1,
                    2 => 7,
                    _ => 7,
                };
                let remaining = crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                m.advance((remaining - cost) as u32);
                m.bus.output_word(0xa4, 2).unwrap();
                m.bus.output(0xa2, 1).unwrap();
                if kind == 2 {
                    for (p, v) in [
                        (0x40, 0),
                        (0x41, 8),
                        (0x42, 0),
                        (0x44, 0),
                        (0x45, 9),
                        (0x46, 2),
                        (0x47, 0),
                        (0x48, 0x80),
                    ] {
                        m.bus.output(p, v).unwrap();
                    }
                    m.bus.ram[0x800..0x802].copy_from_slice(&[0x51, 0xa2]);
                }
                if kind == 3 {
                    // Position a real SDMA event, consume only its resulting stall.
                    // At frame end-minus-seven the previous 128-clock event is 117.
                    for (p, v) in [
                        (0x4a, 0),
                        (0x4b, 8),
                        (0x4c, 0),
                        (0x4e, 1),
                        (0x4f, 0),
                        (0x50, 0),
                        (0x52, 0x83),
                    ] {
                        m.bus.output(p, v).unwrap();
                    }
                    // Queue the event on an independent prior tick and reposition
                    // to the following frame boundary without draining CPU stalls.
                    let until = (117 + 128 - m.bus.cycles % 128) % 128;
                    m.advance(until as u32);
                    m.bus.output(0x52, 0).unwrap();
                    let remaining =
                        crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                    m.advance((remaining - 7) as u32);
                    m.bus.output_word(0xa4, 2).unwrap();
                    m.bus.output(0xa2, 1).unwrap();
                }
                let before = m.bus.cycles;
                let frame = m.frame_number();
                m.audio_samples();
                m.run_frame().unwrap();
                assert_eq!(m.bus.cycles, before + cost);
                assert_eq!(m.frame_number(), frame + 1);
                assert_eq!(m.bus.raster_line, 0);
                assert_eq!(m.audio_samples().len(), 2);
                assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                assert_eq!(m.cpu.registers[3], 0);
                if kind == 0 {
                    assert_eq!(
                        (m.cpu.ip, m.cpu.flags, m.cpu.registers[4]),
                        (0x300, 0xf002, 0x1ffa)
                    );
                    assert_eq!(
                        (word(&m.bus, 0x1ffa), word(&m.bus, 0x1ffe)),
                        (0x1001, 0xf102)
                    );
                } else {
                    assert_eq!(
                        (m.cpu.ip, m.cpu.flags, m.cpu.registers[4]),
                        (0x1000, 0xf102, 0x2000)
                    );
                    if kind == 2 {
                        assert_eq!(&m.bus.ram[0x900..0x902], &[0x51, 0xa2]);
                    }
                }
                m.cpu.halted = false;
            }
        }
    }
}
