// SPDX-License-Identifier: CC0-1.0
// Independent literal bytes/state and separate fetch/execution expectations.
// Literal opcode checks use instruction_step to observe FLAGS before TF entry.
// Ordinary delivery and saved arithmetic FLAGS are covered in trap_tests.
use super::*;

fn modes() -> [(Model, bool); 5] {
    [
        (Model::Mono, false),
        (Model::Color, false),
        (Model::Color, true),
        (Model::Crystal, false),
        (Model::Crystal, true),
    ]
}

fn setup(model: Model, color: bool, code: &[u8]) -> (Cpu, Bus) {
    let mut b = Bus::new(model);
    b.ports[0x60] = if color { 0x80 } else { 0 };
    b.ram.fill(0xa5); // includes apparent data operands and stack sentinels
    b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
    let c = Cpu {
        registers: [0x1234, 0x5678, 0x79, 0x800, 0x2000, 0x820, 0x840, 0x860],
        segments: [0x10, 0, 0x20, 0x30],
        ip: 0x1000,
        ..Cpu::default()
    };
    (c, b)
}

#[test]
fn fpo1_every_opcode_and_second_byte_preserve_state_and_alignment() {
    for (model, color) in modes() {
        for opcode in 0xd8..=0xdf {
            for second in 0..=255 {
                for (flags, registers) in [
                    (
                        0xf002,
                        [0x1234, 0x5678, 0x79, 0x800, 0x2000, 0x820, 0x840, 0x860],
                    ),
                    (
                        0xffd7,
                        [0xffff, 0, 0xffff, 0xffff, 0x2001, 0xffff, 0xffff, 0xffff],
                    ),
                ] {
                    // 06/46/86 look like displacement forms; F0/F2/F3/segment
                    // bytes must be swallowed, as must HALT and unknown opcodes.
                    let (mut c, mut b) = setup(model, color, &[opcode, second, 0xb9, 0xcd, 0xab]);
                    c.flags = flags;
                    c.registers = registers;
                    let ram = b.ram.clone();
                    let ports = b.ports;
                    assert_eq!(
                        c.instruction_step(&mut b).unwrap(),
                        3,
                        "{model:?}/{color}: {opcode:02x} {second:02x}"
                    );
                    assert_eq!(
                        b.accesses.get(),
                        2,
                        "only two byte fetches, no operand read/write"
                    );
                    assert_eq!(3 - b.accesses.get(), 1, "execution component");
                    assert_eq!(c.ip, 0x1002);
                    assert_eq!(c.registers, registers);
                    assert_eq!(c.segments, [0x10, 0, 0x20, 0x30]);
                    assert_eq!(c.flags, flags);
                    assert!(!c.halted);
                    assert_eq!(c.interrupt_shadow, 0);
                    assert!(b.ram == ram);
                    assert_eq!(b.ports, ports);
                    assert_eq!(b.cycles, 0, "CPU returns cost; caller advances devices");
                    c.instruction_step(&mut b).unwrap();
                    let mut next = registers;
                    next[1] = 0xabcd;
                    assert_eq!(c.registers, next);
                    assert_eq!(c.ip, 0x1005);
                    assert_eq!(c.flags, flags);
                }
            }
        }
    }
}

#[test]
fn poll_preserves_literal_patterns_and_carry_for_following_adc() {
    for (model, color) in modes() {
        for (flags, ax, result, next_flags) in [
            (0xf002, 0x1271, 0x12e2, 0xf886),
            (0xffd7, 0xff7f, 0xfff1, 0xff92),
        ] {
            let (mut c, mut b) = setup(model, color, &[0x9b, 0x14, 0x71]); // ADC AL,71
            c.flags = flags;
            c.registers[0] = ax;
            let registers = c.registers;
            let ram = b.ram.clone();
            let ports = b.ports;
            assert_eq!(c.instruction_step(&mut b).unwrap(), 10);
            assert_eq!(b.accesses.get(), 1);
            assert_eq!(10 - b.accesses.get(), 9);
            assert_eq!(c.ip, 0x1001);
            assert_eq!(c.registers, registers);
            assert_eq!(c.segments, [0x10, 0, 0x20, 0x30]);
            assert_eq!(c.flags, flags);
            assert!(!c.halted);
            assert!(b.ram == ram);
            assert_eq!(b.ports, ports);
            assert_eq!(b.cycles, 0);
            c.instruction_step(&mut b).unwrap();
            assert_eq!(c.ip, 0x1003);
            assert_eq!(c.registers[0], result);
            assert_eq!(c.flags, next_flags);
        }
    }
}

