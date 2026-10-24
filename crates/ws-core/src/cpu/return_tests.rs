// SPDX-License-Identifier: CC0-1.0
use super::call_tests::{mapped_code, setup};
use super::*;

// Literal pre-edit vectors: docs/return-timing.md. Each tuple is
// opcode, words popped, even-opcode total, odd-opcode total, immediate form.
const FORMS: [(u8, u16, u32, u32, bool); 5] = [
    (0xc3, 1, 7, 7, false),
    (0xc2, 1, 9, 8, true),
    (0xcb, 2, 9, 9, false),
    (0xca, 2, 12, 11, true),
    (0xcf, 3, 11, 11, false),
];

fn modes(model: Model) -> impl Iterator<Item = u8> {
    [0, 0x80, 0, 0xc0, 0, 0xe0, 0]
        .into_iter()
        .filter(move |mode| model != Model::Mono || *mode == 0)
}

// Setup only; independent address arithmetic, never a CPU timing helper.
fn put_frame(b: &mut Bus, ss: u16, sp: u16, values: &[u16]) {
    for (i, value) in values.iter().enumerate() {
        for (lane, byte) in value.to_le_bytes().into_iter().enumerate() {
            let offset = sp.wrapping_add((i * 2 + lane) as u16);
            b.write(((u32::from(ss) << 4) + u32::from(offset)) & 0xfffff, byte);
        }
    }
}

