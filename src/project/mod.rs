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
    pub bpm: u16,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub note_value: u8,
    pub measures: u8,
    pub step: usize,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub master_volume_db: f32,
}

impl Project {
    pub fn new() -> Self {
        let time_sig_numerator: u8 = 4;
        let note_value: u8 = 4;
        let measures: u8 = 1;
        let rows = time_sig_numerator as usize * note_value as usize * measures as usize;
        Self {
            patterns: vec![Pattern::new(1, rows)],
            order: vec![0],
            current_order_idx: 0,
            tracks: Track::defaults(),
            bpm: 120,
            time_sig_numerator,
            time_sig_denominator: 4,
            note_value,
            measures,
            step: 1,
            scale_index: ScaleIndex::default(),
            transpose: 0,
            master_volume_db: 0.0,
        }
    }

    pub fn computed_rows(&self) -> usize {
        self.time_sig_numerator as usize
            * self.note_value as usize
            * self.measures as usize
    }

    pub fn rows_per_beat(&self) -> usize {
        self.note_value as usize
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
    fn default_project_rows() {
        let p = Project::new();
        assert_eq!(p.time_sig_numerator, 4);
        assert_eq!(p.time_sig_denominator, 4);
        assert_eq!(p.note_value, 4);
        assert_eq!(p.measures, 1);
        assert_eq!(p.computed_rows(), 16);
        assert_eq!(p.current_pattern().rows, 16);
    }

    #[test]
    fn computed_rows_4_4_16th_2_bars() {
        let mut p = Project::new();
        p.time_sig_numerator = 4;
        p.note_value = 16;
        p.measures = 2;
        assert_eq!(p.computed_rows(), 128);
    }

    #[test]
    fn computed_rows_7_8_8th_1_bar() {
        let mut p = Project::new();
        p.time_sig_numerator = 7;
        p.time_sig_denominator = 8;
        p.note_value = 8;
        p.measures = 1;
        assert_eq!(p.computed_rows(), 56);
    }

    #[test]
    fn computed_rows_3_4_4th_3_bars() {
        let mut p = Project::new();
        p.time_sig_numerator = 3;
        p.note_value = 4;
        p.measures = 3;
        assert_eq!(p.computed_rows(), 36);
    }

    #[test]
    fn rows_per_beat_equals_note_value() {
        let mut p = Project::new();
        p.note_value = 16;
        assert_eq!(p.rows_per_beat(), 16);
        p.note_value = 8;
        assert_eq!(p.rows_per_beat(), 8);
        p.note_value = 1;
        assert_eq!(p.rows_per_beat(), 1);
    }

    #[test]
    fn resize_on_time_sig_change() {
        let mut p = Project::new();
        assert_eq!(p.current_pattern().rows, 16);
        p.time_sig_numerator = 3;
        p.measures = 2;
        let new_rows = p.computed_rows();
        p.current_pattern_mut().resize(new_rows);
        assert_eq!(p.current_pattern().rows, 24);
    }
}
