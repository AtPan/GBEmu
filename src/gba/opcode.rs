use crate::cpu::register::types::Flags;

use self::types::{JumpCondition, LoadDirection, MathOp, OpcodeIndirectRegister16, OpcodeRegister16, OpcodeRegister8};

// mod types {{{
pub mod types {
    use crate::cpu::register::types::{Flags, Register16, Register8};

    // enum OpcodeRegister8 {{{
    #[repr(u8)]
    #[derive(Debug)]
    pub enum OpcodeRegister8 {
        B = 0, C, D, E, H, L, HL, A
    }

    impl From<u8> for OpcodeRegister8 {
        fn from(value: u8) -> Self {
            match value {
                0..=7 => unsafe { std::mem::transmute(value) },
                _ => panic!("Unrecognized value for OpcodeRegister8: {:?}", value),
            }
        }
    }

    impl From<OpcodeRegister8> for Register8 {
        fn from(value: OpcodeRegister8) -> Self {
            use OpcodeRegister8::*;
            match value {
                HL => Register8::F,
                A => Register8::A,
                _ => unsafe { std::mem::transmute(value as u8) },
            }
        }
    }
    // }}}

    // enum OpcodeRegister16Indirect {{{
    #[repr(u8)]
    #[derive(Debug)]
    pub enum OpcodeIndirectRegister16 {
        BC = 0, DE, HLInc, HLDec,
    }

    impl From<u8> for OpcodeIndirectRegister16 {
        fn from(value: u8) -> Self {
            match value {
                0..=3 => unsafe { std::mem::transmute(value) },
                _ => panic!("Unrecognized value for OpcodeRegister16Indirect: {:?}", value),
            }
        }
    }

    impl From<OpcodeIndirectRegister16> for Register16 {
        fn from(value: OpcodeIndirectRegister16) -> Self {
            use OpcodeIndirectRegister16::*;
            match value {
                BC => Self::BC,
                DE => Self::DE,
                _ => Self::HL,
            }
        }
    }
    // }}}

    // enum LoadDirection {{{
    #[derive(Debug)]
    pub enum LoadDirection {
        Memory,
        Accumulator,
    }
    // }}}

    // enum OpcodeRegister16 {{{
    #[repr(u8)]
    #[derive(Debug)]
    pub enum OpcodeRegister16 {
        BC = 0, DE, HL, AF,
    }

    impl From<u8> for OpcodeRegister16 {
        fn from(value: u8) -> Self {
            match value {
                0..=3 => unsafe { std::mem::transmute(value) },
                _ => panic!("Unrecognized value for OpcodeRegister16: {:?}", value),
            }
        }
    }

    impl From<OpcodeRegister16> for Register16 {
        fn from(value: OpcodeRegister16) -> Self {
            unsafe { std::mem::transmute(value) }
        }
    }
    // }}}

    // enum JumpCondition {{{
    #[derive(Debug)]
    pub enum JumpCondition {
        Always,
        SetFlag(Flags),
        UnsetFlag(Flags),
    }
    //}}}

    // enum MathOp {{{
    #[repr(u8)]
    #[derive(Debug, Copy, Clone)]
    pub enum MathOp {
        Add = 0, Adc, Sub, Sbc, And, Xor, Or, Cp,
    }

    impl From<u8> for MathOp {
        fn from(value: u8) -> Self {
            match value {
                0..=7 => unsafe { std::mem::transmute(value) },
                _ => panic!("Unrecognized value for MathOp: `${:#02X}`", value),
            }
        }
    }
    //}}}
}
//}}}

#[derive(Debug)]
pub enum Opcode {
    // 8-bit Loads {{{
    LoadR8(OpcodeRegister8, OpcodeRegister8),
    LoadImm8(OpcodeRegister8),
    LoadIndR16(OpcodeIndirectRegister16, LoadDirection),
    LoadIndOffImm8(LoadDirection),
    LoadIndOffRegC(LoadDirection),
    LoadIndImm16(LoadDirection),
    // }}}
    
    // 16-bit Loads {{{
    LoadImm16(OpcodeRegister16),
    LoadIndImm16SP,
    LoadSPHL,
    PushR16(OpcodeRegister16),
    PopR16(OpcodeRegister16),
    LoadHLOffSp,
    // }}}

    // 8-bit Arithmetic / Logical {{{
    MathR8(MathOp, OpcodeRegister8),
    MathImm8(MathOp),
    IncR8(OpcodeRegister8),
    DecR8(OpcodeRegister8),
    ComplementCarryFlag,
    SetCarryFlag,
    DecimalAdjustAccumulator,
    ComplementAccumulator,
    // }}}

    // 16-bit Arithmetic {{{
    IncR16(OpcodeRegister16),
    DecR16(OpcodeRegister16),
    AddR16(OpcodeRegister16),
    AddSPImm8,
    // }}}

    // Rotate / Shift / Bit {{{
    RotateLeftCircularAccumulator,
    RotateRightCircularAccumulator,
    RotateLeftAccumulator,
    RotateRightAccumulator,
    // }}}

    // Control Flow {{{
    JumpImm16(JumpCondition),
    JumpHL,
    JumpOffImm8(JumpCondition),
    CallImm16(JumpCondition),
    Return(JumpCondition),
    ReturnInterupt,
    Restart(u8),
    // }}}

