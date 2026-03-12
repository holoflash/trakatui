pub mod input;
pub mod keybindings;
pub mod playback;
pub mod scale;

use std::sync::Arc;

use crate::audio::mixer::{SCOPE_SIZE, ScopeBuffer};

use crate::app::keybindings::KeyBindings;
use crate::project::Cell;

use std::sync::atomic::{AtomicU32, AtomicUsize};

use crate::audio::AudioEngine;
use crate::project::Project;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Edit,
}

#[derive(Clone)]
pub enum ClipboardData {
    Notes(Vec<Vec<Vec<Cell>>>),
}

pub struct MovePreview {
    #[allow(clippy::type_complexity)]
    pub cells: Vec<(usize, usize, usize, Cell)>,
    pub origin_anchor: (usize, usize, usize),
    pub origin_cursor: (usize, usize, usize),
}

pub struct Cursor {
    pub channel: usize,
    pub voice: usize,
    pub row: usize,
    pub octave: u8,
    pub selection_anchor: Option<(usize, usize, usize)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaveformDrag {
    RegionStart,
    RegionEnd,
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

    pub current_track: usize,
    pub keybindings: KeyBindings,
    pub show_controls_modal: bool,
    pub show_about_modal: bool,
    pub clipboard: Option<ClipboardData>,
    pub muted_channels: Vec<bool>,
    pub envelope_point_idx: usize,
    pub dragging_envelope_point: Option<usize>,

    pub follow_scroll_offset: f32,
    pub show_sidebar: bool,
    pub text_editing: bool,
    pub channel_scopes: Arc<Vec<ScopeBuffer>>,
    pub display_scopes: Vec<[f32; SCOPE_SIZE]>,

    pub undo_stack: Vec<Project>,
    pub redo_stack: Vec<Project>,
    pub project_path: Option<std::path::PathBuf>,
    pub dirty: bool,
    pub show_quit_confirm: bool,
    pub show_new_confirm: bool,
    pub move_preview: Option<MovePreview>,
    pub dragging_waveform: Option<WaveformDrag>,
    pub pitch_envelope_point_idx: usize,
    pub dragging_pitch_env_point: Option<usize>,
    pub filter_envelope_point_idx: usize,
    pub dragging_filter_env_point: Option<usize>,
    pub poly_input: bool,
}

impl App {
    pub fn new() -> Self {
        let audio = AudioEngine::new();
        let peak_level = audio.peak_level.clone();
        let playback_row = audio.playback_row.clone();
        let channel_scopes = audio.channel_scopes.clone();
        Self {
            project: Project::new(),
            cursor: Cursor {
                channel: 0,
                voice: 0,
                row: 0,
                octave: 4,
                selection_anchor: None,
            },
            mode: Mode::Edit,
            playback: playback::PlaybackState::new(),
            playback_row_display: 0,
            playback_order_display: 0,
            audio,
            peak_level,
            playback_row,
            display_peak: 0.0,

            current_track: 0,
            keybindings: KeyBindings::defaults(),
            show_controls_modal: false,
            show_about_modal: false,
            clipboard: None,
            muted_channels: vec![false; 32],
            envelope_point_idx: 0,
            dragging_envelope_point: None,

            follow_scroll_offset: 0.0,
            show_sidebar: true,
            text_editing: false,
            channel_scopes,
            display_scopes: vec![[0.0; SCOPE_SIZE]; 32],

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            project_path: None,
            dirty: false,
            show_quit_confirm: false,
            show_new_confirm: false,
            move_preview: None,
            dragging_waveform: None,
            pitch_envelope_point_idx: 0,
            dragging_pitch_env_point: None,
            filter_envelope_point_idx: 0,
            dragging_filter_env_point: None,
            poly_input: false,
        }
    }

