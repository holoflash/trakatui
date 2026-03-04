use std::sync::atomic::Ordering;

use super::App;

pub struct PlaybackState {
    pub playing: bool,
}

impl PlaybackState {
    pub const fn new() -> Self {
        Self { playing: false }
    }
}

impl App {
    pub fn start_playback(&mut self, from_cursor: bool) {
        let row = if from_cursor { self.cursor.row } else { 0 };
        let order_idx = self.project.current_order_idx;
        self.playback.playing = true;
        self.audio.start_playback(
            row,
            order_idx,
            &self.project.patterns,
            &self.project.order,
            &self.project.instruments,
            self.project.bpm,
            self.project.master_volume_linear(),
            &self.muted_channels,
        );
    }

    pub fn stop_playback(&mut self) {
        self.playback.playing = false;
        self.audio.stop_all();
    }

    pub fn tick(&mut self) {
        if !self.playback.playing {
            return;
        }
        let row = self.playback_row.load(Ordering::Relaxed);
        self.playback_row_display = row;
        let order = self.audio.playback_order.load(Ordering::Relaxed);
        self.playback_order_display = order;

        self.audio.update_settings(
            &self.project.instruments,
            self.project.bpm,
            self.project.master_volume_linear(),
            &self.muted_channels,
        );
        self.audio
            .update_patterns(&self.project.patterns, &self.project.order);
    }
}
