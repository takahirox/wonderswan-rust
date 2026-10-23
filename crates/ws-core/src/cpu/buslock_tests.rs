// SPDX-License-Identifier: CC0-1.0
// Independent instruction bytes and literal architectural/bus-cost expectations.
use super::*;

const MODELS: [Model; 3] = [Model::Mono, Model::Color, Model::Crystal];

#[test]
fn buslock_literal_memory_stack_io_and_ineffective_vectors() {
    // code, AX, SP, data bytes, word below SP, palette bytes, flags, bus accesses.
    // Every vector is followed by MOV CX,ABCD to catch operand overconsumption.
    type Vector = (&'static [u8], u16, u16, [u8; 4], u16, [u8; 2], u16, u32);
    let vectors: &[Vector] = &[
        (
            &[0xf0, 0xa0, 0, 8],
            0x1278,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xa1, 0, 8],
            0x5678,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xa2, 0, 8],
            0x1234,
            0x2000,
            [0x34, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xa3, 0, 8],
            0x1234,
            0x2000,
            [0x34, 0x12, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0x8a, 0x47, 2],
            0x12bc,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            5,
        ),
        (
            &[0xf0, 0x8b, 0x86, 2, 0],
            0x9abc,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            6,
        ),
        (
            &[0xf0, 0x88, 0x47, 2],
            0x1234,
            0x2000,
            [0x78, 0x56, 0x34, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            5,
        ),
        (
            &[0xf0, 0x89, 0x86, 2, 0],
            0x1234,
            0x2000,
            [0x78, 0x56, 0x34, 0x12],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            6,
        ),
        (
            &[0xf0, 0xc6, 0x47, 2, 0xa5],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xa5, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            6,
        ),
        (
            &[0xf0, 0xc7, 0x86, 2, 0, 0xcd, 0xab],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xcd, 0xab],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            8,
        ),
        (
            &[0xf0, 0x81, 0x3e, 0, 8, 0x78, 0x56],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf046,
            8,
        ),
        (
            &[0xf0, 0x83, 0x06, 0, 8, 1],
            0x1234,
            0x2000,
            [0x79, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf002,
            8,
        ),
        (
            &[0xf0, 0x87, 0x06, 0, 8],
            0x5678,
            0x2000,
            [0x34, 0x12, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            7,
        ),
        (
            &[0xf0, 0x50],
            0x1234,
            0x1ffe,
            [0x78, 0x56, 0xbc, 0x9a],
            0x1234,
            [0x10, 0x32],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0x58],
            0xbeef,
            0x2002,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0xe4, 0x20],
            0x1210,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xe5, 0x20],
            0x3210,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xe6, 0x20],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x34, 0x32],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xe7, 0x20],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x34, 0x12],
            0xf003,
            4,
        ),
        (
            &[0xf0, 0xec],
            0x1210,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0xed],
            0x3210,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0xee],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x34, 0x32],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0xef],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x34, 0x12],
            0xf003,
            3,
        ),
        (
            &[0xf0, 0x90],
            0x1234,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            2,
        ),
        (
            &[0xf0, 0x01, 0xc0],
            0x2468,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf002,
            3,
        ),
        (
            &[0xf0, 0xb8, 0x78, 0x56],
            0x5678,
            0x2000,
            [0x78, 0x56, 0xbc, 0x9a],
            0xbeef,
            [0x10, 0x32],
            0xf003,
            3,
        ),
    ];
    for model in MODELS {
        for &(code, ax, sp, data, stack, ports, flags, accesses) in vectors {
            let mut b = Bus::new(model);
            b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
            b.ram[0x1000 + code.len()..0x1003 + code.len()].copy_from_slice(&[0xb9, 0xcd, 0xab]);
            b.ram[0x800..0x804].copy_from_slice(&[0x78, 0x56, 0xbc, 0x9a]);
            b.ram[0x1ffe..0x2002].copy_from_slice(&[0xef, 0xbe, 0xef, 0xbe]);
            b.output(0x20, 0x10).unwrap();
            b.output(0x21, 0x32).unwrap();
            let mut expected_ram = b.ram.clone();
            expected_ram[0x800..0x804].copy_from_slice(&data);
            expected_ram[0x1ffe..0x2000].copy_from_slice(&stack.to_le_bytes());
            let mut expected_ports = b.ports;
            expected_ports[0x20..0x22].copy_from_slice(&ports);
            let mut c = Cpu {
                registers: [0x1234, 2, 0x20, 0x800, 0x2000, 0x800, 0x800, 0x900],
                segments: [0; 4],
                ip: 0x1000,
                flags: 0xf003,
                ..Cpu::default()
            };
            assert_eq!(
                c.step(&mut b).unwrap(),
                1 + accesses,
                "{model:?}: {code:02x?}"
            );
            assert_eq!(b.accesses.get(), accesses);
            assert_eq!(c.ip, 0x1000 + code.len() as u16);
            assert_eq!(c.registers, [ax, 2, 0x20, 0x800, sp, 0x800, 0x800, 0x900]);
            assert_eq!(c.segments, [0; 4]);
            assert_eq!(c.flags, flags);
            assert!(!c.halted);
            assert!(b.ram == expected_ram, "RAM {model:?}: {code:02x?}");
            assert_eq!(b.ports, expected_ports);
            // The following MOV has an aligned immediate after odd-length code.
            assert_eq!(
                c.step(&mut b).unwrap(),
                if code.len() % 2 == 0 { 4 } else { 3 }
            );
            assert_eq!(c.ip, 0x1003 + code.len() as u16);
            assert_eq!(
                c.registers,
                [ax, 0xabcd, 0x20, 0x800, sp, 0x800, 0x800, 0x900]
            );
            assert_eq!(c.flags, flags);
        }
    }
}

