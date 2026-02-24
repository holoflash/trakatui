use std::time::Duration;

use rodio::source::{SineWave, Source};
use rodio::{DeviceSinkBuilder, MixerDeviceSink};

use crate::pattern::Pattern;

pub struct AudioEngine {
    device_sink: MixerDeviceSink,
}

impl AudioEngine {
    pub fn new() -> Self {
        let mut device_sink =
            DeviceSinkBuilder::open_default_sink().expect("Failed to open audio output");
        device_sink.log_on_drop(false);
        Self { device_sink }
    }

    pub fn play_row(&self, pattern: &Pattern, row: usize, step_duration: Duration) {
        for ch in 0..pattern.channels {
            if let Some(note) = pattern.get(ch, row) {
                let freq = note.frequency();
                let source = SineWave::new(freq)
                    .take_duration(step_duration)
                    .amplify(0.3);
                self.device_sink.mixer().add(source);
            }
        }
    }

    pub fn preview_note(&self, freq: f32) {
        let source = SineWave::new(freq)
            .take_duration(Duration::from_millis(150))
            .amplify(0.25);
        self.device_sink.mixer().add(source);
    }
}
