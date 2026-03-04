pub mod input;
pub mod keybindings;
pub mod playback;
pub mod scale;

use std::sync::Arc;

use crate::app::keybindings::KeyBindings;
use crate::project::{Cell, Effect, Instrument};

use std::sync::atomic::{AtomicU32, AtomicUsize};

use crate::audio::AudioEngine;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
    Settings,
    SynthEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SynthSettingsField {
    Instrument,
    Envelope,
    EnvPoints,
    EnvPoint,
    EnvTick,
    EnvValue,
    EnvSustain,
    EnvLoopStart,
    EnvLoopEnd,
    LoopType,
    LoopStart,
    LoopLength,
    Fadeout,
    VibratoType,
    VibratoSweep,
    VibratoDepth,
    VibratoRate,
}

impl SynthSettingsField {
    pub const fn next(self) -> Self {
        match self {
            Self::Instrument => Self::Envelope,
            Self::Envelope => Self::LoopType,
            Self::LoopType => Self::LoopStart,
            Self::LoopStart => Self::LoopLength,
            Self::LoopLength => Self::EnvPoints,
            Self::EnvPoints => Self::EnvPoint,
            Self::EnvPoint => Self::EnvTick,
            Self::EnvTick => Self::EnvValue,
            Self::EnvValue => Self::EnvSustain,
            Self::EnvSustain => Self::EnvLoopStart,
            Self::EnvLoopStart => Self::EnvLoopEnd,
            Self::EnvLoopEnd => Self::Fadeout,
            Self::Fadeout => Self::VibratoType,
            Self::VibratoType => Self::VibratoSweep,
            Self::VibratoSweep => Self::VibratoDepth,
            Self::VibratoDepth => Self::VibratoRate,
            Self::VibratoRate => Self::Instrument,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Instrument => Self::VibratoRate,
            Self::Envelope => Self::Instrument,
            Self::LoopType => Self::Envelope,
            Self::LoopStart => Self::LoopType,
            Self::LoopLength => Self::LoopStart,
            Self::EnvPoints => Self::LoopLength,
            Self::EnvPoint => Self::EnvPoints,
            Self::EnvTick => Self::EnvPoint,
            Self::EnvValue => Self::EnvTick,
            Self::EnvSustain => Self::EnvValue,
            Self::EnvLoopStart => Self::EnvSustain,
            Self::EnvLoopEnd => Self::EnvLoopStart,
            Self::Fadeout => Self::EnvLoopEnd,
            Self::VibratoType => Self::Fadeout,
            Self::VibratoSweep => Self::VibratoType,
            Self::VibratoDepth => Self::VibratoSweep,
            Self::VibratoRate => Self::VibratoDepth,
        }
    }

    pub fn adjust(self, inst: &mut Instrument, delta: i16) {
        match self {
            Self::Instrument => {}
            Self::Envelope => {
                inst.vol_envelope.enabled = !inst.vol_envelope.enabled;
                if inst.vol_envelope.enabled && inst.vol_envelope.points.len() < 2 {
                    inst.vol_envelope.points = vec![(0, 64), (16, 48), (48, 32), (96, 0)];
                    inst.vol_envelope.sustain_point = Some(1);
                }
            }
            Self::EnvPoints
            | Self::EnvPoint
            | Self::EnvTick
            | Self::EnvValue
            | Self::EnvSustain
            | Self::EnvLoopStart
            | Self::EnvLoopEnd => {}
            Self::LoopType => {
                let sd = Arc::make_mut(&mut inst.sample_data);
                sd.loop_type = if delta > 0 {
                    sd.loop_type.next()
                } else {
                    sd.loop_type.prev()
                };
            }
            Self::LoopStart => {
                let sd = Arc::make_mut(&mut inst.sample_data);
                let max = sd.samples_f32.len().saturating_sub(sd.loop_length);
                let step = (sd.samples_f32.len() / 100).max(1);
                sd.loop_start = if delta > 0 {
                    (sd.loop_start + step).min(max)
                } else {
                    sd.loop_start.saturating_sub(step)
                };
            }
            Self::LoopLength => {
                let sd = Arc::make_mut(&mut inst.sample_data);
                let max = sd.samples_f32.len().saturating_sub(sd.loop_start);
                let step = (sd.samples_f32.len() / 100).max(1);
                sd.loop_length = if delta > 0 {
                    (sd.loop_length + step).min(max)
                } else {
                    sd.loop_length.saturating_sub(step)
                };
            }
            Self::Fadeout => {
                inst.vol_fadeout =
                    (i32::from(inst.vol_fadeout) + i32::from(delta) * 16).clamp(0, 4095) as u16;
            }
            Self::VibratoType => {
                inst.vibrato_type = if delta > 0 {
                    (inst.vibrato_type + 1).min(3)
                } else {
                    inst.vibrato_type.saturating_sub(1)
                };
            }
            Self::VibratoSweep => {
                inst.vibrato_sweep = (i16::from(inst.vibrato_sweep) + delta).clamp(0, 255) as u8;
            }
            Self::VibratoDepth => {
                inst.vibrato_depth = (i16::from(inst.vibrato_depth) + delta).clamp(0, 15) as u8;
            }
            Self::VibratoRate => {
                inst.vibrato_rate = (i16::from(inst.vibrato_rate) + delta).clamp(0, 63) as u8;
            }
        }
    }

