use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::Source;

use crate::project::{Cell, ChannelSettings, Effect, Envelope, SampleData, Waveform};

pub const SAMPLE_RATE: u32 = 44100;
const SAMPLE_RATE_F: f32 = SAMPLE_RATE as f32;
const DEFAULT_SPEED: u16 = 6;
const PREVIEW_DURATION_SECS: f32 = 0.2;

pub enum Command {
    Play {
        start_row: usize,
        pattern: Arc<PatternSnapshot>,
        settings: Arc<PlaybackSettings>,
    },
    Stop,
    UpdateSettings {
        settings: Arc<PlaybackSettings>,
    },
    UpdatePattern {
        pattern: Arc<PatternSnapshot>,
    },
    PreviewNote {
        frequency: f32,
        waveform: Waveform,
        volume: f32,
        envelope: Envelope,
        sample_data: Option<Arc<SampleData>>,
        master_volume: f32,
    },
}

pub struct PlaybackSettings {
    pub bpm: u16,
    pub master_volume: f32,
    pub channel_settings: Vec<ChannelSettings>,
}

pub struct PatternSnapshot {
    pub channels: usize,
    pub rows: usize,
    data: Vec<Vec<Cell>>,
    volumes: Vec<Vec<Option<u8>>>,
    effects: Vec<Vec<Option<Effect>>>,
}

impl PatternSnapshot {
    pub fn from_pattern(pattern: &crate::project::Pattern) -> Self {
        Self {
            channels: pattern.channels,
            rows: pattern.rows,
            data: pattern.data.clone(),
            volumes: pattern.volumes.clone(),
            effects: pattern.effects.clone(),
        }
    }

    fn gate_rows(&self, channel: usize, row: usize) -> usize {
        let mut count = 1;
        for r in (row + 1)..self.rows {
            match self.data[channel][r] {
                Cell::NoteOn(_) | Cell::NoteOff => break,
                Cell::Empty => count += 1,
            }
        }
        count
    }
}

struct Channel {
    active: bool,
    waveform: Waveform,
    frequency: f32,
    phase: f32,
    noise_held: f32,
    sample_data: Option<Arc<SampleData>>,
    sample_position: f64,
    sample_step: f64,
    envelope: Envelope,
    volume: f32,
    elapsed_samples: u32,
    total_samples: u32,
    note_duration: f32,
    base_frequency: f32,
    period: f32,
    porta_speed: i16,
}

impl Channel {
    fn new() -> Self {
        Self {
            active: false,
            waveform: Waveform::Sine,
            frequency: 0.0,
            phase: 0.0,
            noise_held: 0.0,
            sample_data: None,
            sample_position: 0.0,
            sample_step: 0.0,
            envelope: Envelope {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.0,
            },
            volume: 1.0,
            elapsed_samples: 0,
            total_samples: 0,
            note_duration: 0.0,
            base_frequency: 0.0,
            period: 0.0,
            porta_speed: 0,
        }
    }

    fn freq_to_period(freq: f32) -> f32 {
        if freq > 0.0 {
            7680.0 - (freq / 8.1758).log2() * 768.0
        } else {
            0.0
        }
    }

    fn period_to_freq(period: f32) -> f32 {
        8.1758 * 2.0_f32.powf((7680.0 - period) / 768.0)
    }

    fn trigger(
        &mut self,
        frequency: f32,
        waveform: Waveform,
        volume: f32,
        envelope: Envelope,
        sample_data: &Option<Arc<SampleData>>,
        total_samples: u32,
    ) {
        self.active = true;
        self.waveform = waveform;
        self.frequency = frequency;
        self.base_frequency = frequency;
        self.period = Self::freq_to_period(frequency);
        self.volume = volume;
        self.envelope = envelope;
        self.phase = 0.0;
        self.elapsed_samples = 0;
        self.total_samples = total_samples;
        self.note_duration = total_samples as f32 / SAMPLE_RATE_F;
        self.noise_held = fastrand::f32().mul_add(2.0, -1.0);
        self.porta_speed = 0;

        if waveform == Waveform::Sampler {
            if let Some(data) = sample_data {
                let base_freq = 440.0 * ((f32::from(data.base_note) - 69.0) / 12.0).exp2();
                let rate = f64::from(frequency) / f64::from(base_freq);
                self.sample_step = (f64::from(data.sample_rate) / f64::from(SAMPLE_RATE)) * rate;
                self.sample_position = 0.0;
                self.sample_data = Some(Arc::clone(data));
            } else {
                self.active = false;
            }
        } else {
            self.sample_data = None;
        }
    }

    fn note_off(&mut self) {
        self.active = false;
    }

    fn next_sample(&mut self) -> f32 {
        if !self.active || self.elapsed_samples >= self.total_samples {
            self.active = false;
            return 0.0;
        }

        let time = self.elapsed_samples as f32 / SAMPLE_RATE_F;
        let env = self.envelope.amplitude(time, self.note_duration);

        let sample = if self.waveform == Waveform::Sampler {
            self.sampler_sample()
        } else if self.waveform == Waveform::Noise {
            self.noise_held
        } else {
            self.waveform.sample(self.phase)
        };

        if self.waveform != Waveform::Sampler {
            self.phase += self.frequency / SAMPLE_RATE_F;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                if self.waveform == Waveform::Noise {
                    self.noise_held = fastrand::f32().mul_add(2.0, -1.0);
                }
            }
        }

