pub mod input;
pub mod playback;

use crate::project::{ChannelSettings, Waveform};

use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize};

use crate::audio::AudioEngine;
use crate::keybindings::KeyBindings;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    Settings,
    SynthEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthSettingsField {
    Channel,
    Waveform,
    Sample,
    Attack,
    Decay,
    Sustain,
    Release,
    Volume,
}

impl SynthSettingsField {
    pub const fn next(self) -> Self {
        match self {
            Self::Channel => Self::Waveform,
            Self::Waveform => Self::Sample,
            Self::Sample => Self::Attack,
            Self::Attack => Self::Decay,
            Self::Decay => Self::Sustain,
            Self::Sustain => Self::Release,
            Self::Release => Self::Volume,
            Self::Volume => Self::Channel,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Channel => Self::Volume,
            Self::Waveform => Self::Channel,
            Self::Sample => Self::Waveform,
            Self::Attack => Self::Sample,
            Self::Decay => Self::Attack,
            Self::Sustain => Self::Decay,
            Self::Release => Self::Sustain,
            Self::Volume => Self::Release,
        }
    }

    pub fn adjust(self, cs: &mut ChannelSettings, delta: i16) {
        match self {
            Self::Channel => {}
            Self::Waveform => {
                cs.waveform = if delta > 0 {
                    cs.waveform.next()
                } else {
                    cs.waveform.prev()
                };
                cs.envelope = cs.waveform.default_envelope();
                cs.volume = if cs.waveform == Waveform::Sampler {
                    1.0
                } else {
                    0.5
                };
            }
            Self::Sample => {}
            Self::Attack => {
                cs.envelope.attack =
                    (cs.envelope.attack + 0.005 * f32::from(delta)).clamp(0.0, 2.0);
            }
            Self::Decay => {
                cs.envelope.decay = (cs.envelope.decay + 0.005 * f32::from(delta)).clamp(0.0, 2.0);
            }
            Self::Sustain => {
                cs.envelope.sustain =
                    (cs.envelope.sustain + 0.05 * f32::from(delta)).clamp(0.0, 1.0);
            }
            Self::Release => {
                cs.envelope.release =
                    (cs.envelope.release + 0.005 * f32::from(delta)).clamp(0.0, 2.0);
            }
            Self::Volume => {
                cs.volume = (cs.volume + 0.05 * f32::from(delta)).clamp(0.0, 1.0);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Scale,
    Bpm,
    PatternLength,
    Subdivision,
    Step,
    Transpose,
}

impl SettingsField {
    pub const fn next(self) -> Self {
        match self {
            Self::Scale => Self::Bpm,
            Self::Bpm => Self::Subdivision,
            Self::Subdivision => Self::Step,
            Self::Step => Self::PatternLength,
            Self::PatternLength => Self::Scale,
            Self::Transpose => Self::Scale,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Scale => Self::Transpose,
            Self::Bpm => Self::Scale,
            Self::Subdivision => Self::Bpm,
            Self::Step => Self::Subdivision,
            Self::PatternLength => Self::Step,
            Self::Transpose => Self::Scale,
        }
    }

    pub fn adjust(self, project: &mut crate::project::Project, _cursor_row: &mut usize) {
        match self {
            Self::Subdivision => {
                project.subdivision = (project.subdivision + 1).min(64);
            }
            Self::Step => {
                project.step = (project.step + 1).min(64);
            }
            Self::Bpm => {
                project.bpm = (project.bpm + 1).min(666);
            }
            Self::PatternLength => {
                let new_len = (project.pattern.rows + 1).min(128);
                project.pattern.resize(new_len);
            }
            Self::Scale => {
                project.scale_index = project.scale_index.next();
            }
            Self::Transpose => {
                project.transpose = (project.transpose + 1).min(12);
            }
        }
    }

    pub fn adjust_down(self, project: &mut crate::project::Project, cursor_row: &mut usize) {
        match self {
            Self::Subdivision => {
                project.subdivision = project.subdivision.saturating_sub(1).max(2);
            }
            Self::Step => {
                project.step = project.step.saturating_sub(1).max(1);
            }
            Self::Bpm => {
                project.bpm = project.bpm.saturating_sub(1).max(20);
            }
            Self::PatternLength => {
                let new_len = project.pattern.rows.saturating_sub(1).max(1);
                project.pattern.resize(new_len);
                if *cursor_row >= project.pattern.rows {
                    *cursor_row = project.pattern.rows - 1;
                }
            }
            Self::Scale => {
                project.scale_index = project.scale_index.prev();
            }
            Self::Transpose => {
                project.transpose = (project.transpose - 1).max(-12);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubColumn {
    Note,
    Effect,
}

pub struct Cursor {
    pub channel: usize,
    pub row: usize,
    pub sub_column: SubColumn,
    pub effect_edit_pos: usize,
    pub selection_anchor: Option<(usize, usize)>,
    pub octave: u8,
}

pub struct App {
    pub project: Project,
    pub cursor: Cursor,
    pub mode: Mode,
    pub playback: playback::PlaybackState,
    pub playback_row_display: usize,
    pub audio: AudioEngine,
    pub peak_level: Arc<AtomicU32>,
    pub playback_row: Arc<AtomicUsize>,
    pub display_peak: f32,
    pub settings_field: SettingsField,
    pub synth_field: SynthSettingsField,
    pub status_message: Option<String>,
    pub keybindings: KeyBindings,
    pub show_controls_modal: bool,
}

impl App {
    pub fn new() -> Self {
        let audio = AudioEngine::new();
        let peak_level = audio.peak_level.clone();
        let playback_row = audio.playback_row.clone();
        Self {
            project: Project::new(),
            cursor: Cursor {
                channel: 0,
                row: 0,
                sub_column: SubColumn::Note,
                effect_edit_pos: 0,
                selection_anchor: None,
                octave: 4,
            },
            mode: Mode::Edit,
            playback: playback::PlaybackState::new(),
            playback_row_display: 0,
            audio,
            peak_level,
            playback_row,
            display_peak: 0.0,
            settings_field: SettingsField::Scale,
            synth_field: SynthSettingsField::Waveform,
            status_message: None,
            keybindings: KeyBindings::defaults(),
            show_controls_modal: false,
        }
    }

    pub fn selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        self.cursor.selection_anchor.map(|(ach, arow)| {
            let min_ch = ach.min(self.cursor.channel);
            let max_ch = ach.max(self.cursor.channel);
            let min_row = arow.min(self.cursor.row);
            let max_row = arow.max(self.cursor.row);
            (min_ch, max_ch, min_row, max_row)
        })
    }

    pub const fn clear_selection(&mut self) {
        self.cursor.selection_anchor = None;
    }

    pub fn do_export(&self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .set_file_name("new_song.wav")
            .set_title("Export WAV")
            .set_can_create_directories(true);

        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }

        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension("wav");
            }
            let _ = crate::audio::export::export_wav(
                &self.project.pattern,
                self.project.bpm,
                &path,
                &self.project.channel_settings,
                self.project.master_volume_linear(),
            );
        }
    }

    pub const fn set_cursor(&mut self, channel: usize, row: usize) {
        if channel < self.project.pattern.channels && row < self.project.pattern.rows {
            self.cursor.channel = channel;
            self.cursor.row = row;
        }
    }
}
