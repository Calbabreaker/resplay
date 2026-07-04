use crate::{Hotkey, audio::setup_audio_stream, texture::TextureMap, ui_window::PopupModal};

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
    pub popup_modal: PopupModal,
    #[serde(skip)]
    audio_stream: Option<cpal::Stream>,

    pub ui_render_time: f32,
    pub recent_file_paths: Vec<std::path::PathBuf>,
    pub selected_quick_save: u8,
}

impl State {
    pub fn update_emulation(&mut self, ctx: &egui::Context) {
        if let Err(err) = self
            .emu
            .update(|pixels| self.texture_map.update_ppu_texture(pixels))
        {
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
                self.popup_modal
                    .error("Failed to initialize audio", format!("{err}"));
            }
        }
    }

    /// Load a nes rom into the emulator
    /// If none loads the most recent file
    pub fn load_nes_rom(&mut self, path: Option<std::path::PathBuf>) {
        let path = match path {
            Some(path) => path,
            None if !self.recent_file_paths.is_empty() => self.recent_file_paths.remove(0),
            None => return,
        };
        log::trace!("Loading {path:?}");

        if let Err(err) = self.emu.load_nes_file(&path) {
            self.popup_modal
                .error("Failed to load NES ROM!".to_string(), format!("{err}"));
            log::error!("{err}");
        } else {
            log::trace!(
                "Loaded cartridge with header: {:?}",
                self.emu.cartridge().unwrap().header()
            );
            // Make sure added path is on top
            self.recent_file_paths.retain(|x| *x != path);
            self.recent_file_paths.insert(0, path);
            self.recent_file_paths.truncate(20);
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
                self.load_nes_rom(None);
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