        self.elapsed_samples += 1;
        sample * env * self.volume
    }

    fn sampler_sample(&mut self) -> f32 {
        let Some(data) = self.sample_data.as_ref() else {
            return 0.0;
        };
        let idx = self.sample_position as usize;
        let frac = (self.sample_position - idx as f64) as f32;
        let samples = &data.samples_f32;

        let sample = if idx >= samples.len() {
            0.0
        } else if idx + 1 < samples.len() {
            samples[idx] + (samples[idx + 1] - samples[idx]) * frac
        } else {
            samples[idx]
        };

        self.sample_position += self.sample_step;
        sample
    }

    fn tick_update(&mut self) {
        if !self.active || self.porta_speed == 0 {
            return;
        }
        self.period = (self.period + self.porta_speed as f32).clamp(50.0, 7680.0);
        self.frequency = Self::period_to_freq(self.period);
        if self.waveform == Waveform::Sampler
            && let Some(data) = self.sample_data.as_ref()
        {
            let base_freq = 440.0 * ((f32::from(data.base_note) - 69.0) / 12.0).exp2();
            let rate = f64::from(self.frequency) / f64::from(base_freq);
            self.sample_step = (f64::from(data.sample_rate) / f64::from(SAMPLE_RATE)) * rate;
        }
    }
}

pub struct TrackerSource {
    channels: Vec<Channel>,
    preview_channel: Channel,
    playing: bool,
    pattern: Option<Arc<PatternSnapshot>>,
    settings: Option<Arc<PlaybackSettings>>,
    current_row: usize,
    samples_per_tick: f64,
    tick_sample_counter: f64,
    tick_in_row: u16,
    speed: u16,
    receiver: mpsc::Receiver<Command>,
    playback_row: Arc<AtomicUsize>,
    master_volume: f32,
}

impl TrackerSource {
    pub fn new(receiver: mpsc::Receiver<Command>, playback_row: Arc<AtomicUsize>) -> Self {
        Self {
            channels: Vec::new(),
            preview_channel: Channel::new(),
            playing: false,
            pattern: None,
            settings: None,
            current_row: 0,
            samples_per_tick: 882.0,
            tick_sample_counter: 0.0,
            tick_in_row: 0,
            speed: DEFAULT_SPEED,
            receiver,
            playback_row,
            master_volume: 1.0,
        }
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.receiver.try_recv() {
            match cmd {
                Command::Play {
                    start_row,
                    pattern,
                    settings,
                } => self.start_playback(start_row, pattern, settings),
                Command::Stop => {
                    self.playing = false;
                    for ch in &mut self.channels {
                        ch.active = false;
                    }
                }
                Command::UpdateSettings { settings } => {
                    if self.playing {
                        self.samples_per_tick =
                            f64::from(SAMPLE_RATE) * 5.0 / (f64::from(settings.bpm) * 2.0);
                        self.master_volume = settings.master_volume;
                        self.settings = Some(settings);
                    }
                }
                Command::UpdatePattern { pattern } => {
                    if self.playing {
                        if self.current_row >= pattern.rows {
                            self.current_row = 0;
                        }
                        self.pattern = Some(pattern);
                    }
                }
                Command::PreviewNote {
                    frequency,
                    waveform,
                    volume,
                    envelope,
                    sample_data,
                    master_volume,
                } => {
                    let total = (PREVIEW_DURATION_SECS * SAMPLE_RATE_F).round() as u32;
                    self.preview_channel.trigger(
                        frequency,
                        waveform,
                        volume * 0.8,
                        envelope,
                        &sample_data,
                        total,
                    );
                    if !self.playing {
                        self.master_volume = master_volume;
                    }
                }
            }
        }
    }

    fn start_playback(
        &mut self,
        start_row: usize,
        pattern: Arc<PatternSnapshot>,
        settings: Arc<PlaybackSettings>,
    ) {
        while self.channels.len() < pattern.channels {
            self.channels.push(Channel::new());
        }
        for ch in &mut self.channels {
            *ch = Channel::new();
        }

        self.samples_per_tick = f64::from(SAMPLE_RATE) * 5.0 / (f64::from(settings.bpm) * 2.0);
        self.master_volume = settings.master_volume;
        self.current_row = start_row;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.pattern = Some(pattern);
        self.settings = Some(settings);
        self.playing = true;

        self.process_row();
        self.playback_row.store(self.current_row, Ordering::Relaxed);
    }

