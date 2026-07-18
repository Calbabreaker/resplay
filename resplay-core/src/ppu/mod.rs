use crate::{cartridge::FixedArray, ppu::sprite::Attributes};

mod bus;
mod palette;
mod registers;
pub mod sprite;
mod vram;

pub use bus::*;
pub use palette::Palette;
pub use registers::*;
pub use sprite::Sprite;
pub use vram::VramRegister;

pub const WIDTH: u16 = 256;
pub const HEIGHT: u16 = 240;
pub const MAX_SPRITES_PER_SCAN: usize = 8;
pub const PRERENDER_SCANLINE: u16 = 261;

pub type ScreenPixels = FixedArray<[u8; 3], { WIDTH as usize * HEIGHT as usize }>;

pub enum PpuClockReport {
    None,
    Nmi,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, PartialEq, Debug, Default, Copy)]
#[serde(default)]
pub struct PpuConfig {
    pub unlimited_sprites: bool,
    pub hide_sprite: bool,
    pub hide_background: bool,
    pub force_grayscale: bool,
}

/// Emulated 2C02 NTSC PPU
#[derive(Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Ppu {
    #[serde(skip)]
    pub config: PpuConfig,
    #[serde(skip)]
    pub palette: Palette,
    #[serde(skip)]
    pub screen_pixels: ScreenPixels,
    #[serde(skip)]
    frame_complete: bool,

    pub registers: Registers,

    // Bits shifted left every render dot so leftmost bit contains low and high bit of the current pixel index in the palette
    bg_shift_bits_low: u16,
    bg_shift_bits_high: u16,
    bg_palette_id: u8,
    bg_palette_bits_low: u8,
    bg_palette_bits_high: u8,

    eval_byte_offset: u8,
    /// Buffer of sprites to render for the current scanline
    sprite_buffer: Vec<Sprite>,
}

impl Ppu {
    /// Gets a RGB color from the palette ram based on index from 0-64 (8 palettes with 4 indexable colors)
    pub fn get_palette_color(&self, palette_index: impl Into<u16>) -> [u8; 3] {
        let color_index = self.registers.read_palette_ram(palette_index.into());
        let emphasis_bits = self.registers.mask.bits() >> 5;
        self.palette.get(color_index % 64, emphasis_bits)
    }

    pub fn get_palette_colors(&self, palette_id: u8) -> [[u8; 3]; 4] {
        let mut palette = [[0; 3]; 4];
        for (i, color) in palette.iter_mut().enumerate() {
            *color = self.get_palette_color(palette_id * 4 + i as u8);
        }
        palette
    }

    pub fn frame_complete(&mut self) -> bool {
        if self.frame_complete {
            self.frame_complete = false;
            true
        } else {
            false
        }
    }

    pub(crate) fn clock(&mut self) -> PpuClockReport {
        let mut report = PpuClockReport::None;

        // Specific scanline timings
        // See https://www.nesdev.org/w/images/default/4/4f/Ppu.svg
        match self.registers.scanline {
            0..=239 => {
                self.frame_complete = false;
                if self.registers.mask.rendering() {
                    self.clock_sprite_render_line();
                    self.clock_bg_render_line();
                }
                if self.registers.on_visble_dot() {
                    self.render_pixel();
                }
            }
            241 if self.registers.dot == 1 => {
                self.frame_complete = true;
                self.registers.status.set(Status::VBLANK, true);
                if self.registers.control.contains(Control::VBLANK_NMI) {
                    report = PpuClockReport::Nmi
                }
            }
            PRERENDER_SCANLINE if self.registers.dot == 1 => {
                self.registers.status.remove(Status::VBLANK);
                self.registers.status.remove(Status::SPRITE_OVERFLOW);
                self.registers.status.remove(Status::SPRITE_0_HIT);
            }
            PRERENDER_SCANLINE if self.registers.mask.rendering() => {
                self.clock_sprite_render_line();
                self.clock_bg_prerender_line();
            }
            _ => (),
        }

        self.registers.next_dot();
        report
    }

