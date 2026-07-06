pub trait UiList: egui::util::id_type_map::SerializableAny + Default + Eq + Copy {
    fn pretty_name(&self) -> &'static str;

    const LIST: &[Self];
}

pub fn ui_list_tab_group<T: UiList + std::fmt::Debug>(ui: &mut egui::Ui) -> T {
    let tab_open: T = ui.memory_mut(|w| w.data.get_persisted(egui::Id::NULL).unwrap_or_default());
    ui.horizontal(|ui| {
        for tab in T::LIST {
            if ui
                .selectable_label(*tab == tab_open, tab.pretty_name())
                .clicked()
            {
                ui.memory_mut(|w| w.data.insert_persisted(egui::Id::NULL, *tab));
            }
        }
    });

    ui.separator();
    tab_open
}

pub fn ui_list_combo_select<T: UiList + std::fmt::Debug>(ui: &mut egui::Ui) -> T {
    let mut selected: T =
        ui.memory_mut(|w| w.data.get_persisted(egui::Id::NULL).unwrap_or_default());
    egui::ComboBox::from_id_salt(std::any::TypeId::of::<T>())
        .selected_text(selected.pretty_name())
        .show_ui(ui, |ui| {
            for kind in T::LIST {
                if ui
                    .selectable_value(&mut selected, *kind, kind.pretty_name())
                    .clicked()
                {
                    ui.memory_mut(|w| w.data.insert_persisted(egui::Id::NULL, selected));
                };
            }
        });

    selected
}

pub fn get_shortcut_text(shortcut: &egui::KeyboardShortcut) -> String {
    shortcut.format(&egui::ModifierNames::NAMES, cfg!(target_os = "macos"))
}

/// Show a list of flags as modifiable checkboxes based on the flag names
pub fn show_flags<T: bitflags::Flags + Copy>(ui: &mut egui::Ui, value: &mut T) {
    egui::Grid::new(std::any::TypeId::of::<T>()).show(ui, |ui| {
        for (i, flag) in T::FLAGS.iter().filter(|f| f.is_named()).enumerate() {
            ui.label(flag.name());
            let mut checked = value.contains(*flag.value());
            ui.checkbox(&mut checked, "");
            value.set(*flag.value(), checked);
            if i % 2 == 1 || T::FLAGS.len() < 5 {
                ui.end_row()
            };
        }
    });
}

/// Draw a rect with a color clipping the rect with clip rect and drawing the clipped part wrapped
/// to the start of the clip rect if the rect is larger than the the clip rect
pub fn draw_rect_wrapped(
    ui: &egui::Ui,
    rect: egui::Rect,
    clip_rect: egui::Rect,
    color: egui::Color32,
) {
    ui.painter()
        .with_clip_rect(clip_rect)
        .rect_filled(rect, 0., color);
    let left_over_size = (rect.max - clip_rect.max).max(egui::Vec2::ZERO);
    ui.painter().rect_filled(
        egui::Rect::from_min_size(clip_rect.min, egui::vec2(left_over_size.x, rect.size().y)),
        0.,
        color,
    );
    ui.painter().rect_filled(
        egui::Rect::from_min_size(clip_rect.min, egui::vec2(rect.size().x, left_over_size.y)),
        0.,
        color,
    );
}

pub fn show_error_dialog(title: impl Into<String>, error: impl Into<String>) {
    let error = error.into();
    log::error!("{error}");
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title(title)
        .set_description(error)
        .show();
}