fn literal_form(form: usize) {
    let (opcode, words, even, odd, immediate) = FORMS[form];
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in modes(model) {
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (ip, base, fetch) in [
                (0x1000u16, even, if immediate { 3 } else { 1 }),
                (0x1001, odd, if immediate { 2 } else { 1 }),
            ] {
                for sp in [0x2800u16, 0x2801] {
                    for target in [0x1200u16, 0x1201] {
                        for flags in [0u16, 0xffff, 0xf002, 0xfcd7] {
                            for adjust in if immediate {
                                &[0u16, 1, 2, 0x8000, 0xffff][..]
                            } else {
                                &[0][..]
                            } {
                                c.segments = [0x10, 0x40, 0x20, 0x30];
                                c.ip = ip;
                                c.registers[4] = sp;
                                c.flags = flags;
                                c.interrupt_shadow = 0;
                                let physical = 0x400 + usize::from(ip);
                                b.ram[physical] = opcode;
                                b.ram[physical + 1..physical + 3]
                                    .copy_from_slice(&adjust.to_le_bytes());
                                put_frame(
                                    &mut b,
                                    0x20,
                                    sp,
                                    &[target, 0x60, flags][..words as usize],
                                );
                                let ram = b.ram.clone();
                                let ports = b.ports;
                                let mut regs = c.registers;
                                regs[4] = sp.wrapping_add(2 * words).wrapping_add(*adjust);
                                let segments =
                                    [0x10, if words > 1 { 0x60 } else { 0x40 }, 0x20, 0x30];
                                let expected_flags = if opcode == 0xcf {
                                    (flags & 0x0fd5) | 0xf002
                                } else {
                                    flags
                                };
                                assert_eq!(
                                    c.instruction_step(&mut b).unwrap(),
                                    base + u32::from(sp & 1) * u32::from(words)
                                        + u32::from(target & 1),
                                    "{opcode:x}/{model:?}/{mode:x}/{ip:x}/{sp:x}/{target:x}"
                                );
                                assert_eq!(
                                    b.accesses.get(),
                                    fetch + u32::from(words) * (1 + u32::from(sp & 1)),
                                    "no duplicate reads or destination fetch"
                                );
                                assert_eq!(
                                    (c.registers, c.segments, c.ip, c.flags),
                                    (regs, segments, target, expected_flags)
                                );
                                assert!(b.ram == ram, "frame and unrelated RAM preserved");
                                assert_eq!(b.ports, ports);
                                assert_eq!(b.cycles, 0);
                                assert!(!c.halted);
                                assert_eq!(c.interrupt_shadow, 0);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn literal_c3_return_vectors() {
    literal_form(0);
}
#[test]
fn literal_c2_return_vectors() {
    literal_form(1);
}
#[test]
fn literal_cb_return_vectors() {
    literal_form(2);
}
#[test]
fn literal_ca_return_vectors() {
    literal_form(3);
}
#[test]
fn literal_cf_return_vectors() {
    literal_form(4);
}

#[test]
fn return_mapped_fetch_stack_prefixes_waits_and_rep_diagnostics() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for mode in modes(model) {
            for wait in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                }
                for (a0, lane, aligned) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                    b.output(0xa0, a0).unwrap();
                    for cs in [0u16, 0x1000, 0x2000] {
                        for ss in [0x20u16, 0x1000, 0x3000] {
                            // Avoid aliasing instruction bytes with the frame.
                            for (sp, internal) in [(0x2800u16, 1), (0x2801, 2)] {
                                for (opcode, words, even, _, immediate) in FORMS {
                                    for target in [0x1200u16, 0x1201] {
                                        for count in 0..=7 {
                                            c.segments = [0x10, cs, ss, 0x30];
                                            c.ip = 0x1000;
                                            c.flags = 0xf042;
                                            c.registers[4] = sp;
                                            let mut code =
                                                [0x26, 0xf0, 0x2e, 0x36, 0x3e, 0xf0, 0xf0][..count]
                                                    .to_vec();
                                            code.push(opcode);
                                            if immediate {
                                                code.extend([1, 0]);
                                            }
                                            let addresses: Vec<u32> = (0..code.len())
                                                .map(|i| u32::from(cs) * 16 + 0x1000 + i as u32)
                                                .collect();
                                            if cs == 0x1000 {
                                                for (&a, &v) in addresses.iter().zip(&code) {
                                                    b.write(a, v);
                                                }
                                            } else {
                                                mapped_code(&mut b, &addresses, &code);
                                            }
                                            if ss == 0x3000 {
                                                let bytes: Vec<u8> = [target, 0x60, 0xf042]
                                                    [..words as usize]
                                                    .iter()
                                                    .flat_map(|w| w.to_le_bytes())
                                                    .collect();
                                                let addresses: Vec<u32> = (0..bytes.len())
                                                    .map(|i| 0x30000 + u32::from(sp) + i as u32)
                                                    .collect();
                                                mapped_code(&mut b, &addresses, &bytes);
                                            } else {
                                                put_frame(
                                                    &mut b,
                                                    ss,
                                                    sp,
                                                    &[target, 0x60, 0xf042][..words as usize],
                                                );
                                            }
                                            let ram = b.ram.clone();
                                            let save = b.cartridge.as_ref().unwrap().save.clone();
                                            let ports = b.ports;
                                            let mut regs = c.registers;
                                            regs[4] += 2 * words + u16::from(immediate);
                                            let sram_lane = if model != Model::Mono && wait != 0 {
                                                2
                                            } else {
                                                1
                                            };
                                            let byte = if cs == 0 {
                                                1
                                            } else if cs == 0x1000 {
                                                sram_lane
                                            } else {
                                                lane
                                            };
                                            let word = if (count + 1) % 2 == 1 {
                                                2 * byte
                                            } else if cs == 0 {
                                                1
                                            } else if cs == 0x1000 {
                                                2 * sram_lane
                                            } else {
                                                aligned
                                            };
                                            let fetch = (count as u32 + 1) * byte
                                                + if immediate { word } else { 0 };
                                            let stack = u32::from(words)
                                                * if ss == 0x20 {
                                                    internal
                                                } else if ss == 0x1000 {
                                                    2 * sram_lane
                                                } else if sp & 1 != 0 {
                                                    2 * lane
                                                } else {
                                                    aligned
                                                };
                                            // Derive residual from independently recorded total by removing
                                            // its literal normal fetch/stack costs, not from production.
                                            let residual = even
                                                - if immediate { 3 } else { 1 }
                                                - u32::from(words);
                                            assert_eq!(c.instruction_step(&mut b).unwrap(),fetch+stack+residual+u32::from(target&1),"{model:?}/{mode:x}/{a0:x}/{cs:x}/{ss:x}/{opcode:x}/{count}");
                                            assert_eq!(b.accesses.get(), fetch + stack);
                                            assert_eq!(
                                                (c.registers, c.segments, c.ip, c.flags),
                                                (
                                                    regs,
                                                    [
                                                        0x10,
                                                        if words > 1 { 0x60 } else { cs },
                                                        ss,
                                                        0x30
                                                    ],
                                                    target,
                                                    0xf042
                                                )
                                            );
                                            assert!(b.ram == ram);
                                            assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                                            assert_eq!(b.ports, ports);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        for (opcode, _, _, _, _) in FORMS {
            for prefix in [0xf2, 0xf3] {
                c.segments[1] = 0;
                c.ip = 0x1000;
                b.ram[0x1000..0x1002].copy_from_slice(&[prefix, opcode]);
                let before = (c.registers, c.segments, c.flags);
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.segments, c.flags), before);
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn return_instruction_offset_bank_and_twenty_bit_wraps() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in modes(model) {
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (control, lane, aligned) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                b.output(0xa0, control).unwrap();
                for (cs, ip, addresses, immediate_cost) in [
                    (0x2000, 0xffff, [0x2ffff, 0x20000, 0x20001], aligned),
                    (0x2000, 0xfffe, [0x2fffe, 0x2ffff, 0x20000], 2 * lane),
                    (0x2800, 0x7fff, [0x2ffff, 0x30000, 0x30001], aligned),
                    (0x2800, 0x7ffe, [0x2fffe, 0x2ffff, 0x30000], 2 * lane),
                    (0xffff, 0xf, [0xfffff, 0, 1], 1),
                    (0xffff, 0xe, [0xffffe, 0xfffff, 0], lane + 1),
                ] {
                    for (opcode, words, even, _, immediate) in FORMS {
                        for target in [0x1200u16, 0x1201] {
                            mapped_code(&mut b, &addresses, &[opcode, 1, 0]);
                            put_frame(
                                &mut b,
                                0x20,
                                0x2800,
                                &[target, 0x60, 0xffff][..words as usize],
                            );
                            c.segments = [0x10, cs, 0x20, 0x30];
                            c.ip = ip;
                            c.flags = 0xfcd7;
                            c.registers[4] = 0x2800;
                            c.interrupt_shadow = 0;
                            let mut regs = c.registers;
                            regs[4] += 2 * words + u16::from(immediate);
                            let ram = b.ram.clone();
                            let ports = b.ports;
                            let fetch = lane + if immediate { immediate_cost } else { 0 };
                            let residual = even - if immediate { 3 } else { 1 } - u32::from(words);
                            assert_eq!(
                                c.instruction_step(&mut b).unwrap(),
                                fetch + u32::from(words) + residual + u32::from(target & 1)
                            );
                            assert_eq!(b.accesses.get(), fetch + u32::from(words));
                            assert_eq!(
                                (c.registers, c.segments, c.ip, c.flags),
                                (
                                    regs,
                                    [0x10, if words > 1 { 0x60 } else { cs }, 0x20, 0x30],
                                    target,
                                    if opcode == 0xcf { 0xffd7 } else { 0xfcd7 }
                                )
                            );
                            assert!(b.ram == ram);
                            assert_eq!(b.ports, ports);
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn return_stack_offset_high_ram_bank_and_twenty_bit_boundaries() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for mode in modes(model) {
            for wait in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                }
                let high = model != Model::Mono && mode != 0;
                let sram_lane = if model != Model::Mono && wait != 0 {
                    2
                } else {
                    1
                };
                for (a0, lane, aligned) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                    b.output(0xa0, a0).unwrap();
                    // Explicit physical addresses and costs per word, before executing.
                    for (ss, sp, addresses, costs) in [
                        (
                            0,
                            0xfffe,
                            [0xfffe, 0xffff, 0, 1, 2, 3],
                            [if high { 1 } else { 2 }, 1, 1],
                        ),
                        (0, 0xffff, [0xffff, 0, 1, 2, 3, 4], [2, 2, 2]),
                        (
                            0x0100,
                            0xffff,
                            [0x10fff, 0x1000, 0x1001, 0x1002, 0x1003, 0x1004],
                            [sram_lane + 1, 2, 2],
                        ),
                        (0xffff, 0xf, [0xfffff, 0, 1, 2, 3, 4], [lane + 1, 2, 2]),
                        (
                            0x2800,
                            0x7fff,
                            [0x2ffff, 0x30000, 0x30001, 0x30002, 0x30003, 0x30004],
                            [2 * lane; 3],
                        ),
                        (
                            0x2800,
                            0x7ffe,
                            [0x2fffe, 0x2ffff, 0x30000, 0x30001, 0x30002, 0x30003],
                            [aligned; 3],
                        ),
                    ] {
                        for (opcode, words, even, _, immediate) in FORMS {
                            for target in [0x1200u16, 0x1201] {
                                let mut values = [target, 0x60, 0xffff];
                                let bytes: Vec<u8> =
                                    values.iter().flat_map(|w| w.to_le_bytes()).collect();
                                for (&address, &value) in addresses.iter().zip(&bytes) {
                                    if address >= 0x20000 {
                                        mapped_code(&mut b, &[address], &[value]);
                                    } else {
                                        b.write(address, value);
                                    }
                                }
                                // Unavailable high RAM ignores writes and reads FF.
                                if ss == 0 && !high {
                                    values[0] = if sp == 0xfffe { 0xffff } else { 0x12ff };
                                }
                                c.segments = [0x10, 0x40, ss, 0x30];
                                c.ip = 0x1000;
                                c.flags = 0xfcd7;
                                c.registers[4] = sp;
                                c.interrupt_shadow = 0;
                                b.ram[0x1400..0x1403].copy_from_slice(&[opcode, 0xff, 0xff]);
                                let ram = b.ram.clone();
                                let save = b.cartridge.as_ref().unwrap().save.clone();
                                let ports = b.ports;
                                let mut regs = c.registers;
                                regs[4] = sp.wrapping_add(words * 2).wrapping_add(if immediate {
                                    0xffff
                                } else {
                                    0
                                });
                                let fetch = if immediate { 3 } else { 1 };
                                let reads: u32 = costs[..words as usize].iter().sum();
                                let residual = even - fetch - u32::from(words);
                                assert_eq!(
                                    c.instruction_step(&mut b).unwrap(),
                                    fetch + reads + residual + u32::from(values[0] & 1),
                                    "{model:?}/{mode:x}/{wait}/{a0}/{ss:x}/{sp:x}/{opcode:x}"
                                );
                                assert_eq!(b.accesses.get(), fetch + reads);
                                assert_eq!(
                                    (c.registers, c.segments, c.ip, c.flags),
                                    (
                                        regs,
                                        [0x10, if words > 1 { values[1] } else { 0x40 }, ss, 0x30],
                                        values[0],
                                        if opcode == 0xcf { 0xffd7 } else { 0xfcd7 }
                                    )
                                );
                                assert!(b.ram == ram);
                                assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                                assert_eq!(b.ports, ports);
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn normal_run_frame_returns_advance_raster_timer_and_audio_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        for mode in modes(model) {
            if model != Model::Mono {
                m.bus.output(0x60, mode).unwrap();
            }
            for (opcode, words, base, _, immediate) in FORMS {
                for sp in [0x2800u16, 0x2801] {
                    for target in [0x1200u16, 0x1201] {
                        m.cpu.segments = [0x10, 0x40, 0x20, 0x30];
                        m.cpu.ip = 0x1000;
                        m.cpu.flags = 0xf042;
                        m.cpu.registers[4] = sp;
                        m.cpu.interrupt_shadow = 0;
                        m.bus.ram[0x1400..0x1403].copy_from_slice(&[opcode, 1, 0]);
                        m.bus.ram[0x1600..0x1602].fill(0x43); // poison INC BX destination
                        m.bus.ram[0x1800..0x1802].fill(0x43);
                        put_frame(
                            &mut m.bus,
                            0x20,
                            sp,
                            &[target, 0x60, 0xf042][..words as usize],
                        );
                        let mut regs = m.cpu.registers;
                        regs[4] += words * 2 + u16::from(immediate);
                        let cost = u64::from(
                            base + u32::from(words) * u32::from(sp & 1) + u32::from(target & 1),
                        );
                        let remaining =
                            crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                        m.advance((remaining - cost) as u32);
                        m.bus.output_word(0xa4, 2).unwrap();
                        m.bus.output(0xa2, 1).unwrap();
                        let before = m.bus.cycles;
                        let frame = m.frame_number();
                        let ram = m.bus.ram.clone();
                        m.audio_samples();
                        m.run_frame().unwrap();
                        assert_eq!(m.bus.cycles, before + cost);
                        assert_eq!((m.frame_number(), m.bus.raster_line), (frame + 1, 0));
                        assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                        assert_eq!(m.audio_samples(), [0.0, 0.0]);
                        assert_eq!(
                            (m.cpu.registers, m.cpu.segments, m.cpu.ip, m.cpu.flags),
                            (
                                regs,
                                [0x10, if words > 1 { 0x60 } else { 0x40 }, 0x20, 0x30],
                                target,
                                0xf042
                            )
                        );
                        assert!(m.bus.ram == ram);
                    }
                }
            }
        }
    }
}

#[test]
fn returns_deliver_tf_or_priority_irq_after_completion_with_separate_entry_cost() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in modes(model) {
            for (opcode, words, base, _, immediate) in FORMS {
                for target in [0x1200u16, 0x1201] {
                    for sp in [0x2800u16, 0x2801] {
                        for pending in [false, true] {
                            let (mut c, mut b) = setup(model);
                            if model != Model::Mono {
                                b.output(0x60, mode).unwrap();
                            }
                            let flags = if pending { 0xf302 } else { 0xf102 };
                            c.segments = [0x10, 0x40, 0x20, 0x30];
                            c.ip = 0x1000;
                            c.registers[4] = sp;
                            c.flags = flags;
                            c.interrupt_shadow = u8::from(pending);
                            b.ram[0x1400..0x1403].copy_from_slice(&[opcode, 1, 0]);
                            b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                            b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                            b.ram[0x300] = 0xcf;
                            b.ram[0x400] = 0xcf;
                            put_frame(&mut b, 0x20, sp, &[target, 0x60, flags][..words as usize]);
                            if pending {
                                b.output(0xb2, 0x80).unwrap();
                                b.request(7);
                            }
                            let after = sp + 2 * words + u16::from(immediate);
                            let entry = if pending { 32 } else { 25 } + 3 * u32::from(after & 1);
                            let cost =
                                base + u32::from(words) * u32::from(sp & 1) + u32::from(target & 1);
                            let mut regs = c.registers;
                            regs[4] = after - 6;
                            let mut ram = b.ram.clone();
                            let cs = if words > 1 { 0x60u16 } else { 0x40 };
                            let frame = [
                                target as u8,
                                (target >> 8) as u8,
                                cs as u8,
                                0,
                                flags as u8,
                                (flags >> 8) as u8,
                            ];
                            let address = 0x200 + usize::from(after - 6);
                            ram[address..address + 6].copy_from_slice(&frame);
                            assert_eq!(c.step(&mut b).unwrap(), cost + entry);
                            assert_eq!(
                                b.accesses.get(),
                                if immediate { 3 } else { 1 }
                                    + u32::from(words) * (1 + u32::from(sp & 1))
                                    + 2
                                    + 3 * (1 + u32::from(after & 1))
                            );
                            assert_eq!(
                                (c.registers, c.segments, c.ip, c.flags),
                                (
                                    regs,
                                    [0x10, 0, 0x20, 0x30],
                                    if pending { 0x400 } else { 0x300 },
                                    0xf002
                                )
                            );
                            assert!(b.ram == ram, "only subsequent entry writes its frame");
                            if pending {
                                b.output(0xb6, 0x80).unwrap();
                            }
                            assert_eq!(
                                c.step(&mut b).unwrap(),
                                11 + 3 * u32::from(after & 1) + u32::from(target & 1)
                            );
                            regs[4] = after;
                            assert_eq!(
                                (c.registers, c.segments, c.ip, c.flags),
                                (regs, [0x10, cs, 0x20, 0x30], target, flags)
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn iret_if_transitions_and_tf_activation_keep_their_delivery_boundaries() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in modes(model) {
            for old in [0xf002, 0xf202] {
                for restored in [0xf002u16, 0xf202, 0xffff] {
                    let (mut c, mut b) = setup(model);
                    if model != Model::Mono {
                        b.output(0x60, mode).unwrap();
                    }
                    c.segments = [0x10, 0x40, 0x20, 0x30];
                    c.ip = 0x1000;
                    c.registers[4] = 0x2800;
                    c.flags = old;
                    b.ram[0x1400] = 0xcf;
                    b.ram[0x1801] = 0x90;
                    b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                    b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                    put_frame(&mut b, 0x20, 0x2800, &[0x1201, 0x60, restored]);
                    assert_eq!(c.step(&mut b).unwrap(), 12);
                    assert_eq!(
                        (c.ip, c.flags, c.registers[4]),
                        (0x1201, (restored & 0x0fd5) | 0xf002, 0x2806)
                    );
                    let shadow = old & 0x200 == 0 && restored & 0x200 != 0;
                    assert_eq!(c.interrupt_shadow, u8::from(shadow));
                    b.output(0xb2, 0x80).unwrap();
                    b.request(7);
                    let before = restored & 0x200 != 0 && !shadow;
                    let trap = restored & 0x100 != 0;
                    assert_eq!(
                        c.step(&mut b).unwrap(),
                        if before {
                            32
                        } else if trap {
                            2 + 32
                        } else {
                            2
                        }
                    );
                    assert_eq!(c.ip, if before || trap { 0x400 } else { 0x1202 });
                    if shadow && !trap {
                        assert_eq!(c.step(&mut b).unwrap(), 32);
                        assert_eq!(c.ip, 0x400);
                    }
                }
            }
        }
    }
}

#[test]
fn returns_follow_separate_dma_reservations_with_existing_capability_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in modes(model) {
            for (opcode, words, base, _, immediate) in FORMS {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.segments = [0x10, 0x40, 0x20, 0x30];
                    c.ip = 0x1000;
                    c.registers[4] = 0x2800;
                    c.flags = 0xf042;
                    b.ram[0x1400..0x1403].copy_from_slice(&[opcode, 1, 0]);
                    put_frame(
                        &mut b,
                        0x20,
                        0x2800,
                        &[0x1201, 0x60, 0xf042][..words as usize],
                    );
                    if model == Model::Mono {
                        assert!(b.output(if sound { 0x52 } else { 0x48 }, 0x80).is_err());
                    } else {
                        b.output(0x60, mode).unwrap();
                        b.ram[0x800..0x802].copy_from_slice(&[0x51, 0xa2]);
                        for (port, value) in if sound {
                            vec![(0x4a, 0), (0x4b, 8), (0x4e, 1), (0x52, 0x83)]
                        } else {
                            vec![
                                (0x40, 0),
                                (0x41, 8),
                                (0x44, 0),
                                (0x45, 9),
                                (0x46, 2),
                                (0x48, 0x80),
                            ]
                        } {
                            b.output(port, value).unwrap();
                        }
                        if sound {
                            b.tick(117);
                        }
                        if sound || mode != 0 {
                            let before = (c.ip, c.registers, c.segments, c.flags);
                            assert_eq!(c.step(&mut b).unwrap(), 7);
                            assert_eq!(b.accesses.get(), 0);
                            assert_eq!((c.ip, c.registers, c.segments, c.flags), before);
                            if !sound {
                                assert_eq!(&b.ram[0x900..0x902], &[0x51, 0xa2]);
                            }
                        }
                    }
                    assert_eq!(c.step(&mut b).unwrap(), base + 1);
                    assert_eq!(
                        (c.ip, c.segments[1], c.registers[4], c.flags),
                        (
                            0x1201,
                            if words > 1 { 0x60 } else { 0x40 },
                            0x2800 + 2 * words + u16::from(immediate),
                            0xf042
                        )
                    );
                    if model != Model::Mono && !sound && mode == 0 {
                        assert_eq!(&b.ram[0x900..0x902], &[0xa5, 0xa5]);
                    }
                }
            }
        }
    }
}
