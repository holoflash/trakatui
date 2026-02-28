use eframe::egui::{self, Key};

use crate::keybindings::Action;
use crate::keys::key_to_note;
use crate::pattern::Cell;

use super::{App, Mode, SettingsField, SynthSettingsField};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        ctx.input(|input| {
            let actions = self.keybindings.active_actions(input);

            if actions.contains(&Action::PlayStop) {
                if self.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(false);
                }
                return false;
            }

            if actions.contains(&Action::PlayFromCursor) {
                if self.playing {
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
            self.synth_channel = self.cursor_channel;
            self.synth_field = SynthSettingsField::Channel;
            return false;
        }

        if actions.contains(&Action::SwitchToSettings) {
            self.clear_selection();
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
            return false;
        }

        let move_action = [
            Action::MoveUp,
            Action::MoveDown,
            Action::MoveLeft,
            Action::MoveRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if let Some(dir) = move_action {
            let (dr, dc): (isize, isize) = match dir {
                Action::MoveUp => (-1, 0),
                Action::MoveDown => (1, 0),
                Action::MoveLeft => (0, -1),
                Action::MoveRight => (0, 1),
                _ => unreachable!(),
            };

            if let Some((min_ch, max_ch, min_row, max_row)) = self.selection_bounds() {
                let in_bounds = min_row.checked_add_signed(dr).is_some()
                    && max_row
                        .checked_add_signed(dr)
                        .is_some_and(|r| r < self.pattern.rows)
                    && min_ch.checked_add_signed(dc).is_some()
                    && max_ch
                        .checked_add_signed(dc)
                        .is_some_and(|c| c < self.pattern.channels);

                if in_bounds {
                    let mut cells = Vec::new();
                    for ch in min_ch..=max_ch {
                        for row in min_row..=max_row {
                            cells.push((ch, row, self.pattern.get(ch, row)));
                            self.pattern.clear(ch, row);
                        }
                    }
                    for (ch, row, cell) in cells {
                        let new_ch = ch.checked_add_signed(dc).unwrap();
                        let new_row = row.checked_add_signed(dr).unwrap();
                        self.pattern.set(new_ch, new_row, cell);
                    }
                    self.cursor_channel = self.cursor_channel.checked_add_signed(dc).unwrap();
                    self.cursor_row = self.cursor_row.checked_add_signed(dr).unwrap();
                    if let Some((ach, arow)) = self.selection_anchor.as_mut() {
                        *ach = ach.checked_add_signed(dc).unwrap();
                        *arow = arow.checked_add_signed(dr).unwrap();
                    }
                }
            } else if let (Some(new_row), Some(new_ch)) = (
                self.cursor_row.checked_add_signed(dr),
                self.cursor_channel.checked_add_signed(dc),
            ) && new_row < self.pattern.rows
                && new_ch < self.pattern.channels
            {
                let cell = self.pattern.get(self.cursor_channel, self.cursor_row);
                self.pattern.clear(self.cursor_channel, self.cursor_row);
                self.pattern.set(new_ch, new_row, cell);
                self.cursor_channel = new_ch;
                self.cursor_row = new_row;
            }
            return false;
        }

        let select_action = [
            Action::SelectUp,
            Action::SelectDown,
            Action::SelectLeft,
            Action::SelectRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if select_action.is_some() && self.selection_anchor.is_none() {
            self.selection_anchor = Some((self.cursor_channel, self.cursor_row));
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

            match dir {
                Action::CursorUp => {
                    if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                    } else {
                        self.cursor_row = self.pattern.rows - 1;
                    }
                }
                Action::CursorDown => {
                    if self.cursor_row < self.pattern.rows - 1 {
                        self.cursor_row += 1;
                    } else {
                        self.cursor_row = 0;
                    }
                }
                Action::CursorLeft => {
                    if self.cursor_channel > 0 {
                        self.cursor_channel -= 1;
                    } else {
                        self.cursor_channel = self.pattern.channels - 1;
                    }
                }
                Action::CursorRight => {
                    if self.cursor_channel < self.pattern.channels - 1 {
                        self.cursor_channel += 1;
                    } else {
                        self.cursor_channel = 0;
                    }
                }
                _ => {}
            }
            return false;
        }

        if let Some(dir) = select_action {
            match dir {
                Action::SelectUp => {
                    if self.cursor_row > 0 {
                        self.cursor_row -= 1;
                    } else {
                        self.cursor_row = self.pattern.rows - 1;
                    }
                }
                Action::SelectDown => {
                    if self.cursor_row < self.pattern.rows - 1 {
                        self.cursor_row += 1;
                    } else {
                        self.cursor_row = 0;
                    }
                }
                Action::SelectLeft => {
                    if self.cursor_channel > 0 {
                        self.cursor_channel -= 1;
                    } else {
                        self.cursor_channel = self.pattern.channels - 1;
                    }
                }
                Action::SelectRight => {
                    if self.cursor_channel < self.pattern.channels - 1 {
                        self.cursor_channel += 1;
                    } else {
                        self.cursor_channel = 0;
                    }
                }
                _ => {}
            }
            return false;
        }

        if actions.contains(&Action::Delete) {
            if let Some((min_ch, max_ch, min_row, max_row)) = self.selection_bounds() {
                for ch in min_ch..=max_ch {
                    for row in min_row..=max_row {
                        self.pattern.clear(ch, row);
                    }
                }
                self.clear_selection();
            } else {
                self.pattern.clear(self.cursor_channel, self.cursor_row);
                self.cursor_row = self.cursor_row.wrapping_sub(1) % self.pattern.rows;
            }
        } else if actions.contains(&Action::NoteOff) {
            self.clear_selection();
            self.pattern
                .set(self.cursor_channel, self.cursor_row, Cell::NoteOff);
            if self.cursor_row < self.pattern.rows - 1
                && self.cursor_row + self.step < self.pattern.rows
            {
                self.cursor_row += self.step;
            } else {
                self.cursor_row = self.pattern.rows - 1;
            }
        } else if actions.contains(&Action::TransposeUp)
            || actions.contains(&Action::TransposeDown)
            || actions.contains(&Action::TransposeOctaveUp)
            || actions.contains(&Action::TransposeOctaveDown)
        {
            let delta: i16 = if actions.contains(&Action::TransposeOctaveUp) {
                12
            } else if actions.contains(&Action::TransposeOctaveDown) {
                -12
            } else if actions.contains(&Action::TransposeUp) {
                1
            } else {
                -1
            };

            let (min_ch, max_ch, min_row, max_row) = if let Some(bounds) = self.selection_bounds() {
                bounds
            } else {
                (
                    self.cursor_channel,
                    self.cursor_channel,
                    self.cursor_row,
                    self.cursor_row,
                )
            };

            let mut min_pitch: Option<u8> = None;
            let mut max_pitch: Option<u8> = None;
            for ch in min_ch..=max_ch {
                for row in min_row..=max_row {
                    if let Cell::NoteOn(note) = self.pattern.get(ch, row) {
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
                        if let Cell::NoteOn(note) = self.pattern.get(ch, row) {
                            let new_pitch = (i16::from(note.pitch) + delta) as u8;
                            self.pattern.set(
                                ch,
                                row,
                                Cell::NoteOn(crate::pattern::Note::new(new_pitch)),
                            );
                        }
                    }
                }
            }
        } else if actions.contains(&Action::OctaveUp) {
            if self.octave < 8 {
                self.octave += 1;
            }
        } else if actions.contains(&Action::OctaveDown) {
            if self.octave > 0 {
                self.octave -= 1;
            }
        } else if actions.contains(&Action::Escape) {
            if self.selection_anchor.is_some() {
                self.clear_selection();
            } else if self.playing {
                self.stop_playback();
            } else {
                return true;
            }
        } else {
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
                    let scale = self.scale_index.scale();
                    if let Some(note) = key_to_note(k, self.octave, scale, self.transpose) {
                        self.pattern
                            .set(self.cursor_channel, self.cursor_row, Cell::NoteOn(note));
                        if !self.playing {
                            self.audio.preview_note(
                                note.frequency(),
                                self.cursor_channel,
                                &self.channel_settings,
                                self.master_volume_linear(),
                            );
                        }
                        self.clear_selection();
                        if self.cursor_row < self.pattern.rows - 1
                            && self.cursor_row + self.step < self.pattern.rows
                        {
                            self.cursor_row += self.step;
                        } else {
                            self.cursor_row = self.pattern.rows - 1;
                        }
                    }
                    break;
                }
            }
        }

        false
    }

    fn handle_settings_input(&mut self, actions: &[Action]) {
        if actions.contains(&Action::Escape) {
            if self.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToEdit) {
            self.mode = Mode::Edit;
        } else if actions.contains(&Action::SwitchToSynth) {
            self.mode = Mode::SynthEdit;
            self.synth_channel = self.cursor_channel;
            self.synth_field = SynthSettingsField::Channel;
        } else if actions.contains(&Action::SwitchToSettings) {
            // already in settings
        } else if actions.contains(&Action::SettingsDown) {
            self.settings_field = self.settings_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.settings_field = self.settings_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.subdivision = (self.subdivision + 1).min(64);
                }
                SettingsField::Step => {
                    self.step = (self.step + 1).min(64);
                }
                SettingsField::Bpm => {
                    self.bpm = (self.bpm + 1).min(666);
                }
                SettingsField::PatternLength => {
                    let new_len = (self.pattern.rows + 1).min(128);
                    self.pattern.resize(new_len);
                }
                SettingsField::Scale => {
                    self.scale_index = self.scale_index.next();
                }
                SettingsField::Transpose => {
                    self.transpose = (self.transpose + 1).min(12);
                }
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.subdivision = self.subdivision.saturating_sub(1).max(2);
                }
                SettingsField::Step => {
                    self.step = self.step.saturating_sub(1).max(1);
                }
                SettingsField::Bpm => {
                    self.bpm = self.bpm.saturating_sub(1).max(20);
                }
                SettingsField::PatternLength => {
                    let new_len = self.pattern.rows.saturating_sub(1).max(1);
                    self.pattern.resize(new_len);
                    if self.cursor_row >= self.pattern.rows {
                        self.cursor_row = self.pattern.rows - 1;
                    }
                }
                SettingsField::Scale => {
                    self.scale_index = self.scale_index.prev();
                }
                SettingsField::Transpose => {
                    self.transpose = (self.transpose - 1).max(-12);
                }
            }
        }
    }

    fn handle_synth_input(&mut self, actions: &[Action]) {
        let ch = self.synth_channel;

        if actions.contains(&Action::Escape) {
            if self.playing {
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
                    self.synth_channel = (self.synth_channel + 1) % self.pattern.channels;
                }
                SynthSettingsField::Waveform => {
                    let cs = &mut self.channel_settings[ch];
                    cs.waveform = cs.waveform.next();
                }
                SynthSettingsField::Attack => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.attack = (cs.envelope.attack + 0.005).min(2.0);
                }
                SynthSettingsField::Decay => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.decay = (cs.envelope.decay + 0.005).min(2.0);
                }
                SynthSettingsField::Sustain => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.sustain = (cs.envelope.sustain + 0.05).min(1.0);
                }
                SynthSettingsField::Release => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.release = (cs.envelope.release + 0.005).min(2.0);
                }
                SynthSettingsField::Volume => {
                    let cs = &mut self.channel_settings[ch];
                    cs.volume = (cs.volume + 0.05).min(1.0);
                }
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            match self.synth_field {
                SynthSettingsField::Channel => {
                    self.synth_channel = if self.synth_channel == 0 {
                        self.pattern.channels - 1
                    } else {
                        self.synth_channel - 1
                    };
                }
                SynthSettingsField::Waveform => {
                    let cs = &mut self.channel_settings[ch];
                    cs.waveform = cs.waveform.prev();
                }
                SynthSettingsField::Attack => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.attack = (cs.envelope.attack - 0.005).max(0.001);
                }
                SynthSettingsField::Decay => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.decay = (cs.envelope.decay - 0.005).max(0.001);
                }
                SynthSettingsField::Sustain => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.sustain = (cs.envelope.sustain - 0.05).max(0.0);
                }
                SynthSettingsField::Release => {
                    let cs = &mut self.channel_settings[ch];
                    cs.envelope.release = (cs.envelope.release - 0.005).max(0.001);
                }
                SynthSettingsField::Volume => {
                    let cs = &mut self.channel_settings[ch];
                    cs.volume = (cs.volume - 0.05).max(0.0);
                }
            }
        }
    }
}
