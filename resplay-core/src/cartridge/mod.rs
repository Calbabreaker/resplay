mod cartridge_banks;
mod cartridge_header;
mod mapper;

pub use cartridge_banks::*;
pub use cartridge_header::*;
pub use mapper::{Mapper, create_mapper};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Cartridge {
    banks: CartridgeBanks,
    header: CartridgeHeader,
    mapper: Box<dyn Mapper>,
}

impl Cartridge {
    fn new(
        header: CartridgeHeader,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
    ) -> Result<Self, NesParseError> {
        let mapper = create_mapper(header.mapper_id)
            .ok_or(NesParseError::UnsupportedMapper(header.mapper_id))?;
        Ok(Cartridge {
            banks: CartridgeBanks::new(
                prg_rom,
                chr_rom,
                header.prg_ram_size,
                header.chr_ram_size,
                mapper.as_ref(),
            ),
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
        let mut chr_rom = vec![0; header.chr_rom_size];
        bytes.read_exact(&mut chr_rom)?;

        let mut cartridge = Self::new(header, prg_rom, chr_rom)?;
        for (i, byte) in trainer_data.iter().enumerate() {
            cartridge.cpu_write((0x7000 + i) as u16, *byte);
        }
        Ok(cartridge)
    }

    pub fn cpu_read(&self, address: u16) -> Option<u8> {
        if let Some(bank) = self.mapper.map_prg_rom(address) {
            self.banks.prg_rom.read(bank, address)
        } else if let Some(bank) = self.mapper.map_prg_ram(address) {
            self.banks.prg_ram.read(bank, address)
        } else {
            None
        }
    }

    pub fn cpu_write(&mut self, address: u16, value: u8) {
        self.mapper.cpu_write(address, value);
        if let Some(bank) = self.mapper.map_prg_ram(address) {
            self.banks.prg_ram.write(bank, address, value);
        }
    }

    pub fn ppu_peek_read(&self, address: u16) -> Option<u8> {
        if let 0x0000..=0x1fff = address {
            let bank = self.mapper.map_chr(address);
            if !self.banks.chr_rom.bytes.is_empty() {
                self.banks.chr_rom.read(bank, address)
            } else {
                self.banks.chr_ram.read(bank, address)
            }
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
            let bank = self.mapper.map_chr(address);
            self.banks.chr_ram.write(bank, address, value);
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
}
