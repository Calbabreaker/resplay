use crate::cartridge::{Bank, KbUnit, Mapper};

/// INES designation for NROM boards
/// https://www.nesdev.org/wiki/NROM
#[derive(Default, Debug)]
pub struct Mapper000 {}

impl Mapper for Mapper000 {
    fn prg_bank_size(&self) -> KbUnit {
        KbUnit::SixTeen
    }

    fn map_cpu_read(&self, address: u16) -> Option<Bank> {
        match address {
            0x8000..=0xbfff => Some(Bank::Number(0)),
            0xc000..=0xffff => Some(Bank::Number(1)),
            _ => None,
        }
    }

    fn cpu_write(&mut self, _: u16, _: u8) {}

    fn chr_bank_size(&self) -> KbUnit {
        KbUnit::Eight
    }

    fn map_ppu(&self, _: u16) -> Bank {
        Bank::Number(0)
    }
}

#[cfg(test)]
mod test {
    use crate::cartridge::mapper::test::create_test_catridge;

    #[test]
    fn prg_ram() {
        let mut cartridge = create_test_catridge(0, 16, &[], 8, &[]);
        cartridge.cpu_write(0x6000, 2);
        assert_eq!(cartridge.cpu_read(0x6000), Some(2));
    }

    #[test]
    fn test() {
        let mut cartridge = create_test_catridge(0, 16, &[&[1, 2, 3]], 8, &[&[69]]);
        assert_eq!(cartridge.cpu_read(0x8000), Some(1));
        assert_eq!(cartridge.cpu_read(0x8002), Some(3));
        assert_eq!(cartridge.cpu_read(0xc002), Some(3));

        assert_eq!(cartridge.ppu_read(0x0000), Some(69));
    }
}
