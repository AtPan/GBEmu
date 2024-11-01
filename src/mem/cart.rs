use std::{cell::RefCell, error::Error, fs::File, io::Read};

pub use std::io::ErrorKind;

use self::types::CartHeader;

static NINTENDO_GRAPHIC: [u8; 48] = [
    0xCE, 0xED, 0x66, 0x66, 0xCC, 0x0D, 0x00, 0x0B, 
    0x03, 0x73, 0x00, 0x83, 0x00, 0x0C, 0x00, 0x0D, 
    0x00, 0x08, 0x11, 0x1F, 0x88, 0x89, 0x00, 0x0E, 
    0xDC, 0xCC, 0x6E, 0xE6, 0xDD, 0xDD, 0xD9, 0x99,
    0xBB, 0xBB, 0x67, 0x63, 0x6E, 0x0E, 0xEC, 0xCC, 
    0xDD, 0xDC, 0x99, 0x9F, 0xBB, 0xB9, 0x33, 0x3E];

// mod types {{{
pub mod types {
    use std::mem::MaybeUninit;

    use super::NINTENDO_GRAPHIC;

    pub struct CartHeader {
        pub entry_point: [u8; 4],
        pub nintendo_graphic: &'static [u8],
        pub title: [u8; 16],
        pub color_type: CartColorType,
        pub licensee: u16,
        pub console_indicator: ConsoleIndicator,
        pub cart_type: CartType, 
        pub rom_size: RomSize,
        pub ram_size: RamSize,
        pub destination_code: DestinationCode,
        pub old_licensee_code: OldLicenseeCode, 
        pub mask_rom_version: u8,
        pub compliment_check: u8,
        pub checksum: u16,
    }

    impl CartHeader {
        pub fn new(data: &[u8]) -> Self {
            let mut s = Self {
                entry_point: [0; 4],
                nintendo_graphic: &NINTENDO_GRAPHIC,
                title: [0; 16],
                color_type: CartColorType::from(data[0x143]),
                licensee: ((data[0x144] as u16) << 8) | data[0x145] as u16,
                console_indicator: ConsoleIndicator::from(data[0x146]),
                cart_type: CartType::from(data[0x147]),
                rom_size: RomSize::from(data[0x148]),
                ram_size: RamSize::from(data[0x149]),
                destination_code: DestinationCode::from(data[0x14A]),
                old_licensee_code: OldLicenseeCode::from(data[0x14B]),
                mask_rom_version: data[0x14C],
                compliment_check: data[0x14D],
                checksum: ((data[0x14E] as u16) << 8) | data[0x14F] as u16,
            };
            s.entry_point.clone_from_slice(&data[0x100..=0x104]);
            s.entry_point.clone_from_slice(&data[0x134..=0x142]);
            s
        }
    }

    pub enum CartColorType {
        GameBoyColor,
        Other,
    }

    impl From<u8> for CartColorType {
        fn from(value: u8) -> Self {
            match value {
                0x80 => Self::GameBoyColor,
                _ => Self::Other,
            }
        }
    }

    pub enum ConsoleIndicator {
        GameBoy,
        SuperGameBoy,
    }

    impl From<u8> for ConsoleIndicator {
        fn from(value: u8) -> Self {
            match value {
                0x00 => Self::GameBoy,
                0x03 => Self::SuperGameBoy,
                _ => panic!("Unrecognized Console Indicator byte `${:#02X}` at position `$0146`", value),
            }
        }
    }

    #[derive(Debug)]
    pub enum CartType {
        RomOnly,
        RomMbc1,
        RomMbc1Ram,
        RomMbc1RamBatt,
        RomMbc2,
        RomMbc2Batt,
        RomRam,
        RomRamBatt,
        RomMmmo1,
        RomMmmo1Sram,
        RomMmmo1SramBatt,
        RomMbc3TimerBatt,
        RomMbc3TimerRamBatt,
        RomMbc3,
        RomMbc3Ram,
        RomMbc3RamBatt,
        RomMbc5,
        RomMbc5Ram,
        RomMbc5RamBatt,
        RomMbc5Rumble,
        RomMbc5RumbleSram,
        RomMbc5RumbleSramBatt,
        PocketCamera,
        BandaiTama5,
        HudsonHuC3,
        HudsonHuC1,
    }

