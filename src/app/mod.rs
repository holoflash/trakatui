pub mod input;

use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::time::{Duration, Instant};

use crate::audio::AudioEngine;
use crate::export;
use crate::pattern::Pattern;
use crate::scale::ScaleIndex;
use crate::synth::ChannelSettings;

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
    Attack,
    Decay,
    Sustain,
    Release,
    Volume,
}

impl SynthSettingsField {
    pub fn next(&self) -> Self {
        match self {
            Self::Channel => Self::Waveform,
            Self::Waveform => Self::Attack,
            Self::Attack => Self::Decay,
            Self::Decay => Self::Sustain,
            Self::Sustain => Self::Release,
            Self::Release => Self::Volume,
            Self::Volume => Self::Channel,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Channel => Self::Volume,
            Self::Waveform => Self::Channel,
            Self::Attack => Self::Waveform,
            Self::Decay => Self::Attack,
            Self::Sustain => Self::Decay,
            Self::Release => Self::Sustain,
            Self::Volume => Self::Release,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsField {
    Bpm,
    PatternLength,
    Subdivision,
    Scale,
    Transpose,
}

impl SettingsField {
    pub fn next(&self) -> Self {
        match self {
            SettingsField::Bpm => SettingsField::Subdivision,
            SettingsField::Subdivision => SettingsField::PatternLength,
            SettingsField::PatternLength => SettingsField::Scale,
            SettingsField::Scale => SettingsField::Transpose,
            SettingsField::Transpose => SettingsField::Bpm,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            SettingsField::Bpm => SettingsField::Transpose,
            SettingsField::Subdivision => SettingsField::Bpm,
            SettingsField::PatternLength => SettingsField::Subdivision,
            SettingsField::Scale => SettingsField::PatternLength,
            SettingsField::Transpose => SettingsField::Scale,
        }
    }
}

pub struct App {
    pub pattern: Pattern,
    pub cursor_channel: usize,
    pub cursor_row: usize,
    pub selection_anchor: Option<(usize, usize)>,
    pub octave: u8,
    pub mode: Mode,
    pub playing: bool,
    pub playback_row: usize,
    pub bpm: u16,
    pub subdivision: usize,
    pub audio: AudioEngine,
    pub settings_field: SettingsField,
    pub synth_field: SynthSettingsField,
    pub channel_settings: Vec<ChannelSettings>,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub status_message: Option<String>,
    pub synth_channel: usize,
    pub master_volume_db: f32,
    pub peak_level: Arc<AtomicU32>,
    pub display_peak: f32,
    pub(crate) last_step_time: Option<Instant>,
}

impl App {
    pub fn new() -> Self {
        let audio = AudioEngine::new();
        let peak_level = audio.peak_level.clone();
        Self {
            pattern: Pattern::new(8, 16),
            cursor_channel: 0,
            cursor_row: 0,
            selection_anchor: None,
            octave: 4,
            mode: Mode::Edit,
            playing: false,
            playback_row: 0,
            bpm: 120,
            subdivision: 4,
            audio,
            settings_field: SettingsField::Bpm,
            synth_field: SynthSettingsField::Waveform,
            channel_settings: ChannelSettings::defaults(),
            scale_index: ScaleIndex::default(),
            transpose: 0,
            status_message: None,
            synth_channel: 0,
            master_volume_db: 0.0,
            peak_level,
            display_peak: 0.0,
            last_step_time: None,
        }
    }

    pub fn selection_bounds(&self) -> Option<(usize, usize, usize, usize)> {
        self.selection_anchor.map(|(ach, arow)| {
            let min_ch = ach.min(self.cursor_channel);
            let max_ch = ach.max(self.cursor_channel);
            let min_row = arow.min(self.cursor_row);
            let max_row = arow.max(self.cursor_row);
            (min_ch, max_ch, min_row, max_row)
        })
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    pub fn step_duration(&self) -> Duration {
        let seconds = 60.0 / self.bpm as f64 / 4.0;
        Duration::from_secs_f64(seconds)
    }

    pub fn master_volume_linear(&self) -> f32 {
        if self.master_volume_db <= -60.0 {
            0.0
        } else {
            10.0_f32.powf(self.master_volume_db / 20.0)
        }
    }

    pub fn do_export(&mut self) {
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
            let _ = export::export_wav(
                &self.pattern,
                self.bpm,
                &path,
                &self.channel_settings,
                self.master_volume_linear(),
            );
        }
    }

    pub(crate) fn start_playback(&mut self, from_cursor: bool) {
        self.playing = true;
        self.playback_row = if from_cursor { self.cursor_row } else { 0 };
        self.last_step_time = Some(Instant::now());
        self.audio.play_row(
            &self.pattern,
            self.playback_row,
            self.step_duration(),
            &self.channel_settings,
            self.master_volume_linear(),
        );
    }

    pub(crate) fn stop_playback(&mut self) {
        self.playing = false;
        self.last_step_time = None;
        self.audio.stop_all();
    }

    pub fn tick(&mut self) {
        if !self.playing {
            return;
        }

        if let Some(last) = self.last_step_time {
            if last.elapsed() >= self.step_duration() {
                self.playback_row = (self.playback_row + 1) % self.pattern.rows;
                self.audio.play_row(
                    &self.pattern,
                    self.playback_row,
                    self.step_duration(),
                    &self.channel_settings,
                    self.master_volume_linear(),
                );
                self.last_step_time = Some(Instant::now());
            }
        }
    }

    pub fn set_cursor(&mut self, channel: usize, row: usize) {
        if channel < self.pattern.channels && row < self.pattern.rows {
            self.cursor_channel = channel;
            self.cursor_row = row;
        }
    }
}
