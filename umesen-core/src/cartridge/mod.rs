mod cartridge_banks;
mod cartridge_header;
mod mapper;

pub use cartridge_banks::*;
pub use cartridge_header::*;
pub use mapper::{Mapper, create_mapper};

pub struct Cartridge {
    banks: CartridgeBanks,
    header: CartridgeHeader,
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    fn new(
        header: CartridgeHeader,
        prg_ram: Vec<u8>,
        prg_rom: Vec<u8>,
        chr_mem: Vec<u8>,
    ) -> Result<Self, NesParseError> {
        let mut mapper = create_mapper(header.mapper_id)
            .ok_or(NesParseError::UnsupportedMapper(header.mapper_id))?;
        mapper.reset();
        Ok(Cartridge {
            banks: CartridgeBanks::new(prg_ram, prg_rom, chr_mem, mapper.as_ref()),
            mapper,
            header,
        })
    }

    pub fn from_nes(mut bytes: impl std::io::Read) -> Result<Self, NesParseError> {
        let mut header_data = [0; 16];
        bytes.read_exact(&mut header_data)?;
        let header = CartridgeHeader::from_nes(header_data)?;

        let mut trainer_data = vec![0; CartridgeHeader::TRAINER_SIZE];
        if header.has_trainer {
            bytes.read_exact(&mut trainer_data)?;
        }

        let mut prg_rom = vec![0; header.prg_rom_size];
        bytes.read_exact(&mut prg_rom)?;
        let mut chr_mem = vec![0; header.chr_mem_size];
        if header.chr_mem_is_rom {
            bytes.read_exact(&mut chr_mem)?;
        }

        let prg_ram = vec![0; header.prg_ram_size];
        let mut cartridge = Self::new(header, prg_ram, prg_rom, chr_mem)?;
        for (i, byte) in trainer_data.iter().enumerate() {
            cartridge.cpu_write((0x7000 + i) as u16, *byte);
        }
        Ok(cartridge)
    }

    pub fn cpu_read(&self, address: u16) -> Option<u8> {
        if let Some(mapping) = self.mapper.map_cpu_read(address) {
            self.banks.prg_rom.read(mapping, address)
        } else if let 0x6000..=0x7fff = address {
            self.banks.prg_ram.read(Bank::Number(0), address)
        } else {
            None
        }
    }

    pub fn cpu_write(&mut self, address: u16, value: u8) {
        if let 0x6000..=0x7fff = address {
            self.banks.prg_ram.write(Bank::Number(0), address, value);
        }
        self.mapper.cpu_write(address, value);
    }

    pub fn ppu_peek_read(&self, address: u16) -> Option<u8> {
        if let 0x0000..=0x1fff = address {
            let mapping = self.mapper.map_ppu(address);
            self.banks.chr_mem.read(mapping, address)
        } else {
            None
        }
    }

    pub fn ppu_read(&mut self, address: u16) -> Option<u8> {
        self.mapper.monitor_ppu(address);
        self.ppu_peek_read(address)
    }

    pub fn ppu_write(&mut self, address: u16, value: u8) {
        self.mapper.monitor_ppu(address);
        if let 0x0000..=0x1fff = address {
            let mapping = self.mapper.map_ppu(address);
            if !self.header.chr_mem_is_rom {
                self.banks.chr_mem.write(mapping, address, value);
            }
        }
    }

    pub fn irq_status(&self) -> bool {
        self.mapper.irq_status()
    }

    pub fn mirroring(&self) -> Mirroring {
        self.mapper.mirroring().unwrap_or(self.header.mirroring)
    }

    pub fn header(&self) -> &CartridgeHeader {
        &self.header
    }

    pub fn debug_mapper(&self) -> String {
        format!("{:?}", self.mapper)
    }

    pub fn reset(&mut self) {
        self.mapper.reset();
    }
}
