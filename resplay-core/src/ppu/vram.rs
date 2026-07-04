use crate::ppu::{NAMETABLE_SIZE_X, NAMETABLE_SIZE_Y, TILE_SIZE};

/// Internal 15-bit registers (t and v) used for rendering and memory access
/// These can act as a 15-bit address to access the ppu bus or a packed bitfield
/// From nesdev wiki: https://www.nesdev.org/wiki/PPU_scrolling
/// 0yyyNNYY YYYXXXXX
///  ||||||| |||+++++---- coarse X scroll
///  |||||++-+++--------- coarse Y scroll
///  |||++--------------- nametable select X and y
///  +++----------------- fine Y scroll
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VramRegister(pub u16);

#[rustfmt::skip]
impl VramRegister {
    // Select bits
    pub const COARSE_X:    u16 = 0b00000000_00011111;
    pub const COARSE_Y:    u16 = 0b00000011_11100000;
    pub const NAMETABLE:   u16 = 0b00001100_00000000;
    pub const NAMETABLE_X: u16 = 0b00000100_00000000;
    pub const NAMETABLE_Y: u16 = 0b00001000_00000000;
    pub const FINE_Y:      u16 = 0b01110000_00000000;
    pub const LOW:         u16 = 0b00000000_11111111;
    pub const HIGH:        u16 = 0b11111111_00000000;
}

impl VramRegister {
    pub fn set(&mut self, select_bits: u16, value: impl Into<u16>) {
        let value = value.into();
        // Check value fits into the bits
        debug_assert!(value <= (select_bits >> select_bits.trailing_zeros()));
        let value_shifted = value << select_bits.trailing_zeros();
        self.0 = value_shifted | (self.0 & (!select_bits));
    }

    pub fn get(&self, select_bits: u16) -> u16 {
        (self.0 & select_bits) >> select_bits.trailing_zeros()
    }

    pub fn copy_x(&mut self, other: VramRegister) {
        let select_bits = Self::COARSE_X | Self::NAMETABLE_X;
        self.set(select_bits, other.get(select_bits));
    }

    pub fn copy_y(&mut self, other: VramRegister) {
        let select_bits = Self::FINE_Y | Self::COARSE_Y | Self::NAMETABLE_Y;
        self.set(select_bits, other.get(select_bits));
    }

    /// Returns the address within the nametable portion of the ppu of which this register contains
    /// The address should contain the tile number for the pattern table
    pub fn nametable_address(&self) -> u16 {
        // Lower 12 bytes should contain the address within the nametable portion of the ppu
        // Nametable begins at 0x2000
        0x2000 | (self.0 & 0x0fff)
    }

    /// Returns the address that contains the attribute byte in the ppu of which this register contains
    pub fn attribute_address(&self) -> u16 {
        // Each attribute byte controls 4x4 tiles
        let tile_x = self.get(Self::COARSE_X) / 4;
        let tile_y = self.get(Self::COARSE_Y) / 4;
        let attribute_number = tile_y * 8 + tile_x;
        let nametable = self.0 & Self::NAMETABLE;
        // Attribute bytes begin at 0x3c0 within a nametable
        0x23c0 | nametable | attribute_number
    }

    /// Shift an attribute byte to get the palette id into based on the coarse xy
    pub fn palette_id(&self, attribute: u8) -> u8 {
        let quadrant_x = (self.get(Self::COARSE_X) % 4) / 2;
        let quadrant_y = (self.get(Self::COARSE_Y) % 4) / 2;
        let shift = (quadrant_x + quadrant_y * 2) * 2;
        (attribute >> shift) & 0b11
    }

    pub fn set_coarse_x(&mut self, coarse_x: u16) {
        if self.set_wrapped(coarse_x, Self::COARSE_X, NAMETABLE_SIZE_X) {
            self.0 ^= Self::NAMETABLE_X;
        }
    }

    pub fn set_coarse_y(&mut self, coarse_y: u16) {
        if self.set_wrapped(coarse_y, Self::COARSE_Y, NAMETABLE_SIZE_Y) {
            self.0 ^= Self::NAMETABLE_Y;
        }
    }

    pub fn scroll_coarse_x(&mut self) {
        self.set_coarse_x(self.get(Self::COARSE_X) + 1);
    }

    pub fn scroll_fine_y(&mut self) {
        if self.set_wrapped(self.get(Self::FINE_Y) + 1, Self::FINE_Y, TILE_SIZE) {
            self.set_coarse_y(self.get(Self::COARSE_Y) + 1);
        }
    }

    fn set_wrapped(&mut self, value: u16, select_bits: u16, size: u16) -> bool {
        debug_assert!(value <= (size * 2));
        self.set(select_bits, value % size);
        value >= size
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn vram_register() {
        let mut register = VramRegister::default();
        register.set(VramRegister::COARSE_X, 10u8);
        register.set(VramRegister::COARSE_Y, 15u8);
        register.set(VramRegister::NAMETABLE, 1u8);
        assert_eq!(register.get(VramRegister::COARSE_X), 10);
        assert_eq!(register.get(VramRegister::COARSE_Y), 15);
        assert_eq!(register.nametable_address(), 0x2400 + 490);
        assert_eq!(register.attribute_address(), 0x27da);
    }
}
