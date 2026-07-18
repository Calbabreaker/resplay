use std::{
    collections::HashSet,
    rc::Rc,
    sync::mpsc::{Receiver, Sender},
};

use crate::{
    Action, DEFAULT_ACTION_MAP, FileLoadInfo, Hotkey, KeybindingMap, audio::setup_audio_stream,
    egui_util::show_error_dialog, ui_window::UiWindowKind,
};

thread_local! {
    pub static FILE_LOAD_CHANNEL: Rc<(Sender<FileLoadInfo>, Receiver<FileLoadInfo>)>
        = Rc::new(std::sync::mpsc::channel());
}

#[derive(serde::Deserialize, serde::Serialize, Default)]
#[serde(default)]
pub struct Preferences {
    pub key_bindings: KeybindingMap,
    pub allow_illegal_press: bool,
    pub continue_on_halt: bool,
    pub ppu: resplay_core::ppu::PpuConfig,
    pub apu: resplay_core::apu::ApuConfig,
}

#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct App {
    ui_windows: HashSet<UiWindowKind>,
    preferences: Preferences,
    state: crate::State,

    #[serde(skip)]
    audio_stream: Option<cpal::Stream>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.storage
            .and_then(|storage| eframe::get_value(storage, eframe::APP_KEY))
            .unwrap_or_default()
    }

    fn show_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("File", |ui| {
            if ui.button("Open ROM...").clicked() {
                show_load_file_dialog("NES ROM", "nes");
                ui.close();
            }

            #[cfg(not(target_arch = "wasm32"))]
            ui.menu_button("Recent ROMs", |ui| {
                let mut paths = self.state.recent_rom_paths.iter();
                let path = paths.find(|path| ui.button(path.to_string_lossy()).clicked());

                if let Some(path) = path {
                    self.state
                        .load_file(FileLoadInfo::new("nes", Err(path.to_path_buf())));
                    ui.close();
                }
            });

            if ui.button("Save state...").clicked() {
                show_save_file_dialog("save.resav", self.state.emu.save_state());
                ui.close();
            }

            if ui.button("Load state...").clicked() {
                show_load_file_dialog("Resplay save state", "resav");
                ui.close();
            }

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
            self.show_hotkey_list(ui, &[PauseResume, Reset, QuickSave, QuickLoad]);

            ui.menu_button("Quick save slot", |ui| {
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

        while let Some(file) = i.raw.dropped_files.pop() {
            if let Some(path) = file.path {
                let ext = path.extension().unwrap_or_default();
                self.state.load_file(FileLoadInfo::new(
                    ext.to_string_lossy().to_string(),
                    Err(path),
                ));
            } else if let Some(bytes) = file.bytes {
                self.state.load_file(FileLoadInfo::new(
                    file.name.split(".").last().unwrap_or_default(),
                    Ok(bytes.to_vec().into_boxed_slice()),
                ));
            };
        }

        // Only create the audio stream after an interaction to make sure it plays
        if self.audio_stream.is_none()
            && (i.pointer.any_click() || cfg!(not(target_arch = "wasm32")))
        {
            match setup_audio_stream(&mut self.state.emu) {
                Ok(stream) => self.audio_stream = Some(stream),
                Err(err) => show_error_dialog("Failed to initialize audio", format!("{err}")),
            }
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        ctx.input_mut(|i| self.check_input(i));
        FILE_LOAD_CHANNEL.with(|channel| {
            if let Ok(info) = channel.1.try_recv() {
                self.state.load_file(info);
            }
        });

        self.state.update_emulation(ctx, &self.preferences);
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

fn show_load_file_dialog(type_name: &'static str, extension: &'static str) {
    let sender = FILE_LOAD_CHANNEL.with(|channel| channel.0.clone());
    run_future(async move {
        if let Some(file) = rfd::AsyncFileDialog::new()
            .add_filter(type_name, &[extension])
            .pick_file()
            .await
        {
            #[cfg(not(target_arch = "wasm32"))]
            let info = FileLoadInfo::new(extension, Err(file.path().to_path_buf()));
            #[cfg(target_arch = "wasm32")]
            let info = FileLoadInfo::new(extension, Ok(file.read().await));
            sender.send(info).unwrap();
        }
    })
}

fn show_save_file_dialog(filename: &'static str, data: Box<[u8]>) {
    run_future(async move {
        if let Some(file) = rfd::AsyncFileDialog::new()
            .set_file_name(filename)
            .save_file()
            .await
            && let Err(err) = file.write(&data).await
        {
            show_error_dialog("Failed to save file", format!("{err}"));
        }
    });
}

fn run_future(
    #[cfg(not(target_arch = "wasm32"))] fut: impl Future<Output = ()> + Send + 'static,
    #[cfg(target_arch = "wasm32")] fut: impl Future<Output = ()> + 'static,
) {
    #[cfg(not(target_arch = "wasm32"))]
    std::thread::spawn(move || pollster::block_on(fut));
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(fut);
}
