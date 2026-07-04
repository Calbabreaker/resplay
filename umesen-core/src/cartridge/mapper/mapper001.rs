use crate::cartridge::{Bank, KbUnit, Mapper, Mirroring};

/// Shift bit to 0b0010_0000
const SHIFT_CHECK_BIT_POS: u8 = 5;

/// INES designation for MMC1 boards
/// https://www.nesdev.org/wiki/MMC1
#[derive(Debug, Default)]
pub struct Mapper001 {
    shift_register: u8,
    control_register: u8,
    prg_bank_number: u8,
    chr_bank_number_0: u8,
    chr_bank_number_1: u8,
}

impl Mapper001 {
    fn write_load_register(&mut self, address: u16, value: u8) {
        if value & 0b1000_0000 != 0 {
            // Keep a one there so we can just check bit 0 to check if it has been shifted 5 times
            self.shift_register = 1 << SHIFT_CHECK_BIT_POS;
            // Swith to bank mode 3
            self.control_register |= 0b01100;
        } else {
            // Shift the new bit just before the old bit
            self.shift_register >>= 1;
            self.shift_register |= (value & 0b1) << SHIFT_CHECK_BIT_POS;
            if self.shift_register & 0b1 != 0 {
                self.shift_register >>= 1;
                match address {
                    0x8000..=0x9fff => self.control_register = self.shift_register,
                    0xa000..=0xbfff => self.chr_bank_number_0 = self.shift_register,
                    0xc000..=0xdfff => self.chr_bank_number_1 = self.shift_register,
                    0xe000..=0xffff => self.prg_bank_number = self.shift_register,
                    _ => unreachable!(),
                }
                self.shift_register = 1 << SHIFT_CHECK_BIT_POS;
            }
        }
    }
}

impl Mapper for Mapper001 {
    fn prg_bank_size(&self) -> KbUnit {
        KbUnit::SixTeen
    }

    fn map_cpu_read(&self, address: u16) -> Option<Bank> {
        let (bank_8000, bank_c000) = match self.control_register & 0b01100 {
            // 32 kib mode
            0b00000 | 0b00100 => half_double_bank(self.prg_bank_number),
            // 16 kib, 8000 = first bank
            0b01000 => ((Bank::Number(0)), (Bank::Number(self.prg_bank_number))),
            // 16 kib, c000 = last bank
            0b01100 => (Bank::Number(self.prg_bank_number), (Bank::FromLast(0))),
            _ => unreachable!(),
        };

        Some(match address {
            0x8000..=0xbfff => bank_8000,
            0xc000..=0xffff => bank_c000,
            _ => return None,
        })
    }

    fn cpu_write(&mut self, address: u16, value: u8) {
        if let 0x8000..=0xffff = address {
            self.write_load_register(address, value);
        }
    }

    fn chr_bank_size(&self) -> KbUnit {
        KbUnit::Four
    }

    fn map_ppu(&self, address: u16) -> Bank {
        let (bank_0000, bank_1000) = match self.control_register & 0b10000 {
            // 8 Kib mode
            0b00000 => half_double_bank(self.chr_bank_number_0),
            // 4 Kib split mode
            0b10000 => (
                Bank::Number(self.chr_bank_number_0),
                Bank::Number(self.chr_bank_number_1),
            ),
            _ => unreachable!(),
        };

        match address {
            0x0000..=0x0fff => bank_0000,
            0x1000..=0x1fff => bank_1000,
            _ => unreachable!(),
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(match self.control_register & 0b11 {
            0 => Mirroring::SingleScreenLow,
            1 => Mirroring::SingleScreenHigh,
            2 => Mirroring::Vertical,
            3 => Mirroring::Horizontal,
            _ => unreachable!(),
        })
    }
}

fn half_double_bank(bank_number: u8) -> (Bank, Bank) {
    (
        Bank::Number(bank_number & !1),
        Bank::Number((bank_number & !1) + 1),
    )
}

#[cfg(test)]
mod test {
    use crate::{
        Cartridge,
        cartridge::{Mirroring, mapper::test::create_test_catridge},
    };