    pub fn adjust_envelope(self, inst: &mut Instrument, delta: i16, point_idx: &mut usize) {
        let env = &mut inst.vol_envelope;
        match self {
            Self::EnvPoints => {
                if delta > 0 {
                    let last_tick = env.points.last().map(|p| p.0).unwrap_or(0);
                    env.points.push((last_tick + 16, 32));
                } else if env.points.len() > 2 {
                    env.points.pop();
                    if *point_idx >= env.points.len() {
                        *point_idx = env.points.len() - 1;
                    }
                    if let Some(sp) = env.sustain_point {
                        if sp >= env.points.len() {
                            env.sustain_point = None;
                        }
                    }
                    if let Some((ls, le)) = env.loop_range {
                        if ls >= env.points.len() || le >= env.points.len() {
                            env.loop_range = None;
                        }
                    }
                }
            }
            Self::EnvPoint => {
                let max = env.points.len().saturating_sub(1);
                *point_idx = (*point_idx as i16 + delta).clamp(0, max as i16) as usize;
            }
            Self::EnvTick => {
                if *point_idx < env.points.len() {
                    let min_tick = if *point_idx > 0 {
                        env.points[*point_idx - 1].0 + 1
                    } else {
                        0
                    };
                    let max_tick = if *point_idx + 1 < env.points.len() {
                        env.points[*point_idx + 1].0 - 1
                    } else {
                        9999
                    };
                    let cur = env.points[*point_idx].0 as i32;
                    env.points[*point_idx].0 =
                        (cur + delta as i32).clamp(min_tick as i32, max_tick as i32) as u16;
                }
            }
            Self::EnvValue => {
                if *point_idx < env.points.len() {
                    let cur = env.points[*point_idx].1 as i16;
                    env.points[*point_idx].1 = (cur + delta).clamp(0, 64) as u16;
                }
            }
            Self::EnvSustain => {
                let max = env.points.len().saturating_sub(1);
                if let Some(ref mut sp) = env.sustain_point {
                    let new_val = *sp as i16 + delta;
                    if new_val < 0 {
                        env.sustain_point = None;
                    } else {
                        *sp = (new_val as usize).min(max);
                    }
                } else if delta > 0 {
                    env.sustain_point = Some(0);
                }
            }
            Self::EnvLoopStart => {
                let max = env.points.len().saturating_sub(1);
                if let Some(ref mut range) = env.loop_range {
                    let new_val = range.0 as i16 + delta;
                    if new_val < 0 {
                        env.loop_range = None;
                    } else {
                        range.0 = (new_val as usize).min(range.1).min(max);
                    }
                } else if delta > 0 {
                    env.loop_range = Some((0, max));
                }
            }
            Self::EnvLoopEnd => {
                let max = env.points.len().saturating_sub(1);
                if let Some(ref mut range) = env.loop_range {
                    let new_val = range.1 as i16 + delta;
                    if new_val < 0 {
                        env.loop_range = None;
                    } else {
                        range.1 = (new_val as usize).max(range.0).min(max);
                    }
                } else if delta > 0 {
                    env.loop_range = Some((0, max));
                }
            }
            _ => {}
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
            Self::PatternLength => Self::Transpose,
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
            Self::Transpose => Self::PatternLength,
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
                let new_len = (project.current_pattern().rows + 1).min(128);
                project.current_pattern_mut().resize(new_len);
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
                let new_len = project.current_pattern().rows.saturating_sub(1).max(1);
                project.current_pattern_mut().resize(new_len);
                if *cursor_row >= project.current_pattern().rows {
                    *cursor_row = project.current_pattern().rows - 1;
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
    Instrument,
    Volume,
    Effect,
}

#[derive(Clone)]
pub enum ClipboardData {
    Notes(Vec<Vec<Cell>>),
    Instruments(Vec<Vec<Option<u8>>>),
    Volumes(Vec<Vec<Option<u8>>>),
    Effects(Vec<Vec<Option<Effect>>>),
    Full {
        notes: Option<Vec<Vec<Cell>>>,
        instruments: Option<Vec<Vec<Option<u8>>>>,
        volumes: Option<Vec<Vec<Option<u8>>>>,
        effects: Option<Vec<Vec<Option<Effect>>>>,
    },
}

pub struct Cursor {
    pub channel: usize,
    pub row: usize,
    pub sub_column: SubColumn,
    pub effect_edit_pos: usize,
    pub volume_edit_pos: usize,
    pub instrument_edit_pos: usize,
    pub selection_anchor: Option<(usize, usize, SubColumn)>,
    pub octave: u8,
}

pub struct App {
    pub project: Project,
    pub cursor: Cursor,
    pub mode: Mode,
    pub playback: playback::PlaybackState,
    pub playback_row_display: usize,
    pub playback_order_display: usize,
    pub audio: AudioEngine,
    pub peak_level: Arc<AtomicU32>,
    pub playback_row: Arc<AtomicUsize>,
    pub display_peak: f32,
    pub settings_field: SettingsField,
    pub synth_field: SynthSettingsField,
    pub current_instrument: usize,
    pub status_message: Option<String>,
    pub keybindings: KeyBindings,
    pub show_controls_modal: bool,
    pub show_about_modal: bool,
    pub clipboard: Option<ClipboardData>,
    pub muted_channels: Vec<bool>,
    pub envelope_point_idx: usize,
    pub follow_playback: bool,
    pub follow_scroll_offset: f32,
    pub show_sidebar: bool,
    pub text_editing: bool,
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
                volume_edit_pos: 0,
                instrument_edit_pos: 0,
                selection_anchor: None,
                octave: 4,
            },
            mode: Mode::Edit,
            playback: playback::PlaybackState::new(),
            playback_row_display: 0,
            playback_order_display: 0,
            audio,
            peak_level,
            playback_row,
            display_peak: 0.0,
            settings_field: SettingsField::Scale,
            synth_field: SynthSettingsField::Instrument,
            current_instrument: 0,
            status_message: None,
            keybindings: KeyBindings::defaults(),
            show_controls_modal: false,
            show_about_modal: false,
            clipboard: None,
            muted_channels: vec![false; 32],
            envelope_point_idx: 0,
            follow_playback: true,
            follow_scroll_offset: 0.0,
            show_sidebar: true,
            text_editing: false,
        }
    }