    impl From<u8> for CartType {
        fn from(value: u8) -> Self {
            match value {
                0x01 => Self::RomOnly,
                0x02 => Self::RomMbc1,
                0x03 => Self::RomMbc1Ram,
                0x05 => Self::RomMbc1RamBatt,
                0x06 => Self::RomMbc2,
                0x08 => Self::RomMbc2Batt,
                0x09 => Self::RomRam,
                0x0B => Self::RomRamBatt,
                0x0C => Self::RomMmmo1,
                0x0D => Self::RomMmmo1Sram,
                0x0F => Self::RomMmmo1SramBatt,
                0x10 => Self::RomMbc3TimerBatt,
                0x11 => Self::RomMbc3TimerRamBatt,
                0x12 => Self::RomMbc3,
                0x13 => Self::RomMbc3Ram,
                0x19 => Self::RomMbc3RamBatt,
                0x1A => Self::RomMbc5,
                0x1B => Self::RomMbc5Ram,
                0x1C => Self::RomMbc5RamBatt,
                0x1D => Self::RomMbc5Rumble,
                0x1E => Self::RomMbc5RumbleSram,
                0x1F => Self::RomMbc5RumbleSramBatt,
                0xFD => panic!("Cartridge Type `$FD` not supported"),
                0xFE => panic!("Cartridge Type `$FE` not supported"),
                0xFF => panic!("Cartridge Type `$FF` not supported"),
                _ => panic!("Unrecognized Cartridge Type `${:#02X}` at position `$0147`", value),
            }
        }
    }

    pub struct RomSize(u32);

    impl From<u8> for RomSize {
        fn from(value: u8) -> Self {
            match value {
                0..=6 => Self(1 << (5 + value)),
                0x52..=0x54 => panic!("Unsupported ROM Size: `${:#02X}`", value),
                _ => panic!("Unrecognized ROM Size: `${:#02X}`", value),
            }
        }
    }

    pub struct RamSize(u32);

    impl From<u8> for RamSize {
        fn from(value: u8) -> Self {
            match value {
                0 => Self(0),
                1 => Self(2),
                2 => Self(8),
                3 => Self(32),
                4 => Self(128),
                _ => panic!("Unrecognized RAM Size: `${:02X}`", value),
            }
        }
    }

    pub enum DestinationCode {
        Japanese,
        NonJapanese,
    }

    impl From<u8> for DestinationCode {
        fn from(value: u8) -> Self {
            match value {
                0 => Self::Japanese,
                1 => Self::NonJapanese,
                _ => panic!("Expected Destination Code at `$014A` to be `$00` or `$01`, instead found `${:#02X}`", value),
            }
        }
    }

    pub enum OldLicenseeCode {
        CheckLicenseeCode,
        Accolade,
        Konami,
    }

    impl From<u8> for OldLicenseeCode {
        fn from(value: u8) -> Self {
            match value {
                0x33 => Self::CheckLicenseeCode,
                0x79 => Self::Accolade,
                0xA4 => Self::Konami,
                _ => panic!("Unrecognized Old Licensee Code at `$014B`: `${:02X}`", value),
            }
        }
    }
}
//}}}

pub struct Cart {
    pub data: Vec<u8>,
    pub data_len: usize,
    pub header: CartHeader,
}

impl Cart {
    pub fn new(name: String) -> Result<Self, ErrorKind> {
        let mut fs = match File::open(name) {
            Ok(fs) => fs,
            Err(_) => return Err(ErrorKind::PermissionDenied),
        };
        let mut data: Vec<u8> = Vec::new();
        if fs.read_to_end(&mut data).is_err() {
            return Err(ErrorKind::PermissionDenied);
        }
        let data_len = data.len();
        let header = CartHeader::new(&data);

        Ok(Self {
            data, data_len, header, 
        })
    }
}
