
use std::{fmt::write, io::ErrorKind};

use crate::{
    cpu::{
        proc::Cpu, 
        register::types::{
            Flags, Register16, Register8, F8
        }
    }, 
    gba::opcode::{
        Opcode,
        types::{JumpCondition, LoadDirection, MathOp, OpcodeIndirectRegister16, OpcodeRegister8}, 
    },
    mem::prelude::{
        Cart, Mem, BOOT_ROM
    }
};

use super::opcode::types::OpcodeRegister16;

pub struct Gba<'a> {
    pub cpu: Cpu,
    pub mem: Mem<'a>,
    pub boot_rom: &'static [u8],
}

#[derive(Debug)]
pub enum OpcodeExecuteError {
    UnsupportedOpcode(String),
    MissingOpcodeSupport(String),
}

impl<'a> Gba<'a> {
    pub fn new(rom: String) -> Result<Self, ErrorKind> {
        let mut cpu = Self { cpu: Cpu::default(), mem: Mem::new(Cart::new(rom)?), boot_rom: &BOOT_ROM };
        /* Execute Boot ROM */
        Ok(cpu)
    }

    pub fn execute(&mut self, opcode: Opcode) -> usize {
        let mut cycles = 1;

        use Opcode::*;
        match opcode {
            // 8-bit Loading {{{
            LoadR8(dst, src) => {
                let src = match src {
                    OpcodeRegister8::HL => {
                        cycles += 1;
                        let addr = self.cpu.registers.get_r16(Register16::HL);
                        self.mem.get_u8(addr)
                    },
                    _ => self.cpu.registers.get_r8(Register8::from(src)),
                };
                match dst {
                    OpcodeRegister8::HL => {
                        cycles += 1;
                        let addr = self.cpu.registers.get_r16(Register16::HL);
                        self.mem.set_u8(addr, src);
                    },
                    _ => self.cpu.registers.set_r8(Register8::from(dst), src),
                };
            },
            LoadImm8(dst) => {
                let (val, cyc) = self.fetch_byte();
                match dst {
                    OpcodeRegister8::HL => {
                        cycles += 1;
                        let addr = self.cpu.registers.get_r16(Register16::HL);
                        self.mem.set_u8(addr, val);
                    },
                    _ => self.cpu.registers.set_r8(Register8::from(dst), val),
                };
                cycles += cyc;
            },
            LoadIndR16(src, direction) => {
                cycles += 2;
                let addr = match src {
                    OpcodeIndirectRegister16::HLInc => {
                        let hl = self.cpu.registers.get_r16(Register16::HL);
                        self.cpu.registers.set_r16(Register16::HL, hl + 1);
                        hl
                    },
                    OpcodeIndirectRegister16::HLDec => {
                        let hl = self.cpu.registers.get_r16(Register16::HL);
                        self.cpu.registers.set_r16(Register16::HL, hl - 1);
                        hl
                    },
                    _ => self.cpu.registers.get_r16(Register16::from(src)),
                };
                if let LoadDirection::Memory = direction 
                    { self.mem.set_u8(addr, self.cpu.registers.a); }
                else { self.cpu.registers.a = self.mem.get_u8(addr); }
            },
            LoadIndOffImm8(direction) => {
                let (off, cyc) = self.fetch_byte();
                let addr = 0xFF00 + off as u16;
                cycles += cyc + 1;
                if let LoadDirection::Memory = direction 
                    { self.mem.set_u8(addr, self.cpu.registers.a); } 
                else { self.cpu.registers.a = self.mem.get_u8(addr); }
            },
            LoadIndOffRegC(direction) => {
                cycles += 1;
                let addr = 0xFF00 + self.cpu.registers.c as u16;
                if let LoadDirection::Memory = direction 
                    { self.mem.set_u8(addr, self.cpu.registers.a); } 
                else { self.cpu.registers.a = self.mem.get_u8(addr); }
            },
            LoadIndImm16(direction) => {
                let (addr, cyc) = self.fetch_word();
                cycles += cyc + 1;
                if let LoadDirection::Memory = direction 
                    { self.mem.set_u8(addr, self.cpu.registers.a); } 
                else { self.cpu.registers.a = self.mem.get_u8(addr); }
            },
            //}}}
            // 16-bit Loading {{{
            LoadImm16(dst) => {
                let (val, cyc) = self.fetch_word();
                cycles += cyc;
                self.cpu.registers.set_r16(Register16::from(dst), val);
            },
            LoadIndImm16SP => {
                let (addr, cyc) = self.fetch_word();
                cycles += cyc + 2;
                self.mem.set_u16(addr, self.cpu.registers.sp);
            },
            LoadSPHL => {
                cycles += 1;
                self.cpu.registers.sp = self.cpu.registers.get_r16(Register16::HL);
            },
            PushR16(src) => {
                let src = self.cpu.registers.get_r16(Register16::from(src));
                cycles += 1 + self.push(src)
            },
            PopR16(dst) => {
                cycles += 2;
                let val = self.mem.get_u16(self.cpu.registers.sp);
                self.cpu.registers.sp += 2;
                self.cpu.registers.set_r16(Register16::from(dst), val);
            },
            LoadHLOffSp => {
                let (off, cyc) = self.fetch_byte();
                cycles += cyc + 1;
                let sp = self.cpu.registers.sp;
                let (addr, over) = sp.overflowing_add(off as u16);
                let val = self.mem.get_u16(addr);

                self.cpu.registers.f &= !(Flags::Zero | Flags::Subtract);
                if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= Flags::Carry; }
                if ((sp & 0x000F) as u8 + (off & 0x0F)) & 0x10 != 0 {
                    self.cpu.registers.f |= Flags::HalfCarry;
                } else {
                    self.cpu.registers.f &= !Flags::HalfCarry;
                }
            },
            //}}}
            // 8-bit Arithmetic {{{
            MathR8(op, src) => {
                let (src, cyc) = self.fetch_register_8(src);
                cycles += cyc;
                match op {
                    MathOp::Add => {
                        let (val, over) = self.cpu.registers.a.overflowing_add(src);

                        self.cpu.registers.f &= !Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) + (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }
                        
                        self.cpu.registers.a = val;
                    },
                    MathOp::Adc => {
                        let (val, over) = self.cpu.registers.a.overflowing_add(src + self.cpu.registers.f.is_set(Flags::Carry) as u8);

                        self.cpu.registers.f &= !Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) + (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }

                        self.cpu.registers.a = val;
                    },
                    MathOp::Sub => {
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src);
                        
                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }
                        
