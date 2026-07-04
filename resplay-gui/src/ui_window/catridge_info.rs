pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    let Some(catridge) = state.emu.cartridge() else {
        ui.label("No catridge attached");
        return;
    };

    ui.label(format!("Mapper ID: {:?}", catridge.header().mapper_id));
    ui.label(format!(
        "PRG ROM size: {:?}",
        catridge.header().prg_rom_size
    ));
    ui.label(format!(
        "PRG RAM size: {:?}",
        catridge.header().prg_ram_size
    ));
    ui.label(format!(
        "CHR {} size: {:?}",
        if catridge.header().chr_mem_is_rom {
            "ROM"
        } else {
            "RAM"
        },
        catridge.header().chr_mem_size
    ));
    ui.label(format!("Mirroring: {:?}", catridge.mirroring()));

    ui.label("Mapper state: ");
    egui::Frame::canvas(ui.style())
        .inner_margin(6.0)
        .show(ui, |ui| ui.label(catridge.debug_mapper()));
}
