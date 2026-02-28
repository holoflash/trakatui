/// Everything in this module is saveable project data.
pub mod channel;
pub mod pattern;
pub mod sample;

pub use channel::{ChannelSettings, Envelope, Waveform};
pub use pattern::{Cell, Note, Pattern, effect_display, parse_pitch_bend};
pub use sample::SampleData;

use crate::scale::ScaleIndex;

pub struct Project {
    pub pattern: Pattern,
    pub channel_settings: Vec<ChannelSettings>,
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
            pattern: Pattern::new(8, 32),
            channel_settings: ChannelSettings::defaults(),
            bpm: 120,
            subdivision: 4,
            step: 1,
            scale_index: ScaleIndex::default(),
            transpose: 0,
            master_volume_db: 0.0,
        }
    }

    pub fn master_volume_linear(&self) -> f32 {
        if self.master_volume_db <= -60.0 {
            0.0
        } else {
            10.0_f32.powf(self.master_volume_db / 20.0)
        }
    }
}
