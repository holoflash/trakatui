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
        let flat_order = self.project.flat_order();
        let order_idx = self.project.item_idx_to_flat_start(self.project.current_item_idx)
            .min(flat_order.len().saturating_sub(1));
        self.playback.playing = true;
        self.audio.playback_ended.store(false, Ordering::Relaxed);
        self.audio.start_playback(
            row,
            order_idx,
            &self.project.patterns,
            &flat_order,
            &self.project.tracks,
            self.project.master_volume_linear(),
            &self.muted_channels,
        );
    }

    pub fn stop_playback(&mut self) {
        self.playback.playing = false;
        self.audio.stop_all();
    }

    pub fn tick(&mut self) {
        for (i, scope) in self.channel_scopes.iter().enumerate() {
            if i < self.display_scopes.len() {
                self.display_scopes[i] = scope.read_all();
            }
        }

        if !self.playback.playing {
            for scope in self.channel_scopes.iter() {
                scope.clear();
            }
            for s in &mut self.display_scopes {
                s.fill(0.0);
            }
            if let Some(scope) = self.channel_scopes.first()
                && !self.display_scopes.is_empty() {
                    self.display_scopes[0] = scope.read_all();
                }
            return;
        }
        if self.audio.playback_ended.swap(false, Ordering::Relaxed) {
            self.playback.playing = false;
        }

        let row = self.playback_row.load(Ordering::Relaxed);
        self.playback_row_display = row;
        let flat_order_idx = self.audio.playback_order.load(Ordering::Relaxed);
        let (item_idx, sub_idx) = self.project.flat_order_to_item_idx(flat_order_idx);
        self.playback_order_display = item_idx;
        self.project.current_item_idx = item_idx;
        self.project.current_sub_pattern_idx = sub_idx;

        self.audio.update_settings(
            &self.project.tracks,
            self.project.master_volume_linear(),
            &self.muted_channels,
        );
        let flat_order = self.project.flat_order();
        self.audio
            .update_patterns(&self.project.patterns, &flat_order);
    }
}
