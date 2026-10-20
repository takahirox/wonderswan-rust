use crate::{bus::Bus, Error};

const CF: u16 = 1;
const PF: u16 = 4;
const AF: u16 = 16;
const ZF: u16 = 64;
const SF: u16 = 128;
const IF: u16 = 512;
const TF: u16 = 256;
const DF: u16 = 1024;
const OF: u16 = 2048;
const FIXED_FLAGS: u16 = 0x7002;
const WRITABLE_FLAGS: u16 = 0x0fd5;

fn normalize_flags(flags: u16) -> u16 {
    // V30MZ bits 1 and 12–14 read as one; bits 3, 5 and 15 read as zero.
    (flags & WRITABLE_FLAGS) | FIXED_FLAGS
}

/// Intel register order: AX CX DX BX SP BP SI DI; segments ES CS SS DS.
#[derive(Clone, Debug)]
pub struct Cpu {
    pub registers: [u16; 8],
    pub segments: [u16; 4],
    pub ip: u16,
    pub flags: u16,
    pub halted: bool,
    interrupt_shadow: u8,
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
            flags: FIXED_FLAGS,
            halted: false,
            interrupt_shadow: 0,
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
        let first = Self::physical(self.segments[1], self.ip);
        let lo = self.byte(bus);
        let second = Self::physical(self.segments[1], self.ip);
        let hi = self.byte(bus);
        bus.word_transfer(first, second);
        u16::from_le_bytes([lo, hi])
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
                    let second = Self::physical(s, o.wrapping_add(1));
                    let hi = bus.read(second);
                    bus.word_transfer(Self::physical(s, o), second);
                    u16::from_le_bytes([lo, hi])
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
        let a = a & mask;
        let b = b & mask;
        let carry = u32::from(matches!(op, 2 | 3) && self.flags & CF != 0);
        let result = match op {
            0 | 2 => (a as u32 + b as u32 + carry) as u16,
            1 => a | b,
            4 => a & b,
            3 | 5 | 7 => a.wrapping_sub(b).wrapping_sub(carry as u16),
            6 => a ^ b,
            _ => return Err(Error::new(format!("Unsupported ALU operation /{op}."))),
        } & mask;
        if matches!(op, 0 | 2 | 3 | 5 | 7) {
            let add = matches!(op, 0 | 2);
            self.flag(
                CF,
                if add {
                    a as u32 + b as u32 + carry > mask as u32
                } else {
                    (a as u32) < b as u32 + carry
                },
            );
            self.flag(AF, (a ^ b ^ result) & 16 != 0);
            self.flag(
                OF,
                if add {
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

    fn interrupt(&mut self, bus: &mut Bus, vector: u8) {
        self.flags = normalize_flags(self.flags);
        self.push(bus, self.flags);
        self.push(bus, self.segments[1]);
        self.push(bus, self.ip);
        self.flags &= !(IF | TF);
        self.halted = false;
        self.ip = self.read(bus, Operand::Memory(0, vector as u16 * 4), true);
        self.segments[1] = self.read(bus, Operand::Memory(0, vector as u16 * 4 + 2), true);
    }
    fn divide_fault(&mut self, bus: &mut Bus, start: u16) {
        self.ip = start;
        self.interrupt(bus, 0);
    }
    fn shift(&mut self, op: u8, mut value: u16, count: u8, wide: bool) -> Result<u16, Error> {
        if op == 6 {
            return Err(Error::new("Unsupported shift extension /6."));
        }
        let mask = if wide { 65535 } else { 255 };
        let sign = if wide { 32768 } else { 128 };
        let count = count & 31;
        if count == 0 {
            return Ok(value);
        }
        let old = value;
        for _ in 0..count {
            let carry = self.flags & CF != 0;
            let new_carry = if matches!(op, 0 | 2 | 4) {
                value & sign != 0
            } else {
                value & 1 != 0
            };
            value = match op {
                0 => (value << 1) | u16::from(new_carry),
                1 => (value >> 1) | if new_carry { sign } else { 0 },
                2 => (value << 1) | u16::from(carry),
                3 => (value >> 1) | if carry { sign } else { 0 },
                4 => value << 1,
                5 => value >> 1,
                7 => (value >> 1) | (value & sign),
                _ => unreachable!(),
            } & mask;
            self.flag(CF, new_carry);
        }
        if op >= 4 {
            self.szp(value, wide);
        }
        if count == 1 {
            self.flag(
                OF,
                match op {
                    0 | 2 | 4 => (value & sign != 0) ^ (self.flags & CF != 0),
                    1 | 3 => (value & sign != 0) ^ (value & (sign >> 1) != 0),
                    5 => old & sign != 0,
                    _ => false,
                },
            );
        }
        Ok(value)
    }

    /// Bounded execution: REP performs one element per step. Cycles are approximate.
    pub(crate) fn step(&mut self, bus: &mut Bus) -> Result<u32, Error> {
        bus.accesses.set(0);
        if self.interrupt_shadow == 0 && self.flags & IF != 0 {
            if let Some(vector) = bus.interrupt() {
                self.interrupt(bus, vector);
                return Ok(12 + bus.accesses.get());
            }
        }
        self.interrupt_shadow = self.interrupt_shadow.saturating_sub(1);
        if self.halted {
            return Ok(1);
        }
        let mut execution = 1u32;
        let start = self.ip;
        let mut segment = None;
        let mut repeat = 0u8;
        let mut lock = false;
        let mut opcode = self.byte(bus);
        let mut prefixes = 0;
        while matches!(opcode, 0x26 | 0x2e | 0x36 | 0x3e | 0xf0 | 0xf2 | 0xf3) {
            prefixes += 1;
            if prefixes > 8 {
                return Err(Error::new("Excessive instruction prefixes."));
            }
            match opcode {
                0x26 => segment = Some(0),
                0x2e => segment = Some(1),
                0x36 => segment = Some(2),
                0x3e => segment = Some(3),
                0xf0 => lock = true,
                _ => repeat = opcode,
            }
            opcode = self.byte(bus);
        }
        if repeat != 0 && !matches!(opcode, 0xa4..=0xa7 | 0xaa..=0xaf | 0x6c..=0x6f) {
            return Err(Error::new("Unsupported REP instruction."));
        }
        if lock {
            let m = bus.read(Self::physical(self.segments[1], self.ip));
            let op = (m >> 3) & 7;
            let valid = matches!(
                opcode,
                0x00 | 0x01
                    | 0x08
                    | 0x09
                    | 0x10
                    | 0x11
                    | 0x18
                    | 0x19
                    | 0x20
                    | 0x21
                    | 0x28
                    | 0x29
                    | 0x30
                    | 0x31
                    | 0x86
                    | 0x87
            ) || matches!(opcode, 0x80 | 0x81 | 0x83) && op != 7
                || matches!(opcode, 0xf6 | 0xf7) && matches!(op, 2 | 3)
                || matches!(opcode, 0xfe | 0xff) && op <= 1;
            if !valid || m >> 6 == 3 {
                return Err(Error::new(
                    "LOCK requires a supported memory read/modify/write instruction.",
                ));
            }
            // Instructions are already atomic on this single-CPU bus (DMA absent).
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
            0x07 | 0x17 | 0x1f => {
                self.segments[(opcode >> 3) as usize] = self.pop(bus);
                if opcode == 0x17 {
                    self.interrupt_shadow = 1;
                }
            }
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
                    execution = 4;
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
                    if s == 2 {
                        self.interrupt_shadow = 1;
                    }
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
            0xa4..=0xa7 | 0xaa..=0xaf | 0x6c..=0x6f => {
                if repeat == 0 || self.registers[1] != 0 {
                    let source =
                        Operand::Memory(self.segments[segment.unwrap_or(3)], self.registers[6]);
                    let dest = Operand::Memory(self.segments[0], self.registers[7]);
                    let compare = matches!(opcode, 0xa6 | 0xa7 | 0xae | 0xaf);
                    match opcode {
                        0xa4 | 0xa5 => {
                            let v = self.read(bus, source, wide);
                            self.write(bus, dest, wide, v);
                        }
                        0xa6 | 0xa7 => {
                            self.alu(
                                7,
                                self.read(bus, source, wide),
                                self.read(bus, dest, wide),
                                wide,
                            )?;
                        }
                        0xaa | 0xab => self.write(bus, dest, wide, self.reg(0, wide)),
                        0xac | 0xad => self.set_reg(0, wide, self.read(bus, source, wide)),
                        0xae | 0xaf => {
                            self.alu(7, self.reg(0, wide), self.read(bus, dest, wide), wide)?;
                        }
                        0x6c | 0x6d => {
                            let port = self.registers[2];
                            let lo = bus.input(port)? as u16;
                            let v = if wide {
                                lo | ((bus.input(port.wrapping_add(1))? as u16) << 8)
                            } else {
                                lo
                            };
                            self.write(bus, dest, wide, v);
                        }
                        0x6e | 0x6f => {
                            let v = self.read(bus, source, wide);
                            let port = self.registers[2];
                            bus.output(port, v as u8)?;
                            if wide {
                                bus.output(port.wrapping_add(1), (v >> 8) as u8)?;
                            }
                        }
                        _ => unreachable!(),
                    }
                    let delta = if wide { 2u16 } else { 1 };
                    let delta = if self.flags & DF != 0 {
                        delta.wrapping_neg()
                    } else {
                        delta
                    };
                    if matches!(opcode, 0xa4..=0xa7 | 0xac | 0xad | 0x6e | 0x6f) {
                        self.registers[6] = self.registers[6].wrapping_add(delta);
                    }
                    if matches!(
                        opcode,
                        0xa4..=0xa7 | 0xaa | 0xab | 0xae | 0xaf | 0x6c | 0x6d
                    ) {
                        self.registers[7] = self.registers[7].wrapping_add(delta);
                    }
                    if repeat != 0 {
                        self.registers[1] -= 1;
                        if self.registers[1] != 0
                            && (!compare || (self.flags & ZF != 0) == (repeat == 0xf3))
                        {
                            self.ip = start;
                        }
                    }
                    execution = 3;
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
            0xc2 | 0xc3 | 0xca | 0xcb => {
                let adjust = if opcode & 1 == 0 { self.word(bus) } else { 0 };
                self.ip = self.pop(bus);
                if opcode & 8 != 0 {
                    self.segments[1] = self.pop(bus);
                }
                self.registers[4] = self.registers[4].wrapping_add(adjust);
                execution = 4;
            }
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
            0xe0..=0xe3 => {
                let displacement = self.byte(bus) as i8 as u16;
                let take = if opcode == 0xe3 {
                    self.registers[1] == 0
                } else {
                    self.registers[1] = self.registers[1].wrapping_sub(1);
                    self.registers[1] != 0
                        && match opcode {
                            0xe0 => self.flags & ZF == 0,
                            0xe1 => self.flags & ZF != 0,
                            _ => true,
                        }
                };
                if take {
                    self.ip = self.ip.wrapping_add(displacement);
                    execution = 4;
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
            0xfa => self.flags &= !IF,
            0xfb => {
                self.flags |= IF;
                self.interrupt_shadow = 1;
            }
            0xf5 => self.flags ^= CF,
            0xfc => self.flags &= !DF,
            0xfd => self.flags |= DF,
            0xfe | 0xff => {
                let m = self.byte(bus);
                let op = (m >> 3) & 7;
                let dest = self.operand(bus, m, segment);
                let value = self.read(bus, dest, wide);
                match op {
                    0 | 1 => {
                        let carry = self.flags & CF;
                        let v = self.alu(if op == 0 { 0 } else { 5 }, value, 1, wide)?;
                        self.write(bus, dest, wide, v);
                        self.flags = (self.flags & !CF) | carry;
                    }
                    2 | 4 if wide => {
                        if op == 2 {
                            self.push(bus, self.ip);
                        }
                        self.ip = value;
                        execution = 4;
                    }
                    3 | 5 if wide => {
                        let Operand::Memory(s, o) = dest else {
                            return Err(Error::new("Far control flow requires memory operand."));
                        };
                        let cs = self.read(bus, Operand::Memory(s, o.wrapping_add(2)), true);
                        if op == 3 {
                            self.push(bus, self.segments[1]);
                            self.push(bus, self.ip);
                        }
                        self.segments[1] = cs;
                        self.ip = value;
                        execution = 6;
                    }
                    6 if wide => self.push(bus, value),
                    _ => return Err(Error::new(format!("Unsupported FE/FF extension /{op}."))),
                }
            }
            0x27 | 0x2f => {
                let old = self.reg(0, false);
                let old_cf = self.flags & CF != 0;
                let subtract = opcode == 0x2f;
                let mut al = old;
                let low = (al & 15) > 9 || self.flags & AF != 0;
                if low {
                    al = if subtract {
                        al.wrapping_sub(6)
                    } else {
                        al.wrapping_add(6)
                    };
                }
                let high = old > 0x99 || old_cf;
                if high {
                    al = if subtract {
                        al.wrapping_sub(0x60)
                    } else {
                        al.wrapping_add(0x60)
                    };
                }
                al &= 255;
                self.set_reg(0, false, al);
                self.flag(AF, low);
                self.flag(CF, high || (subtract && low && old < 6));
                self.szp(al, false);
            }
            0x37 | 0x3f => {
                let adjust = self.reg(0, false) & 15 > 9 || self.flags & AF != 0;
                if adjust {
                    self.registers[0] = if opcode == 0x37 {
                        self.registers[0].wrapping_add(0x106)
                    } else {
                        self.registers[0].wrapping_sub(0x106)
                    };
                }
                self.set_reg(0, false, self.reg(0, false) & 15);
                self.flag(AF, adjust);
                self.flag(CF, adjust);
            }
            0x60 => {
                let sp = self.registers[4];
                for r in 0..8 {
                    self.push(bus, if r == 4 { sp } else { self.registers[r] });
                }
            }
            0x61 => {
                for r in (0..8).rev() {
                    let v = self.pop(bus);
                    if r != 4 {
                        self.registers[r] = v;
                    }
                }
            }
            0x62 => {
                let m = self.byte(bus);
                let Operand::Memory(s, o) = self.operand(bus, m, segment) else {
                    return Err(Error::new("BOUND requires memory."));
                };
                let v = self.registers[((m >> 3) & 7) as usize] as i16;
                let lo = self.read(bus, Operand::Memory(s, o), true) as i16;
                let hi = self.read(bus, Operand::Memory(s, o.wrapping_add(2)), true) as i16;
                if v < lo || v > hi {
                    self.ip = start;
                    self.interrupt(bus, 5);
                }
            }
            0x68 | 0x6a => {
                let v = if opcode == 0x68 {
                    self.word(bus)
                } else {
                    self.byte(bus) as i8 as u16
                };
                self.push(bus, v);
            }
            0x69 | 0x6b => {
                let m = self.byte(bus);
                let operand = self.operand(bus, m, segment);
                let source = self.read(bus, operand, true) as i16 as i32;
                let imm = if opcode == 0x69 {
                    self.word(bus) as i16 as i32
                } else {
                    self.byte(bus) as i8 as i32
                };
                let result = source * imm;
                self.registers[((m >> 3) & 7) as usize] = result as u16;
                let overflow = result != result as i16 as i32;
                self.flag(CF, overflow);
                self.flag(OF, overflow);
                execution = 12;
            }
            0x86 | 0x87 => {
                let m = self.byte(bus);
                let rm = self.operand(bus, m, segment);
                let r = ((m >> 3) & 7) as usize;
                let value = self.read(bus, rm, wide);
                self.write(bus, rm, wide, self.reg(r, wide));
                self.set_reg(r, wide, value);
            }
            0x8d => {
                let m = self.byte(bus);
                let Operand::Memory(_, offset) = self.operand(bus, m, segment) else {
                    return Err(Error::new("LEA requires memory operand."));
                };
                self.registers[((m >> 3) & 7) as usize] = offset;
            }
            0x8f => {
                let m = self.byte(bus);
                if m & 0x38 != 0 {
                    return Err(Error::new("Invalid POP extension."));
                }
                let dest = self.operand(bus, m, segment);
                let value = self.pop(bus);
                self.write(bus, dest, true, value);
            }
            0x91..=0x97 => self.registers.swap(0, (opcode & 7) as usize),
            0x98 => self.registers[0] = self.reg(0, false) as i8 as i16 as u16,
            0x99 => {
                self.registers[2] = if self.registers[0] & 0x8000 != 0 {
                    65535
                } else {
                    0
                }
            }
            0x9a => {
                let ip = self.word(bus);
                let cs = self.word(bus);
                self.push(bus, self.segments[1]);
                self.push(bus, self.ip);
                self.ip = ip;
                self.segments[1] = cs;
                execution = 6;
            }
            0x9c => self.push(bus, normalize_flags(self.flags)),
            0x9d => {
                let previous_if = self.flags & IF;
                self.flags = normalize_flags(self.pop(bus));
                if previous_if == 0 && self.flags & IF != 0 {
                    // Enabling IRQs via POPF also protects the following instruction.
                    self.interrupt_shadow = 1;
                }
            }
            0x9e => self.flags = (self.flags & !0xd5) | (self.reg(4, false) & 0xd5) | 2,
            0x9f => self.set_reg(4, false, (self.flags & 0xd5) | 2),
            0xc0 | 0xc1 | 0xd0..=0xd3 => {
                let m = self.byte(bus);
                let dest = self.operand(bus, m, segment);
                let count = if opcode < 0xd0 {
                    self.byte(bus)
                } else if opcode & 2 == 0 {
                    1
                } else {
                    self.reg(1, false) as u8
                };
                let v = self.shift((m >> 3) & 7, self.read(bus, dest, wide), count, wide)?;
                self.write(bus, dest, wide, v);
                execution = 2 + (count & 31) as u32;
            }
            0xc4 | 0xc5 => {
                let m = self.byte(bus);
                let Operand::Memory(s, o) = self.operand(bus, m, segment) else {
                    return Err(Error::new("LES/LDS requires memory."));
                };
                self.registers[((m >> 3) & 7) as usize] =
                    self.read(bus, Operand::Memory(s, o), true);
                self.segments[if opcode == 0xc4 { 0 } else { 3 }] =
                    self.read(bus, Operand::Memory(s, o.wrapping_add(2)), true);
            }
            0xc8 => {
                let size = self.word(bus);
                let level = self.byte(bus) & 31;
                self.push(bus, self.registers[5]);
                let frame = self.registers[4];
                if level > 0 {
                    for _ in 1..level {
                        self.registers[5] = self.registers[5].wrapping_sub(2);
                        let v = self.read(
                            bus,
                            Operand::Memory(self.segments[2], self.registers[5]),
                            true,
                        );
                        self.push(bus, v);
                    }
                    self.push(bus, frame);
                }
                self.registers[5] = frame;
                self.registers[4] = self.registers[4].wrapping_sub(size);
            }
            0xc9 => {
                self.registers[4] = self.registers[5];
                self.registers[5] = self.pop(bus);
            }
            0xcc..=0xce => {
                let vector = if opcode == 0xcc {
                    3
                } else if opcode == 0xcd {
                    self.byte(bus)
                } else {
                    4
                };
                if opcode != 0xce || self.flags & OF != 0 {
                    self.interrupt(bus, vector);
                    execution = 12;
                }
            }
            0xcf => {
                self.ip = self.pop(bus);
                self.segments[1] = self.pop(bus);
                self.flags = normalize_flags(self.pop(bus));
                execution = 6;
            }
            0xd4 => {
                let base = self.byte(bus) as u16;
                if base == 0 {
                    self.divide_fault(bus, start);
                } else {
                    let al = self.reg(0, false);
                    self.set_reg(4, false, al / base);
                    self.set_reg(0, false, al % base);
                    self.szp(al % base, false);
                }
                execution = 16;
            }
            0xd5 => {
                let base = self.byte(bus) as u16;
                let al = (self.reg(0, false) + self.reg(4, false) * base) & 255;
                self.registers[0] = al;
                self.szp(al, false);
            }
            0xd7 => {
                let address = self.registers[3].wrapping_add(self.reg(0, false));
                self.set_reg(
                    0,
                    false,
                    bus.read(Self::physical(self.segments[segment.unwrap_or(3)], address)) as u16,
                );
            }
            0xf6 | 0xf7 => {
                let m = self.byte(bus);
                let op = (m >> 3) & 7;
                let dest = self.operand(bus, m, segment);
                let v = self.read(bus, dest, wide);
                match op {
                    0 => {
                        let imm = if wide {
                            self.word(bus)
                        } else {
                            self.byte(bus) as u16
                        };
                        self.alu(4, v, imm, wide)?;
                    }
                    2 => self.write(bus, dest, wide, !v),
                    3 => {
                        let result = self.alu(5, 0, v, wide)?;
                        self.write(bus, dest, wide, result);
                    }
                    4 | 5 => {
                        let signed = op == 5;
                        let result = if signed {
                            if wide {
                                (self.registers[0] as i16 as i32 * v as i16 as i32) as u32
                            } else {
                                (self.reg(0, false) as i8 as i16 * v as i8 as i16) as u16 as u32
                            }
                        } else {
                            self.reg(0, wide) as u32 * v as u32
                        };
                        let overflow = if signed {
                            if wide {
                                result as i32 != result as i16 as i32
                            } else {
                                result as i16 != result as i8 as i16
                            }
                        } else {
                            result > if wide { 65535 } else { 255 }
                        };
                        if wide {
                            self.registers[0] = result as u16;
                            self.registers[2] = (result >> 16) as u16;
                        } else {
                            self.registers[0] = result as u16;
                        }
                        self.flag(CF, overflow);
                        self.flag(OF, overflow);
                        execution = if wide { 20 } else { 12 };
                    }
                    6 | 7 => {
                        let dividend = if wide {
                            ((self.registers[2] as u32) << 16) | self.registers[0] as u32
                        } else {
                            self.registers[0] as u32
                        };
                        let signed = op == 7;
                        let numerator = if signed {
                            if wide {
                                dividend as i32 as i64
                            } else {
                                dividend as i16 as i64
                            }
                        } else {
                            dividend as i64
                        };
                        let denominator = if signed {
                            if wide {
                                v as i16 as i64
                            } else {
                                v as i8 as i64
                            }
                        } else {
                            v as i64
                        };
                        let quotient = if denominator == 0 {
                            None
                        } else {
                            Some(numerator / denominator)
                        };
                        let range = if signed {
                            if wide {
                                -32768..=32767
                            } else {
                                -128..=127
                            }
                        } else if wide {
                            0..=65535
                        } else {
                            0..=255
                        };
                        if let Some(q) = quotient.filter(|q| range.contains(q)) {
                            let rem = numerator % denominator;
                            if wide {
                                self.registers[0] = q as u16;
                                self.registers[2] = rem as u16;
                            } else {
                                self.set_reg(0, false, q as u16);
                                self.set_reg(4, false, rem as u16);
                            }
                        } else {
                            self.divide_fault(bus, start);
                        }
                        execution = if wide { 32 } else { 24 };
                    }
                    _ => return Err(Error::new(format!("Unsupported F6/F7 extension /{op}."))),
                }
            }
            _ => {
                return Err(Error::new(format!(
                    "Unsupported opcode {opcode:02X} at {:04X}:{start:04X}.",
                    self.segments[1]
                )))
            }
        }
        bus.take_fault()?;
        Ok(execution + bus.accesses.get())
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
        cpu.flags = CF;
        assert_eq!(cpu.alu(2, 0, 0, true).unwrap(), 1);
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
            &[0xff, 0xf8],
            &[0xc6, 0xc8, 0],
            &[0x8e, 0xc8],
            &[0xf3, 0x90],
            &[0xe4, 0x79],
            &[0xe6, 0x79],
            &[0xf6, 0xc8],
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

#[cfg(test)]
mod compatibility_tests {
    use super::*;
    use crate::Model;
    fn setup(code: &[u8]) -> (Cpu, Bus) {
        let mut b = Bus::new(Model::Crystal);
        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(code);
        let mut c = Cpu {
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        c.registers[4] = 0x2000;
        (c, b)
    }
    fn run(code: &[u8]) -> (Cpu, Bus) {
        let (mut c, mut b) = setup(code);
        for _ in 0..300 {
            let cycles = c.step(&mut b).unwrap();
            b.tick(cycles);
            if c.halted {
                return (c, b);
            }
        }
        panic!("probe did not halt");
    }
    fn word(b: &Bus, a: usize) -> u16 {
        u16::from_le_bytes([b.ram[a], b.ram[a + 1]])
    }
    #[test]
    fn popf_enabling_interrupts_delays_pending_irq_until_after_next_instruction() {
        let (mut c, mut b) = setup(&[
            0x9d, // POPF enables IF
            0xbb, 0x34, 0x12, // MOV BX,1234 must execute before the IRQ
            0xf4,
        ]);
        c.push(&mut b, IF);
        b.output(0xb0, 0x20).unwrap();
        b.output(0xb2, 0x80).unwrap();
        b.request(7);
        b.ram[0x9c..0xa0].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[0x300..0x302].copy_from_slice(&[0x89, 0xda]); // MOV DX,BX
        c.step(&mut b).unwrap();
        assert_ne!(c.flags & IF, 0);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x1004);
        assert_eq!(c.registers[3], 0x1234);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x300);
        assert_eq!(word(&b, 0x1ffa), 0x1004);
        c.step(&mut b).unwrap();
        assert_eq!(c.registers[2], 0x1234);
    }
    #[test]
    fn popf_without_enabling_if_does_not_create_an_interrupt_shadow() {
        for (before, restored) in [(IF, IF), (IF, 0), (0, 0)] {
            let (mut c, mut b) = setup(&[0x9d, 0xbb, 0x34, 0x12]);
            c.flags |= before;
            c.push(&mut b, restored);
            c.step(&mut b).unwrap();
            b.output(0xb2, 0x80).unwrap();
            b.request(7);
            b.ram[0x1c..0x20].copy_from_slice(&[0, 3, 0, 0]);
            c.step(&mut b).unwrap();
            if restored & IF != 0 {
                assert_eq!(c.ip, 0x300);
                assert_eq!(c.registers[3], 0);
            } else {
                assert_eq!(c.ip, 0x1004);
                assert_eq!(c.registers[3], 0x1234);
            }
        }
    }
    #[test]
    fn pushf_and_popf_preserve_fixed_bits_and_mask_reserved_bits() {
        assert_eq!(Cpu::default().flags, 0x7002);
        let (c, _) = run(&[0x31, 0xc0, 0x9c, 0x58, 0xf4]); // XOR AX,AX; PUSHF; POP AX
        assert_eq!(c.registers[0], 0x7046);
        for (input, expected) in [(0, 0x7002), (0xffff, 0x7fd7)] {
            let (mut c, mut b) = setup(&[0x9d, 0x9c, 0x58]);
            c.push(&mut b, input);
            c.step(&mut b).unwrap();
            assert_eq!(c.flags, expected);
            c.step(&mut b).unwrap();
            assert_eq!(word(&b, 0x1ffe), expected);
            c.step(&mut b).unwrap();
            assert_eq!(c.registers[0], expected);
            assert_eq!(c.registers[4], 0x2000);
        }
    }
    #[test]
    fn interrupt_and_iret_normalize_flags_on_the_stack_and_on_restore() {
        for (restored, expected) in [(0, 0x7002), (0xffff, 0x7fd7)] {
            let (mut c, mut b) = setup(&[0xcd, 0x30]);
            c.flags = 0xffff;
            b.ram[0xc0..0xc4].copy_from_slice(&[0, 2, 0x10, 0]);
            b.ram[0x300] = 0xcf; // IRET at 0010:0200
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, 0x200);
            assert_eq!(c.segments[1], 0x10);
            assert_eq!(c.registers[4], 0x1ffa);
            assert_eq!(word(&b, 0x1ffa), 0x1002);
            assert_eq!(word(&b, 0x1ffc), 0);
            assert_eq!(word(&b, 0x1ffe), 0x7fd7);
            assert_eq!(c.flags, 0x7cd7); // IF and TF cleared during the handler
            b.ram[0x1ffe..0x2000].copy_from_slice(&u16::to_le_bytes(restored));
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, 0x1002);
            assert_eq!(c.segments[1], 0);
            assert_eq!(c.registers[4], 0x2000);
            assert_eq!(c.flags, expected);
        }
    }
    #[test]
    fn adc_sbb_exhaustive_byte_results_and_signed_carry_flags() {
        let mut c = Cpu::default();
        for a in 0..=255u16 {
            for b in 0..=255u16 {
                for carry in [0, 1] {
                    for op in [2, 3] {
                        c.flags = carry;
                        let signed = if op == 2 {
                            a as u8 as i8 as i32 + b as u8 as i8 as i32 + carry as i32
                        } else {
                            a as u8 as i8 as i32 - b as u8 as i8 as i32 - carry as i32
                        };
                        let unsigned = if op == 2 {
                            a as i32 + b as i32 + carry as i32
                        } else {
                            a as i32 - b as i32 - carry as i32
                        };
                        let value = c.alu(op, a, b, false).unwrap();
                        assert_eq!(value, unsigned as u8 as u16);
                        assert_eq!(c.flags & CF != 0, !(0..=255).contains(&unsigned));
                        assert_eq!(c.flags & OF != 0, !(-128..=127).contains(&signed));
                        assert_eq!(
                            c.flags & AF != 0,
                            if op == 2 {
                                (a & 15) + (b & 15) + carry > 15
                            } else {
                                (a & 15) < (b & 15) + carry
                            }
                        );
                    }
                }
            }
        }
        c.flags = CF;
        assert_eq!(c.alu(2, 0x7fff, 0, true).unwrap(), 0x8000);
        assert_ne!(c.flags & OF, 0);
        c.flags = CF;
        assert_eq!(c.alu(3, 0x8000, 0, true).unwrap(), 0x7fff);
        assert_ne!(c.flags & OF, 0);
    }
    #[test]
    fn rotations_shifts_counts_and_carry_overflow_have_independent_vectors() {
        for (op, value, count, wide, carry, result, cf, of) in [
            (0, 0x81, 1, false, false, 3, true, true),
            (1, 1, 1, false, false, 0x80, true, true),
            (2, 0x80, 1, false, true, 1, true, true),
            (3, 1, 1, false, true, 0x80, true, true),
            (4, 0x80, 1, false, false, 0, true, true),
            (5, 0x80, 1, false, false, 0x40, false, true),
            (7, 0x81, 1, false, false, 0xc0, true, false),
            (4, 0x8001, 1, true, false, 2, true, true),
        ] {
            let mut c = Cpu::default();
            c.flag(CF, carry);
            assert_eq!(c.shift(op, value, count, wide).unwrap(), result);
            assert_eq!(c.flags & CF != 0, cf);
            assert_eq!(c.flags & OF != 0, of);
        }
        let mut c = Cpu {
            flags: 0x8d7,
            ..Cpu::default()
        };
        assert_eq!(c.shift(4, 123, 0, true).unwrap(), 123);
        assert_eq!(c.flags, 0x8d7);
        assert_eq!(c.shift(4, 123, 32, true).unwrap(), 123);
        let (c, _) = run(&[
            0xb8, 1, 0, 0xb9, 4, 0, 0xd3, 0xe0, 0xc1, 0xc8, 1, 0xd1, 0xf8, 0xf4,
        ]);
        assert_eq!(c.registers[0], 4);
    }
    #[test]
    fn multiply_divide_byte_word_signed_unsigned_and_immediate() {
        for (code, ax, dx, overflow) in [
            (
                vec![0xb8, 0xff, 0, 0xbb, 2, 0, 0xf6, 0xe3, 0xf4],
                0x1fe,
                0,
                true,
            ),
            (
                vec![0xb8, 0xfd, 0xff, 0xbb, 7, 0, 0xf6, 0xeb, 0xf4],
                0xffeb,
                0,
                false,
            ),
            (
                vec![0xb8, 0xff, 0xff, 0xbb, 2, 0, 0xf7, 0xe3, 0xf4],
                0xfffe,
                1,
                true,
            ),
            (
                vec![0xb8, 0, 0x80, 0xbb, 0xff, 0xff, 0xf7, 0xeb, 0xf4],
                0x8000,
                0,
                true,
            ),
            (
                vec![0xb8, 0xe8, 3, 0xbb, 7, 0, 0xf6, 0xf3, 0xf4],
                0x068e,
                0,
                false,
            ),
            (
                vec![0xb8, 0xeb, 0xff, 0xbb, 4, 0, 0xf6, 0xfb, 0xf4],
                0xfffb,
                0,
                false,
            ),
            (
                vec![0xb8, 0xe8, 3, 0xbb, 7, 0, 0xf7, 0xf3, 0xf4],
                142,
                6,
                false,
            ),
            (
                vec![0xb8, 0xeb, 0xff, 0x99, 0xbb, 4, 0, 0xf7, 0xfb, 0xf4],
                0xfffb,
                0xffff,
                false,
            ),
            (
                vec![0xbb, 0xfd, 0xff, 0x6b, 0xc3, 7, 0xf4],
                0xffeb,
                0,
                false,
            ),
            (vec![0xbb, 0, 0x40, 0x69, 0xc3, 4, 0, 0xf4], 0, 0, true),
        ] {
            let (c, _) = run(&code);
            assert_eq!(c.registers[0], ax, "{code:x?}");
            assert_eq!(c.registers[2], dx, "{code:x?}");
            if matches!(code[6], 0xe3 | 0xeb) || code[3] == 0x6b || code[3] == 0x69 {
                assert_eq!(c.flags & CF != 0, overflow);
                assert_eq!(c.flags & OF != 0, overflow);
            }
        }
    }
    #[test]
    fn divide_zero_and_quotient_overflow_deliver_vector_zero_without_host_panics() {
        for (opcode, ext, ax, dx, bx) in [
            (0xf6, 0xf3, 123, 0, 0),
            (0xf7, 0xf3, 0, 1, 1),
            (0xf6, 0xfb, 0x8000, 0, 255),
            (0xf7, 0xfb, 0, 0x8000, 65535),
        ] {
            let (mut c, mut b) = setup(&[opcode, ext]);
            c.registers[0] = ax;
            c.registers[2] = dx;
            c.registers[3] = bx;
            c.flags = IF | CF | 2;
            b.ram[0..4].copy_from_slice(&[0, 3, 0, 0]);
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, 0x300);
            assert_eq!(c.segments[1], 0);
            assert_eq!(c.registers[4], 0x1ffa);
            assert_eq!(word(&b, 0x1ffa), 0x1000);
            assert_eq!(word(&b, 0x1ffe), IF | CF | 0x7002);
            assert_eq!(c.flags & IF, 0);
        }
    }
    #[test]
    fn software_interrupt_into_iret_and_far_control_preserve_segments_and_stack() {
        let (mut c, mut b) = setup(&[0xcd, 0x30, 0xce, 0xf4]);
        b.ram[0xc0..0xc4].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[16..20].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[0x300] = 0xcf;
        c.flags = IF | OF | DF | CF | 2;
        c.step(&mut b).unwrap();
        assert_eq!(word(&b, 0x1ffa), 0x1002);
        assert_eq!(word(&b, 0x1ffc), 0);
        assert_eq!(word(&b, 0x1ffe), IF | OF | DF | CF | 0x7002);
        assert_eq!(c.flags & (IF | TF), 0);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x1002);
        assert_eq!(c.flags, IF | OF | DF | CF | 0x7002);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x300);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x1003);
        let mut code = vec![0x90; 0x51];
        code[..17].copy_from_slice(&[
            0x9a, 0x30, 0x10, 0, 0, 0xff, 0x1e, 0, 8, 0xb8, 0x40, 0x10, 0xff, 0xd0, 0xff, 0x2e, 4,
        ]);
        code[17] = 8;
        code[0x30..0x32].copy_from_slice(&[0x43, 0xcb]);
        code[0x40..0x42].copy_from_slice(&[0x43, 0xc3]);
        code[0x50] = 0xf4;
        let (mut c, mut b) = setup(&code);
        b.ram[0x800..0x808].copy_from_slice(&[0x30, 0x10, 0, 0, 0x50, 0x10, 0, 0]);
        for _ in 0..20 {
            c.step(&mut b).unwrap();
            if c.halted {
                break;
            }
        }
        assert!(c.halted);
        assert_eq!(c.registers[3], 3);
        assert_eq!(c.registers[4], 0x2000);
        assert_eq!(c.segments[1], 0);
    }
    #[test]
    fn sti_and_ss_shadow_hlt_wakeup_and_interrupt_acknowledgement() {
        let (mut c, mut b) = setup(&[0xfb, 0xf4, 0x90, 0xf4]);
        b.output(0xb0, 0x20).unwrap();
        b.output(0xb2, 0x80).unwrap();
        b.request(7);
        b.ram[0x9c..0xa0].copy_from_slice(&[0, 3, 0, 0]);
        b.ram[0x300..0x305].copy_from_slice(&[0xb0, 0x80, 0xe6, 0xb6, 0xcf]);
        c.step(&mut b).unwrap();
        assert_eq!(c.ip, 0x1001);
        c.step(&mut b).unwrap();
        assert!(c.halted);
        c.step(&mut b).unwrap();
        assert!(!c.halted);
        assert_eq!(c.ip, 0x300);
        assert_eq!(word(&b, 0x1ffa), 0x1002);
        for _ in 0..3 {
            c.step(&mut b).unwrap();
        }
        assert_eq!(c.ip, 0x1002);
        assert_eq!(b.interrupt(), None);
        assert_ne!(c.flags & IF, 0);
        for code in [&[0x8e, 0xd0, 0x90][..], &[0x17, 0x90][..]] {
            let (mut c, mut b) = setup(code);
            c.flags = IF | 2;
            c.step(&mut b).unwrap();
            b.output(0xb2, 128).unwrap();
            b.request(7);
            let before = c.ip;
            c.step(&mut b).unwrap();
            assert_eq!(c.ip, before + 1);
        }
    }
    #[test]
    fn decimal_conversion_flags_and_extended_string_comparisons() {
        for (code, expected) in [
            (vec![0xb0, 0x9b, 0x27, 0xf4], 1),
            (vec![0xb0, 0x9b, 0x2f, 0xf4], 0x35),
            (vec![0xb8, 9, 0, 0x04, 8, 0x37, 0xf4], 0x107),
            (vec![0xb8, 1, 1, 0x2c, 2, 0x3f, 0xf4], 9),
            (vec![0xb0, 42, 0xd4, 10, 0xf4], 0x402),
            (vec![0xb8, 2, 4, 0xd5, 10, 0xf4], 42),
        ] {
            let (c, _) = run(&code);
            assert_eq!(c.registers[0], expected, "{code:x?}");
        }
        let (mut c, mut b) = setup(&[0xbe, 0, 8, 0xbf, 0, 9, 0xb9, 3, 0, 0xf3, 0xa6, 0xf4]);
        b.ram[0x800..0x803].copy_from_slice(&[1, 2, 3]);
        b.ram[0x900..0x903].copy_from_slice(&[1, 9, 3]);
        for _ in 0..12 {
            c.step(&mut b).unwrap();
            if c.halted {
                break;
            }
        }
        assert_eq!(c.registers[1], 1);
        assert_eq!(c.registers[6], 0x802);
        assert_eq!(c.registers[7], 0x902);
        assert_eq!(c.flags & ZF, 0);
        let (mut c, mut b) = setup(&[0xbf, 0, 9, 0xb9, 3, 0, 0xb0, 7, 0xf2, 0xae, 0xf4]);
        b.ram[0x900..0x903].copy_from_slice(&[1, 7, 9]);
        for _ in 0..12 {
            c.step(&mut b).unwrap();
            if c.halted {
                break;
            }
        }
        assert_eq!(c.registers[1], 1);
        assert_eq!(c.registers[7], 0x902);
        assert_ne!(c.flags & ZF, 0);
        let (c, b) = run(&[
            0xb8, 0x80, 0, 0x98, 0x99, 0x9c, 0x58, 0x9e, 0x9f, 0x68, 0x34, 0x12, 0x8f, 0x06, 0, 8,
            0xf4,
        ]);
        assert_eq!(c.registers[2], 65535);
        assert_eq!(word(&b, 0x800), 0x1234);
    }
    #[test]
    fn cycle_accounting_depends_on_instruction_operand_bus_waits_and_halt() {
        let (mut c, mut b) = setup(&[0x90, 0xb8, 1, 0, 0xa1, 0, 8, 0xf7, 0xe0, 0xf4]);
        let mut costs = Vec::new();
        for _ in 0..5 {
            costs.push(c.step(&mut b).unwrap());
        }
        assert!(costs[0] < costs[1]);
        assert!(costs[1] < costs[2]);
        assert!(costs[2] < costs[3]);
        assert_eq!(c.step(&mut b).unwrap(), 1);
        let mut rom = crate::cartridge::tests::rom(131072, 0, 0);
        rom[65536] = 0x90;
        crate::cartridge::tests::checksum(&mut rom);
        let cart = crate::cartridge::Cartridge::parse(&rom).unwrap();
        let mut b = Bus::new(Model::Crystal);
        b.cartridge = Some(cart);
        let mut c = Cpu::default();
        c.segments[1] = 0xf000;
        let fast = c.step(&mut b).unwrap();
        c.ip = 0;
        b.cartridge.as_mut().unwrap().header.rom_wait = true;
        let slow = c.step(&mut b).unwrap();
        assert_eq!(slow, fast + 1);
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use crate::Model;
    #[test]
    fn lock_only_accepts_atomic_memory_targets_and_push_all_enter_leave_restore_registers() {
        let mut b = Bus::new(Model::Mono);
        let code = [
            0xf0, 0x83, 0x06, 0, 8, 1, 0x60, 0x31, 0xc0, 0x61, 0xc8, 4, 0, 0, 0xc9, 0xf4,
        ];
        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        let mut c = Cpu {
            registers: [1, 2, 3, 4, 0x2000, 5, 6, 7],
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        for _ in 0..10 {
            c.step(&mut b).unwrap();
            if c.halted {
                break;
            }
        }
        assert!(c.halted);
        assert_eq!(b.ram[0x800], 1);
        assert_eq!(c.registers, [1, 2, 3, 4, 0x2000, 5, 6, 7]);
        for code in [&[0xf0, 0x90][..], &[0xf0, 0x01, 0xc0][..]] {
            b.ram[..code.len()].copy_from_slice(code);
            c.ip = 0;
            c.halted = false;
            assert!(c.step(&mut b).is_err());
        }
    }
    #[test]
    fn pointer_loads_lea_lods_and_string_io_use_segments_and_registers() {
        let code = [
            0xc4, 0x1e, 0, 8, 0xc5, 0x16, 4, 8, 0x8d, 0x4a, 5, 0x93, 0xf4,
        ];
        let mut b = Bus::new(Model::Crystal);
        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        b.ram[0x800..0x808].copy_from_slice(&[0x34, 0x12, 0, 1, 0x78, 0x56, 0, 2]);
        let mut c = Cpu {
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        c.registers[5] = 16;
        c.registers[6] = 3;
        for _ in 0..5 {
            c.step(&mut b).unwrap();
        }
        assert_eq!(c.registers[0], 0x1234);
        assert_eq!(c.registers[2], 0x5678);
        assert_eq!(c.registers[1], 24);
        assert_eq!(c.segments[0], 0x100);
        assert_eq!(c.segments[3], 0x200);
        let code = [
            0xbe, 0, 8, 0xad, 0xbf, 0, 9, 0xba, 0x20, 0, 0x6f, 0x6d, 0xf4,
        ];
        b.ram[0x1000..0x1000 + code.len()].copy_from_slice(&code);
        b.ram[0x800..0x804].copy_from_slice(&[0xcd, 0xab, 0x10, 0x32]);
        c = Cpu {
            segments: [0; 4],
            ip: 0x1000,
            ..Cpu::default()
        };
        for _ in 0..8 {
            c.step(&mut b).unwrap();
            if c.halted {
                break;
            }
        }
        assert_eq!(c.registers[0], 0xabcd);
        assert_eq!(c.registers[6], 0x804);
        assert_eq!(c.registers[7], 0x902);
        assert_eq!(&b.ram[0x900..0x902], &[0x10, 0x32]);
    }
}
