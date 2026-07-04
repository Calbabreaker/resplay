mod catridge_info;
mod debugger;
pub mod hex_viewer;
pub mod ppu_memory;
mod ppu_state;
mod preferences;
mod stats;

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, Hash, PartialEq, Eq)]
pub enum UiWindowKind {
    Debugger,
    HexViewer,
    PpuMemory,
    PpuState,
    Stats,
    Preferences,
    CatridgeInfo,
}

impl UiWindowKind {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Debugger => "Debugger",
            Self::HexViewer => "Hex Viewer",
            Self::Stats => "Stats",
            Self::CatridgeInfo => "Catridge Info",
            Self::PpuMemory => "Ppu Memory",
            Self::PpuState => "Ppu State",
            Self::Preferences => "Preferences",
        }
    }

    // Returns whether the window is still open
    pub fn show(
        &self,
        ctx: &egui::Context,
        state: &mut crate::State,
        preferences: &mut crate::Preferences,
    ) -> bool {
        let mut open = true;
        egui::Window::new(self.title())
            .pivot(egui::Align2::CENTER_CENTER)
            .min_width(200.)
            .open(&mut open)
            .show(ctx, |ui| match self {
                Self::Debugger => debugger::show(ui, state),
                Self::HexViewer => hex_viewer::show(ui, state),
                Self::PpuMemory => ppu_memory::show(ui, state),
                Self::Stats => stats::show(ui, state),
                Self::PpuState => ppu_state::show(ui, state),
                Self::Preferences => preferences::show(ui, preferences),
                Self::CatridgeInfo => catridge_info::show(ui, state),
            });

        open
    }
}
