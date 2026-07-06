use crate::cartridge::{Bank, Mapper};

/// INES designation for CNROM boards
/// https://www.nesdev.org/wiki/CNROM
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mapper003 {
    bank_number: u8,
}

#[typetag::serde]
impl Mapper for Mapper003 {
    fn cpu_write(&mut self, address: u16, value: u8) {
        if let 0x8000..=0xffff = address {
            self.bank_number = value;
        }
    }

    fn map_chr_rom(&self, address: u16) -> Option<Bank> {
        match address {
            0x0000..=0x1fff => Some(Bank::Number(self.bank_number)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod test {
    use crate::cartridge::mapper::test::create_test_catridge;

    #[test]
    fn test() {
        let mut cartridge = create_test_catridge(3, &[&[2]], &[&[1], &[2]]);

        assert_eq!(cartridge.cpu_read(0x8000), Some(2));

        assert_eq!(cartridge.ppu_read(0x0000), Some(1));

        cartridge.cpu_write(0x8000, 1);
        assert_eq!(cartridge.ppu_read(0x0000), Some(2));
    }
}