#[test]
fn buslock_redundant_prefixes_last_segment_and_defensive_bound() {
    for model in MODELS {
        for prefixes in 1..=8 {
            for mixed in [false, true] {
                let mut b = Bus::new(model);
                let mut code = vec![0xf0; prefixes];
                if mixed && prefixes >= 2 {
                    // Alternating segment overrides, with ES last. All prefixes
                    // through seven are guaranteed; eight is emulator policy.
                    for i in (1..prefixes).step_by(2) {
                        code[i] = 0x36;
                    }
                    code[prefixes - 1] = 0x26;
                }
                code.extend_from_slice(&[0xa1, 0x80, 0, 0x90]);
                b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
                b.ram[0x1080..0x1082].copy_from_slice(&[0x34, 0x12]);
                b.ram[0x2080..0x2082].copy_from_slice(&[0x78, 0x56]);
                b.ram[0x3080..0x3082].copy_from_slice(&[0xbc, 0x9a]);
                let mut c = Cpu {
                    segments: [0x100, 0, 0x200, 0x300],
                    ip: 0x1000,
                    ..Cpu::default()
                };
                let operand_clocks = if prefixes % 2 == 1 { 1 } else { 2 };
                assert_eq!(
                    c.step(&mut b).unwrap(),
                    prefixes as u32 + 3 + operand_clocks
                );
                assert_eq!(b.accesses.get(), prefixes as u32 + 2 + operand_clocks);
                assert_eq!(
                    c.registers[0],
                    if mixed && prefixes >= 2 {
                        0x1234
                    } else {
                        0x9abc
                    }
                );
                assert_eq!(c.ip, 0x1003 + prefixes as u16);
                assert_eq!(c.flags, 0xf002);
                c.step(&mut b).unwrap();
                assert_eq!(c.ip, 0x1004 + prefixes as u16);
            }
        }
        let mut b = Bus::new(model);
        b.ram[0x1000..0x100a]
            .copy_from_slice(&[0xf0, 0x26, 0xf0, 0x36, 0xf0, 0x3e, 0xf0, 0x26, 0xf0, 0x90]);
        let mut c = Cpu {
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        assert_eq!(
            c.step(&mut b).unwrap_err().to_string(),
            "Excessive instruction prefixes."
        );
        assert_eq!(c.ip, 0x1009);
        assert_eq!(b.accesses.get(), 9);
    }
}

#[test]
fn buslock_keeps_seven_prefixes_with_instruction_under_interrupt_shadow() {
    for model in MODELS {
        let mut b = Bus::new(model);
        let code = [
            0xfb, 0xf0, 0x26, 0xf0, 0x36, 0x3e, 0xf0, 0x26, 0xa1, 0x80, 0, 0x90,
        ];
        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        b.ram[0x1080..0x1082].copy_from_slice(&[0x34, 0x12]);
        b.ram[0x1c..0x20].copy_from_slice(&[0, 3, 0, 0]);
        b.output(0xb2, 0x80).unwrap();
        let mut c = Cpu {
            segments: [0x100, 0, 0, 0],
            registers: [0, 0, 0, 0, 0x2000, 0, 0, 0],
            ip: 0x1000,
            ..Cpu::default()
        };
        c.step(&mut b).unwrap(); // STI protects the entire following instruction.
        b.request(7);
        assert_eq!(c.step(&mut b).unwrap(), 12); // 7 prefixes + opcode + odd operand (2) + aligned data (1) + execution
        assert_eq!(c.ip, 0x100b);
        assert_eq!(c.registers[0], 0x1234);
        assert_eq!(c.flags, 0xf202);
        c.step(&mut b).unwrap();
        assert_eq!((c.segments[1], c.ip), (0, 0x300));
        assert_eq!(&b.ram[0x1ffa..0x2000], &[0x0b, 0x10, 0, 0, 2, 0xf2]);
    }
}

#[test]
fn buslock_rep_stays_bounded_and_restarts_at_all_prefixes_after_irq() {
    for model in MODELS {
        let mut b = Bus::new(model);
        let code = [0xf0, 0x3e, 0xf0, 0x26, 0xf0, 0xf0, 0xf3, 0xa5, 0x90];
        b.ram[0x1000..0x1009].copy_from_slice(&code);
        b.ram[0x1080..0x1084].copy_from_slice(&[0x34, 0x12, 0x78, 0x56]);
        b.ram[0x1c..0x20].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[0x300..0x305].copy_from_slice(&[0xb0, 0x80, 0xe6, 0xb6, 0xcf]);
        b.output(0xb2, 0x80).unwrap();
        let mut c = Cpu {
            segments: [0x100, 0, 0, 0],
            registers: [0, 2, 0, 0, 0x2000, 0, 0x80, 0x90],
            ip: 0x1000,
            flags: 0xf202,
            ..Cpu::default()
        };
        assert_eq!(c.step(&mut b).unwrap(), 13); // eight fetches + two aligned word transfers + execution 3
        assert_eq!(b.accesses.get(), 10);
        assert_eq!(c.ip, 0x1000);
        assert_eq!(c.registers, [0, 1, 0, 0, 0x2000, 0, 0x82, 0x92]);
        assert_eq!(&b.ram[0x1090..0x1094], &[0x34, 0x12, 0, 0]);
        b.request(7);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x300);
        assert_eq!(&b.ram[0x1ffa..0x1ffc], &[0, 0x10]);
        for _ in 0..3 {
            c.step(&mut b).unwrap();
        } // acknowledge and IRET
        assert_eq!(c.ip, 0x1000);
        assert_eq!(c.step(&mut b).unwrap(), 13);
        assert_eq!(c.ip, 0x1008);
        assert_eq!(c.registers, [0x80, 0, 0, 0, 0x2000, 0, 0x84, 0x94]);
        assert_eq!(c.flags, 0xf202);
        assert_eq!(&b.ram[0x1090..0x1094], &[0x34, 0x12, 0x78, 0x56]);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x1009);
    }
}