    fn render_pixel(&mut self) {
        let mut force_mask = Mask::default();
        force_mask.set(Mask::GRAYSCALE, self.config.force_grayscale);
        self.registers.mask |= force_mask;

        let x = self.registers.dot - 1;
        let color_index = if self.registers.mask.rendering() {
            let bg_color_index = self.render_bg_pixel(x);
            self.render_fg_pixel(x, bg_color_index)
        } else if matches!(self.registers.v.0, PALETTE_START..=0x3fff) {
            // If v register is pointing to pallette address then use that color instead
            // of the backdrop color
            (self.registers.v.0 - PALETTE_START) as u8
        } else {
            0
        };
        let i = x + self.registers.scanline * WIDTH;
        self.screen_pixels[i as usize] = self.get_palette_color(color_index);
    }

    // Scanlines when the PPU is actually drawing to the screen
    fn clock_bg_render_line(&mut self) {
        match self.registers.dot {
            // When at visible dots, shift registers and load bits
            // Last 16 dots load the shift register with the next tiles on the next scanline
            1..=255 | 328..=336 => {
                self.shift_registers();
                if (self.registers.dot - 1).is_multiple_of(8) {
                    self.load_background_shift_bits();
                }
            }
            // Scroll y and reset x when end of visible scanline
            256 => self.registers.v.scroll_fine_y(),
            257 => self.registers.v.copy_x(self.registers.t),
            _ => (),
        }
    }

    fn clock_bg_prerender_line(&mut self) {
        match self.registers.dot {
            280..=304 => self.registers.v.copy_y(self.registers.t),
            // Skip last cycle on odd frames
            339 if self.registers.frame_count % 2 == 1 => self.registers.dot += 1,
            // Prerender line does same stuff as render line but with extra stuff
            _ => self.clock_bg_render_line(),
        }
    }

    /// Load the next tile data into the background shift bits for the current tile at the dot scanline
    /// Technically the address calculation and reads are supposed to happen in different
    /// cycles every 8 cycles but just have it all at once for simplicity (maybe slightly less accurate)
    fn load_background_shift_bits(&mut self) {
        let registers = &mut self.registers;
        let tile_number = registers.bus.read(registers.v.nametable_address());
        let attribute_byte = registers.bus.read(registers.v.attribute_address());
        self.bg_palette_id = registers.v.palette_id(attribute_byte);
        let (address_lsb, address_msb) = get_pattern_tile_addresses(
            tile_number as u16 + registers.control.background_table_offset(),
            registers.v.get(VramRegister::FINE_Y),
        );

        self.bg_shift_bits_low &= 0xff00;
        self.bg_shift_bits_high &= 0xff00;
        self.bg_shift_bits_low |= registers.bus.read(address_lsb) as u16;
        self.bg_shift_bits_high |= registers.bus.read(address_msb) as u16;
        registers.v.scroll_coarse_x();
    }

    fn clock_sprite_render_line(&mut self) {
        match self.registers.dot {
            64 => self.eval_byte_offset = self.registers.oam_address,
            // Technically supposed to happen for the entire scanline but do it once at the end for simplicity
            256 if self.registers.scanline != PRERENDER_SCANLINE => self.eval_sprites(),
            261 => {
                self.load_sprites();
                self.registers.oam_address = 0;
            }
            _ => (),
        }
    }

    /// Populates the sprite buffer for the next scanline and checks SPRITE_OVERFLOW
    fn eval_sprites(&mut self) {
        self.sprite_buffer.clear();
        let height = self.registers.control.sprite_height() as u16;

        let mut i = 0;
        while let Some(sprite) = self
            .registers
            .get_oam_sprite(i, self.eval_byte_offset as usize)
        {
            // Add to sprite buffer if sprite part of scanline
            // Note that it's loading sprites for the next scanline so all sprite y is offset by one
            let overflowed =
                self.sprite_buffer.len() == MAX_SPRITES_PER_SCAN && !self.config.unlimited_sprites;
            if self.registers.scanline >= sprite.y as u16
                && self.registers.scanline < sprite.y as u16 + height
            {
                // Check sprite overflow
                if overflowed {
                    self.registers.status.insert(Status::SPRITE_OVERFLOW);
                    break;
                }
                self.sprite_buffer.push(sprite);
            } else if overflowed {
                // After 8 sprites has been filled, the PPU will check for overflow by
                // searching for another sprite that is in the scanline.
                // But for some reason, when it doesn't find a sprite after filled,
                // the next OAM y it checks is offseted by one extra byte which causes buggy
                // behaviour when setting SPRITE_OVERFLOW flag.
                self.eval_byte_offset += 1;
            }

            i += 1;
        }
    }

