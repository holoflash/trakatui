use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};

use crate::pattern::{Cell, Pattern};
use crate::synth::{ChannelSettings, SynthSource};

pub struct PeakMonitor<S> {
    source: S,
    peak: Arc<AtomicU32>,
}

impl<S> PeakMonitor<S> {
    pub const fn new(source: S, peak: Arc<AtomicU32>) -> Self {
        Self { source, peak }
    }
}

impl<S: Source<Item = f32>> Iterator for PeakMonitor<S> {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let sample = self.source.next()?;
        let abs = sample.abs();
        let mut current = self.peak.load(Ordering::Relaxed);
        loop {
            let current_f = f32::from_bits(current);
            if abs <= current_f {
                break;
            }
            match self.peak.compare_exchange_weak(
                current,
                abs.to_bits(),
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(c) => current = c,
            }
        }
        Some(sample)
    }
}

impl<S: Source<Item = f32>> Source for PeakMonitor<S> {
    fn current_span_len(&self) -> Option<usize> {
        self.source.current_span_len()
    }

    fn channels(&self) -> NonZero<u16> {
        self.source.channels()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.source.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.source.total_duration()
    }
}

pub struct AudioEngine {
    device_sink: Option<MixerDeviceSink>,
    pub peak_level: Arc<AtomicU32>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            device_sink: Some(Self::create_sink()),
            peak_level: Arc::new(AtomicU32::new(0u32)),
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
        master_volume: f32,
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
                    let monitored =
                        PeakMonitor::new(source.amplify(master_volume), self.peak_level.clone());
                    self.device_sink.as_ref().unwrap().mixer().add(monitored);
                }
                Cell::NoteOff | Cell::Empty => {}
            }
        }
    }

    pub fn preview_note(
        &self,
        freq: f32,
        channel: usize,
        channel_settings: &[ChannelSettings],
        master_volume: f32,
    ) {
        let cs = &channel_settings[channel % channel_settings.len()];
        let source = SynthSource::new(
            cs.waveform,
            freq,
            Duration::from_millis(200),
            cs.volume * 0.8,
            cs.envelope,
        );
        let monitored = PeakMonitor::new(source.amplify(master_volume), self.peak_level.clone());
        self.device_sink.as_ref().unwrap().mixer().add(monitored);
    }
}
