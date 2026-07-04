use crate::Hotkey;

pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    ui.label(format!("PC: ${0:02x}", state.emu.cpu.pc));
    ui.label(format!("SP: ${0:02x}", state.emu.cpu.sp));
    ui.label(format!("A:  ${0:02x}", state.emu.cpu.a));
    ui.label(format!("X:  ${0:02x}", state.emu.cpu.x));
    ui.label(format!("Y:  ${0:02x}", state.emu.cpu.y));
    ui.label(format!("CYCLES: {}", state.emu.cpu.bus.cpu_cycles_total));
    crate::egui_util::show_flags(ui, &mut state.emu.cpu.flags);

    ui.horizontal(|ui| {
        ui.label("Speed:");
        ui.style_mut().spacing.slider_width = 160.0;
        ui.add(
            egui::Slider::new(&mut state.emu.speed, 0.00001..=10.000)
                .step_by(0.00001)
                .logarithmic(true),
        );
    });

    ui.horizontal(|ui| {
        if ui
            .button(if state.emu.running { "⏸" } else { "⏵" })
            .clicked()
        {
            state.do_hotkey(Hotkey::PauseResume);
        }

        if ui.button("⟳").clicked() {
            state.do_hotkey(Hotkey::SoftReset);
        }

        if ui.button("Step").clicked() {
            state.do_hotkey(Hotkey::Step);
        }

        if ui.button("Next Frame").clicked() {
            state.do_hotkey(Hotkey::NextFrame);
        }
    });

    ui.separator();

    let mut disassembler = umesen_core::cpu::Disassembler::new(&state.emu.cpu);

    let frame = egui::Frame::canvas(ui.style()).inner_margin(6.0);
    egui::CollapsingHeader::new("Disassemble")
        .default_open(true)
        .show_unindented(ui, |ui| {
            frame.show(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(egui::RichText::new(disassembler.disassemble_lines(1)).strong());
                    ui.label(disassembler.disassemble_lines(31));
                    ui.allocate_space(egui::Vec2::new(ui.available_width(), 0.));
                });
            });
        });
}
