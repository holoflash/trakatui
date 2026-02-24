use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink};

use crate::pattern::{Cell, Pattern};
use crate::synth::{CHANNEL_INSTRUMENTS, SynthSource};

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
            match pattern.get(ch, row) {
                Cell::NoteOn(note) => {
                    let waveform = CHANNEL_INSTRUMENTS[ch % CHANNEL_INSTRUMENTS.len()];
                    let source = SynthSource::new(waveform, note.frequency(), step_duration, 0.3);
                    self.device_sink.mixer().add(source);
                }
                Cell::NoteOff | Cell::Empty => {}
            }
        }
    }

    pub fn preview_note(&self, freq: f32, channel: usize) {
        let waveform = CHANNEL_INSTRUMENTS[channel % CHANNEL_INSTRUMENTS.len()];
        let source = SynthSource::new(waveform, freq, Duration::from_millis(200), 0.25);
        self.device_sink.mixer().add(source);
    }
}