    fn load_sprites(&mut self) {
        // The ppu will always fetch 8 sprites so we need to do that here (for mappers that track
        // ppu reads) unless of course unlimited_sprites is on
        for i in 0..MAX_SPRITES_PER_SCAN.max(self.sprite_buffer.len()) {
            let mut empty_sprite = Sprite::new(&[0xff, 0xff, 0xff, 0xff], 0);
            let sprite = self.sprite_buffer.get_mut(i).unwrap_or(&mut empty_sprite);
            sprite.load_shift_bits(self.registers.scanline, &mut self.registers);
        }
        // Prender scanline still makes same tile fetches but nothing gets rendered
        if self.registers.scanline == PRERENDER_SCANLINE {
            self.sprite_buffer.clear();
        }
    }

    /// Returns the index into palette ram for the current pixel of the background (0 means no color/transparent)
    fn render_bg_pixel(&mut self, scan_x: u16) -> u8 {
        if !self.registers.mask.can_show_background(scan_x) {
            return 0;
        }

        let bit_mask = 0b1000_0000 >> self.registers.fine_x;
        let mut color_index = add_bit_planes(
            (self.bg_shift_bits_low >> 8) as u8,
            (self.bg_shift_bits_high >> 8) as u8,
            bit_mask,
        );
        if color_index != 0 {
            // Add palette offset if not transparent
            let palette_id = add_bit_planes(
                self.bg_palette_bits_low,
                self.bg_palette_bits_high,
                bit_mask,
            );
            color_index += 4 * palette_id
        }
        color_index
    }

    /// Returns the palette ram index for the current pixel if a sprite is there or the background based on piority
    fn render_fg_pixel(&mut self, scan_x: u16, bg_color_index: u8) -> u8 {
        if self.registers.mask.can_show_sprite(scan_x) {
            for sprite in self.sprite_buffer.iter() {
                let color_index = sprite.color_index(scan_x);
                if color_index == 0 {
                    continue;
                }

                // Check if sprite 0 is rendering and set status flag
                if sprite.oam_index == 0 && bg_color_index != 0 && scan_x != 255 {
                    self.registers.status.insert(Status::SPRITE_0_HIT);
                }

                let palette_id = sprite.attributes.palette() + 4;
                let behind_bg = sprite.attributes.contains(Attributes::RENDER_BEHIND);
                if behind_bg && bg_color_index != 0 || self.config.hide_sprite {
                    // Sprite still gets drawn on top of later sprites but it uses background
                    break;
                } else {
                    return color_index + palette_id * 4;
                }
            }
        }

        if self.config.hide_background {
            0
        } else {
            bg_color_index
        }
    }

    fn shift_registers(&mut self) {
        self.bg_shift_bits_low <<= 1;
        self.bg_shift_bits_high <<= 1;
        // Shift and add new bits to the shift register
        let palette_lsb = self.bg_palette_id & 0b01;
        let palette_msb = (self.bg_palette_id & 0b10) >> 1;
        self.bg_palette_bits_low = (self.bg_palette_bits_low << 1) | palette_lsb;
        self.bg_palette_bits_high = (self.bg_palette_bits_high << 1) | palette_msb;
    }
}

/// Adds two low and high bits specified by the bit mask
pub fn add_bit_planes(lsb_plane: u8, msb_plane: u8, bit_mask: u8) -> u8 {
    let lsb = ((lsb_plane & bit_mask) != 0) as u8;
    let msb = ((msb_plane & bit_mask) != 0) as u8;
    lsb | (msb << 1)
}

/// Gets the pattern table tile planes least and most significant byte addresses
/// Returns (lsb plane address, msb plane address)
pub fn get_pattern_tile_addresses(tile_number: u16, fine_y: u16) -> (u16, u16) {
    // From nes wiki: https://www.nesdev.org/wiki/PPU_pattern_tables#Addressing
    // DCBA98 76543210
    // ---------------
    // 0HNNNN NNNNPyyy
    // |||||| |||||+++- T: Fine Y offset, the pixel row number within a tile
    // |||||| ||||+---- P: Bit plane (0: less significant bit; 1: more significant bit)
    // ||++++-++++----- N: Tile number from name table
    // |+-------------- H: Half of pattern table (0: "left"; 1: "right")
    // +--------------- 0: Pattern table is at $0000-$1FFF
    let address = ((tile_number) << 4) | (fine_y % TILE_SIZE);
    ((address), (address + TILE_SIZE))
}