    pub fn selection_bounds(&self) -> Option<(usize, usize, usize, usize, SubColumn, SubColumn)> {
        self.cursor.selection_anchor.map(|(ach, arow, asub)| {
            let min_ch = ach.min(self.cursor.channel);
            let max_ch = ach.max(self.cursor.channel);
            let min_row = arow.min(self.cursor.row);
            let max_row = arow.max(self.cursor.row);
            let (a_flat, b_flat) = (
                ach * 4 + asub as usize,
                self.cursor.channel * 4 + self.cursor.sub_column as usize,
            );
            let min_sub = if a_flat <= b_flat {
                asub
            } else {
                self.cursor.sub_column
            };
            let max_sub = if a_flat >= b_flat {
                asub
            } else {
                self.cursor.sub_column
            };
            (min_ch, max_ch, min_row, max_row, min_sub, max_sub)
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
                &self.project.patterns,
                &self.project.order,
                self.project.bpm,
                &path,
                &self.project.instruments,
                self.project.master_volume_linear(),
            );
        }
    }

    pub fn set_cursor(&mut self, channel: usize, row: usize) {
        if channel < self.project.current_pattern().channels
            && row < self.project.current_pattern().rows
        {
            self.cursor.channel = channel;
            self.cursor.row = row;
        }
    }
}
