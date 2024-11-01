use super::register::Registers;

#[derive(Debug, Default)]
pub struct Cpu {
    pub registers: Registers,
    pub ime: u8,
    pub cache: u16,
}
