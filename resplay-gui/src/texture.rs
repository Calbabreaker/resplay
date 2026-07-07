pub struct Texture {
    handle: Option<egui::TextureHandle>,
    image_data: Option<egui::ColorImage>,
    size: [usize; 2],
}

impl Texture {
    pub fn new(size: [usize; 2]) -> Self {
        Self {
            size,
            handle: None,
            image_data: None,
        }
    }

    pub fn image(&mut self, ui: &egui::Ui) -> egui::Image<'_> {
        let handle = self.handle.get_or_insert_with(|| {
            ui.ctx().load_texture(
                "",
                egui::ColorImage::new([0, 0], Vec::new()),
                egui::TextureOptions::NEAREST,
            )
        });

        if let Some(data) = self.image_data.take() {
            handle.set(data, egui::TextureOptions::NEAREST);
        }

        egui::Image::new(self.handle.as_ref().unwrap())
    }

    pub fn update_pixels(&mut self, pixels: Vec<egui::Color32>) {
        self.image_data = Some(egui::ColorImage::new(self.size, pixels));
    }
}

#[derive(Default)]
pub struct TextureMap(pub std::collections::HashMap<String, Texture>);

impl TextureMap {
    pub fn update_ppu_texture(&mut self, pixels: &resplay_core::ppu::ScreenPixels) {
        let texture = self.0.entry("ppu_output".into()).or_insert_with(|| {
            Texture::new([
                resplay_core::ppu::WIDTH as usize,
                resplay_core::ppu::HEIGHT as usize,
            ])
        });

        texture.update_pixels(
            pixels
                .iter()
                .map(|c| egui::Color32::from_rgb(c[0], c[1], c[2]))
                .collect(),
        );
    }

    pub fn get(&mut self, name: impl ToString, size: [usize; 2]) -> &mut Texture {
        self.0
            .entry(name.to_string())
            .or_insert_with(|| Texture::new(size))
    }
}
