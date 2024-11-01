use std::ops::Index;

use self::types::{Register16, Register8, F8};

// mod types {{{
pub mod types {
    use std::{ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not}, ptr::addr_of};

    #[inline(always)]
    pub fn as_u8_slice<T>(r: &T) -> &[u8] where T: Sized {
        unsafe { std::slice::from_raw_parts((r as *const T) as *const u8, std::mem::size_of::<T>()) }
    }

    #[inline(always)]
    pub fn as_u8_slice_mut<T>(r: &mut T) -> &mut [u8] where T: Sized {
        unsafe { std::slice::from_raw_parts_mut((r as *mut T) as *mut u8, std::mem::size_of::<T>()) }
    }

    #[inline(always)]
    pub fn as_u16_slice<T>(r: &T) -> &[u16] where T: Sized {
        unsafe { std::slice::from_raw_parts((r as *const T) as *const u16, std::mem::size_of::<T>() / 2) }
    }

    #[inline(always)]
    pub fn as_u16_slice_mut<T>(r: &mut T) -> &mut [u16] where T: Sized {
        unsafe { std::slice::from_raw_parts_mut((r as *mut T) as *mut u16, std::mem::size_of::<T>() / 2) }
    }

    // enum Flags {{{
    #[repr(u8)]
    #[derive(Debug)]
    pub enum Flags {
        Zero = 0x80_u8,
        Subtract = 0x40_u8,
        HalfCarry = 0x20_u8,
        Carry = 0x10_u8,
    }

    impl From<u8> for Flags {
        fn from(value: u8) -> Self {
            match value {
                0x80 | 0x40 | 0x20 | 0x10 => unsafe { std::mem::transmute(value) },
                _ => panic!("Value not a valid flag: {:?}", value),
            }
        }
    }

    impl BitOr for Flags {
        type Output = F8;
        fn bitor(self, rhs: Self) -> Self::Output {
            F8(self as u8 | rhs as u8)
        }
    }

    impl BitAnd for Flags {
        type Output = F8;
        fn bitand(self, rhs: Self) -> Self::Output {
            F8(self as u8 & rhs as u8)
        }
    }

    impl Not for Flags {
        type Output = F8;
        fn not(self) -> Self::Output {
            F8(!(self as u8))
        }
    }
    //}}}

    // struct F8 {{{
    #[derive(Copy, Clone, Default, Debug)]
    pub struct F8(u8);

    impl F8 {
        pub fn is_set(&self, value: Flags) -> bool {
            self.0 & value as u8 != 0
        }

        pub fn set(&mut self, value: Flags) {
            self.0 |= value as u8;
        }

        pub fn unset(&mut self, value: Flags) {
            self.0 &= u8::from(!value);
        }

        pub fn flip(&mut self, value: Flags) {
            self.0 ^= value as u8;
        }

        pub fn is_set_all(&self, value: F8) -> bool {
            (*self & value).0 != 0
        }

        pub fn set_all(&mut self, value: F8) {
            *self |= value;
        }

        pub fn unset_all(&mut self, value: F8) {
            *self &= !value;
        }

        pub fn flip_all(&mut self, value: F8) {
            *self ^= value;
        }
    }

    impl From<u8> for F8 {
        fn from(value: u8) -> Self {
            Self(value)
        }
    }

    impl From<F8> for u8 {
        fn from(value: F8) -> Self {
            value.0
        }
    }

    impl BitAnd for F8 {
        type Output = Self;
        fn bitand(self, rhs: Self) -> Self::Output {
            Self(self.0 & rhs.0)
        }
    }

    impl BitAnd<Flags> for F8 {
        type Output = Self;
        fn bitand(self, rhs: Flags) -> Self::Output {
            Self(self.0 & rhs as u8)
        }
    }

    impl BitAndAssign for F8 {
        fn bitand_assign(&mut self, rhs: Self) {
            self.0 &= rhs.0;
        }
    }

    impl BitAndAssign<Flags> for F8 {
        fn bitand_assign(&mut self, rhs: Flags) {
            self.0 &= rhs as u8;
        }
    }

    impl BitOr for F8 {
        type Output = Self;
        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    impl BitOr<Flags> for F8 {
        type Output = Self;
        fn bitor(self, rhs: Flags) -> Self::Output {
            Self(self.0 | rhs as u8)
        }
    }

    impl BitOrAssign for F8 {
        fn bitor_assign(&mut self, rhs: Self) {
            self.0 |= rhs.0;
        }
    }

    impl BitOrAssign<Flags> for F8 {
        fn bitor_assign(&mut self, rhs: Flags) {
            self.0 |= rhs as u8;
        }
    }

    impl BitXor for F8 {
        type Output = F8;
        fn bitxor(self, rhs: Self) -> Self::Output {
            Self(self.0 ^ rhs.0)
        }
    }

    impl BitXor<Flags> for F8 {
        type Output = Self;
        fn bitxor(self, rhs: Flags) -> Self::Output {
            Self(self.0 ^ rhs as u8)
        }
    }
    
    impl BitXorAssign for F8 {
        fn bitxor_assign(&mut self, rhs: Self) {
            self.0 ^= rhs.0;
        }
    }

    impl BitXorAssign<Flags> for F8 {
        fn bitxor_assign(&mut self, rhs: Flags) {
            self.0 ^= rhs as u8;
        }
    }

    impl Not for F8 {
        type Output = Self;
        fn not(self) -> Self::Output {
            Self(!self.0)
        }
    }
    //}}}

    // enum Register8 {{{
    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum Register8 {
        B = 0, C, D, E, H, L, A, F, SPHigh, SPLow, PCHigh, PCLow,
    }

    impl From<u8> for Register8 {
        fn from(value: u8) -> Self {
            match value {
                0..=11 => {
                    unsafe { std::mem::transmute(value) }
                },
                _ => panic!("Invalid value for Register8: {:?}", value),
            }
        }
    }
    // }}}

    // enum Register16 {{{
    #[repr(u8)]
    #[derive(Copy, Clone)]
    pub enum Register16 {
        BC = 0, DE, HL, AF, SP, PC,
    }

    impl From<u8> for Register16 {
        fn from(value: u8) -> Self {
            match value {
                0..=5 => {
                    unsafe { std::mem::transmute(value) }
                },
                _ => panic!("Invalid value for Register16: {:?}", value),
            }
        }
    }
    // }}}
}
//}}}

#[derive(Debug, Default)]
pub struct Registers {
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub a: u8,
    pub f: F8,
    pub sp: u16,
    pub pc: u16,
}

impl Registers {
    pub fn get_r8(&self, value: Register8) -> u8 {
        unsafe { self::types::as_u8_slice(self)[value as usize] }
    }

    pub fn get_r16(&self, value: Register16) -> u16 {
        unsafe { self::types::as_u16_slice(self)[value as usize] }
    }

    pub fn set_r8(&mut self, reg: Register8, value: u8) {
        unsafe { self::types::as_u8_slice_mut(self)[reg as usize] = value; }
    }

    pub fn set_r16(&mut self, reg: Register16, value: u16) {
        unsafe { self::types::as_u16_slice_mut(self)[reg as usize] = value; }
    }
}
