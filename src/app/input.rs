use eframe::egui::{self, Key};

use crate::app::keybindings::Action;
use crate::app::scale::{Scale, map_key_index_to_midi};
use crate::project::Note;
use crate::project::{Cell, SampleData};

use super::{App, Mode, SettingsField, SubColumn, SynthSettingsField};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        ctx.input(|input| {
            let actions = self.keybindings.active_actions(input);

            if actions.contains(&Action::PlayStop) {
                if self.playback.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(false);
                }
                return false;
            }

            if actions.contains(&Action::PlayFromCursor) {
                if self.playback.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(true);
                }
                return false;
            }

            match self.mode {
                Mode::Edit => self.handle_edit_input(input, &actions),
                Mode::Settings => {
                    self.handle_settings_input(&actions);
                    false
                }
                Mode::SynthEdit => {
                    self.handle_synth_input(&actions);
                    false
                }
            }
        })
    }

    fn handle_edit_input(&mut self, input: &egui::InputState, actions: &[Action]) -> bool {
        if actions.contains(&Action::SwitchToSynth) {
            self.clear_selection();
            self.mode = Mode::SynthEdit;
            self.synth_field = SynthSettingsField::Instrument;
            return false;
        }

        if actions.contains(&Action::SwitchToSettings) {
            self.clear_selection();
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Scale;
            return false;
        }

        if self.handle_move(actions) {
            return false;
        }

        if self.handle_cursor_and_select(actions) {
            return false;
        }

        if actions.contains(&Action::Delete) {
            self.handle_delete();
            return false;
        }

        if actions.contains(&Action::NoteOff) && self.cursor.sub_column == SubColumn::Note {
            self.handle_note_off();
            return false;
        }

        if self.handle_transpose(actions) {
            return false;
        }

        if actions.contains(&Action::OctaveUp) {
            if self.cursor.octave < 8 {
                self.cursor.octave += 1;
            }
            return false;
        }

        if actions.contains(&Action::OctaveDown) {
            if self.cursor.octave > 0 {
                self.cursor.octave -= 1;
            }
            return false;
        }

        if actions.contains(&Action::Escape) {
            if self.cursor.selection_anchor.is_some() {
                self.clear_selection();
            } else if self.playback.playing {
                self.stop_playback();
            } else {
                return true;
            }
            return false;
        }

        if self.cursor.sub_column == SubColumn::Effect {
            self.handle_effect_keys(input);
        } else if self.cursor.sub_column == SubColumn::Volume {
            self.handle_volume_keys(input);
        } else if self.cursor.sub_column == SubColumn::Instrument {
            self.handle_instrument_keys(input);
        } else {
            self.handle_note_keys(input);
        }

        false
    }

    fn handle_move(&mut self, actions: &[Action]) -> bool {
        let move_action = [
            Action::MoveUp,
            Action::MoveDown,
            Action::MoveLeft,
            Action::MoveRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        let Some(dir) = move_action else {
            return false;
        };

        let (dr, dc): (isize, isize) = match dir {
            Action::MoveUp => (-1, 0),
            Action::MoveDown => (1, 0),
            Action::MoveLeft => (0, -1),
            Action::MoveRight => (0, 1),
            _ => unreachable!(),
        };

        let on_note = self.cursor.sub_column;

        if let Some((min_ch, max_ch, min_row, max_row)) = self.selection_bounds() {
            let in_bounds = min_row.checked_add_signed(dr).is_some()
                && max_row
                    .checked_add_signed(dr)
                    .is_some_and(|r| r < self.project.current_pattern().rows)
                && min_ch.checked_add_signed(dc).is_some()
                && max_ch
                    .checked_add_signed(dc)
                    .is_some_and(|c| c < self.project.current_pattern().channels);

            if in_bounds {
                let mut cells = Vec::new();
                for ch in min_ch..=max_ch {
                    for row in min_row..=max_row {
                        let cell = self.project.current_pattern().get(ch, row);
                        let inst = self.project.current_pattern().get_instrument(ch, row);
                        let vol = self.project.current_pattern().get_volume(ch, row);
                        let fx = self.project.current_pattern().get_effect(ch, row);
                        cells.push((ch, row, cell, inst, vol, fx));
                        match on_note {
                            SubColumn::Note => self.project.current_pattern_mut().clear(ch, row),
                            SubColumn::Instrument => {
                                self.project.current_pattern_mut().clear_instrument(ch, row)
                            }
                            SubColumn::Volume => {
                                self.project.current_pattern_mut().clear_volume(ch, row)
                            }
                            SubColumn::Effect => {
                                self.project.current_pattern_mut().clear_effect(ch, row)
                            }
                        }
                    }
                }
                for (ch, row, cell, inst, vol, fx) in cells {
                    let new_ch = ch.checked_add_signed(dc).unwrap();
                    let new_row = row.checked_add_signed(dr).unwrap();
                    match on_note {
                        SubColumn::Note => self
                            .project
                            .current_pattern_mut()
                            .set(new_ch, new_row, cell),
                        SubColumn::Instrument => self
                            .project
                            .current_pattern_mut()
                            .set_instrument(new_ch, new_row, inst),
                        SubColumn::Volume => self
                            .project
                            .current_pattern_mut()
                            .set_volume(new_ch, new_row, vol),
                        SubColumn::Effect => self
                            .project
                            .current_pattern_mut()
                            .set_effect(new_ch, new_row, fx),
                    }
                }
                self.cursor.channel = self.cursor.channel.checked_add_signed(dc).unwrap();
                self.cursor.row = self.cursor.row.checked_add_signed(dr).unwrap();
                if let Some((ach, arow)) = self.cursor.selection_anchor.as_mut() {
                    *ach = ach.checked_add_signed(dc).unwrap();
                    *arow = arow.checked_add_signed(dr).unwrap();
                }
            }
        } else if let (Some(new_row), Some(new_ch)) = (
            self.cursor.row.checked_add_signed(dr),
            self.cursor.channel.checked_add_signed(dc),
        ) && new_row < self.project.current_pattern().rows
            && new_ch < self.project.current_pattern().channels
        {
            if on_note == SubColumn::Note {
                let cell = self
                    .project
                    .current_pattern()
                    .get(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set(new_ch, new_row, cell);
            } else if on_note == SubColumn::Instrument {
                let inst = self
                    .project
                    .current_pattern()
                    .get_instrument(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_instrument(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_instrument(new_ch, new_row, inst);
            } else if on_note == SubColumn::Volume {
                let vol = self
                    .project
                    .current_pattern()
                    .get_volume(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_volume(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_volume(new_ch, new_row, vol);
            } else {
                let fx = self
                    .project
                    .current_pattern()
                    .get_effect(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_effect(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_effect(new_ch, new_row, fx);
            }
            self.cursor.channel = new_ch;
            self.cursor.row = new_row;
        }

        true
    }

    fn handle_cursor_and_select(&mut self, actions: &[Action]) -> bool {
        let select_action = [
            Action::SelectUp,
            Action::SelectDown,
            Action::SelectLeft,
            Action::SelectRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if select_action.is_some() && self.cursor.selection_anchor.is_none() {
            self.cursor.selection_anchor = Some((self.cursor.channel, self.cursor.row));
        }

        let cursor_action = [
            Action::CursorUp,
            Action::CursorDown,
            Action::CursorLeft,
            Action::CursorRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if let Some(dir) = cursor_action {
            if select_action.is_none() {
                self.clear_selection();
            }
            self.move_cursor(dir);
            return true;
        }

        if let Some(dir) = select_action {
            self.move_cursor(dir);
            return true;
        }

        false
    }

    fn move_cursor(&mut self, dir: Action) {
        match dir {
            Action::CursorUp | Action::SelectUp => {
                if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                } else {
                    self.cursor.row = self.project.current_pattern().rows - 1;
                }
            }
            Action::CursorDown | Action::SelectDown => {
                if self.cursor.row < self.project.current_pattern().rows - 1 {
                    self.cursor.row += 1;
                } else {
                    self.cursor.row = 0;
                }
            }
            Action::CursorLeft => {
                if self.cursor.sub_column == SubColumn::Effect {
                    self.cursor.sub_column = SubColumn::Volume;
                    self.cursor.volume_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Instrument;
                    self.cursor.instrument_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Note;
                } else if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                } else {
                    self.cursor.channel = self.project.current_pattern().channels - 1;
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                }
            }
            Action::CursorRight => {
                if self.cursor.sub_column == SubColumn::Note {
                    self.cursor.sub_column = SubColumn::Instrument;
                    self.cursor.instrument_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Volume;
                    self.cursor.volume_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                } else if self.cursor.channel < self.project.current_pattern().channels - 1 {
                    self.cursor.channel += 1;
                    self.cursor.sub_column = SubColumn::Note;
                } else {
                    self.cursor.channel = 0;
                    self.cursor.sub_column = SubColumn::Note;
                }
            }
            Action::SelectLeft => {
                if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                } else {
                    self.cursor.channel = self.project.current_pattern().channels - 1;
                }
            }
            Action::SelectRight => {
                if self.cursor.channel < self.project.current_pattern().channels - 1 {
                    self.cursor.channel += 1;
                } else {
                    self.cursor.channel = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_delete(&mut self) {
        if let Some((min_ch, max_ch, min_row, max_row)) = self.selection_bounds() {
            for ch in min_ch..=max_ch {
                for row in min_row..=max_row {
                    match self.cursor.sub_column {
                        SubColumn::Note => self.project.current_pattern_mut().clear(ch, row),
                        SubColumn::Instrument => {
                            self.project.current_pattern_mut().clear_instrument(ch, row)
                        }
                        SubColumn::Volume => {
                            self.project.current_pattern_mut().clear_volume(ch, row)
                        }
                        SubColumn::Effect => {
                            self.project.current_pattern_mut().clear_effect(ch, row)
                        }
                    }
                }
            }
            self.clear_selection();
        } else {
            match self.cursor.sub_column {
                SubColumn::Note => {
                    self.project
                        .current_pattern_mut()
                        .clear(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Instrument => {
                    self.project
                        .current_pattern_mut()
                        .clear_instrument(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Volume => {
                    self.project
                        .current_pattern_mut()
                        .clear_volume(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Effect => {
                    self.project
                        .current_pattern_mut()
                        .clear_effect(self.cursor.channel, self.cursor.row);
                }
            }
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.current_pattern().rows;
        }
    }

    fn handle_note_off(&mut self) {
        self.clear_selection();
        self.project
            .current_pattern_mut()
            .set(self.cursor.channel, self.cursor.row, Cell::NoteOff);
        self.advance_cursor();
    }

    fn handle_transpose(&mut self, actions: &[Action]) -> bool {
        let delta: i16 = if actions.contains(&Action::TransposeOctaveUp) {
            12
        } else if actions.contains(&Action::TransposeOctaveDown) {
            -12
        } else if actions.contains(&Action::TransposeUp) {
            1
        } else if actions.contains(&Action::TransposeDown) {
            -1
        } else {
            return false;
        };

        let (min_ch, max_ch, min_row, max_row) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.row,
            self.cursor.row,
        ));

        let mut min_pitch: Option<u8> = None;
        let mut max_pitch: Option<u8> = None;
        for ch in min_ch..=max_ch {
            for row in min_row..=max_row {
                if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, row) {
                    min_pitch = Some(min_pitch.map_or(note.pitch, |p: u8| p.min(note.pitch)));
                    max_pitch = Some(max_pitch.map_or(note.pitch, |p: u8| p.max(note.pitch)));
                }
            }
        }

        let can_transpose = if delta > 0 {
            max_pitch.is_some_and(|p| (i16::from(p) + delta) <= 127)
        } else {
            min_pitch.is_some_and(|p| (i16::from(p) + delta) >= 0)
        };

        if can_transpose {
            for ch in min_ch..=max_ch {
                for row in min_row..=max_row {
                    if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, row) {
                        let new_pitch = u8::try_from(i16::from(note.pitch) + delta).unwrap();
                        self.project.current_pattern_mut().set(
                            ch,
                            row,
                            Cell::NoteOn(crate::project::Note::new(new_pitch)),
                        );
                    }
                }
            }
        }

        true
    }

    fn handle_note_keys(&mut self, input: &egui::InputState) {
        let note_keys = [
            Key::Z,
            Key::X,
            Key::C,
            Key::V,
            Key::B,
            Key::N,
            Key::M,
            Key::A,
            Key::S,
            Key::D,
            Key::F,
            Key::G,
            Key::H,
            Key::J,
            Key::K,
            Key::L,
            Key::Q,
            Key::W,
            Key::E,
            Key::R,
            Key::T,
            Key::Y,
            Key::U,
            Key::I,
            Key::O,
            Key::P,
        ];
        for &k in &note_keys {
            if input.key_pressed(k) {
                let scale = self.project.scale_index.scale();
                if let Some(note) =
                    key_to_note(k, self.cursor.octave, scale, self.project.transpose)
                {
                    self.project.current_pattern_mut().set(
                        self.cursor.channel,
                        self.cursor.row,
                        Cell::NoteOn(note),
                    );
                    if !self.playback.playing {
                        self.audio.preview_note(
                            note.frequency(),
                            self.current_instrument,
                            &self.project.instruments,
                            self.project.master_volume_linear(),
                        );
                    }
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn advance_cursor(&mut self) {
        if self.cursor.row < self.project.current_pattern().rows - 1
            && self.cursor.row + self.project.step < self.project.current_pattern().rows
        {
            self.cursor.row += self.project.step;
        } else {
            self.cursor.row = self.project.current_pattern().rows - 1;
        }
    }

    fn handle_effect_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.effect_edit_pos;

                let mut fx = self
                    .project
                    .current_pattern()
                    .get_effect(ch, row)
                    .unwrap_or(crate::project::Effect { kind: 0, param: 0 });

                match pos {
                    0 => fx.kind = value,
                    1 => fx.param = (fx.param & 0x0F) | (value << 4),
                    _ => fx.param = (fx.param & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_effect(ch, row, Some(fx));

                if pos < 2 {
                    self.cursor.effect_edit_pos = pos + 1;
                } else {
                    self.cursor.effect_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_volume_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.volume_edit_pos;

                let mut vol = self
                    .project
                    .current_pattern()
                    .get_volume(ch, row)
                    .unwrap_or(0);

                match pos {
                    0 => vol = (value << 4) | (vol & 0x0F),
                    _ => vol = (vol & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_volume(ch, row, Some(vol));

                if pos < 1 {
                    self.cursor.volume_edit_pos = pos + 1;
                } else {
                    self.cursor.volume_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_instrument_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.instrument_edit_pos;

                let mut inst = self
                    .project
                    .current_pattern()
                    .get_instrument(ch, row)
                    .unwrap_or(0);

                match pos {
                    0 => inst = (value << 4) | (inst & 0x0F),
                    _ => inst = (inst & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_instrument(ch, row, Some(inst));

                if pos < 1 {
                    self.cursor.instrument_edit_pos = pos + 1;
                } else {
                    self.cursor.instrument_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_mode_switch(&mut self, actions: &[Action]) -> bool {
        if actions.contains(&Action::Escape) {
            if self.playback.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
            return true;
        }
        if actions.contains(&Action::SwitchToEdit) {
            self.mode = Mode::Edit;
            return true;
        }
        false
    }

    fn handle_settings_input(&mut self, actions: &[Action]) {
        if self.handle_mode_switch(actions) {
            return;
        }

        if actions.contains(&Action::SwitchToSynth) {
            self.mode = Mode::SynthEdit;
            self.synth_field = SynthSettingsField::Instrument;
        } else if actions.contains(&Action::SwitchToSettings) {
        } else if actions.contains(&Action::SettingsDown) {
            self.settings_field = self.settings_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.settings_field = self.settings_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            self.settings_field
                .adjust(&mut self.project, &mut self.cursor.row);
        } else if actions.contains(&Action::SettingsDecrease) {
            self.settings_field
                .adjust_down(&mut self.project, &mut self.cursor.row);
        }
    }

    fn handle_synth_input(&mut self, actions: &[Action]) {
        if self.handle_mode_switch(actions) {
            return;
        }

        if actions.contains(&Action::SwitchToSynth) {
        } else if actions.contains(&Action::SwitchToSettings) {
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Scale;
        } else if actions.contains(&Action::SettingsDown) {
            self.synth_field = self.synth_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.synth_field = self.synth_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            if self.synth_field == SynthSettingsField::Instrument {
                self.current_instrument =
                    (self.current_instrument + 1) % self.project.instruments.len();
            } else {
                let idx = self.current_instrument;
                self.synth_field
                    .adjust(&mut self.project.instruments[idx], 1);
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            if self.synth_field == SynthSettingsField::Instrument {
                if self.current_instrument == 0 {
                    self.current_instrument = self.project.instruments.len() - 1;
                } else {
                    self.current_instrument -= 1;
                }
            } else {
                let idx = self.current_instrument;
                self.synth_field
                    .adjust(&mut self.project.instruments[idx], -1);
            }
        } else if actions.contains(&Action::LoadSample) {
            self.load_sample_for_instrument(self.current_instrument);
        }
    }

    fn load_sample_for_instrument(&mut self, inst_idx: usize) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Audio Files", &["wav"])
            .set_title("Load Sample");

        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }

        if let Some(path) = dialog.pick_file()
            && let Ok(data) = SampleData::load_from_path(&path)
        {
            self.project.instruments[inst_idx].sample_data = data;
        }
    }
}

pub fn key_to_note(key: Key, octave: u8, scale: &Scale, transpose: i8) -> Option<Note> {
    let idx: Option<u8> = match key {
        Key::Z => Some(0),
        Key::X => Some(1),
        Key::C => Some(2),
        Key::V => Some(3),
        Key::B => Some(4),
        Key::N => Some(5),
        Key::M => Some(6),
        Key::A => Some(7),
        Key::S => Some(8),
        Key::D => Some(9),
        Key::F => Some(10),
        Key::G => Some(11),

        Key::H => Some(12),
        Key::J => Some(13),
        Key::K => Some(14),
        Key::L => Some(15),
        Key::Q => Some(16),
        Key::W => Some(17),
        Key::E => Some(18),
        Key::R => Some(19),
        Key::T => Some(20),
        Key::Y => Some(21),
        Key::U => Some(22),
        Key::I => Some(23),
        Key::O => Some(24),
        Key::P => Some(25),

        _ => None,
    };

    idx.map(|i| {
        let midi = map_key_index_to_midi(i, octave, scale, transpose);
        Note::new(midi)
    })
}