                        self.cpu.registers.a = val;
                    },
                    MathOp::Sbc => {
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src - self.cpu.registers.f.is_set(Flags::Carry) as u8);

                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }
                        
                        self.cpu.registers.a = val;
                    },
                    MathOp::And => {
                        self.cpu.registers.a &= src;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry);
                        self.cpu.registers.f |= Flags::HalfCarry;
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                    MathOp::Xor => {
                        self.cpu.registers.a ^= src;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry | Flags::HalfCarry);
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                    MathOp::Or => {
                        self.cpu.registers.a |= src;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry | Flags::HalfCarry);
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                    MathOp::Cp => {
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src);

                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }
                    },
                };
            },
            MathImm8(op) => {
                match op {
                    MathOp::Add => {
                        let (src, cyc) = self.fetch_byte();
                        let (val, over) = self.cpu.registers.a.overflowing_add(src);

                        self.cpu.registers.f &= !Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) + (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }

                        self.cpu.registers.a = val;
                        cycles += cyc;
                    },
                    MathOp::Adc => {
                        let (src, cyc) = self.fetch_byte();
                        let (val, over) = self.cpu.registers.a.overflowing_add(src + self.cpu.registers.f.is_set(Flags::Carry) as u8);

                        self.cpu.registers.f &= !Flags::Subtract;
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) + (src & 0x0F) + (self.cpu.registers.f.is_set(Flags::Carry) as u8)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }

                        cycles += cyc;
                        self.cpu.registers.a = val;
                    },
                    MathOp::Sub => {
                        let (src, cyc) = self.fetch_byte();
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src);

                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }

                        cycles += cyc;
                        self.cpu.registers.a = val;
                    },
                    MathOp::Sbc => {
                        let (src, cyc) = self.fetch_byte();
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src - self.cpu.registers.f.is_set(Flags::Carry) as u8);

                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F) - self.cpu.registers.f.is_set(Flags::Carry) as u8) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }

                        cycles += cyc;
                        self.cpu.registers.a = val;
                    },
                    MathOp::Cp => {
                        let (src, cyc) = self.fetch_byte();
                        let (val, over) = self.cpu.registers.a.overflowing_sub(src);

                        self.cpu.registers.f |= Flags::Subtract;
                        if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                        if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                        if ((self.cpu.registers.a & 0x0F) - (src & 0x0F)) == 0x10 {
                            self.cpu.registers.f |= Flags::HalfCarry;
                        } else {
                            self.cpu.registers.f &= !Flags::HalfCarry;
                        }

                        cycles += cyc;
                    },
                    MathOp::And => {
                        let (val, cyc) = self.fetch_byte();
                        cycles += cyc;
                        self.cpu.registers.a &= val;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry);
                        self.cpu.registers.f |= Flags::HalfCarry;
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                    MathOp::Or => {
                        let (val, cyc) = self.fetch_byte();
                        cycles += cyc;
                        self.cpu.registers.a |= val;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry | Flags::HalfCarry);
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                    MathOp::Xor => {
                        let (val, cyc) = self.fetch_byte();
                        cycles += cyc;
                        self.cpu.registers.a ^= val;

                        self.cpu.registers.f &= !(Flags::Subtract | Flags::Carry | Flags::HalfCarry);
                        if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                    },
                };
            },
            IncR8(reg) => {
                let mut val = 0;
                match reg {
                    OpcodeRegister8::HL => {
                        let addr = self.cpu.registers.get_r16(Register16::HL);
                        val = self.mem.get_u8(addr);
                        self.mem.set_u8(addr, val + 1);
                        cycles += 2;
                    },
                    _ => {
                        let reg = Register8::from(reg);
                        val = self.cpu.registers.get_r8(reg);
                        self.cpu.registers.set_r8(reg, val + 1);
                    },
                };

                self.cpu.registers.f &= !Flags::Subtract;
                if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                if val & 0x08 != 0 { self.cpu.registers.f |= Flags::HalfCarry; } else { self.cpu.registers.f &= !Flags::HalfCarry; }
            },
            DecR8(reg) => {
                let mut val = 0;
                match reg {
                    OpcodeRegister8::HL => {
                        let addr = self.cpu.registers.get_r16(Register16::HL);
                        val = self.mem.get_u8(addr);
                        self.mem.set_u8(addr, val + 1);
                        cycles += 2;
                    },
                    _ => {
                        let reg = Register8::from(reg);
                        val = self.cpu.registers.get_r8(reg);
                        self.cpu.registers.set_r8(reg, val + 1);
                    },
                };

                self.cpu.registers.f &= !Flags::Subtract;
                if val == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                if (val & 0x0F) - 1 != 0 { self.cpu.registers.f |= Flags::HalfCarry; } else { self.cpu.registers.f &= !Flags::HalfCarry; }
            },
            ComplementCarryFlag => {
                self.cpu.registers.f ^= Flags::Carry;
                self.cpu.registers.f &= !(Flags::Subtract | Flags::HalfCarry);
            },
            SetCarryFlag => {
                self.cpu.registers.f |= Flags::Carry;
                self.cpu.registers.f &= !(Flags::Subtract | Flags::HalfCarry);
            },
            DecimalAdjustAccumulator => {
                if self.cpu.registers.f.is_set(Flags::Subtract) {
                    if self.cpu.registers.f.is_set(Flags::Carry) || self.cpu.registers.a > 0x99 {
                        self.cpu.registers.a += 0x60;
                        self.cpu.registers.f |= Flags::Carry;
                    }
                    if self.cpu.registers.f.is_set(Flags::HalfCarry) || (self.cpu.registers.a & 0x0F) > 0x09 {
                        self.cpu.registers.a += 0x06;
                    }
                }
                else {
                    if self.cpu.registers.f.is_set(Flags::Carry) {
                        self.cpu.registers.a -= 0x60;
                    }
                    if self.cpu.registers.f.is_set(Flags::HalfCarry) {
                        self.cpu.registers.a -= 0x06;
                    }
                }

                if self.cpu.registers.a == 0 { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                self.cpu.registers.f &= !Flags::HalfCarry;
            },
            ComplementAccumulator => {
                self.cpu.registers.a ^= 0xFF;
                self.cpu.registers.f |= Flags::Subtract | Flags::HalfCarry;
            },
            //}}}
            // 16-bit Arithmetic {{{
            IncR16(src) => {
                cycles += 1;
                crate::cpu::register::types::as_u16_slice_mut(&mut self.cpu.registers)[Register16::from(src) as usize] += 1;
            },
            DecR16(src) => {
                cycles += 1;
                crate::cpu::register::types::as_u16_slice_mut(&mut self.cpu.registers)[Register16::from(src) as usize] -= 1;
            },
            AddR16(src) => {
                let hl = self.cpu.registers.get_r16(Register16::HL);
                let src = self.cpu.registers.get_r16(Register16::from(src));
                let (val, over) = hl.overflowing_add(src);

                cycles += 1;
                self.cpu.registers.set_r16(Register16::HL, val);

                self.cpu.registers.f &= !Flags::Subtract;
                if over { self.cpu.registers.f |= Flags::Carry; } else { self.cpu.registers.f &= !Flags::Carry; }
                if ((hl & 0x0F00) + (src & 0x0F00)) & 0x1000 != 0 {
                    self.cpu.registers.f |= Flags::HalfCarry;
                } else {
                    self.cpu.registers.f &= !Flags::HalfCarry;
                }
            },
            AddSPImm8 => {
                let (off, cyc) = self.fetch_byte();
                let sp = self.cpu.registers.sp;
                let (sp, over) = if off as i8 >= 0 { sp.overflowing_add(off as u16) } else { sp.overflowing_sub(-(off as i8) as u16) };
                self.cpu.registers.sp = sp;

                self.cpu.registers.f &= !(Flags::Zero | Flags::Subtract);
                if over { self.cpu.registers.f |= Flags::Zero; } else { self.cpu.registers.f &= !Flags::Zero; }
                if ((sp & 0x000F) + (off as u16 & 0x000F)) == 0x0010 {
                    self.cpu.registers.f |= Flags::HalfCarry;
                } else {
                    self.cpu.registers.f &= !Flags::HalfCarry;
                }
            },
            //}}}
            // Rotate, Shift, and Bit {{{
            RotateLeftCircularAccumulator => {
                let (a, c) = self.cpu.registers.a.overflowing_shl(1);
                self.cpu.registers.f = if c { F8::from(Flags::Carry as u8) } else { F8::from(0) };
                self.cpu.registers.a = a | (c as u8);
            },
            RotateLeftAccumulator => {
                let (a, c) = self.cpu.registers.a.overflowing_shl(1);
                let old_c = self.cpu.registers.f.is_set(Flags::Carry) as u8;
                self.cpu.registers.f = if c { F8::from(Flags::Carry as u8) } else { F8::from(0) };
                self.cpu.registers.a = a | old_c;
            },
            RotateRightCircularAccumulator => {
                let (a, c) = self.cpu.registers.a.overflowing_shr(1);
                self.cpu.registers.f = if c { F8::from(Flags::Carry as u8) } else { F8::from(0) };
                self.cpu.registers.a = a | ((c as u8) << 7);
            },
            RotateRightAccumulator => {
                let (a, c) = self.cpu.registers.a.overflowing_shr(1);
                let old_c = self.cpu.registers.f.is_set(Flags::Carry) as u8;
                self.cpu.registers.f = if c { F8::from(Flags::Carry as u8) } else { F8::from(0) };
                self.cpu.registers.a = a | (old_c << 7);
            },
            //}}}
            // Control Flow {{{
            JumpImm16(condition) => {
                let (addr, cyc) = self.fetch_word();
                cycles += cyc + self.jump(addr, condition);
            },
            JumpHL => {
                let addr = self.cpu.registers.get_r16(Register16::HL);
                self.cpu.registers.pc = addr;
            },
            JumpOffImm8(condition) => {
                let (off, cyc) = self.fetch_byte();
                let addr = self.cpu.registers.pc + (off as i8) as u16;
                cycles += cyc + self.jump(addr, condition)
            },
            CallImm16(condition) => {
                let (addr, cyc) = self.fetch_word();
                cycles += cyc + self.call(addr, condition);
            },
            Return(condition) => {
                cycles += match condition {
                    JumpCondition::Always => {
                        self.cpu.registers.pc = self.mem.get_u16(self.cpu.registers.sp);
                        self.cpu.registers.sp += 2;
                        4
                    },
                    JumpCondition::SetFlag(flag) => {
                        if self.cpu.registers.f.is_set(flag) {
                            self.cpu.registers.pc = self.mem.get_u16(self.cpu.registers.sp);
                            self.cpu.registers.sp += 2;
                            4
                        } else { 1 }
                    },
                    JumpCondition::UnsetFlag(flag) => {
                        if !self.cpu.registers.f.is_set(flag) {
                            self.cpu.registers.pc = self.mem.get_u16(self.cpu.registers.sp);
                            self.cpu.registers.sp += 2;
                            4
                        } else { 1 }
                    }
                };
            },
            ReturnInterupt => {
                self.cpu.registers.pc = self.mem.get_u16(self.cpu.registers.pc);
                self.cpu.registers.sp += 2;
                self.cpu.ime = 1;
                cycles += 3;
            },
            Restart(vector) => {
                cycles += 1 + self.push(self.cpu.registers.pc);
                self.cpu.registers.pc = vector as u16;
            },
            //}}}
            // Misc. {{{
            DisableInterrupts => self.cpu.ime = 0,
            EnableInterrupts => self.cpu.ime = 1,
            Noop => (),
            //}}}
            _ => panic!("Cannot execute opcode: {:?}", opcode),
        };

        cycles
    }

    pub fn call(&mut self, addr: u16, condition: JumpCondition) -> usize {
        use JumpCondition::*;
        match condition {
            Always => {
                let cyc = self.push(self.cpu.registers.pc);
                self.cpu.registers.pc = addr;
                cyc + 1
            },
            SetFlag(flag) => {
                if self.cpu.registers.f.is_set(flag) {
                    let cyc = self.push(self.cpu.registers.pc);
                    self.cpu.registers.pc = addr;
                    cyc + 1
                } else { 0 }
            },
            UnsetFlag(flag) => {
                if !self.cpu.registers.f.is_set(flag) {
                    let cyc = self.push(self.cpu.registers.pc);
                    self.cpu.registers.pc = addr;
                    cyc + 1
                } else { 0 }
            },
        }
    }

    pub fn push(&mut self, val: u16) -> usize {
        self.cpu.registers.sp -= 2;
        self.mem.set_u16(self.cpu.registers.sp, val);
        2
    }

    pub fn jump(&mut self, addr: u16, condition: JumpCondition) -> usize {
        use JumpCondition::*;
        match condition {
            Always => { self.cpu.registers.pc = addr; 1 },
            SetFlag(flag) => {
                if self.cpu.registers.f.is_set(flag) {
                    self.cpu.registers.pc = addr;
                    1
                } else { 0 }
            },
            UnsetFlag(flag) => {
                if !self.cpu.registers.f.is_set(flag) {
                    self.cpu.registers.pc = addr;
                    1
                } else { 0 }
            },
        }
    }

    pub fn fetch_register_8(&self, reg: OpcodeRegister8) -> (u8, usize) {
        match reg {
            OpcodeRegister8::HL => (self.mem.get_u8(self.cpu.registers.get_r16(Register16::HL)), 1),
            _ => (self.cpu.registers.get_r8(Register8::from(reg)), 0),
        }
    }

    #[inline(always)]
    pub fn fetch_register_16(&self, reg: OpcodeRegister16) -> (u16, usize) {
        (self.cpu.registers.get_r16(Register16::from(reg)), 1)
    }

    pub fn fetch_byte(&mut self) -> (u8, usize) {
        let byte = self.mem.get_u8(self.cpu.registers.pc);
        self.cpu.registers.pc += 1;
        (byte, 1)
    }

    pub fn fetch_word(&mut self) -> (u16, usize) {
        let word = self.mem.get_u16(self.cpu.registers.pc);
        self.cpu.registers.pc += 2;
        (word, 2)
    }

    pub fn execute_boot_rom(&mut self) {
        todo!("Execute Boot ROM")
    }

}
