use std::{borrow::Borrow, hint::unreachable_unchecked, ops::{Index, IndexMut}, slice::SliceIndex};

use super::prelude::Cart;

pub struct Mem<'a> {
    cart:         Cart,
    rom_bank:     &'a [u8],
    rom_switch:   &'a [u8],
    ram:          [u8; 0x6000],
    sprite_oam:   [u8; 0x00A0],
    io_ports:     [u8; 0x004C],
    ram_stack:    [u8; 0x0080],
}

impl<'a, T> Index<T> for Mem<'a>
    where T: Into<u16>
{
    type Output = u8;

    fn index(&self, index: T) -> &Self::Output {
        let index = index.into() as usize;
        match index {
            0xFF80..=0xFFFF => &self.ram_stack[index - 0xFF80], /* Internal RAM */
            0xFF4C..=0xFF7F => panic!("Accessing memory ${:#04X}: Empty but unusable for I/O", index),
            0xFF00..=0xFF4B => &self.io_ports[index - 0xFF00], /* I/O Ports */
            0xFEA0..=0xFEFF => panic!("Accessing memory ${:#04X}: Empty but unusable for I/O", index),
            0xFE00..=0xFE9F => &self.sprite_oam[index - 0xFE00], /* Sprite Attrib Memory (OAM) */

            0xE000..=0xFDFF => &self.ram[index - 0xA000], /* Echo of 8kB Internal RAM */
            0x8000..=0xDFFF => &self.ram[index - 0x8000],
            //0xC000..=0xDFFF => self.ram_internal[index - 0xC000], /* 8kB Internal RAM */
            //0xA000..=0xBFFF => self.ram_bank[index - 0xA000], /* 8kB Switchable RAM Bank */
            //0x8000..=0x9FFF => self.ram_video[index - 0x8000], /* 8kB Video RAM */

            0x4000..=0x7FFF => &self.rom_switch[index - 0x4000],
            0x0000..=0x3FFF => &self.rom_bank[index],

            /* Required due to matching on usize,
             * but gauranteed to be unreachable by
             * forming the usize from a u16 */
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

impl<'a, T> IndexMut<T> for Mem<'a>
    where T: Into<u16>
{
    fn index_mut(&mut self, index: T) -> &mut Self::Output {
        let index = index.into() as usize;
        match index {
            0xFF80..=0xFFFF => &mut self.ram_stack[index - 0xFF80], /* Internal RAM */
            0xFF4C..=0xFF7F => panic!("Accessing memory ${:#04X}: Empty but unusable for I/O", index),
            0xFF00..=0xFF4B => &mut self.io_ports[index - 0xFF00], /* I/O Ports */
            0xFEA0..=0xFEFF => panic!("Accessing memory ${:#04X}: Empty but unusable for I/O", index),
            0xFE00..=0xFE9F => &mut self.sprite_oam[index - 0xFE00], /* Sprite Attrib Memory (OAM) */

            0xE000..=0xFDFF => &mut self.ram[index - 0xA000], /* Echo of 8kB Internal RAM */
            0x8000..=0xDFFF => &mut self.ram[index - 0x8000],
            //0xC000..=0xDFFF => self.ram_internal[index - 0xC000], /* 8kB Internal RAM */
            //0xA000..=0xBFFF => self.ram_bank[index - 0xA000], /* 8kB Switchable RAM Bank */
            //0x8000..=0x9FFF => self.ram_video[index - 0x8000], /* 8kB Video RAM */

            0x0000..=0x7FFF => panic!("Modifying ROM memory ${:#04X}", index), /* 32kB ROM */

            /* Required due to matching on usize,
             * but gauranteed to be unreachable by
             * forming the usize from a u16 */
            _ => unreachable!(),
        }
    }
}

impl<'a> Mem<'a> {
    pub fn new(cart: Cart) -> Self {
        let rom_bank = unsafe { std::slice::from_raw_parts(cart.data.as_ptr(), 0x4000) };
        let rom_switch = unsafe { std::slice::from_raw_parts(cart.data.as_ptr(), 0x4000) };

        Self {
            cart,
            rom_bank,
            rom_switch,
            ram:          [0; 0x6000],
            sprite_oam:   [0; 0x00A0],
            io_ports:     [0; 0x004C],
            ram_stack:    [0; 0x0080],
        }
    }

    #[inline(always)]
    pub fn get_u8<T>(&self, index: T) -> u8 where T: Into<u16> {
        self[index]
    }

    pub fn get_u16<T>(&self, index: T) -> u16 where T: Into<u16> {
        let index = index.into();
        let low = self.get_u8(index) as u16;
        low | ((self.get_u8(index + 1) as u16) << 8)
    }

    #[inline(always)]
    pub fn set_u8<T>(&mut self, index: T, value: u8) where T: Into<u16> {
        self[index] = value;
    }

    pub fn set_u16<T>(&mut self, index: T, value: u16) where T: Into<u16> {
        let index = index.into();
        self.set_u8(index, (value & 0x00ff) as u8);
        self.set_u8(index + 1, (value >> 8) as u8);
    }

    pub fn switch_rom_bank(&mut self, bank: usize) {
        todo!("Switch ROM bank")
    }

    pub fn switch_ram_bank(&mut self, bank: usize) {
        todo!("Switch RAM bank")
    }
}
