use eframe::egui::{self, Key};

fn physical_key_pressed(input: &egui::InputState, key: Key) -> bool {
    input.events.iter().any(|e| {
        matches!(e, egui::Event::Key {
            physical_key: Some(pk),
            pressed: true,
            ..
        } if *pk == key)
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyCombo {
    pub key: Key,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
}

impl KeyCombo {
    pub const fn new(key: Key) -> Self {
        Self {
            key,
            shift: false,
            ctrl: false,
            alt: false,
        }
    }

    pub const fn shift(mut self) -> Self {
        self.shift = true;
        self
    }

    pub const fn ctrl(mut self) -> Self {
        self.ctrl = true;
        self
    }

    pub const fn alt(mut self) -> Self {
        self.alt = true;
        self
    }

    pub fn label(self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Cmd");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(key_name(self.key));
        parts.join("+")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    PlayStop,
    PlayFromCursor,
    CursorUp,
    CursorDown,
    CursorLeft,
    CursorRight,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    SelectUp,
    SelectDown,
    SelectLeft,
    SelectRight,
    Delete,
    NoteOff,
    OctaveUp,
    OctaveDown,
    TransposeUp,
    TransposeDown,
    TransposeOctaveUp,
    TransposeOctaveDown,
    Escape,
    SwitchToEdit,
    SwitchToSynth,
    SynthUp,
    SynthDown,
    SynthIncrease,
    SynthDecrease,
    LoadSample,
    FillAscending,
    FillDescending,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub action: Action,
    pub combo: KeyCombo,
    pub title: &'static str,
    pub description: &'static str,
    pub category: &'static str,
}

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub bindings: Vec<Binding>,
}

impl KeyBindings {
    pub fn active_actions(&self, input: &egui::InputState) -> Vec<Action> {
        let shift = input.modifiers.shift;
        let ctrl = input.modifiers.command;
        let alt = input.modifiers.alt;

        self.bindings
            .iter()
            .filter(|b| {
                b.combo.shift == shift
                    && b.combo.ctrl == ctrl
                    && b.combo.alt == alt
                    && physical_key_pressed(input, b.combo.key)
            })
            .map(|b| b.action)
            .collect()
    }

    pub fn defaults() -> Self {
        Self {
            bindings: vec![
                Binding {
                    action: Action::PlayStop,
                    combo: KeyCombo::new(Key::Enter),
                    title: "Play / Stop",
                    description: "Toggle playback from the beginning",
                    category: "Global",
                },
                Binding {
                    action: Action::PlayFromCursor,
                    combo: KeyCombo::new(Key::Space),
                    title: "Play from cursor",
                    description: "Toggle playback starting at the cursor row",
                    category: "Global",
                },
                Binding {
                    action: Action::CursorUp,
                    combo: KeyCombo::new(Key::ArrowUp),
                    title: "Cursor up",
                    description: "Move cursor one row up",
                    category: "Edit",
                },
                Binding {
                    action: Action::CursorDown,
                    combo: KeyCombo::new(Key::ArrowDown),
                    title: "Cursor down",
                    description: "Move cursor one row down",
                    category: "Edit",
                },
                Binding {
                    action: Action::CursorLeft,
                    combo: KeyCombo::new(Key::ArrowLeft),
                    title: "Cursor left",
                    description: "Move cursor one channel left",
                    category: "Edit",
                },
                Binding {
                    action: Action::CursorRight,
                    combo: KeyCombo::new(Key::ArrowRight),
                    title: "Cursor right",
                    description: "Move cursor one channel right",
                    category: "Edit",
                },
                Binding {
                    action: Action::MoveUp,
                    combo: KeyCombo::new(Key::ArrowUp).alt(),
                    title: "Move up",
                    description: "Move note / selection one row up",
                    category: "Edit",
                },
                Binding {
                    action: Action::MoveDown,
                    combo: KeyCombo::new(Key::ArrowDown).alt(),
                    title: "Move down",
                    description: "Move note / selection one row down",
                    category: "Edit",
                },
                Binding {
                    action: Action::MoveLeft,
                    combo: KeyCombo::new(Key::ArrowLeft).alt(),
                    title: "Move left",
                    description: "Move note / selection one channel left",
                    category: "Edit",
                },
                Binding {
                    action: Action::MoveRight,
                    combo: KeyCombo::new(Key::ArrowRight).alt(),
                    title: "Move right",
                    description: "Move note / selection one channel right",
                    category: "Edit",
                },
                Binding {
                    action: Action::SelectUp,
                    combo: KeyCombo::new(Key::ArrowUp).shift(),
                    title: "Select up",
                    description: "Begin / extend selection upward",
                    category: "Edit",
                },
                Binding {
                    action: Action::SelectDown,
                    combo: KeyCombo::new(Key::ArrowDown).shift(),
                    title: "Select down",
                    description: "Begin / extend selection downward",
                    category: "Edit",
                },
                Binding {
                    action: Action::SelectLeft,
                    combo: KeyCombo::new(Key::ArrowLeft).shift(),
                    title: "Select left",
                    description: "Begin / extend selection left",
                    category: "Edit",
                },
                Binding {
                    action: Action::SelectRight,
                    combo: KeyCombo::new(Key::ArrowRight).shift(),
                    title: "Select right",
                    description: "Begin / extend selection right",
                    category: "Edit",
                },
                Binding {
                    action: Action::Delete,
                    combo: KeyCombo::new(Key::Delete),
                    title: "Delete",
                    description: "Clear note at cursor or selection",
                    category: "Edit",
                },
                Binding {
                    action: Action::Delete,
                    combo: KeyCombo::new(Key::Backspace),
                    title: "Backspace",
                    description: "Clear note at cursor or selection",
                    category: "Edit",
                },
                Binding {
                    action: Action::NoteOff,
                    combo: KeyCombo::new(Key::Tab),
                    title: "Note off",
                    description: "Insert a note-off marker",
                    category: "Edit",
                },
                Binding {
                    action: Action::OctaveUp,
                    combo: KeyCombo::new(Key::Period),
                    title: "Octave up",
                    description: "Raise the keyboard octave",
                    category: "Edit",
                },
                Binding {
                    action: Action::OctaveDown,
                    combo: KeyCombo::new(Key::Comma),
                    title: "Octave down",
                    description: "Lower the keyboard octave",
                    category: "Edit",
                },
                Binding {
                    action: Action::TransposeUp,
                    combo: KeyCombo::new(Key::Period).ctrl(),
                    title: "Transpose +1",
                    description: "Transpose note(s) up by one semitone",
                    category: "Edit",
                },
                Binding {
                    action: Action::TransposeDown,
                    combo: KeyCombo::new(Key::Comma).ctrl(),
                    title: "Transpose −1",
                    description: "Transpose note(s) down by one semitone",
                    category: "Edit",
                },
                Binding {
                    action: Action::TransposeOctaveUp,
                    combo: KeyCombo::new(Key::Period).ctrl().shift(),
                    title: "Transpose +12",
                    description: "Transpose note(s) up by one octave",
                    category: "Edit",
                },
                Binding {
                    action: Action::TransposeOctaveDown,
                    combo: KeyCombo::new(Key::Comma).ctrl().shift(),
                    title: "Transpose −12",
                    description: "Transpose note(s) down by one octave",
                    category: "Edit",
                },
                Binding {
                    action: Action::Escape,
                    combo: KeyCombo::new(Key::Escape),
                    title: "Escape",
                    description: "Clear selection, stop playback, or quit",
                    category: "Global",
                },
                Binding {
                    action: Action::SwitchToEdit,
                    combo: KeyCombo::new(Key::Num1).ctrl(),
                    title: "Pattern mode",
                    description: "Switch to the pattern editor",
                    category: "Mode",
                },
                Binding {
                    action: Action::SwitchToSynth,
                    combo: KeyCombo::new(Key::Num2).ctrl(),
                    title: "Synth mode",
                    description: "Switch to the synth editor",
                    category: "Mode",
                },
                Binding {
                    action: Action::SynthUp,
                    combo: KeyCombo::new(Key::ArrowUp),
                    title: "Field up",
                    description: "Move to the previous synth field",
                    category: "Synth",
                },
                Binding {
                    action: Action::SynthDown,
                    combo: KeyCombo::new(Key::ArrowDown),
                    title: "Field down",
                    description: "Move to the next synth field",
                    category: "Synth",
                },
                Binding {
                    action: Action::SynthIncrease,
                    combo: KeyCombo::new(Key::ArrowRight),
                    title: "Increase value",
                    description: "Increase the value of the selected field",
                    category: "Synth",
                },
                Binding {
                    action: Action::SynthDecrease,
                    combo: KeyCombo::new(Key::ArrowLeft),
                    title: "Decrease value",
                    description: "Decrease the value of the selected field",
                    category: "Synth",
                },
                Binding {
                    action: Action::LoadSample,
                    combo: KeyCombo::new(Key::L),
                    title: "Load sample",
                    description: "Open file dialog to load a sample",
                    category: "Sampler",
                },
                Binding {
                    action: Action::FillAscending,
                    combo: KeyCombo::new(Key::I).ctrl(),
                    title: "Fill ascending",
                    description: "Fill empty rows downward with ascending values",
                    category: "Edit",
                },
                Binding {
                    action: Action::FillDescending,
                    combo: KeyCombo::new(Key::D).ctrl(),
                    title: "Fill descending",
                    description: "Fill empty rows downward with descending values",
                    category: "Edit",
                },
            ],
        }
    }
}

const fn key_name(key: Key) -> &'static str {
    match key {
        Key::ArrowUp => "↑",
        Key::ArrowDown => "↓",
        Key::ArrowLeft => "←",
        Key::ArrowRight => "→",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Escape => "Esc",
        Key::Tab => "Tab",
        Key::Delete => "Del",
        Key::Backspace => "Backspace",
        Key::Period => ".",
        Key::Comma => ",",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::I => "I",
        Key::D => "D",
        Key::L => "L",
        _ => "?",
    }
}
