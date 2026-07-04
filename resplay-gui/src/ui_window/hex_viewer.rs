use std::fmt::Write;

#[derive(Default, Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum HexViewKind {
    #[default]
    Cpu,
    Ppu,
}

impl crate::egui_util::UiList for HexViewKind {
    fn pretty_name(&self) -> &'static str {
        match self {
            HexViewKind::Cpu => "CPU",
            HexViewKind::Ppu => "PPU",
        }
    }

    const LIST: &[Self] = &[Self::Cpu, Self::Ppu];
}

pub fn show(ui: &mut egui::Ui, state: &mut crate::State) {
    ui.style_mut().override_text_style = Some(egui::TextStyle::Monospace);
    let selected = crate::egui_util::ui_list_combo_select(ui);
    ui.add_space(4.);

    let bus = &state.emu.cpu.bus;
    match selected {
        HexViewKind::Cpu => show_hex_view(ui, |address| bus.peek_read(address), 0x1000),
        HexViewKind::Ppu => show_hex_view(
            ui,
            |address| bus.ppu.registers.bus.peek_read(address),
            0x400,
        ),
    }
}

/// Display the memory dump of a range of rows
/// Each row shows 16 bytes so 0..0x1000 shows the entire range
fn show_hex_view(ui: &mut egui::Ui, read_fn: impl Fn(u16) -> u8, total_rows: usize) {
    let frame = egui::Frame::canvas(ui.style()).inner_margin(6.0);
    frame.show(ui, |ui| {
        let row_height =
            ui.text_style_height(&egui::TextStyle::Monospace) - ui.spacing().item_spacing.y;
        egui::ScrollArea::vertical().show_rows(ui, row_height, total_rows, |ui, row_range| {
            for i in row_range {
                let line_address_start = i * 0x10;
                let mut output = format!("${line_address_start:04x}:");
                for i in 0..0x10 {
                    let byte = read_fn(line_address_start as u16 + i);
                    write!(output, " {byte:02x}").unwrap();
                }

                ui.label(output);
            }
        });
    });
}
