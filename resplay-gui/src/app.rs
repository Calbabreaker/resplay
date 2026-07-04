use std::collections::HashSet;

use crate::{
    Action, DEFAULT_ACTION_MAP, Hotkey, KeybindingMap, NesRomSource, State, ui_window::UiWindowKind,
};

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Preferences {
    pub key_bindings: KeybindingMap,
    pub allow_illegal_press: bool,
    pub ppu: resplay_core::ppu::PpuConfig,
    pub apu: resplay_core::apu::ApuConfig,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    ui_windows: HashSet<UiWindowKind>,
    preferences: Preferences,
    state: crate::State,

    #[serde(skip)]
    file_picker_channel: (
        std::sync::mpsc::Sender<NesRomSource>,
        std::sync::mpsc::Receiver<NesRomSource>,
    ),
}

impl Default for App {
    fn default() -> Self {
        Self {
            ui_windows: HashSet::default(),
            preferences: Preferences::default(),
            state: State::default(),
            file_picker_channel: std::sync::mpsc::channel(),
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut app: App = cc
            .storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default();
        app.state.load_nes_rom(NesRomSource::MostRecent);
        #[cfg(not(target_arch = "wasm32"))]
        app.state.setup_audio_stream();
        app
    }

    fn show_file_dialog(&mut self) {
        let dialog = rfd::AsyncFileDialog::new()
            .add_filter("NES ROM", &["nes"])
            .pick_file();
        let sender = self.file_picker_channel.0.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            if let Some(file) = pollster::block_on(dialog) {
                let source = NesRomSource::Path(file.path().to_path_buf());
                sender.send(source).unwrap();
            }
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(async move {
            if let Some(file) = dialog.await {
                let bytes = NesRomSource::Bytes(file.read().await);
                sender.send(bytes).unwrap();
            }
        });
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Open ROM...").clicked() {
                self.show_file_dialog();
                ui.close();
            }

            #[cfg(not(target_arch = "wasm32"))]
            ui.menu_button("Recent ROMS", |ui| {
                let mut paths = self.state.recent_file_paths.iter();
                let path = paths.find(|path| ui.button(path.to_string_lossy()).clicked());

                if let Some(path) = path {
                    self.state.load_nes_rom(NesRomSource::Path(path.clone()));
                    ui.close();
                }
            });

            if ui.button("Preferences...").clicked() {
                self.ui_windows.insert(UiWindowKind::Preferences);
                ui.close();
            }
        });

        ui.menu_button("View", |ui| {
            use UiWindowKind::*;
            for kind in [
                Debugger,
                HexViewer,
                PpuMemory,
                PpuState,
                Stats,
                CatridgeInfo,
            ] {
                let mut open = self.ui_windows.contains(&kind);
                let text = format!("{}...", kind.title());
                if ui.toggle_value(&mut open, text).clicked() {
                    if open {
                        self.ui_windows.insert(kind);
                    } else {
                        self.ui_windows.remove(&kind);
                    }
                }
            }
        });

        ui.menu_button("Emulation", |ui| {
            use Hotkey::*;
            self.show_hotkey_list(
                ui,
                &[PauseResume, SoftReset, HardReset, QuickSave, QuickLoad],
            );

            ui.menu_button("Quick Save Slot", |ui| {
                for i in 0..9 {
                    let mut text = egui::RichText::new(format!("Slot {i}"));
                    if i == self.state.selected_quick_save {
                        text = text.underline();
                    }
                    if ui.button(text).clicked() {
                        self.state.selected_quick_save = i;
                    }
                }
            });
        });
    }

    fn show_hotkey_list(&mut self, ui: &mut egui::Ui, list: &[Hotkey]) {
        for hotkey in list.iter() {
            let action = Action::Hotkey(*hotkey);
            let binding = self.preferences.key_bindings.actions.get(&action);
            let shortcut = binding.unwrap_or(&DEFAULT_ACTION_MAP[&action]);
            let button = egui::Button::new(action.name())
                .shortcut_text(crate::egui_util::get_shortcut_text(shortcut));
            if ui.add(button).clicked() {
                self.state.do_hotkey(*hotkey);
                ui.close();
            }
        }
    }

    fn check_input(&mut self, i: &mut egui::InputState) {
        for (action, shortcut) in self.preferences.key_bindings.iter_map() {
            match action {
                Action::Controller(number, button) => {
                    let controller = self.state.emu.controller(number);
                    let key_down = i.key_down(shortcut.logical_key);
                    controller.set_button(button, key_down, self.preferences.allow_illegal_press);
                }
                Action::Hotkey(hotkey) => {
                    if i.consume_shortcut(&shortcut) {
                        self.state.do_hotkey(hotkey);
                    }
                }
            }
        }

        self.preferences.key_bindings.check_key_down(i);

        if let Some(file) = i.raw.dropped_files.pop() {
            if let Some(path) = file.path {
                self.state.load_nes_rom(NesRomSource::Path(path));
            } else if let Some(bytes) = file.bytes {
                self.state.load_nes_rom(NesRomSource::Bytes(bytes.to_vec()));
            }
        }
        i.raw.dropped_files.clear();

        // Only create the audio stream after an interaction to make sure it plays
        #[cfg(target_arch = "wasm32")]
        if i.pointer.any_click() && self.state.audio_stream.is_none() {
            self.state.setup_audio_stream();
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.input_mut(|i| self.check_input(i));
        if let Ok(source) = self.file_picker_channel.1.try_recv() {
            self.state.load_nes_rom(source);
        }

        self.state.emu.ppu().config = self.preferences.ppu.clone();
        self.state.emu.apu().config = self.preferences.apu.clone();

        self.state.update_emulation(ctx);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let default_bg = ui.style().visuals.noninteractive().bg_fill;
        egui::Panel::top("top_panel")
            .frame(egui::Frame::default().fill(default_bg).inner_margin(6.0))
            .show(ui, |ui| {
                egui::MenuBar::new().ui(ui, |ui| self.show_top_bar(ui))
            });

        self.ui_windows
            .retain(|kind| kind.show(ui, &mut self.state, &mut self.preferences));

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                ui.centered_and_justified(|ui| {
                    if let Some(texture) = self.state.texture_map.0.get_mut("ppu_output") {
                        ui.add(texture.image(ui).fit_to_fraction(egui::vec2(1., 1.)));
                    }
                });
            });

        self.state.ui_render_time = frame.info().cpu_usage.unwrap_or(0.);
    }
}
