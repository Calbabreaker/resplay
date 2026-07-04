// Color ephamsis scaling factor for rgb ephamsis bits (r = low bit)
// Measurement by Quietust found on https://forums.nesdev.org/viewtopic.php?p=21102
const EMPHASIS_FACTOR: [[f32; 3]; 8] = [
    [1.00, 1.00, 1.00],
    [1.00, 0.80, 0.81],
    [0.78, 0.94, 0.66],
    [0.79, 0.77, 0.63],
    [0.82, 0.83, 1.12],
    [0.81, 0.71, 0.87],
    [0.68, 0.79, 0.79],
    [0.70, 0.70, 0.70],
];

pub struct Palette {
    /// Palettes for each ephamsis mode
    palettes: [[[u8; 3]; 64]; 8],
}

impl Default for Palette {
    fn default() -> Self {
        Self::from_pal(&include_bytes!("default.pal")[..]).unwrap()
    }
}

impl Palette {
    pub fn new(colors: [[u8; 3]; 64]) -> Self {
        let mut palettes = [[[0; 3]; 64]; 8];
        for (emphasis_bits, palette) in palettes.iter_mut().enumerate() {
            for (i, color) in palette.iter_mut().enumerate() {
                for (byte_i, color_byte) in color.iter_mut().enumerate() {
                    let color = (colors[i][byte_i] as f32) * EMPHASIS_FACTOR[emphasis_bits][byte_i];
                    *color_byte = color as u8;
                }
            }
        }
        Self { palettes }
    }

    pub fn from_pal(mut bytes: impl std::io::Read) -> std::io::Result<Self> {
        let mut color_bytes = [[0; 3]; 64];
        for palette_byte in color_bytes.iter_mut() {
            bytes.read_exact(palette_byte)?;
        }
        Ok(Palette::new(color_bytes))
    }

    /// Gets the RGBA color value of the index in the palette
    pub fn get(&self, index: u8, rgb_emphasis_bits: u8) -> [u8; 3] {
        debug_assert!(rgb_emphasis_bits < 8);
        debug_assert!(index < 64);
        self.palettes[rgb_emphasis_bits as usize][index as usize]
    }
}

#[cfg(test)]
mod test {
    use crate::ppu::palette::Palette;

    #[test]
    pub fn parse_correct() {
        let palette = Palette::default();
        assert_eq!(palette.get(0, 0), [0x62, 0x62, 0x62]);
        assert_eq!(palette.get(1, 0), [0x00, 0x2e, 0x98]);
        assert_eq!(palette.get(2, 0), [0x0c, 0x11, 0xc2]);
    }
}
