use eframe::egui::{self, Key};

use crate::keys::key_to_note;
use crate::pattern::Cell;

use super::{App, Mode, SettingsField, SynthSettingsField};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        ctx.input(|input| {
            if input.key_pressed(Key::Enter) || input.key_pressed(Key::Space) {
                if self.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(input.key_pressed(Key::Space));
                }
                return false;
            }

            match self.mode {
                Mode::Edit => self.handle_edit_input(input),
                Mode::Settings => {
                    self.handle_settings_input(input);
                    false
                }
                Mode::SynthEdit => {
                    self.handle_synth_input(input);
                    false
                }
            }
        })
    }

    fn handle_edit_input(&mut self, input: &egui::InputState) -> bool {
        if input.key_pressed(Key::Num2) {
            self.clear_selection();
            self.mode = Mode::SynthEdit;
            self.synth_channel = self.cursor_channel;
            self.synth_field = SynthSettingsField::Channel;
            return false;
        }

        if input.key_pressed(Key::Num3) {
            self.clear_selection();
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
            return false;
        }

        let alt = input.modifiers.alt;

        let arrow_pressed = input.key_pressed(Key::ArrowUp)
            || input.key_pressed(Key::ArrowDown)
            || input.key_pressed(Key::ArrowLeft)
            || input.key_pressed(Key::ArrowRight);

        if arrow_pressed && alt && self.selection_anchor.is_none() {
            self.selection_anchor = Some((self.cursor_channel, self.cursor_row));
        } else if arrow_pressed && !alt {
            self.clear_selection();
        }

        if input.key_pressed(Key::ArrowUp) {
            if self.cursor_row > 0 {
                self.cursor_row -= 1;
            } else {
                self.cursor_row = self.pattern.rows - 1;
            }
        } else if input.key_pressed(Key::ArrowDown) {
            if self.cursor_row < self.pattern.rows - 1 {
                self.cursor_row += 1;
            } else {
                self.cursor_row = 0;
            }
        } else if input.key_pressed(Key::ArrowLeft) {
            if self.cursor_channel > 0 {
                self.cursor_channel -= 1;
            } else {
                self.cursor_channel = self.pattern.channels - 1;
            }
        } else if input.key_pressed(Key::ArrowRight) {
            if self.cursor_channel < self.pattern.channels - 1 {
                self.cursor_channel += 1;
            } else {
                self.cursor_channel = 0;
            }
        } else if input.key_pressed(Key::Delete) || input.key_pressed(Key::Backspace) {
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
        } else if input.key_pressed(Key::Tab) {
            self.clear_selection();
            self.pattern
                .set(self.cursor_channel, self.cursor_row, Cell::NoteOff);
            if self.cursor_row < self.pattern.rows - 1 {
                self.cursor_row += 1;
            }
        } else if input.key_pressed(Key::Period) {
            if self.octave < 8 {
                self.octave += 1;
            }
        } else if input.key_pressed(Key::Comma) {
            if self.octave > 0 {
                self.octave -= 1;
            }
        } else if input.key_pressed(Key::Escape) {
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
                        if self.cursor_row < self.pattern.rows - 1 {
                            self.cursor_row += 1;
                        }
                    }
                    break;
                }
            }
        }

        false
    }

    fn handle_settings_input(&mut self, input: &egui::InputState) {
        if input.key_pressed(Key::Escape) {
            if self.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
        } else if input.key_pressed(Key::Num1) {
            self.mode = Mode::Edit;
        } else if input.key_pressed(Key::Num2) {
            self.mode = Mode::SynthEdit;
            self.synth_channel = self.cursor_channel;
            self.synth_field = SynthSettingsField::Channel;
        } else if input.key_pressed(Key::Num3) {
        } else if input.key_pressed(Key::ArrowDown) {
            self.settings_field = self.settings_field.next();
        } else if input.key_pressed(Key::ArrowUp) {
            self.settings_field = self.settings_field.prev();
        } else if input.key_pressed(Key::ArrowRight) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.subdivision = (self.subdivision + 1).min(64);
                }
                SettingsField::Bpm => {
                    self.bpm = (self.bpm + 1).min(300);
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
        } else if input.key_pressed(Key::ArrowLeft) {
            match self.settings_field {
                SettingsField::Subdivision => {
                    self.subdivision = self.subdivision.saturating_sub(1).max(2);
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

    fn handle_synth_input(&mut self, input: &egui::InputState) {
        let ch = self.synth_channel;

        if input.key_pressed(Key::Escape) {
            if self.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
        } else if input.key_pressed(Key::Num1) {
            self.mode = Mode::Edit;
        } else if input.key_pressed(Key::Num2) {
        } else if input.key_pressed(Key::Num3) {
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
        } else if input.key_pressed(Key::ArrowDown) {
            self.synth_field = self.synth_field.next();
        } else if input.key_pressed(Key::ArrowUp) {
            self.synth_field = self.synth_field.prev();
        } else if input.key_pressed(Key::ArrowRight) {
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
        } else if input.key_pressed(Key::ArrowLeft) {
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
