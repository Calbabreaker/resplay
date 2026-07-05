use crate::{Hotkey, audio::setup_audio_stream, texture::TextureMap};

#[derive(Debug)]
pub enum NesRomSource {
    Path(std::path::PathBuf),
    Bytes(Vec<u8>),
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct State {
    #[serde(skip)]
    pub emu: resplay_core::Emulator,
    #[serde(skip)]
    pub texture_map: TextureMap,
    #[serde(skip)]
    pub quick_saves: std::collections::HashMap<u8, Vec<u8>>,
    #[serde(skip)]
    pub audio_stream: Option<cpal::Stream>,
    #[serde(skip)]
    last_rom_source: Option<NesRomSource>,

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
            Err(err) => crate::egui_util::show_error_dialog("Failed to initialize audio", err),
        }
    }

    /// Load a nes rom into the emulator
    pub fn load_nes_rom(&mut self, source: NesRomSource) {
        let result = match source {
            NesRomSource::Path(ref path) => {
                log::trace!("Loading {path:?}");
                self.emu.load_nes_file(path).inspect(|_| {
                    // Make sure added path is on top
                    self.recent_file_paths.retain(|x| *x != *path);
                    self.recent_file_paths.insert(0, path.clone());
                    self.recent_file_paths.truncate(20)
                })
            }
            NesRomSource::Bytes(ref bytes) => self.emu.load_nes_rom(&bytes[..]),
        };

        if let Err(err) = result {
            crate::egui_util::show_error_dialog("Failed to load NES ROM", err);
        } else {
            log::trace!(
                "Loaded cartridge with header: {:?}",
                self.emu.cartridge().unwrap().header()
            );
            self.emu.running = true;
            self.last_rom_source = Some(source)
        }
    }

    pub fn do_hotkey(&mut self, hotkey: Hotkey) {
        match hotkey {
            Hotkey::SoftReset => {
                self.emu.cpu.reset();
                self.emu.running = true;
            }
            Hotkey::HardReset => {
                self.emu.load_cpu(resplay_core::Cpu::default());
                if let Some(source) = self.last_rom_source.take() {
                    self.load_nes_rom(source);
                }
            }
            Hotkey::PauseResume => self.emu.running = !self.emu.running,
            Hotkey::Step => {
                self.emu.running = false;
                self.emu.cpu.execute_next().ok();
            }
            Hotkey::QuickSave => match postcard::to_allocvec(&self.emu.cpu) {
                Err(err) => crate::egui_util::show_error_dialog("Failed to serialize", err),
                Ok(data) => {
                    self.quick_saves.insert(self.selected_quick_save, data);
                }
            },
            Hotkey::QuickLoad => {
                if let Some(data) = self.quick_saves.get(&self.selected_quick_save) {
                    match postcard::from_bytes(data) {
                        Err(err) => {
                            crate::egui_util::show_error_dialog("Failed to deserialize", err);
                        }
                        Ok(cpu) => self.emu.load_cpu(cpu),
                    }
                }
            }
            Hotkey::NextFrame => {
                self.emu.running = false;
                self.emu.next_frame().ok();
            }
        }
    }
}
