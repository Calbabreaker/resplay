use crate::cartridge::{Bank, KbUnit, Mirroring};

mod mapper000;
mod mapper001;
mod mapper002;
mod mapper003;
mod mapper004;
mod mapper007;

/// Generic trait for underlying circuitry inside a catridge that maps to specific ROM and RAM banks
/// in a cartridge
#[typetag::serde]
#[allow(unused_variables)]
pub trait Mapper: std::fmt::Debug {
    fn cpu_write(&mut self, address: u16, value: u8) {}

    /// Static size of a bank return from map_cpu_read
    #[inline(always)]
    fn prg_rom_bank_size(&self) -> KbUnit {
        KbUnit::ThirtyTwo
    }

    /// Maps entire range by default
    fn map_prg_rom(&self, address: u16) -> Option<Bank> {
        if let 0x8000..=0xffff = address {
            Some(Bank::Number(0))
        } else {
            None
        }
    }

    /// Most mappers have unmapped memory between 0x6000 and 0x7fff
    fn map_prg_ram(&self, address: u16) -> Option<Bank> {
        if let 0x6000..=0x7fff = address {
            Some(Bank::Number(0))
        } else {
            None
        }
    }

    /// Called whenever the PPU reads or writes to any address in its range
    fn monitor_ppu(&mut self, address: u16) {}

    /// Static size of a bank return from chr_bank_size
    #[inline(always)]
    fn chr_bank_size(&self) -> KbUnit {
        KbUnit::Eight
    }

    /// Maps the range from 0x0000 to 0x1fff
    fn map_chr(&self, address: u16) -> Bank {
        Bank::Number(0)
    }

    #[inline(always)]
    fn chr_bank_index(&self, address: u16) -> u16 {
        // And out high bit in case of prg address
        (address & 0x8000) / self.prg_rom_bank_size() as u16
    }

    /// Used to send irq to cpu
    fn irq_status(&self) -> bool {
        false
    }

    /// Option to override mirroring from header
    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
}

pub fn create_mapper(id: u16) -> Option<Box<dyn Mapper>> {
    Some(match id {
        0 => Box::new(mapper000::Mapper000::default()),
        1 => Box::new(mapper001::Mapper001::default()),
        2 => Box::new(mapper002::Mapper002::default()),
        3 => Box::new(mapper003::Mapper003::default()),
        4 => Box::new(mapper004::Mapper004::default()),
        7 => Box::new(mapper007::Mapper007::default()),
        _ => return None,
    })
}

#[cfg(test)]
mod test {
    use crate::{
        Cartridge,
        cartridge::{CartridgeHeader, create_mapper},
    };

    pub fn create_test_catridge(
        mapper_id: u16,
        prg_rom_banks_values: &[&[u8]],
        chr_rom_banks_values: &[&[u8]],
    ) -> Cartridge {
        let mapper = create_mapper(mapper_id).unwrap();
        let prg_rom = create_banks_rom(mapper.prg_rom_bank_size() as usize, prg_rom_banks_values);
        let chr_rom = create_banks_rom(mapper.chr_bank_size() as usize, chr_rom_banks_values);
        let header = CartridgeHeader {
            mapper_id,
            prg_ram_size: 1024,
            ..Default::default()
        };
        Cartridge::new(header, prg_rom, chr_rom).unwrap()
    }

    fn create_banks_rom(bank_size: usize, banks_values: &[&[u8]]) -> Vec<u8> {
        let mut rom = vec![0; bank_size * banks_values.len()];
        for (i, bank) in banks_values.iter().enumerate() {
            for (j, value) in bank.iter().enumerate() {
                rom[i * bank_size + j] = *value;
            }
        }
        rom
    }
}
