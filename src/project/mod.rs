pub mod channel;
pub mod file;
pub mod pattern;
pub mod sample;

pub use channel::Track;
pub use pattern::{Cell, Note, Pattern};
pub use sample::SampleData;

use crate::app::scale::ScaleIndex;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Project {
    pub patterns: Vec<Pattern>,
    pub order: Vec<usize>,
    pub current_order_idx: usize,
    pub tracks: Vec<Track>,
    pub step: usize,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub master_volume_db: f32,
}

impl Project {
    pub fn new() -> Self {
        Self {
            patterns: vec![Pattern::new(1, 16)],
            order: vec![0],
            current_order_idx: 0,
            tracks: Track::defaults(),
            step: 1,
            scale_index: ScaleIndex::default(),
            transpose: 0,
            master_volume_db: 0.0,
        }
    }

    pub fn current_pattern(&self) -> &Pattern {
        let pat_idx = self.order[self.current_order_idx];
        &self.patterns[pat_idx]
    }

    pub fn current_pattern_mut(&mut self) -> &mut Pattern {
        let pat_idx = self.order[self.current_order_idx];
        &mut self.patterns[pat_idx]
    }

    pub fn master_volume_linear(&self) -> f32 {
        if self.master_volume_db <= -60.0 {
            0.0
        } else {
            10.0_f32.powf(self.master_volume_db / 20.0)
        }
    }

    pub fn add_track(&mut self) {
        let idx = self.tracks.len();
        self.tracks
            .push(Track::new_empty(&format!("Track {:02}", idx)));
        for pat in &mut self.patterns {
            pat.add_channel();
        }
    }

    pub fn delete_track(&mut self, idx: usize) {
        if self.tracks.len() <= 1 || idx >= self.tracks.len() {
            return;
        }
        self.tracks.remove(idx);
        for pat in &mut self.patterns {
            pat.remove_channel(idx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_project() {
        let p = Project::new();
        assert_eq!(p.current_pattern().rows, 16);
        assert_eq!(p.current_pattern().bpm, 120);
        assert_eq!(p.current_pattern().computed_rows(), 16);
    }
}

