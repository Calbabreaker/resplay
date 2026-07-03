use crate::cartridge::{Bank, KbUnit, Mapper};

/// INES designation for UxROM boards
/// https://www.nesdev.org/wiki/UxROM
#[derive(Default, Debug)]
pub struct Mapper002 {
    bank_number_low: u8,
}

impl Mapper for Mapper002 {
    fn prg_bank_size(&self) -> KbUnit {
        KbUnit::SixTeen
    }

    fn map_cpu_read(&self, address: u16) -> Option<Bank> {
        Some(match address {
            0x8000..=0xbfff => Bank::Number(self.bank_number_low),
            0xc000..=0xffff => Bank::FromLast(0),
            _ => return None,
        })
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if let 0x8000..=0xffff = address {
            self.bank_number_low = value;
        }
    }

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
    fn test() {
        let mut cartridge = create_test_catridge(2, &[&[1], &[2], &[3]], &[&[2]]);
        assert_eq!(cartridge.ppu_read(0x0000), Some(2));

        assert_eq!(cartridge.cpu_read(0x8000), Some(1));
        assert_eq!(cartridge.cpu_read(0xc000), Some(3));

        cartridge.cpu_write(0x8000, 1);
        assert_eq!(cartridge.cpu_read(0x8000), Some(2));
        assert_eq!(cartridge.cpu_read(0xc000), Some(3));
    }
}
