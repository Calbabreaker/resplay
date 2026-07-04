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

        let result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(App::new(cc)))),
            )
            .await;

        // Remove the loading text
        let loading_element = document.get_element_by_id("loading").unwrap();
        match result {
            Ok(_) => loading_element.remove(),
            Err(e) => {
                loading_element.set_inner_html("Failed to load app :(");
                log::error!("Failed to start eframe: {e:?}");
            }
        }
    });
}