    fn process_row(&mut self) {
        let Some(pattern) = self.pattern.as_ref() else {
            return;
        };
        let Some(settings) = self.settings.as_ref() else {
            return;
        };

        let samples_per_row = (self.samples_per_tick * f64::from(self.speed)).round() as u32;

        for ch_idx in 0..pattern.channels.min(self.channels.len()) {
            let cs = &settings.channel_settings[ch_idx % settings.channel_settings.len()];
            let effect = pattern.effects[ch_idx][self.current_row];
            let volume = pattern.volumes[ch_idx][self.current_row];
            let cell = pattern.data[ch_idx][self.current_row];
            let channel = &mut self.channels[ch_idx];

            match cell {
                Cell::NoteOn(note) => {
                    let gate_rows = pattern.gate_rows(ch_idx, self.current_row);
                    let gate_samples = samples_per_row * gate_rows as u32;
                    let release_samples = (cs.envelope.release * SAMPLE_RATE_F).round() as u32;
                    let vol = volume.map_or(cs.volume, |v| v.min(64) as f32 / 64.0);

                    channel.trigger(
                        note.frequency(),
                        cs.waveform,
                        vol,
                        cs.envelope,
                        &cs.sample_data,
                        gate_samples + release_samples,
                    );
                }
                Cell::NoteOff => channel.note_off(),
                Cell::Empty => {}
            }

            if let Some(v) = volume {
                channel.volume = v.min(64) as f32 / 64.0;
            }

            match effect {
                Some(Effect { kind: 1, param }) => {
                    channel.porta_speed = -(param as i16);
                }
                Some(Effect { kind: 2, param }) => {
                    channel.porta_speed = param as i16;
                }
                _ => {
                    channel.porta_speed = 0;
                }
            }
        }
    }

    fn tick(&mut self) {
        self.tick_in_row += 1;
        if self.tick_in_row >= self.speed {
            self.tick_in_row = 0;
            if let Some(pattern) = self.pattern.as_ref() {
                self.current_row = (self.current_row + 1) % pattern.rows;
                self.playback_row.store(self.current_row, Ordering::Relaxed);
            }
            self.process_row();
        } else {
            for ch in &mut self.channels {
                ch.tick_update();
            }
        }
    }
}

impl Iterator for TrackerSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.process_commands();

        let mut mixed = 0.0_f32;

        if self.playing {
            self.tick_sample_counter += 1.0;
            if self.tick_sample_counter >= self.samples_per_tick {
                self.tick_sample_counter -= self.samples_per_tick;
                self.tick();
            }
            for ch in &mut self.channels {
                mixed += ch.next_sample();
            }
        }

        mixed += self.preview_channel.next_sample();

        Some(mixed * self.master_volume)
    }
}

impl Source for TrackerSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub fn export_source(
    pattern: &crate::project::Pattern,
    bpm: u16,
    channel_settings: &[ChannelSettings],
    master_volume: f32,
) -> (TrackerSource, usize) {
    let (sender, receiver) = mpsc::channel();
    let playback_row = Arc::new(AtomicUsize::new(0));

    let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
    let settings = Arc::new(PlaybackSettings {
        bpm,
        master_volume,
        channel_settings: channel_settings.to_vec(),
    });

    let samples_per_tick = f64::from(SAMPLE_RATE) * 5.0 / (f64::from(bpm) * 2.0);
    let samples_per_row = (samples_per_tick * f64::from(DEFAULT_SPEED)).round() as usize;
    let total_samples = samples_per_row * pattern.rows;

    let _ = sender.send(Command::Play {
        start_row: 0,
        pattern: snapshot,
        settings,
    });
    drop(sender);

    let source = TrackerSource::new(receiver, playback_row);
    (source, total_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source() -> (mpsc::Sender<Command>, TrackerSource, Arc<AtomicUsize>) {
        let (tx, rx) = mpsc::channel();
        let row = Arc::new(AtomicUsize::new(0));
        let source = TrackerSource::new(rx, row.clone());
        (tx, source, row)
    }

    #[test]
    fn tick_timing_125bpm() {
        let (tx, mut source, row) = make_source();

        let mut pattern = crate::project::Pattern::new(1, 4);
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        let snapshot = Arc::new(PatternSnapshot::from_pattern(&pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm: 125,
            master_volume: 1.0,
            channel_settings: vec![ChannelSettings::default_for(Waveform::Sine)],
        });

        tx.send(Command::Play {
            start_row: 0,
            pattern: snapshot,
            settings,
        })
        .unwrap();

        for _ in 0..5292 {
            source.next();
        }
        source.next();
        assert_eq!(row.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn stop_silences_output() {
        let (tx, mut source, _) = make_source();

        let mut pattern = crate::project::Pattern::new(1, 4);
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        let snapshot = Arc::new(PatternSnapshot::from_pattern(&pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm: 125,
            master_volume: 1.0,
            channel_settings: vec![ChannelSettings::default_for(Waveform::Sine)],
        });

        tx.send(Command::Play {
            start_row: 0,
            pattern: snapshot,
            settings,
        })
        .unwrap();

        for _ in 0..100 {
            source.next();
        }

        tx.send(Command::Stop).unwrap();
        source.next();

        for _ in 0..100 {
            assert_eq!(source.next(), Some(0.0));
        }
    }
}
