use crate::{
    Cartridge,
    cartridge::{FixedArray, Mirroring},
};

const PALETTE_RAM_SIZE: usize = 0x20;
pub const PALETTE_START: u16 = 0x3f00;
/// Size of one pattern table in number of tiles, add this to tile number to access the next pattern table
pub const PATTERN_TILE_COUNT: u16 = 256;

pub const TILE_SIZE: u16 = 8;
pub const NAMETABLE_SIZE_X: u16 = 32;
pub const NAMETABLE_SIZE_Y: u16 = 30;

#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PpuBus {
    pub palette_ram: FixedArray<u8, PALETTE_RAM_SIZE>,
    pub nametable_ram: FixedArray<u8, 0x800>,
    pub(crate) cartridge: Option<Cartridge>,
}

impl PpuBus {
    pub fn peek_read(&self, address: u16) -> u8 {
        std::debug_assert_matches!(address, 0x0000..=0x3fff);
        if let Some(cart) = self.cartridge.as_ref()
            && let Some(value) = cart.ppu_peek_read(address)
        {
            return value;
        }

        match address {
            0x2000..=0x3eff => self.nametable_ram[self.mirror_nametable(address)],
            // Palette byte is only 6 bit
            PALETTE_START..=0x3fff => self.palette_ram[mirror_palette(address)],
            _ => 0,
        }
    }

    pub fn read(&mut self, address: u16) -> u8 {
        if let Some(value) = self.cartridge.as_mut().and_then(|c| c.ppu_read(address)) {
            value
        } else {
            self.peek_read(address)
        }
    }

    pub fn write(&mut self, address: u16, value: u8) {
        std::debug_assert_matches!(address, 0x0000..=0x3fff);
        if let Some(cartridge) = self.cartridge.as_mut() {
            cartridge.ppu_write(address, value);
        }

        match address {
            0x2000..=0x3eff => {
                let address = self.mirror_nametable(address);
                self.nametable_ram[address] = value;
            }
            PALETTE_START..=0x3fff => self.palette_ram[mirror_palette(address)] = value,
            _ => (),
        }
    }

    fn mirror_nametable(&self, address: u16) -> usize {
        let cart = self.cartridge.as_ref();
        mirror_nametable(address, cart.map(|c| c.mirroring()).unwrap_or_default())
    }
}

/// Maps the address into nametable ram index
fn mirror_nametable(address: u16, mirroring: Mirroring) -> usize {
    let table_address = address & 0x0fff;

    let table_number = match mirroring {
        Mirroring::Horizontal => match table_address {
            0x000..=0x7ff => 0,
            0x800.. => 1,
        },
        Mirroring::Vertical => match table_address {
            0x000..=0x3ff => 0,
            0x400..=0x7ff => 1,
            0x800..=0xbff => 0,
            0xc00.. => 1,
        },
        Mirroring::SingleScreenLow => 0,
        Mirroring::SingleScreenHigh => 1,
        Mirroring::FourScreen => todo!(),
    };
    (table_number * 0x400 + (address % 0x400)) as usize
}

fn mirror_palette(address: u16) -> usize {
    let address = address as usize % PALETTE_RAM_SIZE;
    match address {
        // Mirror sprite palettes to background
        0x10 | 0x14 | 0x18 | 0x1c => address - 0x10,
        _ => address,
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn mirroring() {
        use Mirroring::*;
        assert_eq!(mirror_nametable(0x2020, Vertical), 0x0020);
        assert_eq!(mirror_nametable(0x2420, Vertical), 0x0420);
        assert_eq!(mirror_nametable(0x2820, Vertical), 0x0020);
        assert_eq!(mirror_nametable(0x2c20, Vertical), 0x0420);

        assert_eq!(mirror_nametable(0x3020, Horizontal), 0x0020);
        assert_eq!(mirror_nametable(0x3420, Horizontal), 0x0020);
        assert_eq!(mirror_nametable(0x3820, Horizontal), 0x0420);
        assert_eq!(mirror_nametable(0x3c20, Horizontal), 0x0420);

        assert_eq!(mirror_nametable(0x2020, SingleScreenLow), 0x0020);
        assert_eq!(mirror_nametable(0x2420, SingleScreenLow), 0x0020);
        assert_eq!(mirror_nametable(0x2820, SingleScreenLow), 0x0020);
        assert_eq!(mirror_nametable(0x2c20, SingleScreenLow), 0x0020);

        assert_eq!(mirror_nametable(0x2020, SingleScreenHigh), 0x0420);
        assert_eq!(mirror_nametable(0x2420, SingleScreenHigh), 0x0420);
        assert_eq!(mirror_nametable(0x2820, SingleScreenHigh), 0x0420);
        assert_eq!(mirror_nametable(0x2c20, SingleScreenHigh), 0x0420);
    }

    #[test]
    fn palette() {
        assert_eq!(mirror_palette(0x3f00), 0);
        assert_eq!(mirror_palette(0x3f01), 1);
        assert_eq!(mirror_palette(0x3f10), 0);
        assert_eq!(mirror_palette(0x3f11), 17);
    }
}
