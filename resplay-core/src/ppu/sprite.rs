use crate::ppu::{Control, PATTERN_TILE_COUNT, Registers, get_pattern_tile_addresses};

bitflags::bitflags! {
    #[derive(Default, Clone, Copy, PartialEq, Eq, Debug)]
    pub struct Attributes: u8 {
        #[bitflags(flag_name = "")]
        const PALLETTE = 0b11;
        const RENDER_BEHIND = 1 << 5;
        const FLIP_HORIZONTAL = 1 << 6;
        const FLIP_VERTICAL = 1 << 7;
    }
}

impl Attributes {
    pub fn palette(&self) -> u8 {
        (*self & Attributes::PALLETTE).bits()
    }
}

#[derive(Clone, Default, Debug)]
pub struct Sprite {
    pub x: u8,
    pub y: u8,
    tile_number: u8,
    pub attributes: Attributes,
    /// OAM index to check sprite 0 hit
    pub oam_index: u8,
    color_bits_low: u8,
    color_bits_high: u8,
}

impl Sprite {
    pub fn new(oam: &[u8], oam_index: u8) -> Self {
        Self {
            x: *oam.get(3).unwrap_or(&0),
            y: *oam.first().unwrap_or(&0),
            tile_number: *oam.get(1).unwrap_or(&0),
            attributes: Attributes::from_bits_truncate(*oam.get(2).unwrap_or(&0)),
            color_bits_low: 0,
            color_bits_high: 0,
            oam_index,
        }
    }

    pub fn tile_number(&self, registers: &Registers) -> u16 {
        let table_number = if registers.control.contains(Control::TALL_SPRITES) {
            // Bit zero contains table number when tall sprites
            self.tile_number & 0b1
        } else {
            registers.control.contains(Control::SPRITE_SECOND_TABLE) as u8
        };

        let tile_number = if registers.control.contains(Control::TALL_SPRITES) {
            self.tile_number & 0b1111_1110
        } else {
            self.tile_number
        };

        tile_number as u16 | (table_number as u16 * PATTERN_TILE_COUNT)
    }

    pub(crate) fn load_shift_bits(&mut self, scanline: u16, registers: &mut Registers) {
        let mut tile_number = self.tile_number(registers);

        let mut fine_y = scanline.saturating_sub(self.y as u16);
        if self.attributes.contains(Attributes::FLIP_VERTICAL) {
            // Flip the fine y
            fine_y = (registers.control.sprite_height() as u16 - 1).wrapping_sub(fine_y);
        }

        // Go to the next tile if y is greater than tile
        if fine_y >= 8 && registers.control.contains(Control::TALL_SPRITES) {
            tile_number += 1;
        }

        let (address_lsb, address_msb) = get_pattern_tile_addresses(tile_number, fine_y);
        self.color_bits_low = registers.bus.read(address_lsb);
        self.color_bits_high = registers.bus.read(address_msb);
    }

    pub(crate) fn color_index(&self, scan_x: usize) -> u8 {
        // Calculate the x position of scan x relative to the sprite x
        let mut x = scan_x.wrapping_sub(self.x as usize);
        if x > 7 {
            return 0;
        }

        if self.attributes.contains(Attributes::FLIP_HORIZONTAL) {
            x = 7 - x;
        }

        crate::ppu::add_bit_planes(self.color_bits_low, self.color_bits_high, 0b1000_0000 >> x)
    }
}