#[test]
fn fpo1_poll_mapped_fetches_live_a0_and_ip_physical_wraps() {
    for (model, color) in modes() {
        for (control, rom_byte_cost) in [(0, 1), (4, 1), (8, 2), (12, 2)] {
            for opcode in [0x9b, 0xd8, 0xd9, 0xda, 0xdb, 0xdc, 0xdd, 0xde, 0xdf] {
                // Explicit physical address pairs: normal ROM, mapped bank
                // boundary, IP wrap within CS, and FFFFF -> RAM physical wrap.
                for (cs, ip, addresses, next_ip) in [
                    (0x2000, 0, [0x20000, 0x20001], 2),
                    (0x2800, 0x7fff, [0x2ffff, 0x30000], 0x8001),
                    (0x2000, 0xffff, [0x2ffff, 0x20000], 1),
                    (0xffff, 0xf, [0xfffff, 0], 0x11),
                ] {
                    let (mut c, mut b) = setup(model, color, &[]);
                    let mut cart = crate::cartridge::Cartridge::parse(
                        &crate::cartridge::tests::rom(262144, 2, 0),
                    )
                    .unwrap();
                    cart.bytes.fill(0x0f); // wrong mapper bank must fail
                    b.ports[0xc2] = 0;
                    b.ports[0xc3] = 1;
                    for (address, value) in addresses.into_iter().zip([opcode, 0xf3]) {
                        if address == 0 {
                            b.ram[0] = value;
                        } else {
                            let index = match address >> 16 {
                                2 => address & 0xffff,
                                3 => 0x10000 | (address & 0xffff),
                                _ => 0x30000 | (address & 0xffff),
                            };
                            cart.bytes[index] = value;
                        }
                    }
                    b.insert_cartridge(cart);
                    b.output(0xa0, control).unwrap(); // live control, independent of header
                    c.segments[1] = cs;
                    c.ip = ip;
                    let fetch = if opcode == 0x9b {
                        rom_byte_cost
                    } else if addresses[1] == 0 {
                        rom_byte_cost + 1
                    } else {
                        rom_byte_cost * 2
                    };
                    let execution = if opcode == 0x9b { 9 } else { 1 };
                    assert_eq!(c.step(&mut b).unwrap(), execution + fetch);
                    assert_eq!(b.accesses.get(), fetch);
                    assert_eq!(
                        c.ip,
                        if opcode == 0x9b {
                            ip.wrapping_add(1)
                        } else {
                            next_ip
                        }
                    );
                    assert_eq!(c.flags, 0xf002);
                    assert!(!c.halted);
                }
            }
        }
        // Offset wrap from accessible RAM to an unmapped ROM byte, without a cartridge.
        let (mut c, mut b) = setup(model, color, &[]);
        c.segments[1] = 0xf001;
        c.ip = 0xffff; // physical 0000F then F0010 after IP wrap (unmapped FF)
        b.ram[0xf] = 0xdf;
        assert_eq!(c.step(&mut b).unwrap(), 3);
        assert_eq!(b.accesses.get(), 2);
        assert_eq!(c.ip, 1);
    }
}

