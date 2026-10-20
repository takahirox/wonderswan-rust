use crate::{bus::Bus, Error};

const CF: u16 = 1;
const PF: u16 = 4;
const AF: u16 = 16;
const ZF: u16 = 64;
const SF: u16 = 128;
const DF: u16 = 1024;
const OF: u16 = 2048;

/// Intel register order: AX CX DX BX SP BP SI DI; segments ES CS SS DS.
#[derive(Clone, Debug)]
pub struct Cpu {
    pub registers: [u16; 8],
    pub segments: [u16; 4],
    pub ip: u16,
    pub flags: u16,
    pub halted: bool,
}

#[derive(Clone, Copy)]
enum Operand {
    Register(usize),
    Memory(u16, u16),
}

impl Default for Cpu {
    fn default() -> Self {
        Self {
            registers: [0; 8],
            segments: [0, 0xffff, 0, 0],
            ip: 0,
            flags: 2,
            halted: false,
        }
    }
}

impl Cpu {
    pub fn physical(segment: u16, offset: u16) -> u32 {
        ((segment as u32 * 16) + offset as u32) & 0xfffff
    }
    fn byte(&mut self, bus: &Bus) -> u8 {
        let value = bus.read(Self::physical(self.segments[1], self.ip));
        self.ip = self.ip.wrapping_add(1);
        value
    }
    fn word(&mut self, bus: &Bus) -> u16 {
        let lo = self.byte(bus);
        u16::from_le_bytes([lo, self.byte(bus)])
    }
    fn reg(&self, r: usize, wide: bool) -> u16 {
        if wide {
            self.registers[r]
        } else {
            (self.registers[r & 3] >> ((r >> 2) * 8)) & 255
        }
    }
    fn set_reg(&mut self, r: usize, wide: bool, value: u16) {
        if wide {
            self.registers[r] = value;
        } else {
            let shift = (r >> 2) * 8;
            self.registers[r & 3] =
                (self.registers[r & 3] & !(255 << shift)) | ((value & 255) << shift);
        }
    }
    fn read(&self, bus: &Bus, operand: Operand, wide: bool) -> u16 {
        match operand {
            Operand::Register(r) => self.reg(r, wide),
            Operand::Memory(s, o) => {
                let lo = bus.read(Self::physical(s, o));
                if wide {
                    u16::from_le_bytes([lo, bus.read(Self::physical(s, o.wrapping_add(1)))])
                } else {
                    lo as u16
                }
            }
        }
    }
    fn write(&mut self, bus: &mut Bus, operand: Operand, wide: bool, value: u16) {
        match operand {
            Operand::Register(r) => self.set_reg(r, wide, value),
            Operand::Memory(s, o) => {
                bus.write(Self::physical(s, o), value as u8);
                if wide {
                    bus.write(Self::physical(s, o.wrapping_add(1)), (value >> 8) as u8);
                }
            }
        }
    }
    fn operand(&mut self, bus: &Bus, m: u8, segment: Option<usize>) -> Operand {
        let r = (m & 7) as usize;
        if m >> 6 == 3 {
            return Operand::Register(r);
        }
        if m >> 6 == 0 && r == 6 {
            return Operand::Memory(self.segments[segment.unwrap_or(3)], self.word(bus));
        }
        let regs = &self.registers;
        let base = match r {
            0 => regs[3].wrapping_add(regs[6]),
            1 => regs[3].wrapping_add(regs[7]),
            2 => regs[5].wrapping_add(regs[6]),
            3 => regs[5].wrapping_add(regs[7]),
            4 => regs[6],
            5 => regs[7],
            6 => regs[5],
            _ => regs[3],
        };
        let disp = match m >> 6 {
            1 => self.byte(bus) as i8 as u16,
            2 => self.word(bus),
            _ => 0,
        };
        let s = segment.unwrap_or(if matches!(r, 2 | 3 | 6) { 2 } else { 3 });
        Operand::Memory(self.segments[s], base.wrapping_add(disp))
    }
    fn push(&mut self, bus: &mut Bus, value: u16) {
        self.registers[4] = self.registers[4].wrapping_sub(2);
        self.write(
            bus,
            Operand::Memory(self.segments[2], self.registers[4]),
            true,
            value,
        );
    }
    fn pop(&mut self, bus: &Bus) -> u16 {
        let v = self.read(
            bus,
            Operand::Memory(self.segments[2], self.registers[4]),
            true,
        );
        self.registers[4] = self.registers[4].wrapping_add(2);
        v
    }
    fn flag(&mut self, mask: u16, value: bool) {
        self.flags = (self.flags & !mask) | if value { mask } else { 0 };
    }
    fn szp(&mut self, value: u16, wide: bool) {
        self.flag(ZF, value == 0);
        self.flag(SF, value & if wide { 0x8000 } else { 0x80 } != 0);
        self.flag(PF, (value as u8).count_ones().is_multiple_of(2));
    }
    fn alu(&mut self, op: u8, a: u16, b: u16, wide: bool) -> Result<u16, Error> {
        let mask = if wide { 65535 } else { 255 };
        let sign = if wide { 0x8000 } else { 0x80 };
        let result = match op {
            0 => a.wrapping_add(b),
            1 => a | b,
            4 => a & b,
            5 | 7 => a.wrapping_sub(b),
            6 => a ^ b,
            _ => return Err(Error::new(format!("Unsupported ALU operation /{op}."))),
        } & mask;
        if matches!(op, 0 | 5 | 7) {
            self.flag(
                CF,
                if op == 0 {
                    a as u32 + b as u32 > mask as u32
                } else {
                    a < b
                },
            );
            self.flag(AF, (a ^ b ^ result) & 16 != 0);
            self.flag(
                OF,
                if op == 0 {
                    (!(a ^ b) & (a ^ result) & sign) != 0
                } else {
                    ((a ^ b) & (a ^ result) & sign) != 0
                },
            );
        } else {
            self.flags &= !(CF | OF | AF);
        }
        self.szp(result, wide);
        Ok(result)
    }
    fn condition(&self, code: u8) -> bool {
        let c = self.flags & CF != 0;
        let z = self.flags & ZF != 0;
        let s = self.flags & SF != 0;
        let o = self.flags & OF != 0;
        let p = self.flags & PF != 0;
        match code {
            0 => o,
            1 => !o,
            2 => c,
            3 => !c,
            4 => z,
            5 => !z,
            6 => c || z,
            7 => !c && !z,
            8 => s,
            9 => !s,
            10 => p,
            11 => !p,
            12 => s != o,
            13 => s == o,
            14 => z || s != o,
            _ => !z && s == o,
        }
    }

