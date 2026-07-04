use std::sync::LazyLock;

use umesen_core::controller::Button;

#[derive(PartialEq, Eq, serde::Serialize, serde::Deserialize, Clone, Hash, Debug, Copy)]
pub enum Hotkey {
    PauseResume,
    SoftReset,
    HardReset,
    Step,
    NextFrame,
    QuickSave,
    QuickLoad,
}

#[derive(PartialEq, Eq, serde::Serialize, serde::Deserialize, Clone, Hash, Debug, Copy)]
pub enum Action {
    Controller(u8, Button),
    Hotkey(Hotkey),
}

impl Action {
    pub fn name(&self) -> String {
        match self {
            Self::Controller(number, button) => {
                format!("Controller {number} {}", button.name())
            }
            Self::Hotkey(hotkey) => match hotkey {
                Hotkey::HardReset => "Hard reset".to_owned(),
                Hotkey::NextFrame => "Step next frame".to_owned(),
                Hotkey::PauseResume => "Pause/Resume".to_owned(),
                Hotkey::SoftReset => "Soft reset".to_owned(),
                Hotkey::Step => "Step Instruction".to_owned(),
                Hotkey::QuickSave => "Quick Save".to_owned(),
                Hotkey::QuickLoad => "Quick Load".to_owned(),
            },
        }
    }
}

type KeyActionMap = indexmap::IndexMap<Action, egui::KeyboardShortcut>;

#[derive(Default, serde::Deserialize, serde::Serialize)]
pub struct KeybindingMap {
    pub actions: KeyActionMap,
    /// The button to bind the next pressed key to
    #[serde(skip)]
    pub action_to_rebind: Option<Action>,
}

pub static DEFAULT_ACTION_MAP: LazyLock<KeyActionMap> = LazyLock::new(|| {
    use Hotkey::*;
    use egui::Key::*;
    let mapping = [
        (Action::Hotkey(PauseResume), F4),
        (Action::Hotkey(SoftReset), F5),
        (Action::Hotkey(HardReset), F6),
        (Action::Hotkey(Step), OpenBracket),
        (Action::Hotkey(QuickSave), W),
        (Action::Hotkey(QuickLoad), O),
        (Action::Hotkey(NextFrame), CloseBracket),
        (Action::Controller(0, Button::UP), I),
        (Action::Controller(0, Button::DOWN), K),
        (Action::Controller(0, Button::LEFT), J),
        (Action::Controller(0, Button::RIGHT), L),
        (Action::Controller(0, Button::A), C),
        (Action::Controller(0, Button::B), X),
        (Action::Controller(0, Button::START), S),
        (Action::Controller(0, Button::SELECT), D),
        (Action::Controller(1, Button::UP), ArrowUp),
        (Action::Controller(1, Button::DOWN), ArrowDown),
        (Action::Controller(1, Button::LEFT), ArrowLeft),
        (Action::Controller(1, Button::RIGHT), ArrowRight),
        (Action::Controller(1, Button::A), Slash),
        (Action::Controller(1, Button::B), Period),
        (Action::Controller(1, Button::SELECT), Quote),
        (Action::Controller(1, Button::START), Semicolon),
    ];
    KeyActionMap::from(mapping.map(|(action, key)| {
        (
            action,
            egui::KeyboardShortcut::new(egui::Modifiers::NONE, key),
        )
    }))
});

impl KeybindingMap {
    pub fn check_key_down(&mut self, input: &egui::InputState) {
        if let Some(key) = input.keys_down.iter().next()
            && let Some(action) = self.action_to_rebind.take()
        {
            self.actions
                .insert(action, egui::KeyboardShortcut::new(input.modifiers, *key));
        }
    }

    pub fn iter_map(&self) -> impl Iterator<Item = (Action, egui::KeyboardShortcut)> {
        DEFAULT_ACTION_MAP
            .iter()
            .map(|(action, shortcut)| (*action, *self.actions.get(action).unwrap_or(shortcut)))
    }
}