#[test]
fn buslock_preserves_underlying_errors_and_carry_direction_operations() {
    for model in MODELS {
        for code in [
            &[0x0f][..],
            &[0xff, 0xf8],
            &[0xc6, 0xc8, 0],
            &[0xf3, 0x90],
            &[0xe4, 0x79],
            &[0xe6, 0x79],
            &[0xec],
            &[0xee],
        ] {
            let mut errors = Vec::new();
            for locked in [false, true] {
                let mut b = Bus::new(model);
                let mut bytes = if locked { vec![0xf0] } else { vec![] };
                bytes.extend_from_slice(code);
                b.ram[0x1000..0x1000 + bytes.len()].copy_from_slice(&bytes);
                let mut c = Cpu {
                    segments: [0; 4],
                    ip: 0x1000,
                    ..Cpu::default()
                };
                c.registers[2] = 0x79;
                let message = c.step(&mut b).unwrap_err().to_string();
                // Unknown-opcode diagnostics include the original prefix address.
                errors.push(message);
            }
            assert_eq!(errors[0], errors[1], "{model:?}: {code:02x?}");
        }
        for (opcode, flags) in [
            (0xf8, 0xf002),
            (0xf9, 0xf003),
            (0xf5, 0xf002),
            (0xfc, 0xf003),
            (0xfd, 0xf403),
        ] {
            let mut b = Bus::new(model);
            b.ram[0x1000..0x1003].copy_from_slice(&[0xf0, opcode, 0x90]);
            let mut c = Cpu {
                segments: [0; 4],
                ip: 0x1000,
                flags: 0xf003,
                ..Cpu::default()
            };
            assert_eq!(c.step(&mut b).unwrap(), 3);
            assert_eq!(b.accesses.get(), 2);
            assert_eq!(c.flags, flags);
            assert_eq!(c.ip, 0x1002);
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, 0x1003);
        }
    }
}