    /// Bounded execution: REP performs one element per step. Cycles are approximate.
    pub(crate) fn step(&mut self, bus: &mut Bus) -> Result<u32, Error> {
        if self.halted {
            return Ok(4);
        }
        let start = self.ip;
        let mut segment = None;
        let mut repeat = false;
        let mut opcode = self.byte(bus);
        let mut prefixes = 0;
        while matches!(opcode, 0x26 | 0x2e | 0x36 | 0x3e | 0xf3) {
            prefixes += 1;
            if prefixes > 8 {
                return Err(Error::new("Excessive instruction prefixes."));
            }
            match opcode {
                0x26 => segment = Some(0),
                0x2e => segment = Some(1),
                0x36 => segment = Some(2),
                0x3e => segment = Some(3),
                _ => repeat = true,
            }
            opcode = self.byte(bus);
        }
        if repeat && !matches!(opcode, 0xa4 | 0xa5 | 0xaa | 0xab) {
            return Err(Error::new("Unsupported REP instruction."));
        }
        let wide = opcode & 1 != 0;
        match opcode {
            0x00..=0x3d if opcode & 7 <= 5 => {
                let op = opcode >> 3;
                let (dest, source) = if opcode & 7 <= 3 {
                    let m = self.byte(bus);
                    let rm = self.operand(bus, m, segment);
                    let reg = Operand::Register(((m >> 3) & 7) as usize);
                    if opcode & 2 == 0 {
                        (rm, self.read(bus, reg, wide))
                    } else {
                        (reg, self.read(bus, rm, wide))
                    }
                } else {
                    (
                        Operand::Register(0),
                        if wide {
                            self.word(bus)
                        } else {
                            self.byte(bus) as u16
                        },
                    )
                };
                let result = self.alu(op, self.read(bus, dest, wide), source, wide)?;
                if op != 7 {
                    self.write(bus, dest, wide, result);
                }
            }
            0x06 | 0x0e | 0x16 | 0x1e => self.push(bus, self.segments[(opcode >> 3) as usize]),
            0x07 | 0x17 | 0x1f => self.segments[(opcode >> 3) as usize] = self.pop(bus),
            0x40..=0x4f => {
                let r = (opcode & 7) as usize;
                let carry = self.flags & CF;
                let result = self.alu(
                    if opcode & 8 == 0 { 0 } else { 5 },
                    self.registers[r],
                    1,
                    true,
                )?;
                self.registers[r] = result;
                self.flags = (self.flags & !CF) | carry;
            }
            0x50..=0x57 => self.push(bus, self.registers[(opcode & 7) as usize]),
            0x58..=0x5f => self.registers[(opcode & 7) as usize] = self.pop(bus),
            0x70..=0x7f => {
                let displacement = self.byte(bus) as i8 as u16;
                if self.condition(opcode & 15) {
                    self.ip = self.ip.wrapping_add(displacement);
                }
            }
            0x80 | 0x81 | 0x83 => {
                let m = self.byte(bus);
                let dest = self.operand(bus, m, segment);
                let value = if opcode == 0x81 {
                    self.word(bus)
                } else if opcode == 0x83 {
                    self.byte(bus) as i8 as u16
                } else {
                    self.byte(bus) as u16
                };
                let op = (m >> 3) & 7;
                let result = self.alu(op, self.read(bus, dest, wide), value, wide)?;
                if op != 7 {
                    self.write(bus, dest, wide, result);
                }
            }
            0x84 | 0x85 | 0x88..=0x8b => {
                let m = self.byte(bus);
                let rm = self.operand(bus, m, segment);
                let reg = Operand::Register(((m >> 3) & 7) as usize);
                if opcode < 0x88 {
                    self.alu(4, self.read(bus, rm, wide), self.read(bus, reg, wide), wide)?;
                } else if opcode & 2 == 0 {
                    self.write(bus, rm, wide, self.read(bus, reg, wide));
                } else {
                    self.write(bus, reg, wide, self.read(bus, rm, wide));
                }
            }
            0x8c | 0x8e => {
                let m = self.byte(bus);
                let s = ((m >> 3) & 7) as usize;
                if s > 3 || (s == 1 && opcode == 0x8e) {
                    return Err(Error::new("Invalid segment register."));
                }
                let rm = self.operand(bus, m, segment);
                if opcode == 0x8c {
                    self.write(bus, rm, true, self.segments[s]);
                } else {
                    self.segments[s] = self.read(bus, rm, true);
                }
            }
            0x90 => {}
            0xa0..=0xa3 => {
                let offset = self.word(bus);
                let mem = Operand::Memory(self.segments[segment.unwrap_or(3)], offset);
                if opcode & 2 == 0 {
                    self.set_reg(0, wide, self.read(bus, mem, wide));
                } else {
                    self.write(bus, mem, wide, self.reg(0, wide));
                }
            }
            0xa4 | 0xa5 | 0xaa | 0xab => {
                if !repeat || self.registers[1] != 0 {
                    let value = if opcode & 8 == 0 {
                        self.read(
                            bus,
                            Operand::Memory(self.segments[segment.unwrap_or(3)], self.registers[6]),
                            wide,
                        )
                    } else {
                        self.reg(0, wide)
                    };
                    self.write(
                        bus,
                        Operand::Memory(self.segments[0], self.registers[7]),
                        wide,
                        value,
                    );
                    let delta = if wide { 2u16 } else { 1u16 };
                    let delta = if self.flags & DF != 0 {
                        delta.wrapping_neg()
                    } else {
                        delta
                    };
                    if opcode & 8 == 0 {
                        self.registers[6] = self.registers[6].wrapping_add(delta);
                    }
                    self.registers[7] = self.registers[7].wrapping_add(delta);
                    if repeat {
                        self.registers[1] -= 1;
                        if self.registers[1] != 0 {
                            self.ip = start;
                        }
                    }
                }
            }
            0xa8 | 0xa9 => {
                let value = if wide {
                    self.word(bus)
                } else {
                    self.byte(bus) as u16
                };
                self.alu(4, self.reg(0, wide), value, wide)?;
            }
            0xb0..=0xbf => {
                let wide = opcode & 8 != 0;
                let value = if wide {
                    self.word(bus)
                } else {
                    self.byte(bus) as u16
                };
                self.set_reg((opcode & 7) as usize, wide, value);
            }
            0xc3 => self.ip = self.pop(bus),
            0xc6 | 0xc7 => {
                let m = self.byte(bus);
                if m & 0x38 != 0 {
                    return Err(Error::new("Invalid MOV immediate extension."));
                }
                let dest = self.operand(bus, m, segment);
                let value = if wide {
                    self.word(bus)
                } else {
                    self.byte(bus) as u16
                };
                self.write(bus, dest, wide, value);
            }
            0xe2 => {
                let displacement = self.byte(bus) as i8 as u16;
                self.registers[1] = self.registers[1].wrapping_sub(1);
                if self.registers[1] != 0 {
                    self.ip = self.ip.wrapping_add(displacement);
                }
            }
            0xe4..=0xe7 | 0xec..=0xef => {
                let port = if opcode & 8 == 0 {
                    self.byte(bus) as u16
                } else {
                    self.registers[2]
                };
                if opcode & 2 == 0 {
                    let low = bus.input(port)? as u16;
                    let value = if wide {
                        low | ((bus.input(port.wrapping_add(1))? as u16) << 8)
                    } else {
                        low
                    };
                    self.set_reg(0, wide, value);
                } else {
                    bus.output(port, self.reg(0, false) as u8)?;
                    if wide {
                        bus.output(port.wrapping_add(1), (self.registers[0] >> 8) as u8)?;
                    }
                }
            }
            0xe8 | 0xe9 => {
                let displacement = self.word(bus);
                if opcode == 0xe8 {
                    self.push(bus, self.ip);
                }
                self.ip = self.ip.wrapping_add(displacement);
            }
            0xea => {
                let offset = self.word(bus);
                self.segments[1] = self.word(bus);
                self.ip = offset;
            }
            0xeb => {
                let displacement = self.byte(bus) as i8 as u16;
                self.ip = self.ip.wrapping_add(displacement);
            }
            0xf4 => self.halted = true,
            0xf8 => self.flags &= !CF,
            0xf9 => self.flags |= CF,
            0xfa => self.flags &= !512,
            0xfc => self.flags &= !DF,
            0xfd => self.flags |= DF,
            0xfe | 0xff => {
                let m = self.byte(bus);
                if m & 0x38 > 8 {
                    return Err(Error::new("Unsupported FE/FF extension."));
                }
                let dest = self.operand(bus, m, segment);
                let carry = self.flags & CF;
                let result = self.alu(
                    if m & 8 == 0 { 0 } else { 5 },
                    self.read(bus, dest, wide),
                    1,
                    wide,
                )?;
                self.write(bus, dest, wide, result);
                self.flags = (self.flags & !CF) | carry;
            }
            _ => {
                return Err(Error::new(format!(
                    "Unsupported opcode {opcode:02X} at {:04X}:{start:04X}.",
                    self.segments[1]
                )))
            }
        }
        Ok(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Model;

    fn execute(code: &[u8]) -> (Cpu, Bus) {
        let mut cpu = Cpu {
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        let mut bus = Bus::new(Model::Crystal);
        bus.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
        for _ in 0..200 {
            cpu.step(&mut bus).unwrap();
            if cpu.halted {
                return (cpu, bus);
            }
        }
        panic!("program did not halt");
    }

    #[test]
    fn arithmetic_and_logic_flags_cover_byte_and_word_boundaries() {
        let mut cpu = Cpu::default();
        let flags = CF | PF | AF | ZF | SF | OF;
        for (op, a, b, wide, value, expected) in [
            (0, 255, 1, false, 0, CF | PF | AF | ZF),
            (0, 127, 1, false, 128, AF | SF | OF),
            (0, 65535, 1, true, 0, CF | PF | AF | ZF),
            (0, 32767, 1, true, 32768, PF | AF | SF | OF),
            (5, 0, 1, false, 255, CF | PF | AF | SF),
            (7, 32768, 1, true, 32767, PF | AF | OF),
            (4, 0xff, 0x0f, false, 15, PF),
            (1, 0x80, 1, false, 0x81, PF | SF),
            (6, 0xffff, 0xffff, true, 0, PF | ZF),
        ] {
            cpu.flags = 2 | flags;
            assert_eq!(cpu.alu(op, a, b, wide).unwrap(), value);
            assert_eq!(cpu.flags & flags, expected, "op {op}, a {a}, b {b}");
            assert_eq!(cpu.flags & 2, 2);
        }
        assert!(cpu.alu(2, 0, 0, true).is_err());
    }

    #[test]
    fn modrm_segments_displacements_byte_halves_and_wrapping_address() {
        let (cpu, bus) = execute(&[
            0xb8, 0x00, 0x01, 0x8e, 0xd0, // SS=0100
            0xbd, 0x02, 0x00, // BP=2
            0xc7, 0x46, 0xfe, 0x34, 0x12, // [SS:BP-2]=1234 (overwrites executed code)
            0x3e, 0xc6, 0x46, 0xfe, 0x56, // [DS:BP-2]=56
            0xbb, 0x20, 0x00, 0xbe, 0x03, 0x00, // BX=20 SI=3
            0xc7, 0x80, 0x00, 0x01, 0xcd, 0xab, // [BX+SI+0100]=ABCD
            0xb4, 0x77, 0x8a, 0x00, // AH=77 AL=[BX+SI]
            0x8b, 0x88, 0x00, 0x01, // CX=[BX+SI+0100]
            0xf4,
        ]);
        assert_eq!(&bus.ram[0x1000..0x1002], &[0x34, 0x12]);
        assert_eq!(bus.ram[0], 0x56);
        assert_eq!(cpu.registers[1], 0xabcd);
        assert_eq!(cpu.registers[0], 0x7700);
        assert_eq!(Cpu::physical(0xffff, 0x0010), 0);
        assert_eq!(Cpu::physical(0x1234, 0xabcd), 0x1cf0d);
    }

    #[test]
    fn call_return_stack_loop_and_inc_dec_preserve_carry() {
        let (cpu, bus) = execute(&[
            0xbc, 0x00, 0x20, // SP=2000
            0xb9, 3, 0, 0xbb, 0, 0, 0xf9, // STC
            0xe8, 5, 0, // call 0012
            0xe2, 0xfb, // loop call
            0xeb, 5, // jump to DEC AX
            0x90, 0x43, 0x53, 0x58, 0xc3, // INC BX; PUSH BX; POP AX; RET
            0x48, // DEC AX
            0xf4,
        ]);
        assert_eq!(cpu.registers[3], 3);
        assert_eq!(cpu.registers[0], 2);
        assert_eq!(cpu.registers[4], 0x2000);
        assert_eq!(cpu.registers[1], 0);
        assert_ne!(cpu.flags & CF, 0);
        assert_eq!(&bus.ram[0x1ffe..0x2000], &[13, 16]);
    }

    #[test]
    fn every_conditional_branch_checks_its_flag_formula() {
        for flags in 0..32 {
            let mut cpu = Cpu::default();
            let [c, z, s, o, p] = [1, 2, 4, 8, 16].map(|bit| flags & bit != 0);
            cpu.flag(CF, c);
            cpu.flag(ZF, z);
            cpu.flag(SF, s);
            cpu.flag(OF, o);
            cpu.flag(PF, p);
            let expected = [
                o,
                !o,
                c,
                !c,
                z,
                !z,
                c || z,
                !c && !z,
                s,
                !s,
                p,
                !p,
                s != o,
                s == o,
                z || s != o,
                !z && s == o,
            ];
            for (code, value) in expected.into_iter().enumerate() {
                assert_eq!(cpu.condition(code as u8), value);
            }
        }
    }

    #[test]
    fn repeated_strings_are_bounded_and_honor_direction_segments_and_zero_count() {
        let (cpu, bus) = execute(&[
            0xbe, 0x00, 0x08, 0xbf, 0x00, 0x09, // SI=800 DI=900
            0xb8, 0x00, 0x01, 0x8e, 0xd8, // DS=100
            0xb9, 3, 0, 0xf3, 0xa4, // copy three zero bytes
            0xfd, 0xb8, 0xcd, 0xab, 0xb9, 2, 0, 0xf3, 0xab, // two words backwards
            0xb9, 0, 0, 0xf3, 0xab, // zero-count REP does not write
            0xfc, 0xf4,
        ]);
        assert_eq!(cpu.registers[6], 0x803);
        assert_eq!(cpu.registers[7], 0x8ff);
        assert_eq!(&bus.ram[0x901..0x905], &[0xcd, 0xab, 0xcd, 0xab]);
        assert_eq!(&bus.ram[0x8ff..0x901], &[0, 0]);
        assert_eq!(cpu.flags & DF, 0);
    }

    #[test]
    fn mov_compare_test_memory_arithmetic_and_io_follow_instruction_bytes() {
        let (cpu, bus) = execute(&[
            0xb8, 0x10, 0x32, 0xe7, 0x20, // OUT palette word
            0xba, 0xb5, 0, 0xb0, 0x20, 0xee, 0xec, // keypad selected, no buttons
            0xa8, 0x0f, // TEST preserves AL
            0xa2, 0x00, 0x08, 0xc7, 0x06, 0x02, 0x08, 0xff, 0xff, 0xf9, 0xff, 0x06, 0x02,
            0x08, // INC word [802] -> 0, carry kept
            0x83, 0x2e, 0x02, 0x08, 0xff, // SUB [802], -1 -> 1
            0x83, 0x3e, 0x02, 0x08, 1, // CMP [802],1 (does not change it)
            0xf4,
        ]);
        assert_eq!(&bus.ports[0x20..0x22], &[0x10, 0x32]);
        assert_eq!(bus.ram[0x800], 0x20);
        assert_eq!(&bus.ram[0x802..0x804], &[1, 0]);
        assert_ne!(cpu.flags & ZF, 0);
    }

    #[test]
    fn unknown_opcodes_extensions_prefixes_and_ports_fail_explicitly() {
        for code in [
            &[0x0f][..],
            &[0xff, 0xd0],
            &[0xc6, 0xc8, 0],
            &[0x8e, 0xc8],
            &[0xf3, 0x90],
            &[0xe4, 0x99],
            &[0xe6, 0x60],
            &[0x14, 1],
            &[0x26; 10],
        ] {
            let mut cpu = Cpu {
                segments: [0; 4],
                ..Cpu::default()
            };
            let mut bus = Bus::new(Model::Crystal);
            bus.ram[..code.len()].copy_from_slice(code);
            assert!(cpu.step(&mut bus).is_err(), "code {code:x?}");
        }
    }
}
