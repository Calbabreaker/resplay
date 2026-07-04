use crate::{Hotkey, audio::setup_audio_stream, texture::TextureMap};

#[derive(Debug)]
pub enum NesRomSource {
    Path(std::path::PathBuf),
    Bytes(Vec<u8>),
    MostRecent,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct State {
    #[serde(skip)]
    pub emu: resplay_core::Emulator,
    #[serde(skip)]
    pub texture_map: TextureMap,
    #[serde(skip)]
    pub quick_saves: std::collections::HashMap<u8, resplay_core::Cpu>,
    #[serde(skip)]
    pub audio_stream: Option<cpal::Stream>,

    pub recent_file_paths: Vec<std::path::PathBuf>,
    pub ui_render_time: f32,
    pub selected_quick_save: u8,
}

impl State {
    pub fn update_emulation(&mut self, ctx: &egui::Context) {
        if let Err(err) = self.emu.update(ctx.time() as f32, |pixels| {
            self.texture_map.update_ppu_texture(pixels)
        }) {
            log::warn!("CPU halted: {err}");
            self.emu.running = false;
        }

        if self.emu.speed < 1. {
            self.texture_map
                .update_ppu_texture(&self.emu.ppu().screen_pixels);
        }
        if self.emu.running {
            ctx.request_repaint();
        }
    }

    pub fn setup_audio_stream(&mut self) {
        match setup_audio_stream(&mut self.emu) {
            Ok(stream) => self.audio_stream = Some(stream),
            Err(err) => {
                rfd::MessageDialog::new()
                    .set_title("Failed to initialize audio")
                    .set_description(format!("{err}"))
                    .show();
            }
        }
    }

    /// Load a nes rom into the emulator
    pub fn load_nes_rom(&mut self, source: NesRomSource) {
        let result = match source {
            NesRomSource::MostRecent => {
                if let Some(path) = self.recent_file_paths.first() {
                    self.emu.load_nes_file(path)
                } else {
                    return;
                }
            }
            NesRomSource::Path(path) => {
                log::trace!("Loading {path:?}");
                self.emu.load_nes_file(&path).inspect(|_| {
                    // Make sure added path is on top
                    self.recent_file_paths.retain(|x| *x != path);
                    self.recent_file_paths.insert(0, path);
                    self.recent_file_paths.truncate(20)
                })
            }
            NesRomSource::Bytes(bytes) => self.emu.load_nes_rom(&bytes[..]),
        };

        if let Err(err) = result {
            rfd::MessageDialog::new()
                .set_title("Failed to load NES ROM")
                .set_description(format!("{err}"))
                .show();
            log::error!("{err}");
        } else {
            log::trace!(
                "Loaded cartridge with header: {:?}",
                self.emu.cartridge().unwrap().header()
            );
            self.emu.running = true;
        }
    }

    pub fn do_hotkey(&mut self, hotkey: Hotkey) {
        match hotkey {
            Hotkey::SoftReset => {
                self.emu.cpu.reset();
                self.emu.running = true;
            }
            Hotkey::HardReset => {
                self.emu = resplay_core::Emulator::default();
                self.setup_audio_stream();
                self.load_nes_rom(NesRomSource::MostRecent);
            }
            Hotkey::PauseResume => self.emu.running = !self.emu.running,
            Hotkey::Step => {
                self.emu.running = false;
                self.emu.cpu.execute_next().ok();
            }
            Hotkey::QuickSave => {
                // self.save_states
                //     .insert(self.selected_quick_save, self.emu.cpu.clone());
            }
            Hotkey::QuickLoad => {
                if let Some(cpu) = self.quick_saves.get(&self.selected_quick_save) {
                    // self.emu.cpu = cpu.clone();
                }
            }
            Hotkey::NextFrame => {
                self.emu.running = false;
                self.emu.next_frame().ok();
            }
        }
    }
}
