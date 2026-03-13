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
        let row = self.playback_row.load(Ordering::Relaxed);
        self.playback_row_display = row;
        let order = self.audio.playback_order.load(Ordering::Relaxed);
        self.playback_order_display = order;
        if order < self.project.order.len() {
            self.project.current_order_idx = order;
        }

        self.audio.update_settings(
            &self.project.tracks,
            self.project.master_volume_linear(),
            &self.muted_channels,
        );
        self.audio
            .update_patterns(&self.project.patterns, &self.project.order);
    }
}
