use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent};

use crate::audio::AudioEngine;
use crate::export;
use crate::keys::key_to_note;
use crate::pattern::{Cell, Pattern};
use crate::scale::ScaleIndex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Bpm,
    PatternLength,
    Scale,
    Transpose,
    ExportWav,
}

impl SettingsField {
    pub fn next(&self) -> Self {
        match self {
            SettingsField::Bpm => SettingsField::PatternLength,
            SettingsField::PatternLength => SettingsField::Scale,
            SettingsField::Scale => SettingsField::Transpose,
            SettingsField::Transpose => SettingsField::ExportWav,
            SettingsField::ExportWav => SettingsField::Bpm,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SettingsField::Bpm => SettingsField::ExportWav,
            SettingsField::PatternLength => SettingsField::Bpm,
            SettingsField::Scale => SettingsField::PatternLength,
            SettingsField::Transpose => SettingsField::Scale,
            SettingsField::ExportWav => SettingsField::Transpose,
        }
    }
}

pub struct App {
    pub pattern: Pattern,
    pub cursor_channel: usize,
    pub cursor_row: usize,
    pub octave: u8,
    pub mode: Mode,
    pub playing: bool,
    pub playback_row: usize,
    pub bpm: u16,
    pub running: bool,
    pub audio: AudioEngine,
    pub settings_field: SettingsField,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub status_message: Option<String>,
    last_step_time: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        Self {
            pattern: Pattern::new(4, 16),
            cursor_channel: 0,
            cursor_row: 0,
            octave: 4,
            mode: Mode::Edit,
            playing: false,
            playback_row: 0,
            bpm: 150,
            running: true,
            audio: AudioEngine::new(),
            settings_field: SettingsField::Bpm,
            scale_index: ScaleIndex::default(),
            transpose: 0,
            status_message: None,
            last_step_time: None,
        }
    }

    pub fn step_duration(&self) -> Duration {
        let seconds = 60.0 / self.bpm as f64 / 4.0;
        Duration::from_secs_f64(seconds)
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Enter && self.playing {
            self.stop_playback();
            return;
        }

        match self.mode {
            Mode::Edit => self.handle_edit_key(key),
            Mode::Settings => self.handle_settings_key(key.code),
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) {
        if key.code == KeyCode::Char('2') {
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Bpm;
            return;
        }

        match key.code {
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                } else {
                    self.cursor_row = self.pattern.rows - 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_row < self.pattern.rows - 1 {
                    self.cursor_row += 1;
                } else {
                    self.cursor_row = 0;
                }
            }
            KeyCode::Left => {
                if self.cursor_channel > 0 {
                    self.cursor_channel -= 1;
                } else {
                    self.cursor_channel = self.pattern.channels - 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_channel < self.pattern.channels - 1 {
                    self.cursor_channel += 1;
                } else {
                    self.cursor_channel = 0;
                }
            }

            KeyCode::Delete | KeyCode::Backspace => {
                self.pattern.clear(self.cursor_channel, self.cursor_row);
                self.cursor_row = (self.cursor_row + 1) % self.pattern.rows;
            }

            KeyCode::Tab => {
                self.pattern
                    .set(self.cursor_channel, self.cursor_row, Cell::NoteOff);
                if self.cursor_row < self.pattern.rows - 1 {
                    self.cursor_row += 1;
                }
            }

            KeyCode::Char('.') => {
                if self.octave < 8 {
                    self.octave += 1;
                }
            }
            KeyCode::Char(',') => {
                if self.octave > 0 {
                    self.octave -= 1;
                }
            }

            KeyCode::Enter => {
                self.start_playback();
            }

            KeyCode::Esc => {
                if self.playing {
                    self.stop_playback();
                } else {
                    self.running = false;
                }
            }

            other => {
                let scale = self.scale_index.scale();
                if let Some(note) = key_to_note(other, self.octave, scale, self.transpose) {
                    self.pattern
                        .set(self.cursor_channel, self.cursor_row, Cell::NoteOn(note));
                    self.audio
                        .preview_note(note.frequency(), self.cursor_channel);
                    if self.cursor_row < self.pattern.rows - 1 {
                        self.cursor_row += 1;
                    }
                }
            }
        }
    }

    fn handle_settings_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                if self.playing {
                    self.stop_playback();
                }
                self.mode = Mode::Edit;
            }
            KeyCode::Char('1') => {
                self.mode = Mode::Edit;
            }
            KeyCode::Char('2') => {}
            KeyCode::Down => {
                self.settings_field = self.settings_field.next();
            }
            KeyCode::Up => {
                self.settings_field = self.settings_field.prev();
            }
            KeyCode::Right => match self.settings_field {
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
                SettingsField::ExportWav => {}
            },
            KeyCode::Left => match self.settings_field {
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
                SettingsField::ExportWav => {}
            },
            KeyCode::Enter => {
                if self.settings_field == SettingsField::ExportWav {
                    self.do_export();
                }
            }
            _ => {}
        }
    }

    fn do_export(&mut self) {
        let path = std::path::PathBuf::from("output.wav");
        match export::export_wav(&self.pattern, self.bpm, &path) {
            Ok(()) => {
                self.status_message = Some(format!("Exported to {}", path.display()));
            }
            Err(e) => {
                self.status_message = Some(format!("Export failed: {}", e));
            }
        }
    }

    fn start_playback(&mut self) {
        self.playing = true;
        self.playback_row = 0;
        self.last_step_time = Some(Instant::now());
        self.audio.play_row(&self.pattern, 0, self.step_duration());
    }

    fn stop_playback(&mut self) {
        self.playing = false;
        self.last_step_time = None;
    }

    pub fn tick(&mut self) {
        if !self.playing {
            return;
        }

        if let Some(last) = self.last_step_time {
            if last.elapsed() >= self.step_duration() {
                self.playback_row = (self.playback_row + 1) % self.pattern.rows;
                self.audio
                    .play_row(&self.pattern, self.playback_row, self.step_duration());
                self.last_step_time = Some(Instant::now());
            }
        }
    }
}
