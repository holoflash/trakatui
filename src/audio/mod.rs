pub mod export;
pub mod synth;

use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::{DeviceSinkBuilder, MixerDeviceSink, Source};

use crate::project::{Cell, ChannelSettings, Pattern, parse_pitch_bend};

use synth::{PitchBendControl, SynthSource};

struct ChannelPlayState {
    bend_control: Arc<PitchBendControl>,
    base_freq: f32,
    note_start_row: Option<usize>,
}

impl ChannelPlayState {
    fn new() -> Self {
        Self {
            bend_control: Arc::new(PitchBendControl::new()),
            base_freq: 0.0,
            note_start_row: None,
        }
    }
}

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
    channel_state: Vec<ChannelPlayState>,
}

impl AudioEngine {
    pub fn new() -> Self {
        Self {
            device_sink: Some(Self::create_sink()),
            peak_level: Arc::new(AtomicU32::new(0u32)),
            channel_state: Vec::new(),
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
        for cs in &mut self.channel_state {
            *cs = ChannelPlayState::new();
        }
    }

    pub fn play_row(
        &mut self,
        pattern: &Pattern,
        row: usize,
        step_duration: Duration,
        channel_settings: &[ChannelSettings],
        master_volume: f32,
    ) {
        while self.channel_state.len() < pattern.channels {
            self.channel_state.push(ChannelPlayState::new());
        }

        for ch in 0..pattern.channels {
            let effect = pattern.get_effect(ch, row);
            let state = &mut self.channel_state[ch];

            match pattern.get(ch, row) {
                Cell::NoteOn(note) => {
                    let cs = &channel_settings[ch % channel_settings.len()];
                    let gate_f64 = pattern.gate_rows(ch, row) as f64;
                    let gate_duration = step_duration.mul_f64(gate_f64);
                    let note_duration =
                        gate_duration + Duration::from_secs_f32(cs.envelope.release);

                    let bend = Arc::new(PitchBendControl::new());

                    if let Some(cmd) = effect
                        && let Some((semitones, steps)) = parse_pitch_bend(cmd)
                        && semitones != 0
                        && steps > 0
                    {
                        let target = note.frequency() * (f32::from(semitones) / 12.0).exp2();
                        let dur = step_duration.as_secs_f32() * f32::from(steps);
                        bend.set(target, 0.0, dur);
                    }

                    let source = SynthSource::new(
                        cs.waveform,
                        note.frequency(),
                        note_duration,
                        cs.volume,
                        cs.envelope,
                        bend.clone(),
                    );
                    let monitored =
                        PeakMonitor::new(source.amplify(master_volume), self.peak_level.clone());
                    self.device_sink.as_ref().unwrap().mixer().add(monitored);

                    state.bend_control = bend;
                    state.base_freq = note.frequency();
                    state.note_start_row = Some(row);
                }
                Cell::NoteOff => {
                    state.note_start_row = None;
                }
                Cell::Empty => {
                    if let (Some(start_row), Some(cmd)) = (state.note_start_row, effect)
                        && let Some((semitones, steps)) = parse_pitch_bend(cmd)
                    {
                        if semitones != 0 && steps > 0 {
                            let offset = step_duration.as_secs_f32() * (row - start_row) as f32;
                            let target = state.base_freq * (f32::from(semitones) / 12.0).exp2();
                            let dur = step_duration.as_secs_f32() * f32::from(steps);
                            state.bend_control.set(target, offset, dur);
                        } else {
                            state.bend_control.reset();
                        }
                    }
                }
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
        let bend = Arc::new(PitchBendControl::new());
        let source = SynthSource::new(
            cs.waveform,
            freq,
            Duration::from_millis(200),
            cs.volume * 0.8,
            cs.envelope,
            bend,
        );
        let monitored = PeakMonitor::new(source.amplify(master_volume), self.peak_level.clone());
        self.device_sink.as_ref().unwrap().mixer().add(monitored);
    }
}
