#[derive(thiserror::Error, Debug)]
pub enum NesParseError {
    #[error("Magic number '{0}' in header is not a valid NES header")]
    InvalidMagicNumber(String),
    #[error("Mapper id '{0}' is not supported")]
    UnsupportedMapper(u16),
    #[error(transparent)]
    IoError(#[from] std::io::Error),
}

#[derive(Default, Debug, PartialEq, Clone, Copy)]
pub enum Mirroring {
    /// A A
    /// B B
    #[default]
    Horizontal,
    /// A B
    /// A B
    Vertical,
    /// A A
    /// A A
    SingleScreenLow,
    /// B B
    /// B B
    SingleScreenHigh,
    /// A B
    /// C D
    FourScreen,
}

#[derive(Default, Debug, PartialEq, Clone)]
pub struct CartridgeHeader {
    pub mapper_id: u16,
    pub submapper_id: u8,
    pub is_v2: bool,
    pub has_trainer: bool,
    pub mirroring: Mirroring,
    pub has_battery: bool,
    pub prg_rom_size: usize,
    pub prg_ram_size: usize,
    pub chr_mem_size: usize,
    pub chr_mem_is_rom: bool,
}

impl CartridgeHeader {
    pub const TRAINER_SIZE: usize = 512;

    pub fn from_nes(data: [u8; 16]) -> Result<Self, NesParseError> {
        if &data[0..4] != b"NES\x1a" {
            let magic_number = String::from_utf8_lossy(&data[0..4]).to_string();
            return Err(NesParseError::InvalidMagicNumber(magic_number));
        }

        let has_battery = data[6] & 0b0000_0010 != 0;
        let is_v2 = data[7] & 0b0000_1100 == 0b0000_1000;
        let mut mapper_id = ((data[6] >> 4) | (data[7] & 0xf0)) as u16;

        let prg_ram_size = if is_v2 {
            get_ram_size(data[10], has_battery)
        } else {
            let units = data[8].max(1);
            (units as usize) * 8 * 1024
        };

        let chr_ram_size = if is_v2 {
            get_ram_size(data[11], false)
        } else {
            8 * 1024
        };

        let mut prg_rom_size = (data[4] as usize) * 16 * 1024;
        let mut chr_rom_size = (data[5] as usize) * 8 * 1024;

        if is_v2 {
            prg_rom_size |= (data[9] as usize & 0x0f) << 8;
            chr_rom_size |= (data[9] as usize & 0xf0) << 4;
            mapper_id |= (data[8] as u16 & 0x0f) << 8;
        }

        if data[12] & 0b01 != 0 {
            log::warn!("Detected PAL ROM, this emulator only supports NTSC");
        }

        Ok(Self {
            prg_rom_size,
            mapper_id,
            submapper_id: data[8] & 0xf0 >> 4,
            mirroring: if data[6] & 0b000_1000 != 0 {
                // Note: this bit could mean a different mirrorings in some mappers?
                Mirroring::FourScreen
            } else if data[6] & 0b000_0001 != 0 {
                Mirroring::Vertical
            } else {
                Mirroring::Horizontal
            },
            has_battery,
            has_trainer: data[6] & 0b0000_0100 != 0,
            chr_mem_size: if chr_rom_size == 0 {
                chr_ram_size
            } else {
                chr_rom_size
            },
            chr_mem_is_rom: chr_rom_size != 0,
            prg_ram_size,
            is_v2,
        })
    }
}

fn get_ram_size(byte: u8, has_battery: bool) -> usize {
    let shift_count = if byte == 0 {
        return 0;
    } else if has_battery {
        (byte & 0xf0) >> 4
    } else {
        byte & 0x0f
    };
    64 << shift_count as usize
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_correctly() {
        let header = CartridgeHeader::from_nes([
            0x4E, 0x45, 0x53, 0x1A, 0x02, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ])
        .unwrap();

        assert_eq!(
            header,
            CartridgeHeader {
                submapper_id: 0,
                has_battery: false,
                mapper_id: 3,
                mirroring: Mirroring::Vertical,
                prg_rom_size: 32 * 1024,
                chr_mem_size: 8 * 1024,
                prg_ram_size: 8 * 1024,
                has_trainer: false,
                chr_mem_is_rom: true,
                is_v2: false
            }
        )
    }
}
