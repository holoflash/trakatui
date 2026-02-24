use std::time::{Duration, Instant};

use crossterm::event::KeyCode;

use crate::audio::AudioEngine;
use crate::keys::key_to_note;
use crate::pattern::{Cell, Pattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    Play,
}

pub struct App {
    pub pattern: Pattern,
    pub cursor_channel: usize,
    pub cursor_row: usize,
    pub octave: u8,
    pub mode: Mode,
    pub playback_row: usize,
    pub bpm: u16,
    pub running: bool,
    pub audio: AudioEngine,
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
            playback_row: 0,
            bpm: 150,
            running: true,
            audio: AudioEngine::new(),
            last_step_time: None,
        }
    }

    pub fn step_duration(&self) -> Duration {
        let seconds = 60.0 / self.bpm as f64 / 4.0;
        Duration::from_secs_f64(seconds)
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match self.mode {
            Mode::Edit => self.handle_edit_key(key),
            Mode::Play => self.handle_play_key(key),
        }
    }

    fn handle_edit_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Up => {
                if self.cursor_row > 0 {
                    self.cursor_row -= 1;
                }
            }
            KeyCode::Down => {
                if self.cursor_row < self.pattern.rows - 1 {
                    self.cursor_row += 1;
                }
            }
            KeyCode::Left => {
                if self.cursor_channel > 0 {
                    self.cursor_channel -= 1;
                }
            }
            KeyCode::Right => {
                if self.cursor_channel < self.pattern.channels - 1 {
                    self.cursor_channel += 1;
                }
            }

            KeyCode::Delete | KeyCode::Backspace => {
                self.pattern.clear(self.cursor_channel, self.cursor_row);
            }

            KeyCode::Tab => {
                self.pattern
                    .set(self.cursor_channel, self.cursor_row, Cell::NoteOff);
                if self.cursor_row < self.pattern.rows - 1 {
                    self.cursor_row += 1;
                }
            }

            KeyCode::Char('+') | KeyCode::Char('=') => {
                if self.octave < 8 {
                    self.octave += 1;
                }
            }
            KeyCode::Char('-') => {
                if self.octave > 0 {
                    self.octave -= 1;
                }
            }

            KeyCode::Char(' ') => {
                self.start_playback();
            }

            KeyCode::Esc => {
                self.running = false;
            }

            other => {
                if let Some(note) = key_to_note(other, self.octave) {
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

    fn handle_play_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(' ') | KeyCode::Esc => {
                self.stop_playback();
            }
            _ => {}
        }
    }

    fn start_playback(&mut self) {
        self.mode = Mode::Play;
        self.playback_row = 0;
        self.last_step_time = Some(Instant::now());
        self.audio.play_row(&self.pattern, 0, self.step_duration());
    }

    fn stop_playback(&mut self) {
        self.mode = Mode::Edit;
        self.last_step_time = None;
    }

    pub fn tick(&mut self) {
        if self.mode != Mode::Play {
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