    // Misc. {{{
    Halt,
    Stop,
    DisableInterrupts,
    EnableInterrupts,
    Noop,
    // }}}
}

impl From<u8> for Opcode {
    fn from(value: u8) -> Self {
        use Opcode::*;
        let low = value & 0x0F;
        let high = value >> 4;
        match value {
            0x00 => Noop,
            0x07 => RotateLeftCircularAccumulator,
            0x08 => LoadIndImm16SP,
            0x0F => RotateRightCircularAccumulator,
            0x10 => Stop,
            0x17 => RotateLeftAccumulator,
            0x1F => RotateRightAccumulator,
            0x20 => JumpOffImm8(JumpCondition::UnsetFlag(Flags::Zero)),
            0x27 => DecimalAdjustAccumulator,
            0x2F => ComplementAccumulator,
            0x30 => JumpOffImm8(JumpCondition::UnsetFlag(Flags::Carry)),
            0x37 => SetCarryFlag,
            0x3F => ComplementCarryFlag,

            0x40..=0x7F => LoadR8(OpcodeRegister8::from((value & 0x38) >> 3), OpcodeRegister8::from(value & 0x07)),
            0x80..=0xBF => MathR8(MathOp::from((value & 0x38) >> 3), OpcodeRegister8::from(value & 0x07)),

            0xC0 => Return(JumpCondition::UnsetFlag(Flags::Zero)),
            0xC2 => JumpImm16(JumpCondition::UnsetFlag(Flags::Zero)),
            0xC3 => JumpImm16(JumpCondition::Always),
            0xC4 => CallImm16(JumpCondition::UnsetFlag(Flags::Zero)),
            0xC8 => Return(JumpCondition::SetFlag(Flags::Zero)),
            0xC9 => Return(JumpCondition::Always),
            0xCA => JumpImm16(JumpCondition::SetFlag(Flags::Zero)),
            0xCB => panic!("Incomplete 8-bit Opcode: `$CB`"),
            0xCC => CallImm16(JumpCondition::SetFlag(Flags::Zero)),
            0xCD => CallImm16(JumpCondition::Always),
            0xD0 => Return(JumpCondition::UnsetFlag(Flags::Carry)),
            0xD2 => JumpImm16(JumpCondition::UnsetFlag(Flags::Carry)),
            0xD4 => CallImm16(JumpCondition::UnsetFlag(Flags::Carry)),
            0xD8 => Return(JumpCondition::SetFlag(Flags::Carry)),
            0xD9 => ReturnInterupt,
            0xDA => JumpImm16(JumpCondition::SetFlag(Flags::Carry)),
            0xDC => CallImm16(JumpCondition::SetFlag(Flags::Carry)),
            0xE0 => LoadIndOffImm8(LoadDirection::Memory),
            0xE2 => LoadIndOffRegC(LoadDirection::Memory),
            0xE8 => AddSPImm8,
            0xE9 => JumpHL,
            0xEA => LoadIndImm16(LoadDirection::Memory),
            0xF0 => LoadIndOffImm8(LoadDirection::Accumulator),
            0xF2 => LoadIndOffRegC(LoadDirection::Accumulator),
            0xF3 => DisableInterrupts,
            0xF8 => LoadHLOffSp,
            0xF9 => LoadSPHL,
            0xFA => LoadIndImm16(LoadDirection::Accumulator),
            0xFB => EnableInterrupts,

            _ => match high {
                0x00..=0x03 => match low {
                    0x01 => LoadImm16(OpcodeRegister16::from(high)),
                    0x02 => LoadIndR16(OpcodeIndirectRegister16::from(high), LoadDirection::Memory),
                    0x03 => IncR16(OpcodeRegister16::from(high)),
                    0x04 => IncR8(OpcodeRegister8::from(high << 1)),
                    0x05 => DecR8(OpcodeRegister8::from(high << 1)),
                    0x06 => LoadImm8(OpcodeRegister8::from(high << 1)),
                    0x09 => AddR16(OpcodeRegister16::from(high)),
                    0x0A => LoadIndR16(OpcodeIndirectRegister16::from(high), LoadDirection::Accumulator),
                    0x0B => DecR16(OpcodeRegister16::from(high)),
                    0x0C => IncR8(OpcodeRegister8::from((high << 1) | 1)),
                    0x0D => DecR8(OpcodeRegister8::from((high << 1) | 1)),
                    0x0E => LoadImm8(OpcodeRegister8::from((high << 1) | 1)),
                    _ => panic!("Uncaught Opcode: `${:#02X}`", value),
                },
                0x0C..=0x0F => match low {
                    0x01 => PopR16(OpcodeRegister16::from(high & 0x03)),
                    0x05 => PushR16(OpcodeRegister16::from(high & 0x03)),
                    0x06 => MathImm8(MathOp::from((high & 0x03) << 1)),
                    0x07 => Restart((high & 0x03) << 4),
                    0x0E => MathImm8(MathOp::from(((high & 0x03) << 1) | 1)),
                    0x0F => Restart(((high & 0x03) << 4) | 0x08),
                    _ => panic!("Uncaught Opcode: `${:#02X}`", value),
                },
                _ => unreachable!(),
            },
        }
    }
}
