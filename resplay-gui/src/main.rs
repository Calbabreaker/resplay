#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod action;
mod app;
mod audio;
mod egui_util;
mod state;
mod texture;
mod ui_window;

pub use action::*;
pub use app::*;
pub use state::*;
pub use texture::Texture;

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    env_logger::builder()
        .parse_default_env()
        .filter_module("resplay", log::LevelFilter::Trace)
        .filter_level(log::LevelFilter::Info)
        .init();

    eframe::run_native(
        "Resplay",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast;
    eframe::WebLogger::init(log::LevelFilter::Info).ok();

    std::panic::set_hook(Box::new(|info| {
        log::error!("panic occurred {}", info.to_string());
    }));

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .get_element_by_id("canvas")
            .unwrap()
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .unwrap();

        if let Err(err) = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await
        {
            crate::egui_util::show_error_dialog("Failed to start egui", format!("{err:?}"));
        }
    });
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn load_nes_rom(bytes: Vec<u8>) {
    FILE_LOAD_CHANNEL.with(|channel| {
        channel.0.send(FileLoadInfo::new("nes", Ok(bytes))).unwrap();
    });
}
