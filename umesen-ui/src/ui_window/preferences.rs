use crate::{ActionKind, Preferences};

#[derive(Clone, Copy, PartialEq, Eq, Default, serde::Deserialize, serde::Serialize, Debug)]
enum Tab {
    #[default]
    KeyBinds,
    Misc,
    Audio,
    Video,
}

impl crate::egui_util::UiList for Tab {
    fn pretty_name(&self) -> &'static str {
        match self {
            Self::KeyBinds => "Key binds",
            Self::Misc => "Misc",
            Self::Audio => "Audio",
            Tab::Video => "Video",
        }
    }

    const LIST: &[Self] = &[Self::KeyBinds, Self::Audio, Self::Video, Self::Misc];
}

pub fn show(ui: &mut egui::Ui, prefs: &mut Preferences) {
    let tab_open = crate::egui_util::ui_list_tab_group(ui);

    match tab_open {
        Tab::Misc => {
            egui::Grid::new("misc prefs").striped(true).show(ui, |ui| {
                ui.label("Allow illegal press ").on_hover_text(
                    "Allow left and right or up and down to be pressed at the same time",
                );
                ui.checkbox(&mut prefs.allow_illegal_press, "");
                ui.end_row();
            });
        }
        Tab::Audio => {
            egui::Grid::new("audio prefs").striped(true).show(ui, |ui| {
                ui.label("Master volume");
                ui.add(egui::Slider::new(&mut prefs.apu.volume, (0.)..=1.));
                ui.end_row();
                ui.label("Pulse 0 volume");
                ui.add(egui::Slider::new(&mut prefs.apu.pulse_0_volume, (0.)..=1.));
                ui.end_row();
                ui.label("Pulse 1 volume");
                ui.add(egui::Slider::new(&mut prefs.apu.pulse_1_volume, (0.)..=1.));
                ui.end_row();
                ui.label("Triangle volume");
                ui.add(egui::Slider::new(&mut prefs.apu.triangle_volume, (0.)..=1.));
                ui.end_row();
                ui.label("Noise volume");
                ui.add(egui::Slider::new(&mut prefs.apu.noise_volume, (0.)..=1.));
                ui.end_row();
                ui.label("DMC volume");
                ui.add(egui::Slider::new(&mut prefs.apu.dmc_volume, (0.)..=1.));
                ui.end_row();
            });
        }
        Tab::Video => {
            egui::Grid::new("video prefs").striped(true).show(ui, |ui| {
                ui.label("Allow unlimited sprites").on_hover_text("Allow unlimited sprites to be rendered on the same scanline at a time instead of the usual 8");
                ui.checkbox(&mut prefs.ppu.unlimited_sprites, "");
                ui.end_row();
                ui.label("Hide sprite");
                ui.checkbox(&mut prefs.ppu.hide_sprite, "");
                ui.end_row();
                ui.label("Hide background");
                ui.checkbox(&mut prefs.ppu.hide_background, "");
                ui.end_row();
                ui.label("Force grayscale mode");
                ui.checkbox(&mut prefs.ppu.force_grayscale, "");
                ui.end_row();
            });
        }
        Tab::KeyBinds => {
            ui.horizontal_top(|ui| {
                use ActionKind::*;
                show_key_map(ui, prefs, "mainkeys", |action| {
                    !matches!(action, ControllerInput(..))
                });
                for i in 0..=1 {
                    show_key_map(
                        ui,
                        prefs,
                        format!("controllerkeys{i}"),
                        |action| matches!(action, ControllerInput(num, _) if num == i),
                    );
                }
            });
        }
    }

    ui.separator();

    if ui.button("Reset").clicked() {
        *prefs = Preferences::default();
    }
}

fn show_key_map(
    ui: &mut egui::Ui,
    prefs: &mut Preferences,
    name: impl egui::AsIdSalt,
    filter_func: impl Fn(ActionKind) -> bool,
) {
    egui::Grid::new(name).striped(true).show(ui, |ui| {
        let mut action_to_rebind = prefs.key_action_map.action_to_rebind;
        for (action, shortcut) in prefs.key_action_map.iter_map() {
            if filter_func(action) {
                ui.label(action.name());

                let text = if action_to_rebind == Some(action) {
                    "...".to_owned()
                } else {
                    crate::egui_util::get_shortcut_text(&shortcut)
                };
                if ui.button(text).clicked() {
                    action_to_rebind = Some(action);
                }
                ui.end_row();
            }
        }
        prefs.key_action_map.action_to_rebind = action_to_rebind;
    });
}