    fn write_register(catridge: &mut Cartridge, address: u16, bits: u8) {
        for i in 0..5 {
            catridge.cpu_write(address, bits >> i);
        }
    }

    fn setup_catridge() -> Cartridge {
        create_test_catridge(1, &[&[1, 69], &[2], &[3], &[4]], &[&[6], &[7], &[8]])
    }

    #[test]
    fn mirroring() {
        let mut catridge = setup_catridge();
        write_register(&mut catridge, 0x8000, 0b00000);
        assert_eq!(catridge.mirroring(), Mirroring::SingleScreenLow);
        write_register(&mut catridge, 0x8000, 0b00001);
        assert_eq!(catridge.mirroring(), Mirroring::SingleScreenHigh);
        write_register(&mut catridge, 0x8000, 0b00010);
        assert_eq!(catridge.mirroring(), Mirroring::Vertical);
        write_register(&mut catridge, 0x8000, 0b00011);
        assert_eq!(catridge.mirroring(), Mirroring::Horizontal);
    }

    #[test]
    fn shift_and_control_register() {
        let mut catridge = setup_catridge();
        assert_eq!(catridge.cpu_read(0xc000), Some(4));

        // Write control register
        catridge.cpu_write(0x9999, 0b0001_1001);
        catridge.cpu_write(0x8000, 0b0001_0010);
        catridge.cpu_write(0x8000, 0b0000_0000);
        catridge.cpu_write(0xcccc, 0b0000_0000);
        catridge.cpu_write(0x8000, 0b0000_0000);
        assert_eq!(catridge.mirroring(), Mirroring::SingleScreenHigh);
        assert_eq!(catridge.cpu_read(0xc000), Some(2));

        catridge.cpu_write(0x8000, 0b0000_0001);
        catridge.cpu_write(0x8000, 0b0000_0001);
        catridge.cpu_write(0x8000, 0b1000_0000); // Reset
        assert_eq!(catridge.cpu_read(0xc000), Some(4));
        catridge.cpu_write(0xcccc, 0b0000_0000);
        catridge.cpu_write(0x8000, 0b0000_0000);
        catridge.cpu_write(0x8000, 0b0000_0000);
        assert_eq!(catridge.mirroring(), Mirroring::SingleScreenHigh);
    }

    #[test]
    fn prg_banks() {
        let mut catridge = setup_catridge();
        // 0xc000 last bank
        write_register(&mut catridge, 0x8000, 0b11111);
        assert_eq!(catridge.cpu_read(0x8000), Some(1));
        write_register(&mut catridge, 0xe000, 0b00010);
        assert_eq!(catridge.cpu_read(0x8000), Some(3));
        assert_eq!(catridge.cpu_read(0xc000), Some(4));

        // 0x8000 first bank
        write_register(&mut catridge, 0x8000, 0b01000);
        assert_eq!(catridge.cpu_read(0x8000), Some(1));
        assert_eq!(catridge.cpu_read(0xc000), Some(3));

        // 32 KB mode
        write_register(&mut catridge, 0x8000, 0b00100);
        assert_eq!(catridge.cpu_read(0x8000), Some(3));
        assert_eq!(catridge.cpu_read(0xc000), Some(4));
        write_register(&mut catridge, 0xe000, 0b00001);
        assert_eq!(catridge.cpu_read(0x8000), Some(1));
        assert_eq!(catridge.cpu_read(0x8001), Some(69));
        assert_eq!(catridge.cpu_read(0xc000), Some(2));
    }

    #[test]
    fn chr_banks() {
        let mut catridge = setup_catridge();
        write_register(&mut catridge, 0x8000, 0b11111);
        write_register(&mut catridge, 0xa000, 0b00001);
        write_register(&mut catridge, 0xc000, 0b00010);
        assert_eq!(catridge.ppu_read(0x0000), Some(7));
        assert_eq!(catridge.ppu_read(0x1000), Some(8));

        write_register(&mut catridge, 0x8000, 0b00000);
        write_register(&mut catridge, 0xa000, 0b00001);
        assert_eq!(catridge.ppu_read(0x0000), Some(6));
    }
}
