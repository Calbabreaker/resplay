pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    ui.label(format!(
        "UI render time: {:.3}ms",
        state.ui_render_time * 1000.
    ));
    ui.label(format!("Frame rate: {:.3}fps", state.emu.frame_rate()));
}
