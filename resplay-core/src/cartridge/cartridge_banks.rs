use crate::cartridge::Mapper;

/// Wrapper around a normal slice but allows for deriving Default for an arbitrary size at compile time
/// because rust devs are too pedantic https://github.com/rust-lang/rust/issues/61415
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[repr(C)]
pub struct FixedArray<T, const C: usize>(Box<[T]>);

impl<T: Default + Clone, const C: usize> Default for FixedArray<T, C> {
    fn default() -> Self {
        Self(vec![Default::default(); C].into_boxed_slice())
    }
}

impl<T, const C: usize> std::ops::Deref for FixedArray<T, C> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T, const C: usize> std::ops::DerefMut for FixedArray<T, C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/// Enum to make sure number of kilobytes is power of 2
#[repr(usize)]
pub enum KbUnit {
    One = 1024,
    Two = 2 * 1024,
    Four = 4 * 1024,
    Eight = 8 * 1024,
    SixTeen = 16 * 1024,
    ThirtyTwo = 32 * 1024,
}

#[derive(Debug, Clone, Copy)]
pub enum Bank {
    Number(u8),
    FromLast(u8),
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct MemoryBanks {
    pub bytes: Vec<u8>,
    bank_size: usize,
    num_banks: usize,
}

impl MemoryBanks {
    pub fn new(mut bytes: Vec<u8>, bank_size_kb: KbUnit) -> Self {
        let bank_size = bank_size_kb as usize;
        let num_banks = bytes.len().div_ceil(bank_size);
        if !bytes.is_empty() && !bytes.len().is_power_of_two() {
            // Make sure bytes is aligned to a power of two so we can use bitwise and to mirror
            bytes.resize(bytes.len().next_power_of_two(), 0);
        }
        Self {
            num_banks,
            bytes,
            bank_size,
        }
    }

    pub fn write(&mut self, bank: Bank, offset: u16, value: u8) {
        let index = self.index(bank, offset);
        if let Some(byte) = self.bytes.get_mut(index) {
            *byte = value
        }
    }

    pub fn read(&self, bank: Bank, offset: u16) -> Option<u8> {
        let index = self.index(bank, offset);
        self.bytes.get(index).copied()
    }

    /// Get the index into the inner vec based on the banks and offset
    /// Offset will be wrapped around bank_size
    fn index(&self, bank: Bank, offset: u16) -> usize {
        let bank_number = match bank {
            Bank::Number(number) => number as usize,
            Bank::FromLast(from_last) => self.num_banks - from_last as usize - 1,
        };
        let bank_start = self.bank_size * bank_number;
        let bank_offset = offset as usize & (self.bank_size - 1);
        (bank_start + bank_offset) & (self.bytes.len() - 1)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct CartridgeBanks {
    pub prg_ram: MemoryBanks,
    #[serde(skip)]
    pub prg_rom: MemoryBanks,
    pub chr_ram: MemoryBanks,
    #[serde(skip)]
    pub chr_rom: MemoryBanks,
}

impl CartridgeBanks {
    pub fn new(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        prg_ram_size: usize,
        chr_ram_size: usize,
        mapper: &dyn Mapper,
    ) -> Self {
        Self {
            prg_ram: MemoryBanks::new(vec![0; prg_ram_size], KbUnit::Eight),
            prg_rom: MemoryBanks::new(prg_rom, mapper.prg_rom_bank_size()),
            chr_ram: MemoryBanks::new(vec![0; chr_ram_size], mapper.chr_bank_size()),
            chr_rom: MemoryBanks::new(chr_rom, mapper.chr_bank_size()),
        }
    }
}
