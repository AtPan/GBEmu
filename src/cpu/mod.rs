#![allow(unused)]

pub mod register;
pub mod proc;

pub mod prelude {
    use super::register::{Registers, types::{Register8, Register16, Flags}};
    use super::proc::Cpu;
}