#[test]
fn fpo1_poll_seven_prefixes_interrupt_shadows_and_following_errors() {
    for (model, color) in modes() {
        for instruction in [&[0x9b][..], &[0xd8, 0xf3], &[0xdf, 0x86]] {
            for shadow in [&[0xfb][..], &[0x8e, 0xd0], &[0x17]] {
                // STI, MOV SS, POP SS
                for count in 0..=7 {
                    let mut code = shadow.to_vec();
                    code.extend_from_slice(&[0xf0, 0x26, 0x2e, 0x36, 0x3e, 0xf0, 0xf0][..count]);
                    code.extend_from_slice(instruction);
                    code.push(0x0f); // subsequent unsupported opcode still fails
                    let (mut c, mut b) = setup(model, color, &code);
                    c.segments = [0; 4];
                    c.registers[0] = 0;
                    b.ram[0x2000..0x2002].fill(0);
                    b.ram[0x1c..0x20].copy_from_slice(&[0, 3, 0, 0]);
                    b.output(0xb2, 0x80).unwrap();
                    if shadow != [0xfb] {
                        c.flags |= IF;
                    }
                    c.step(&mut b).unwrap();
                    b.request(7);
                    let fetch = count as u32 + instruction.len() as u32;
                    let execution = if instruction[0] == 0x9b { 9 } else { 1 };
                    assert_eq!(c.step(&mut b).unwrap(), execution + fetch);
                    assert_eq!(b.accesses.get(), fetch);
                    let after = 0x1000 + code.len() as u16 - 1;
                    assert_eq!(c.ip, after);
                    assert_eq!(c.flags, 0xf202);
                    c.step(&mut b).unwrap();
                    assert_eq!(c.ip, 0x300);
                    let sp = c.registers[4] as usize;
                    assert_eq!(&b.ram[sp..sp + 2], &after.to_le_bytes());
                    b.output(0xb6, 0x80).unwrap();
                    c.ip = after;
                    assert_eq!(
                        c.step(&mut b).unwrap_err().to_string(),
                        format!("Unsupported opcode 0F at 0000:{after:04X}.")
                    );
                }
            }
            for prefix in [0xf2, 0xf3] {
                let mut code = vec![0xf0, prefix];
                code.extend_from_slice(instruction);
                let (mut c, mut b) = setup(model, color, &code);
                assert_eq!(
                    c.step(&mut b).unwrap_err().to_string(),
                    "Unsupported REP instruction."
                );
                assert_eq!(c.ip, 0x1003); // rejected before any second-byte fetch
            }
        }
        let (mut c, mut b) = setup(model, color, &[0xd8, 0xf4, 0xf4, 0x9b]);
        c.step(&mut b).unwrap(); // swallowed HALT
        assert!(!c.halted);
        c.step(&mut b).unwrap(); // actual HALT
        assert!(c.halted);
        assert_eq!(c.step(&mut b).unwrap(), 1);
        assert_eq!(c.ip, 0x1003);
    }
}

#[test]
fn fpo1_poll_return_costs_advance_shared_clock_once_after_separate_dma() {
    for (model, color) in modes() {
        let (c, mut b) = setup(model, color, &[0x9b, 0xdf, 0x86, 0xf4]);
        let cart = crate::cartridge::Cartridge::parse(&crate::cartridge::tests::rom(131072, 2, 0))
            .unwrap();
        b.insert_cartridge(cart);
        let mut m = crate::Machine::new(model);
        m.cpu = c;
        m.bus = b;
        if color {
            m.bus.ram[0x800..0x802].copy_from_slice(&[0x12, 0x34]);
            for (port, value) in [
                (0x40, 0),
                (0x41, 8),
                (0x42, 0),
                (0x44, 0),
                (0x45, 9),
                (0x46, 2),
                (0x47, 0),
                (0x48, 0x80),
            ] {
                m.bus.output(port, value).unwrap();
            }
            let dma = m.cpu.step(&mut m.bus).unwrap();
            assert_eq!(dma, 7); // one RAM word plus separate DMA startup
            assert_eq!(m.cpu.ip, 0x1000);
            assert_eq!(&m.bus.ram[0x900..0x902], &[0x12, 0x34]);
            m.advance(dma);
            assert_eq!(m.bus.cycles, 7);
        }
        let start = m.bus.cycles;
        for (cost, ip, fetch, elapsed) in
            [(10, 0x1001, 1, 10), (3, 0x1003, 2, 13), (2, 0x1004, 1, 15)]
        {
            let actual = m.cpu.step(&mut m.bus).unwrap();
            assert_eq!(actual, cost);
            assert_eq!(m.bus.accesses.get(), fetch);
            assert_eq!(m.cpu.ip, ip);
            assert_eq!(m.bus.cycles, start + elapsed - u64::from(cost));
            m.advance(actual);
            assert_eq!(m.bus.cycles, start + elapsed);
        }
        assert!(m.cpu.halted);
    }
}
