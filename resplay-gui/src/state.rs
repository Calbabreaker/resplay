use crate::{Hotkey, audio::setup_audio_stream, egui_util::show_error_dialog, texture::TextureMap};

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
            Err(err) => show_error_dialog("Failed to initialize audio", format!("{err}")),
        }
    }

    /// Load a nes rom into the emulator
    pub fn load_nes_rom(&mut self, data: &[u8]) {
        if let Err(err) = self.emu.load_nes_rom(data) {
            show_error_dialog("Failed to load NES ROM", format!("{err}"));
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
                self.emu.load_cpu(resplay_core::Cpu::default());
            }
            Hotkey::PauseResume => self.emu.running = !self.emu.running,
            Hotkey::Step => {
                self.emu.running = false;
                self.emu.cpu.execute_next().ok();
            }
            Hotkey::QuickSave => {
                self.quick_saves
                    .insert(self.selected_quick_save, self.emu.save_state());
            }
            Hotkey::QuickLoad => {
                if let Some(data) = self.quick_saves.get(&self.selected_quick_save)
                    && let Err(err) = self.emu.load_state(data)
                {
                    show_error_dialog("Failed to load quick save", format!("{err}"));
                }
            }
            Hotkey::NextFrame => {
                self.emu.running = false;
                self.emu.next_frame().ok();
            }
        }
    }
}
