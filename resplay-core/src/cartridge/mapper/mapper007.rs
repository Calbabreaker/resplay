use crate::cartridge::{Bank, KbUnit, Mapper, Mirroring};

/// INES designation for AxROM boards
/// https://www.nesdev.org/wiki/AxROM
#[derive(Default, Debug, serde::Serialize, serde::Deserialize)]
pub struct Mapper007 {
    bank_select: u8,
}

#[typetag::serde]
impl Mapper for Mapper007 {
    fn prg_bank_size(&self) -> KbUnit {
        KbUnit::ThirtyTwo
    }

    fn map_cpu_read(&self, address: u16) -> Option<Bank> {
        Some(match address {
            0x8000..=0xffff => Bank::Number(self.bank_select & 0b0000_0111),
            _ => return None,
        })
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if let 0x8000..=0xffff = address {
            self.bank_select = value;
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(if self.bank_select & 0b0001_0000 == 0 {
            Mirroring::SingleScreenLow
        } else {
            Mirroring::SingleScreenHigh
        })
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
    use crate::{
        Cartridge,
        cartridge::{Mirroring, mapper::test::create_test_catridge},
    };

    fn setup_catridge() -> Cartridge {
        create_test_catridge(7, &[&[1], &[2], &[3]], &[&[2]])
    }

    #[test]
    fn test() {
        let mut cartridge = setup_catridge();
        assert_eq!(cartridge.ppu_read(0x0000), Some(2));

        assert_eq!(cartridge.cpu_read(0x8000), Some(1));
        assert_eq!(cartridge.cpu_read(0xc000), Some(0));

        cartridge.cpu_write(0x8000, 0xf1);
        assert_eq!(cartridge.cpu_read(0x8000), Some(2));
        assert_eq!(cartridge.cpu_read(0xc000), Some(0));
    }

    #[test]
    fn mirroring() {
        let mut cartridge = setup_catridge();
        cartridge.cpu_write(0x8000, 0b0001_0000);
        assert_eq!(cartridge.mirroring(), Mirroring::SingleScreenHigh);
        cartridge.cpu_write(0x8000, 0b0000_0000);
        assert_eq!(cartridge.mirroring(), Mirroring::SingleScreenLow);
    }
}
