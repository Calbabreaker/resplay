use crate::{Hotkey, Preferences, egui_util::show_error_dialog, texture::TextureMap};

pub struct FileLoadInfo {
    extension: String,
    source: Result<Box<[u8]>, std::path::PathBuf>,
}

impl FileLoadInfo {
    pub fn new(
        extension: impl Into<String>,
        source: Result<Box<[u8]>, std::path::PathBuf>,
    ) -> Self {
        Self {
            extension: extension.into(),
            source,
        }
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct State {
    #[serde(skip)]
    pub emu: resplay_core::Emulator,
    #[serde(skip)]
    pub texture_map: TextureMap,
    #[serde(skip)]
    pub quick_saves: std::collections::HashMap<u8, Box<[u8]>>,
    #[serde(skip)]
    pub ui_render_time: f32,

    pub recent_rom_paths: Vec<std::path::PathBuf>,
    pub selected_quick_save: u8,
}

impl State {
    pub fn update_emulation(&mut self, ctx: &egui::Context, prefs: &Preferences) {
        self.emu.ppu().config = prefs.ppu;
        self.emu.apu().config = prefs.apu;

        if let Err(err) = self.emu.update(ctx.time() as f32, |pixels| {
            self.texture_map.update_ppu_texture(pixels)
        }) && !prefs.continue_on_halt
        {
            show_error_dialog(
                "CPU halted (can be disabled in preferences)",
                format!("{err}"),
            );
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

    pub fn load_file(&mut self, info: FileLoadInfo) {
        let mut loaded_file_path = None;
        let data = match info.source {
            Ok(bytes) => bytes,
            Err(path) => match std::fs::read(&path) {
                Ok(bytes) => {
                    loaded_file_path = Some(path);
                    bytes.into_boxed_slice()
                }
                Err(err) => return show_error_dialog("Failed to load file", format!("{err}")),
            },
        };

        match info.extension.as_str() {
            "nes" => {
                if let Err(err) = self.emu.load_nes_rom(&data[..]) {
                    show_error_dialog("Failed to load NES ROM", format!("{err}"));
                } else {
                    log::trace!(
                        "Loaded cartridge with header: {:?}",
                        self.emu.cartridge().unwrap().header()
                    );
                    self.emu.running = true;
                    // Put the file in recent files if we loaded sucessfully
                    if let Some(path) = loaded_file_path {
                        // Make sure added path is on top
                        self.recent_rom_paths.retain(|x| *x != path);
                        self.recent_rom_paths.insert(0, path.clone());
                        self.recent_rom_paths.truncate(20);
                    }
                }
            }
            "resav" => {
                if let Err(err) = self.emu.load_state(&data) {
                    show_error_dialog("Failed to load state", format!("{err}"));
                }
            }
            name => crate::egui_util::show_error_dialog(
                "Unknown file extension",
                format!("Unrecognised file extension with {name}"),
            ),
        }
    }

    pub fn do_hotkey(&mut self, hotkey: Hotkey) {
        match hotkey {
            Hotkey::Reset => {
                self.emu.cpu.reset();
                self.emu.running = true;
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
