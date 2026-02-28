use eframe::egui::{self, Key};

use crate::keybindings::Action;
use crate::keys::key_to_note;
use crate::project::Cell;

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
            self.cursor.synth_channel = self.cursor.channel;
            self.synth_field = SynthSettingsField::Channel;
            return false;
        }

        if actions.contains(&Action::SwitchToSettings) {
            self.clear_selection();
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
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
        } else if actions.contains(&Action::NoteOff) {
            if self.cursor.sub_column == SubColumn::Note {
                self.handle_note_off();
            }
        } else if self.handle_transpose(actions) {
        } else if actions.contains(&Action::OctaveUp) {
            if self.cursor.octave < 8 {
                self.cursor.octave += 1;
            }
        } else if actions.contains(&Action::OctaveDown) {
            if self.cursor.octave > 0 {
                self.cursor.octave -= 1;
            }
        } else if actions.contains(&Action::Escape) {
            if self.cursor.selection_anchor.is_some() {
                self.clear_selection();
            } else if self.playback.playing {
                self.stop_playback();
            } else {
                return true;
            }
        } else if self.cursor.sub_column == SubColumn::Effect {
            self.handle_effect_keys(input);
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

        let on_note = self.cursor.sub_column == SubColumn::Note;

        if let Some((min_ch, max_ch, min_row, max_row)) = self.selection_bounds() {
            let in_bounds = min_row.checked_add_signed(dr).is_some()
                && max_row
                    .checked_add_signed(dr)
                    .is_some_and(|r| r < self.project.pattern.rows)
                && min_ch.checked_add_signed(dc).is_some()
                && max_ch
                    .checked_add_signed(dc)
                    .is_some_and(|c| c < self.project.pattern.channels);

            if in_bounds {
                let mut cells = Vec::new();
                for ch in min_ch..=max_ch {
                    for row in min_row..=max_row {
                        let cell = self.project.pattern.get(ch, row);
                        let fx = self.project.pattern.get_effect(ch, row);
                        cells.push((ch, row, cell, fx));
                        if on_note {
                            self.project.pattern.clear(ch, row);
                        } else {
                            self.project.pattern.clear_effect(ch, row);
                        }
                    }
                }
                for (ch, row, cell, fx) in cells {
                    let new_ch = ch.checked_add_signed(dc).unwrap();
                    let new_row = row.checked_add_signed(dr).unwrap();
                    if on_note {
                        self.project.pattern.set(new_ch, new_row, cell);
                    } else {
                        self.project.pattern.set_effect(new_ch, new_row, fx);
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
        ) && new_row < self.project.pattern.rows
            && new_ch < self.project.pattern.channels
        {
            if on_note {
                let cell = self
                    .project
                    .pattern
                    .get(self.cursor.channel, self.cursor.row);
                self.project
                    .pattern
                    .clear(self.cursor.channel, self.cursor.row);
                self.project.pattern.set(new_ch, new_row, cell);
            } else {
                let fx = self
                    .project
                    .pattern
                    .get_effect(self.cursor.channel, self.cursor.row);
                self.project
                    .pattern
                    .clear_effect(self.cursor.channel, self.cursor.row);
                self.project.pattern.set_effect(new_ch, new_row, fx);
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
                    self.cursor.row = self.project.pattern.rows - 1;
                }
            }
            Action::CursorDown | Action::SelectDown => {
                if self.cursor.row < self.project.pattern.rows - 1 {
                    self.cursor.row += 1;
                } else {
                    self.cursor.row = 0;
                }
            }
            Action::CursorLeft => {
                if self.cursor.sub_column == SubColumn::Effect {
                    self.cursor.sub_column = SubColumn::Note;
                } else if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                    self.cursor.sub_column = SubColumn::Effect;
                } else {
                    self.cursor.channel = self.project.pattern.channels - 1;
                    self.cursor.sub_column = SubColumn::Effect;
                }
                self.cursor.effect_edit_pos = 0;
            }
            Action::CursorRight => {
                if self.cursor.sub_column == SubColumn::Note {
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                } else if self.cursor.channel < self.project.pattern.channels - 1 {
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
                    self.cursor.channel = self.project.pattern.channels - 1;
                }
            }
            Action::SelectRight => {
                if self.cursor.channel < self.project.pattern.channels - 1 {
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
                    if self.cursor.sub_column == SubColumn::Note {
                        self.project.pattern.clear(ch, row);
                    } else {
                        self.project.pattern.clear_effect(ch, row);
                    }
                }
            }
            self.clear_selection();
        } else if self.cursor.sub_column == SubColumn::Effect {
            self.project
                .pattern
                .clear_effect(self.cursor.channel, self.cursor.row);
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.pattern.rows;
        } else {
            self.project
                .pattern
                .clear(self.cursor.channel, self.cursor.row);
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.pattern.rows;
        }
    }

    fn handle_note_off(&mut self) {
        self.clear_selection();
        self.project
            .pattern
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
                if let Cell::NoteOn(note) = self.project.pattern.get(ch, row) {
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
                    if let Cell::NoteOn(note) = self.project.pattern.get(ch, row) {
                        let new_pitch = u8::try_from(i16::from(note.pitch) + delta).unwrap();
                        self.project.pattern.set(
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
                    self.project.pattern.set(
                        self.cursor.channel,
                        self.cursor.row,
                        Cell::NoteOn(note),
                    );
                    if !self.playback.playing {
                        self.audio.preview_note(
                            note.frequency(),
                            self.cursor.channel,
                            &self.project.channel_settings,
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

    const fn advance_cursor(&mut self) {
        if self.cursor.row < self.project.pattern.rows - 1
            && self.cursor.row + self.project.step < self.project.pattern.rows
        {
            self.cursor.row += self.project.step;
        } else {
            self.cursor.row = self.project.pattern.rows - 1;
        }
    }

    fn handle_effect_keys(&mut self, input: &egui::InputState) {
        let effect_keys = [
            (Key::A, b'A'),
            (Key::B, b'B'),
            (Key::C, b'C'),
            (Key::D, b'D'),
            (Key::E, b'E'),
            (Key::F, b'F'),
            (Key::G, b'G'),
            (Key::H, b'H'),
            (Key::I, b'I'),
            (Key::J, b'J'),
            (Key::K, b'K'),
            (Key::L, b'L'),
            (Key::M, b'M'),
            (Key::N, b'N'),
            (Key::O, b'O'),
            (Key::P, b'P'),
            (Key::Q, b'Q'),
            (Key::R, b'R'),
            (Key::S, b'S'),
            (Key::T, b'T'),
            (Key::U, b'U'),
            (Key::V, b'V'),
            (Key::W, b'W'),
            (Key::X, b'X'),
            (Key::Y, b'Y'),
            (Key::Z, b'Z'),
            (Key::Num0, b'0'),
            (Key::Num1, b'1'),
            (Key::Num2, b'2'),
            (Key::Num3, b'3'),
            (Key::Num4, b'4'),
            (Key::Num5, b'5'),
            (Key::Num6, b'6'),
            (Key::Num7, b'7'),
            (Key::Num8, b'8'),
            (Key::Num9, b'9'),
        ];

        for &(key, byte) in &effect_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.effect_edit_pos;

                let mut cmd = self
                    .project
                    .pattern
                    .get_effect(ch, row)
                    .unwrap_or([b'.', b'.', b'.', b'.']);
                cmd[pos] = byte;
                self.project.pattern.set_effect(ch, row, Some(cmd));

                if pos < 3 {
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

    fn handle_settings_input(&mut self, actions: &[Action]) {
        if actions.contains(&Action::Escape) {
            if self.playback.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToEdit) {
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToSynth) {
            self.mode = Mode::SynthEdit;
            self.cursor.synth_channel = self.cursor.channel;
            self.synth_field = SynthSettingsField::Channel;
        } else if actions.contains(&Action::SwitchToSettings) {
        } else if actions.contains(&Action::SettingsDown) {
            self.settings_field = self.settings_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.settings_field = self.settings_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.project.subdivision = (self.project.subdivision + 1).min(64);
                }
                SettingsField::Step => {
                    self.project.step = (self.project.step + 1).min(64);
                }
                SettingsField::Bpm => {
                    self.project.bpm = (self.project.bpm + 1).min(666);
                }
                SettingsField::PatternLength => {
                    let new_len = (self.project.pattern.rows + 1).min(128);
                    self.project.pattern.resize(new_len);
                }
                SettingsField::Scale => {
                    self.project.scale_index = self.project.scale_index.next();
                }
                SettingsField::Transpose => {
                    self.project.transpose = (self.project.transpose + 1).min(12);
                }
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.project.subdivision = self.project.subdivision.saturating_sub(1).max(2);
                }
                SettingsField::Step => {
                    self.project.step = self.project.step.saturating_sub(1).max(1);
                }
                SettingsField::Bpm => {
                    self.project.bpm = self.project.bpm.saturating_sub(1).max(20);
                }
                SettingsField::PatternLength => {
                    let new_len = self.project.pattern.rows.saturating_sub(1).max(1);
                    self.project.pattern.resize(new_len);
                    if self.cursor.row >= self.project.pattern.rows {
                        self.cursor.row = self.project.pattern.rows - 1;
                    }
                }
                SettingsField::Scale => {
                    self.project.scale_index = self.project.scale_index.prev();
                }
                SettingsField::Transpose => {
                    self.project.transpose = (self.project.transpose - 1).max(-12);
                }
            }
        }
    }

    fn handle_synth_input(&mut self, actions: &[Action]) {
        let ch = self.cursor.synth_channel;

        if actions.contains(&Action::Escape) {
            if self.playback.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToEdit) {
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToSynth) {
        } else if actions.contains(&Action::SwitchToSettings) {
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
        } else if actions.contains(&Action::SettingsDown) {
            self.synth_field = self.synth_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.synth_field = self.synth_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            match self.synth_field {
                SynthSettingsField::Channel => {
                    self.cursor.synth_channel =
                        (self.cursor.synth_channel + 1) % self.project.pattern.channels;
                }
                SynthSettingsField::Waveform => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.waveform = cs.waveform.next();
                }
                SynthSettingsField::Attack => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.attack = (cs.envelope.attack + 0.005).min(2.0);
                }
                SynthSettingsField::Decay => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.decay = (cs.envelope.decay + 0.005).min(2.0);
                }
                SynthSettingsField::Sustain => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.sustain = (cs.envelope.sustain + 0.05).min(1.0);
                }
                SynthSettingsField::Release => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.release = (cs.envelope.release + 0.005).min(2.0);
                }
                SynthSettingsField::Volume => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.volume = (cs.volume + 0.05).min(1.0);
                }
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            match self.synth_field {
                SynthSettingsField::Channel => {
                    self.cursor.synth_channel = if self.cursor.synth_channel == 0 {
                        self.project.pattern.channels - 1
                    } else {
                        self.cursor.synth_channel - 1
                    };
                }
                SynthSettingsField::Waveform => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.waveform = cs.waveform.prev();
                }
                SynthSettingsField::Attack => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.attack = (cs.envelope.attack - 0.005).max(0.001);
                }
                SynthSettingsField::Decay => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.decay = (cs.envelope.decay - 0.005).max(0.001);
                }
                SynthSettingsField::Sustain => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.sustain = (cs.envelope.sustain - 0.05).max(0.0);
                }
                SynthSettingsField::Release => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.envelope.release = (cs.envelope.release - 0.005).max(0.001);
                }
                SynthSettingsField::Volume => {
                    let cs = &mut self.project.channel_settings[ch];
                    cs.volume = (cs.volume - 0.05).max(0.0);
                }
            }
        }
    }
}
