// SPDX-License-Identifier: CC0-1.0
use super::call_tests::{frame, mapped_code, setup};
use super::*;

#[test]
fn literal_register_vectors_include_sp_sampling_and_full_state() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for call in [false, true] {
                for r in 0..8 {
                    for start in [0x1000u16, 0x1001] {
                        for target in [0x1200u16, 0x1201] {
                            for sp in [0x2800u16, 0x2801] {
                                for flags in [0, 0xffff, 0xf002, 0xfcd7] {
                                    c.ip = start;
                                    c.segments = [0x10, 0, 0x20, 0x30];
                                    c.registers[4] = sp;
                                    c.registers[r] = target;
                                    c.flags = flags;
                                    b.ram[start as usize..start as usize + 2].copy_from_slice(&[
                                        0xff,
                                        (if call { 0xd0 } else { 0xe0 }) | r as u8,
                                    ]);
                                    let mut regs = c.registers;
                                    let mut ram = b.ram.clone();
                                    let ports = b.ports;
                                    let mut accesses = 2;
                                    if call {
                                        accesses += if regs[4] & 1 == 0 { 1 } else { 2 };
                                        regs[4] =
                                            frame(&mut ram, 0x20, regs[4], 0, start + 2, false);
                                    }
                                    assert_eq!(
                                        c.instruction_step(&mut b).unwrap(),
                                        accesses + 4 + u32::from(target & 1)
                                    );
                                    assert_eq!(
                                        b.accesses.get(),
                                        accesses,
                                        "no destination pre-read or duplicate stack write"
                                    );
                                    assert_eq!(
                                        (c.registers, c.segments, c.ip, c.flags),
                                        (regs, [0x10, 0, 0x20, 0x30], target, flags)
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
    }
}

#[test]
fn literal_memory_modrm_displacements_overrides_and_full_state() {
    // Explicit effective offsets: DS/SS/override selection is part of the oracle.
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (rm, disp, offset, ss) in [
                (0x00, &[][..], 0x800u16, false), // BX+SI
                (0x01, &[][..], 0x800, false),    // BX+DI
                (0x02, &[][..], 0x800, true),     // BP+SI
                (0x03, &[][..], 0x800, true),     // BP+DI
                (0x04, &[][..], 0x400, false),
                (0x05, &[][..], 0x400, false),
                (0x07, &[][..], 0x400, false),
                (0x06, &[0, 8][..], 0x800, false),
                (0x46, &[0x80][..], 0x380, true), // BP-128
                (0x47, &[0x7f][..], 0x47f, false),
                (0x84, &[0xff, 0x7f][..], 0x83ff, false),
                (0x83, &[0, 0x80][..], 0x8800, true),
            ] {
                // High effective offsets use a wrapping segment alias into low RAM.
                for (prefix, segment) in [
                    (None, None),
                    (Some(0x26), Some(0)),
                    (Some(0x2e), Some(1)),
                    (Some(0x36), Some(2)),
                    (Some(0x3e), Some(3)),
                ] {
                    for (extension, call, far, residual) in [
                        (2, true, false, 4),
                        (3, true, true, 8),
                        (4, false, false, 4),
                        (5, false, true, 8),
                    ] {
                        for start in [0x1000u16, 0x1001] {
                            for odd_pointer in [0u16, 1] {
                                for target in [0x1240u16, 0x1241] {
                                    for sp in [0x2800u16, 0x2801] {
                                        c.registers = [
                                            0x1234, 0x4567, 0x5678, 0x400, sp, 0x400, 0x400, 0x400,
                                        ];
                                        c.segments = [0x10, 0, 0x20, 0x30];
                                        c.ip = start;
                                        c.flags = 0xfcd7;
                                        // Shift all EA base registers for no displacement/direct forms.
                                        // For paired registers shift BX/BP only; singles shift SI/DI/BX/BP.
                                        let base_r = match rm & 7 {
                                            0 | 1 | 7 => 3,
                                            2 | 3 | 6 => 5,
                                            4 => 6,
                                            _ => 7,
                                        };
                                        let mut disp = disp.to_vec();
                                        if rm == 6 {
                                            disp[0] = odd_pointer as u8;
                                        } else {
                                            c.registers[base_r] += odd_pointer;
                                        }
                                        let offset = offset + odd_pointer;
                                        let selected = segment.unwrap_or(if ss { 2 } else { 3 });
                                        if offset >= 0x8000 {
                                            if selected == 1 || selected == 2 {
                                                continue;
                                            } // Keep CS and the independent SS stack in low RAM.
                                            c.segments[selected] = 0xf800;
                                        }
                                        let pointer = (((u32::from(c.segments[selected]) << 4)
                                            + u32::from(offset))
                                            & 0xfffff)
                                            as usize;
                                        let mut code = Vec::new();
                                        if let Some(p) = prefix {
                                            code.push(p);
                                        }
                                        code.extend_from_slice(&[0xff, rm | extension << 3]);
                                        code.extend_from_slice(&disp);
                                        b.ram[start as usize..start as usize + code.len()]
                                            .copy_from_slice(&code);
                                        b.ram[pointer..pointer + 4].copy_from_slice(&[
                                            target as u8,
                                            (target >> 8) as u8,
                                            0x56,
                                            0x34,
                                        ]);
                                        let mut ram = b.ram.clone();
                                        let mut regs = c.registers;
                                        let mut segments = c.segments;
                                        let ports = b.ports;
                                        let next = start + code.len() as u16;
                                        let fetch = code.len() as u32
                                            - u32::from(
                                                disp.len() == 2
                                                    && (start + u16::from(prefix.is_some())) & 1
                                                        == 0,
                                            );
                                        let reads =
                                            (1 + u32::from(offset & 1)) * if far { 2 } else { 1 };
                                        let writes = if call {
                                            (1 + u32::from(sp & 1)) * if far { 2 } else { 1 }
                                        } else {
                                            0
                                        };
                                        if call {
                                            regs[4] =
                                                frame(&mut ram, segments[2], sp, 0, next, far);
                                        }
                                        if far {
                                            segments[1] = 0x3456;
                                        }
                                        assert_eq!(
                                            c.instruction_step(&mut b).unwrap(),
                                            fetch
                                                + reads
                                                + writes
                                                + residual
                                                + u32::from(target & 1),
                                            "{rm:x}/{prefix:?}/{extension}/{start:x}/{offset:x}"
                                        );
                                        assert_eq!(b.accesses.get(), fetch + reads + writes);
                                        assert_eq!(
                                            (c.registers, c.segments, c.ip, c.flags),
                                            (regs, segments, target, 0xfcd7)
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
        }
    }
}

#[test]
fn aliasing_near_samples_before_push_far_reads_after_both_pushes() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for start in [0x1000u16, 0x1001] {
            for (pointer, loaded_ip, loaded_cs) in [
                (0x27fau16, 0x1240, 0x1004),
                (0x27fc, 0x1004, 0),
                (0x27fe, 0, 0x3456),
                (0x2800, 0x1240, 0x3456),
            ] {
                for far in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.ip = start;
                    c.segments = [0x10, 0, 0, 0];
                    b.ram[start as usize..start as usize + 4].copy_from_slice(&[
                        0xff,
                        if far { 0x1e } else { 0x16 },
                        pointer as u8,
                        (pointer >> 8) as u8,
                    ]);
                    b.ram[pointer as usize..pointer as usize + 4]
                        .copy_from_slice(&[0x40, 0x12, 0x56, 0x34]);
                    let mut ram = b.ram.clone();
                    let mut regs = c.registers;
                    regs[4] = frame(&mut ram, 0, 0x2800, 0, start + 4, far);
                    let target = if far && loaded_ip == 0x1004 {
                        start + 4
                    } else if far {
                        loaded_ip
                    } else {
                        0x1240
                    };
                    let cs = if far && loaded_cs == 0x1004 {
                        start + 4
                    } else if far {
                        loaded_cs
                    } else {
                        0
                    };
                    let fetch = if start & 1 == 0 { 3 } else { 4 };
                    let transfers = if far { 4 } else { 2 };
                    assert_eq!(
                        c.instruction_step(&mut b).unwrap(),
                        fetch + transfers + if far { 8 } else { 4 } + u32::from(target & 1)
                    );
                    assert_eq!(b.accesses.get(), fetch + transfers);
                    assert_eq!(
                        (c.registers, c.segments, c.ip, c.flags),
                        (regs, [0x10, cs, 0, 0], target, 0xf002)
                    );
                    assert!(b.ram == ram);
                }
            }
        }
    }
}

#[test]
fn mapped_rom_fetch_pointer_prefixes_live_a0_and_invalid_forms() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (a0, byte, word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                for cs in [0, 0x2000u16] {
                    for ds in [0, 0x2000u16] {
                        for start in [0x1000u16, 0x1001] {
                            for pointer in [0x800u16, 0x801] {
                                for count in 0..=7 {
                                    for (ext, call, far, residual) in [
                                        (2, true, false, 4),
                                        (3, true, true, 8),
                                        (4, false, false, 4),
                                        (5, false, true, 8),
                                    ] {
                                        for target in [0x1240u16, 0x1241] {
                                            // Each sequence's last segment override is DS. BUSLOCK costs only its byte.
                                            let mut code = if count == 0 {
                                                Vec::new()
                                            } else {
                                                let mut prefixes =
                                                    [0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0]
                                                        [..count - 1]
                                                        .to_vec();
                                                prefixes.push(0x3e);
                                                prefixes
                                            };
                                            code.extend_from_slice(&[
                                                0xff,
                                                ext << 3 | 6,
                                                pointer as u8,
                                                (pointer >> 8) as u8,
                                            ]);
                                            let addresses: Vec<u32> = (0..code.len())
                                                .map(|i| {
                                                    (u32::from(cs) << 4)
                                                        + u32::from(start)
                                                        + i as u32
                                                })
                                                .collect();
                                            mapped_code(&mut b, &addresses, &code);
                                            let addresses: Vec<u32> = (0..4)
                                                .map(|i| {
                                                    (u32::from(ds) << 4) + u32::from(pointer) + i
                                                })
                                                .collect();
                                            mapped_code(
                                                &mut b,
                                                &addresses,
                                                &[target as u8, (target >> 8) as u8, 0x56, 0x34],
                                            );
                                            b.output(0xa0, a0).unwrap();
                                            c.ip = start;
                                            c.segments = [0x10, cs, 0x20, ds];
                                            c.registers[4] = 0x2801;
                                            c.flags = 0xfcd7;
                                            let mut regs = c.registers;
                                            let mut ram = b.ram.clone();
                                            let ports = b.ports;
                                            if call {
                                                regs[4] = frame(
                                                    &mut ram,
                                                    0x20,
                                                    0x2801,
                                                    cs,
                                                    start + code.len() as u16,
                                                    far,
                                                );
                                            }
                                            let lane = if cs == 0 { 1 } else { byte };
                                            let fetch = (count as u32 + 2) * lane
                                                + if (start as usize + count) & 1 == 0 {
                                                    if cs == 0 {
                                                        1
                                                    } else {
                                                        word
                                                    }
                                                } else {
                                                    2 * lane
                                                };
                                            let reads = if ds == 0 {
                                                1 + u32::from(pointer & 1)
                                            } else if pointer & 1 == 0 {
                                                word
                                            } else {
                                                2 * byte
                                            } * if far { 2 } else { 1 };
                                            let writes = if call {
                                                if far {
                                                    4
                                                } else {
                                                    2
                                                }
                                            } else {
                                                0
                                            };
                                            assert_eq!(
                                                c.instruction_step(&mut b).unwrap(),
                                                fetch
                                                    + reads
                                                    + writes
                                                    + residual
                                                    + u32::from(target & 1)
                                            );
                                            assert_eq!(b.accesses.get(), fetch + reads + writes);
                                            assert_eq!(
                                                (c.registers, c.segments, c.ip, c.flags),
                                                (
                                                    regs,
                                                    [0x10, if far { 0x3456 } else { cs }, 0x20, ds],
                                                    target,
                                                    0xfcd7
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
            }
        }
        for ext in [2, 3, 4, 5] {
            for prefix in [0xf2, 0xf3] {
                c.ip = 0x1000;
                c.segments[1] = 0;
                b.ram[0x1000..0x1005].copy_from_slice(&[prefix, 0xff, ext << 3 | 6, 0, 8]);
                let before = (c.registers, c.segments, c.flags);
                let ram = b.ram.clone();
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.segments, c.flags), before);
                assert!(b.ram == ram);
                assert_eq!(b.accesses.get(), 2);
            }
        }
        for ext in [3, 5] {
            for r in 0..8 {
                c.ip = 0x1000;
                c.segments[1] = 0;
                b.ram[0x1000..0x1002].copy_from_slice(&[0xff, 0xc0 | ext << 3 | r]);
                let before = (c.registers, c.segments, c.flags);
                let ram = b.ram.clone();
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Far control flow requires memory operand."
                );
                assert_eq!(c.ip, 0x1002);
                assert_eq!((c.registers, c.segments, c.flags), before);
                assert!(b.ram == ram);
                assert_eq!(b.accesses.get(), 2);
            }
        }
    }
}

#[test]
fn literal_instruction_and_pointer_offset_bank_and_twenty_bit_wraps() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            if model != Model::Mono {
                b.output(0x60, mode).unwrap();
            }
            for (a0, lane, word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
                for (cs, ip, addresses, fetch) in [
                    (
                        0x2000,
                        0xffff,
                        [0x2ffff, 0x20000, 0x20001, 0x20002],
                        4 * lane,
                    ),
                    (
                        0x2000,
                        0xfffe,
                        [0x2fffe, 0x2ffff, 0x20000, 0x20001],
                        2 * lane + word,
                    ),
                    (
                        0x2800,
                        0x7fff,
                        [0x2ffff, 0x30000, 0x30001, 0x30002],
                        4 * lane,
                    ),
                    (
                        0x2800,
                        0x7ffe,
                        [0x2fffe, 0x2ffff, 0x30000, 0x30001],
                        2 * lane + word,
                    ),
                    (0xffff, 0xf, [0xfffff, 0, 1, 2], lane + 3),
                    (0xffff, 0xe, [0xffffe, 0xfffff, 0, 1], 2 * lane + 1),
                ] {
                    for (ds, pointer, paddresses, near_reads, far_reads) in [
                        (
                            0x2000,
                            0xffff,
                            [0x2ffff, 0x20000, 0x20001, 0x20002],
                            2 * lane,
                            4 * lane,
                        ),
                        (
                            0x2800,
                            0x7fff,
                            [0x2ffff, 0x30000, 0x30001, 0x30002],
                            2 * lane,
                            4 * lane,
                        ),
                        (0xffff, 0xf, [0xfffff, 0, 1, 2], lane + 1, lane + 3),
                        (0xffff, 0xe, [0xffffe, 0xfffff, 0, 1], word, word + 1),
                    ] {
                        if addresses.iter().any(|a| paddresses.contains(a)) {
                            continue;
                        }
                        for (ext, call, far, residual) in [
                            (2, true, false, 4),
                            (3, true, true, 8),
                            (4, false, false, 4),
                            (5, false, true, 8),
                        ] {
                            for target in [0x1240u16, 0x1241] {
                                mapped_code(
                                    &mut b,
                                    &addresses,
                                    &[0xff, ext << 3 | 6, pointer as u8, (pointer >> 8) as u8],
                                );
                                mapped_code(
                                    &mut b,
                                    &paddresses,
                                    &[target as u8, (target >> 8) as u8, 0x56, 0x34],
                                );
                                b.output(0xa0, a0).unwrap();
                                c.ip = ip;
                                c.segments = [0x10, cs, 0x20, ds];
                                c.registers[4] = 0x2800;
                                c.flags = 0xfcd7;
                                let mut regs = c.registers;
                                let mut ram = b.ram.clone();
                                let ports = b.ports;
                                if call {
                                    regs[4] =
                                        frame(&mut ram, 0x20, 0x2800, cs, ip.wrapping_add(4), far);
                                }
                                let reads = if far { far_reads } else { near_reads };
                                let writes = if call {
                                    if far {
                                        2
                                    } else {
                                        1
                                    }
                                } else {
                                    0
                                };
                                assert_eq!(
                                    c.instruction_step(&mut b).unwrap(),
                                    fetch + reads + writes + residual + u32::from(target & 1)
                                );
                                assert_eq!(b.accesses.get(), fetch + reads + writes);
                                assert_eq!(
                                    (c.registers, c.segments, c.ip, c.flags),
                                    (
                                        regs,
                                        [0x10, if far { 0x3456 } else { cs }, 0x20, ds],
                                        target,
                                        0xfcd7
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
}

#[test]
fn mapped_sram_pointer_and_stack_waits_wraps_and_actual_returns() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
        for mode in [0, 0x80, 0, 0xc0, 0, 0xe0, 0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for wait in [0, 2] {
                if model != Model::Mono {
                    b.output(0x60, mode | wait).unwrap();
                }
                let sram_word = if model != Model::Mono && wait != 0 {
                    4
                } else {
                    2
                };
                let high_ram = model != Model::Mono && mode != 0;
                for (ext, call, far, residual) in [
                    (2, true, false, 4),
                    (3, true, true, 8),
                    (4, false, false, 4),
                    (5, false, true, 8),
                ] {
                    for ds in [0, 0x1000] {
                        for ss in [0, 0x1000] {
                            for pointer in [0x800u16, 0x801, 0xffff] {
                                if ds == 0 && pointer == 0xffff && !high_ram {
                                    continue;
                                }
                                for sp in [0u16, 1, 2, 3, 0x2800, 0x2801] {
                                    for target in [0x1200u16, 0x1201] {
                                        c.ip = 0x1000;
                                        c.segments = [0x10, 0, ss, ds];
                                        c.registers[4] = sp;
                                        c.flags = 0xfcd7;
                                        b.ram[0x1000..0x1004].copy_from_slice(&[
                                            0xff,
                                            ext << 3 | 6,
                                            pointer as u8,
                                            (pointer >> 8) as u8,
                                        ]);
                                        // An independent image for each writable region, including mirrored SRAM offsets.
                                        for (i, byte) in [target as u8, (target >> 8) as u8, 0, 0]
                                            .into_iter()
                                            .enumerate()
                                        {
                                            let off = pointer.wrapping_add(i as u16) as usize;
                                            if ds == 0 {
                                                b.ram[off] = byte;
                                            } else {
                                                b.cartridge.as_mut().unwrap().save[off & 0x7fff] =
                                                    byte;
                                            }
                                        }
                                        b.ram[target as usize] = if far { 0xcb } else { 0xc3 };
                                        let before_regs = c.registers;
                                        let before_segments = c.segments;
                                        let mut regs = c.registers;
                                        let mut ram = b.ram.clone();
                                        let mut save = b.cartridge.as_ref().unwrap().save.clone();
                                        let ports = b.ports;
                                        if call {
                                            for value in if far {
                                                vec![0, 0x1004u16]
                                            } else {
                                                vec![0x1004u16]
                                            } {
                                                regs[4] = regs[4].wrapping_sub(2);
                                                for (off, byte) in [
                                                    (regs[4], value as u8),
                                                    (regs[4].wrapping_add(1), (value >> 8) as u8),
                                                ] {
                                                    if ss == 0 {
                                                        if high_ram || off < 0x4000 {
                                                            ram[off as usize] = byte;
                                                        }
                                                    } else {
                                                        save[off as usize & 0x7fff] = byte;
                                                    }
                                                }
                                            }
                                        }
                                        let read = if ds != 0 {
                                            sram_word
                                        } else {
                                            1 + u32::from(pointer & 1)
                                        } * if far { 2 } else { 1 };
                                        let per_write = if ss != 0 {
                                            sram_word
                                        } else if sp & 1 == 0 && (sp >= 2 || high_ram) {
                                            1
                                        } else {
                                            2
                                        };
                                        let writes = if !call {
                                            0
                                        } else if far && ss == 0 && !high_ram && sp == 2 {
                                            3
                                        } else {
                                            per_write * if far { 2 } else { 1 }
                                        };
                                        // Offset FFFF aliases wrapping stack bytes; far CALL must read their new values.
                                        let image = if ds == 0 { &ram[..] } else { &save[..] };
                                        let mask = if ds == 0 { 0xffff } else { 0x7fff };
                                        let sampled = |off: u16| {
                                            u16::from_le_bytes([
                                                image[off as usize & mask],
                                                image[off.wrapping_add(1) as usize & mask],
                                            ])
                                        };
                                        let loaded_target = if far && call {
                                            sampled(pointer)
                                        } else {
                                            target
                                        };
                                        let loaded_cs = if far {
                                            sampled(pointer.wrapping_add(2))
                                        } else {
                                            0
                                        };
                                        assert_eq!(c.instruction_step(&mut b).unwrap(),3+read+writes+residual+u32::from(loaded_target&1),"{model:?}/{mode:x}/{wait}/{ext}/{ds:x}/{ss:x}/{pointer:x}/{sp:x}");
                                        assert_eq!(b.accesses.get(), 3 + read + writes);
                                        assert_eq!(
                                            (c.registers, c.segments, c.ip, c.flags),
                                            (
                                                regs,
                                                [0x10, loaded_cs, ss, ds],
                                                loaded_target,
                                                0xfcd7
                                            )
                                        );
                                        assert!(b.ram == ram);
                                        assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                                        assert_eq!(b.ports, ports);
                                        // Intact frames return to even 1004; charge corrected near/far residuals.
                                        if call
                                            && (ss != 0 || high_ram || sp >= 4)
                                            && loaded_target == target
                                            && loaded_cs == 0
                                            && pointer != 0xffff
                                        {
                                            assert_eq!(
                                                c.instruction_step(&mut b).unwrap(),
                                                if far { 7 } else { 6 }
                                                    + per_write * if far { 2 } else { 1 }
                                            );
                                            assert_eq!(
                                                (c.registers, c.segments, c.ip, c.flags),
                                                (before_regs, before_segments, 0x1004, 0xfcd7)
                                            );
                                            assert!(b.ram == ram);
                                            assert_eq!(b.cartridge.as_ref().unwrap().save, save);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn physical_stack_wrap_and_mapped_nonzero_caller_retf() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (mut c, mut b) = setup(model);
        for (a0, lane, word) in [(0, 1, 2), (4, 1, 1), (8, 2, 4), (12, 2, 2)] {
            for far in [false, true] {
                mapped_code(
                    &mut b,
                    &[0x21001, 0x21002, 0x21003, 0x21004],
                    &[0xff, if far { 0x1e } else { 0x16 }, 0, 8],
                );
                b.output(0xa0, a0).unwrap();
                b.ram[0xb00..0xb04].copy_from_slice(&[0, 0x12, 0, 0]);
                c.ip = 0x1001;
                c.segments = [0x10, 0x2000, 0xffff, 0x30];
                c.registers[4] = 0x11;
                c.flags = 0xfcd7;
                let mut ram = b.ram.clone();
                ram[0] = if far { 0x20 } else { 0x10 };
                let mut regs = c.registers;
                regs[4] = if far { 0xd } else { 0xf };
                let writes = if far { 3 * lane + 1 } else { lane + 1 };
                let reads = if far { 2 } else { 1 };
                assert_eq!(
                    c.instruction_step(&mut b).unwrap(),
                    4 * lane + reads + writes + if far { 8 } else { 4 }
                );
                assert_eq!(b.accesses.get(), 4 * lane + reads + writes);
                assert_eq!(
                    (c.registers, c.segments, c.ip, c.flags),
                    (
                        regs,
                        [0x10, if far { 0 } else { 0x2000 }, 0xffff, 0x30],
                        0x1200,
                        0xfcd7
                    )
                );
                assert!(b.ram == ram);
            }
            mapped_code(
                &mut b,
                &[0x21000, 0x21001, 0x21002, 0x21003],
                &[0xff, 0x1e, 0, 8],
            );
            b.ram[0xb00..0xb04].copy_from_slice(&[0, 0x12, 0, 0]);
            b.ram[0x1200] = 0xcb;
            c.ip = 0x1000;
            c.segments = [0x10, 0x2000, 0x20, 0x30];
            c.registers[4] = 0x2800;
            let regs = c.registers;
            assert_eq!(
                c.instruction_step(&mut b).unwrap(),
                2 * lane + word + 2 + 2 + 8
            );
            assert_eq!(&b.ram[0x29fc..0x2a00], &[4, 0x10, 0, 0x20]);
            assert_eq!(c.instruction_step(&mut b).unwrap(), 9);
            assert_eq!(
                (c.registers, c.segments, c.ip, c.flags),
                (regs, [0x10, 0x2000, 0x20, 0x30], 0x1004, 0xfcd7)
            );
        }
    }
}

// Six instruction encodings with independent literal dispatch costs. The helper
// only installs bytes/state; expected cycles in callers never consult CPU helpers.
fn install(c: &mut Cpu, b: &mut Bus, form: u8, odd: bool) {
    c.registers[0] = 0x1200 + u16::from(odd);
    let code: &[u8] = match form {
        0 => &[0xff, 0xd0],
        1 => &[0xff, 0x16, 0, 8],
        2 => &[0xff, 0x1e, 0, 8],
        3 => &[0xff, 0xe0],
        4 => &[0xff, 0x26, 0, 8],
        _ => &[0xff, 0x2e, 0, 8],
    };
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    let pointer = (u32::from(c.segments[3]) << 4) as usize + 0x800;
    b.ram[pointer..pointer + 4].copy_from_slice(&[
        u8::from(odd),
        0x12,
        if form == 2 || form == 5 { 0x40 } else { 0 },
        0,
    ]);
    b.ram[0x1600..0x1602].fill(0x43);
    b.ram[0x1200..0x1202].fill(0x43); // INC BX exposes an extra destination instruction.
}

#[test]
fn normal_run_frame_indirect_advances_raster_timer_audio_once() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        let (c, mut b) = setup(model);
        mapped_code(&mut b, &[], &[]);
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
            for (form, call, far, base, odd_stack) in [
                (0, true, false, 7, 1),
                (1, true, false, 9, 1),
                (2, true, true, 15, 2),
                (3, false, false, 6, 0),
                (4, false, false, 8, 0),
                (5, false, true, 13, 0),
            ] {
                for odd in [false, true] {
                    for (sp, extra) in [(0x2800, 0), (0x2801, odd_stack)] {
                        install(&mut m.cpu, &mut m.bus, form, odd);
                        m.cpu.segments[1] = 0;
                        m.cpu.ip = 0x1000;
                        m.cpu.flags = 0xf042;
                        m.cpu.registers[4] = sp;
                        let mut regs = m.cpu.registers;
                        regs[4] -= if !call {
                            0
                        } else if far {
                            4
                        } else {
                            2
                        };
                        let cost = base + u64::from(odd) + extra;
                        let remaining =
                            crate::CYCLES_PER_FRAME - m.bus.cycles % crate::CYCLES_PER_FRAME;
                        m.advance((remaining - cost) as u32);
                        m.bus.output_word(0xa4, 2).unwrap();
                        m.bus.output(0xa2, 1).unwrap();
                        let before = m.bus.cycles;
                        let frame = m.frame_number();
                        m.audio_samples();
                        m.run_frame().unwrap();
                        assert_eq!(m.bus.cycles, before + cost);
                        assert_eq!((m.frame_number(), m.bus.raster_line), (frame + 1, 0));
                        assert_eq!(m.bus.input_word(0xa8).unwrap(), 1);
                        assert_eq!(m.audio_samples(), [0.0, 0.0]);
                        assert_eq!(
                            m.cpu.segments,
                            [0x10, if far { 0x40 } else { 0 }, 0x20, 0x30]
                        );
                        assert_eq!(
                            (m.cpu.ip, m.cpu.registers, m.cpu.flags),
                            (0x1200 + u16::from(odd), regs, 0xf042)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn indirect_irq_tf_priority_shadow_nested_frames_and_iret_return() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (form, call, far, base, stack_extra, fetch) in [
                (0, true, false, 7, 1, 2),
                (1, true, false, 9, 1, 4),
                (2, true, true, 15, 2, 5),
                (3, false, false, 6, 0, 2),
                (4, false, false, 8, 0, 4),
                (5, false, true, 13, 0, 5),
            ] {
                for odd in [false, true] {
                    for (pending, tf, shadow) in [
                        (true, false, false),
                        (true, false, true),
                        (true, true, true),
                        (false, true, false),
                    ] {
                        for (sp, extra, irq, trap) in
                            [(0x2800, 0, 32, 25), (0x2801, stack_extra, 35, 28)]
                        {
                            let (mut c, mut b) = setup(model);
                            if model != Model::Mono {
                                b.output(0x60, mode).unwrap();
                            }
                            install(&mut c, &mut b, form, odd);
                            c.segments[2] = 0;
                            c.registers[4] = sp;
                            c.flags = 0xf242 | if tf { TF } else { 0 };
                            c.interrupt_shadow = u8::from(shadow);
                            let flags = c.flags;
                            let registers = c.registers;
                            b.ram[4..8].copy_from_slice(&[0, 3, 0, 0]);
                            b.ram[0x1c..0x20].copy_from_slice(&[0, 4, 0, 0]);
                            b.ram[0x300] = 0xcf;
                            b.ram[0x400] = 0xcf;
                            b.output(0xb2, 0x80).unwrap();
                            if pending {
                                b.request(7);
                            }
                            let before = pending && !shadow;
                            let entered = before || tf;
                            let cost = base + u32::from(odd) + extra;
                            let entry = if pending { irq } else { trap };
                            assert_eq!(
                                c.step(&mut b).unwrap(),
                                if before {
                                    irq
                                } else {
                                    cost + if tf { entry } else { 0 }
                                }
                            );
                            let call_words = if !call {
                                0
                            } else if far {
                                2
                            } else {
                                1
                            };
                            let after_call = sp - if before { 0 } else { 2 * call_words };
                            let continuation = if before {
                                0x1000
                            } else {
                                0x1200 + u16::from(odd)
                            };
                            let continuation_cs = if !before && far { 0x40 } else { 0 };
                            let mut regs = registers;
                            regs[4] = after_call - if entered { 6 } else { 0 };
                            assert_eq!(c.registers, regs);
                            if entered {
                                assert_eq!(
                                    (c.segments[1], c.ip, c.flags),
                                    (0, if pending { 0x400 } else { 0x300 }, flags & !0x300)
                                );
                                let frame = (after_call - 6) as usize;
                                assert_eq!(
                                    &b.ram[frame..frame + 6],
                                    &[
                                        continuation as u8,
                                        (continuation >> 8) as u8,
                                        continuation_cs as u8,
                                        0,
                                        flags as u8,
                                        (flags >> 8) as u8
                                    ]
                                );
                                assert_eq!(
                                    b.accesses.get(),
                                    if before {
                                        irq - 27
                                    } else {
                                        fetch
                                            + u32::from(call_words) * (1 + u32::from(sp & 1))
                                            + entry
                                            - if pending { 27 } else { 20 }
                                    }
                                );
                                if pending {
                                    b.output(0xb6, 0x80).unwrap();
                                }
                                c.step(&mut b).unwrap();
                                assert_eq!(
                                    (c.ip, c.segments[1], c.flags, c.registers[4]),
                                    (continuation, continuation_cs, flags, after_call)
                                );
                                if !before && call {
                                    b.ram[continuation as usize + if far { 0x400 } else { 0 }] =
                                        if far { 0xcb } else { 0xc3 };
                                    c.instruction_step(&mut b).unwrap();
                                    assert_eq!(
                                        (c.ip, c.segments[1], c.registers[4]),
                                        (if form == 0 { 0x1002 } else { 0x1004 }, 0, sp)
                                    );
                                }
                            } else {
                                assert_eq!(
                                    (c.ip, c.segments[1], c.flags),
                                    (continuation, continuation_cs, flags)
                                );
                                assert_eq!(c.step(&mut b).unwrap(), irq);
                                assert_eq!(c.ip, 0x400);
                                assert_eq!(
                                    &b.ram[(after_call - 6) as usize..(after_call - 4) as usize],
                                    &continuation.to_le_bytes()
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn indirect_after_separate_dma_reservations_retains_model_and_mode_gates() {
    for model in [Model::Mono, Model::Color, Model::Crystal] {
        for mode in [0, 0x80, 0xc0, 0xe0] {
            if model == Model::Mono && mode != 0 {
                continue;
            }
            for (form, cost, fetch, sp) in [
                (0, 8, 3, 0x27fe),
                (1, 10, 5, 0x27fe),
                (2, 16, 7, 0x27fc),
                (3, 7, 2, 0x2800),
                (4, 9, 4, 0x2800),
                (5, 14, 5, 0x2800),
            ] {
                for sound in [false, true] {
                    let (mut c, mut b) = setup(model);
                    c.flags = 0xf042;
                    c.registers[1] = 0;
                    install(&mut c, &mut b, form, true);
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
                            let before = (c.ip, c.registers, c.flags);
                            assert_eq!(c.step(&mut b).unwrap(), 7);
                            assert_eq!((c.ip, c.registers, c.flags), before);
                            assert_eq!(b.accesses.get(), 0);
                            if !sound {
                                assert_eq!(&b.ram[0x900..0x902], &[0x51, 0xa2]);
                            }
                        }
                    }
                    assert_eq!(c.step(&mut b).unwrap(), cost);
                    assert_eq!(b.accesses.get(), fetch);
                    assert_eq!((c.ip, c.registers[1], c.flags), (0x1201, 0, 0xf042));
                    assert_eq!(c.registers[4], sp);
                    assert_eq!(c.segments[1], if form == 2 || form == 5 { 0x40 } else { 0 });
                    if model != Model::Mono && !sound && mode == 0 {
                        assert_eq!(
                            &b.ram[0x900..0x902],
                            &[0xa5, 0xa5],
                            "GDMA requires color enable"
                        );
                    }
                }
            }
        }
    }
}
