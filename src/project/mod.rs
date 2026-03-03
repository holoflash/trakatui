/// Everything in this module is saveable project data.
pub mod channel;
pub mod pattern;
pub mod sample;

pub use channel::{Envelope, Instrument, Waveform};
pub use pattern::{
    Cell, Effect, Note, Pattern, effect_display, instrument_display, volume_display,
};
pub use sample::SampleData;

use crate::app::scale::ScaleIndex;

pub struct Project {
    pub patterns: Vec<Pattern>,
    pub order: Vec<usize>,
    pub current_order_idx: usize,
    pub instruments: Vec<Instrument>,
    pub bpm: u16,
    pub subdivision: usize,
    pub step: usize,
    pub scale_index: ScaleIndex,
    pub transpose: i8,
    pub master_volume_db: f32,
}

impl Project {
    pub fn new() -> Self {
        Self {
            patterns: vec![Pattern::new(6, 32)],
            order: vec![0],
            current_order_idx: 0,
            instruments: Instrument::defaults(),
            bpm: 120,
            subdivision: 4,
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
}