    pub fn selection_bounds(&self) -> Option<(usize, usize, usize, usize, usize, usize)> {
        self.cursor.selection_anchor.map(|(ach, avoice, arow)| {
            let min_ch = ach.min(self.cursor.channel);
            let max_ch = ach.max(self.cursor.channel);
            let min_voice = if ach == self.cursor.channel {
                avoice.min(self.cursor.voice)
            } else if ach < self.cursor.channel {
                avoice
            } else {
                self.cursor.voice
            };
            let max_voice = if ach == self.cursor.channel {
                avoice.max(self.cursor.voice)
            } else if ach > self.cursor.channel {
                avoice
            } else {
                self.cursor.voice
            };
            let min_row = arow.min(self.cursor.row);
            let max_row = arow.max(self.cursor.row);
            (min_ch, max_ch, min_voice, max_voice, min_row, max_row)
        })
    }

    pub fn clear_selection(&mut self) {
        if self.move_preview.is_some() {
            self.cancel_move_preview();
        }
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
                &self.project.tracks,
                self.project.master_volume_linear(),
            );
        }
    }

    pub fn set_cursor(&mut self, channel: usize, voice: usize, row: usize) {
        let pat = self.project.current_pattern();
        if channel < pat.channels && row < pat.rows && voice < pat.voice_count(channel) {
            self.cursor.channel = channel;
            self.cursor.voice = voice;
            self.cursor.row = row;
        }
    }

    const MAX_UNDO: usize = 100;

    pub fn save_undo_snapshot(&mut self) {
        self.undo_stack.push(self.project.clone());
        if self.undo_stack.len() > Self::MAX_UNDO {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        self.dirty = true;
    }

    pub fn undo(&mut self) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(self.project.clone());
            self.project = prev;
        }
    }

    pub fn redo(&mut self) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(self.project.clone());
            self.project = next;
        }
    }

    pub fn do_quick_save(&mut self) {
        if let Some(ref path) = self.project_path {
            let _ = crate::project::file::save(&self.project, path);
            self.dirty = false;
        } else {
            self.do_save_as();
        }
    }

    pub fn do_save_as(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Psikat Project", &["psikat"])
            .set_file_name(self.project_name())
            .set_title("Save Project")
            .set_can_create_directories(true);
        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }
        if let Some(mut path) = dialog.save_file() {
            if path.extension().is_none() {
                path.set_extension("psikat");
            }
            let _ = crate::project::file::save(&self.project, &path);
            self.project_path = Some(path);
            self.dirty = false;
        }
    }

    pub fn do_open(&mut self) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Psikat Project", &["psikat"])
            .set_title("Open Project");
        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }
        if let Some(path) = dialog.pick_file() {
            match crate::project::file::load(&path) {
                Ok(project) => {
                    self.save_undo_snapshot();
                    self.project = project;
                    self.project_path = Some(path);
                    self.dirty = false;
                    self.cursor.channel = 0;
                    self.cursor.voice = 0;
                    self.cursor.row = 0;
                    self.current_track = 0;
                    self.envelope_point_idx = 0;
                }
                Err(e) => {
                    eprintln!("Failed to open project: {e}");
                }
            }
        }
    }

    pub fn do_new_project(&mut self) {
        if self.dirty {
            self.show_new_confirm = true;
        } else {
            self.reset_project();
        }
    }

    pub fn reset_project(&mut self) {
        self.project = Project::new();
        self.project_path = None;
        self.dirty = false;
        self.undo_stack.clear();
        self.redo_stack.clear();
        self.cursor.channel = 0;
        self.cursor.voice = 0;
        self.cursor.row = 0;
        self.current_track = 0;
        self.envelope_point_idx = 0;
    }

    pub fn project_name(&self) -> String {
        self.project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "untitled.psikat".into())
    }

    pub fn project_status(&self) -> &str {
        if self.dirty {
            "[unsaved]"
        } else if self.project_path.is_some() {
            "[saved]"
        } else {
            ""
        }
    }

    pub fn voices_for_channel(&self, ch: usize) -> usize {
        self.project
            .tracks
            .get(ch)
            .map(|t| t.polyphony.max(1) as usize)
            .unwrap_or(1)
    }
}
