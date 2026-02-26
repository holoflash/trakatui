use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink};

use crate::pattern::{Cell, Pattern};
use crate::synth::{ChannelSettings, SynthSource};

pub struct AudioEngine {
    device_sink: Option<MixerDeviceSink>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            device_sink: Some(Self::create_sink()),
        }
    }

    fn create_sink() -> MixerDeviceSink {
        let mut device_sink =
            DeviceSinkBuilder::open_default_sink().expect("Failed to open audio output");
        device_sink.log_on_drop(false);
        device_sink
    }

    pub fn stop_all(&mut self) {
        self.device_sink.take();
        self.device_sink = Some(Self::create_sink());
    }

    pub fn play_row(
        &self,
        pattern: &Pattern,
        row: usize,
        step_duration: Duration,
        channel_settings: &[ChannelSettings],
    ) {
        for ch in 0..pattern.channels {
            match pattern.get(ch, row) {
                Cell::NoteOn(note) => {
                    let cs = &channel_settings[ch % channel_settings.len()];
                    let gate = pattern.gate_rows(ch, row);
                    let gate_duration = step_duration.mul_f32(gate as f32);
                    let note_duration =
                        gate_duration + Duration::from_secs_f32(cs.envelope.release);
                    let source = SynthSource::new(
                        cs.waveform,
                        note.frequency(),
                        note_duration,
                        cs.volume,
                        cs.envelope,
                    );
                    self.device_sink.as_ref().unwrap().mixer().add(source);
                }
                Cell::NoteOff | Cell::Empty => {}
            }
        }
    }

    pub fn preview_note(&self, freq: f32, channel: usize, channel_settings: &[ChannelSettings]) {
        let cs = &channel_settings[channel % channel_settings.len()];
        let source = SynthSource::new(
            cs.waveform,
            freq,
            Duration::from_millis(200),
            cs.volume * 0.8,
            cs.envelope,
        );
        self.device_sink.as_ref().unwrap().mixer().add(source);
    }
}
