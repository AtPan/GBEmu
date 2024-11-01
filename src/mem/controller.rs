use super::cart::types::CartType;

pub enum Controller {
    MBC1,
}

impl From<CartType> for Controller {
    fn from(value: CartType) -> Self {
        use CartType::*;
        match value {
            RomMbc1 | RomMbc1Ram | RomMbc1RamBatt => Self::MBC1,
            _ => panic!("Unsupported CartType: {:?}", value),
        }
    }
}
