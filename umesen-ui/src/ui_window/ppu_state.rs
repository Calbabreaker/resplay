pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    let ppu = &mut state.emu.cpu.bus.ppu;
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    ui.label(format!(
        "T: ${:04x} ({})",
        ppu.registers.t.0, ppu.registers.t
    ));
    ui.label(format!(
        "V: ${:04x} ({})",
        ppu.registers.v.0, ppu.registers.v,
    ));
    ui.label(format!("Fine x: {}", ppu.registers.fine_x));
    ui.label(format!("Latch: {}", ppu.registers.latch));
    ui.label(format!("Scanline: {}", ppu.registers.scanline));
    ui.label(format!("Dot: {}", ppu.registers.dot));
    ui.separator();
    ui.label("Control flags");
    crate::egui_util::show_flags(ui, &mut ppu.registers.control);
    ui.separator();
    ui.label("Status flags");
    crate::egui_util::show_flags(ui, &mut ppu.registers.status);
    ui.separator();
    ui.label("Mask flags");
    crate::egui_util::show_flags(ui, &mut ppu.registers.mask);
}
