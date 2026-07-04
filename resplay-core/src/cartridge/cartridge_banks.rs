use crate::cartridge::Mapper;

/// Wrapper around a normal slice but allows for deriving Default for an arbitrary size at compile time
/// because rust devs are too pedantic https://github.com/rust-lang/rust/issues/61415
#[derive(Clone, Debug)]
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
pub enum KbUnit {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    SixTeen = 16,
    ThirtyTwo = 32,
}

#[derive(Debug, Clone, Copy)]
pub enum Bank {
    Number(u8),
    FromLast(u8),
}

#[derive(Default)]
pub struct MemoryBanks {
    bytes: Vec<u8>,
    bank_size: usize,
    num_banks: usize,
}

impl MemoryBanks {
    pub fn new(mut bytes: Vec<u8>, bank_size_kb: KbUnit) -> Self {
        let bank_size = 1024 * bank_size_kb as usize;
        let num_banks = bytes.len().div_ceil(bank_size);
        // Make sure bytes is aligned to bank size
        bytes.resize(num_banks * bank_size, 0);
        Self {
            num_banks,
            bytes,
            bank_size,
        }
    }

    pub fn write(&mut self, bank: Bank, offset: u16, value: u8) {
        if !self.bytes.is_empty() {
            let index = self.index(bank, offset);
            self.bytes[index] = value;
        }
    }

    pub fn read(&self, bank: Bank, offset: u16) -> Option<u8> {
        if !self.bytes.is_empty() {
            Some(self.bytes[self.index(bank, offset)])
        } else {
            None
        }
    }

    /// Get the index into the inner vec based on the banks and offset
    /// Offset will be wrapped around bank_size
    fn index(&self, bank: Bank, offset: u16) -> usize {
        let bank_number = match bank {
            Bank::Number(number) => number as usize,
            Bank::FromLast(from_last) => self.num_banks - from_last as usize - 1,
        };
        let bank_start = self.bank_size * (bank_number % self.num_banks);
        let bank_offset = offset as usize & (self.bank_size - 1);
        bank_start + bank_offset
    }
}

pub struct CartridgeBanks {
    pub prg_ram: MemoryBanks,
    pub prg_rom: MemoryBanks,
    pub chr_mem: MemoryBanks,
}

impl CartridgeBanks {
    pub fn new(prg_ram: Vec<u8>, prg_rom: Vec<u8>, chr_mem: Vec<u8>, mapper: &dyn Mapper) -> Self {
        Self {
            prg_ram: MemoryBanks::new(prg_ram, KbUnit::Eight),
            prg_rom: MemoryBanks::new(prg_rom, mapper.prg_bank_size()),
            chr_mem: MemoryBanks::new(chr_mem, mapper.chr_bank_size()),
        }
    }
}
