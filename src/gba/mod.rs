#![allow(unused)]

pub mod console;
pub mod opcode;
pub mod mcycle;

pub mod prelude {
    pub use super::console::Gba;
    pub use super::opcode::Opcode;
}
