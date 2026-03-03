use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::Source;

use crate::project::{Cell, Effect, Envelope, Instrument, SampleData};

pub const SAMPLE_RATE: u32 = 44100;
const SAMPLE_RATE_F: f32 = SAMPLE_RATE as f32;
const DEFAULT_SPEED: u16 = 6;
const PREVIEW_DURATION_SECS: f32 = 0.2;

pub enum Command {
    Play {
        start_row: usize,
        start_order: usize,
        patterns: Vec<Arc<PatternSnapshot>>,
        order: Vec<usize>,
        settings: Arc<PlaybackSettings>,
    },
    Stop,
    UpdateSettings {
        settings: Arc<PlaybackSettings>,
    },
    UpdatePatterns {
        patterns: Vec<Arc<PatternSnapshot>>,
        order: Vec<usize>,
    },
    PreviewNote {
        frequency: f32,
        volume: f32,
        envelope: Envelope,
        sample_data: Arc<SampleData>,
        master_volume: f32,
    },
}

pub struct PlaybackSettings {
    pub bpm: u16,
    pub master_volume: f32,
    pub instruments: Vec<Instrument>,
}

pub struct PatternSnapshot {
    pub channels: usize,
    pub rows: usize,
    data: Vec<Vec<Cell>>,
    instruments: Vec<Vec<Option<u8>>>,
    volumes: Vec<Vec<Option<u8>>>,
    effects: Vec<Vec<Option<Effect>>>,
}

impl PatternSnapshot {
    pub fn from_pattern(pattern: &crate::project::Pattern) -> Self {
        Self {
            channels: pattern.channels,
            rows: pattern.rows,
            data: pattern.data.clone(),
            instruments: pattern.instruments.clone(),
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
    sample_data: Arc<SampleData>,
    sample_position: f64,
    sample_step: f64,
    envelope: Envelope,
    volume: f32,
    elapsed_samples: u32,
    total_samples: u32,
    note_duration: f32,
    period: f32,
    porta_speed: i16,
    current_instrument: usize,
}

impl Channel {
    fn new() -> Self {
        Self {
            active: false,
            sample_data: SampleData::silent(),
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
            period: 0.0,
            porta_speed: 0,
            current_instrument: 0,
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

    fn compute_sample_step(frequency: f32, data: &SampleData) -> f64 {
        let base_freq = 440.0 * ((f32::from(data.base_note) - 69.0) / 12.0).exp2();
        let rate = f64::from(frequency) / f64::from(base_freq);
        (f64::from(data.sample_rate) / f64::from(SAMPLE_RATE)) * rate
    }

    fn trigger(
        &mut self,
        frequency: f32,
        volume: f32,
        envelope: Envelope,
        sample_data: &Arc<SampleData>,
        total_samples: u32,
    ) {
        self.active = true;
        self.period = Self::freq_to_period(frequency);
        self.volume = volume;
        self.envelope = envelope;
        self.elapsed_samples = 0;
        self.total_samples = total_samples;
        self.note_duration = total_samples as f32 / SAMPLE_RATE_F;
        self.porta_speed = 0;
        self.sample_data = Arc::clone(sample_data);
        self.sample_step = Self::compute_sample_step(frequency, sample_data);
        self.sample_position = 0.0;
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

        let data = &self.sample_data;
        let samples = &data.samples_f32;
        let len = samples.len();

        if len == 0 {
            self.elapsed_samples += 1;
            return 0.0;
        }

        let idx = self.sample_position as usize;
        let frac = (self.sample_position - idx as f64) as f32;

        let sample = if idx >= len {
            0.0
        } else if idx + 1 < len {
            samples[idx] + (samples[idx + 1] - samples[idx]) * frac
        } else {
            samples[idx]
        };

        self.sample_position += self.sample_step;

        if data.loop_length > 0 {
            let loop_end = data.loop_start + data.loop_length;
            if self.sample_position >= loop_end as f64 {
                self.sample_position -= data.loop_length as f64;
            }
        }

        self.elapsed_samples += 1;
        sample * env * self.volume
    }

    fn tick_update(&mut self) {
        if !self.active || self.porta_speed == 0 {
            return;
        }
        self.period = (self.period + self.porta_speed as f32).clamp(50.0, 7680.0);
        let freq = Self::period_to_freq(self.period);
        self.sample_step = Self::compute_sample_step(freq, &self.sample_data);
    }
}

pub struct TrackerSource {
    channels: Vec<Channel>,
    preview_channel: Channel,
    playing: bool,
    patterns: Vec<Arc<PatternSnapshot>>,
    order: Vec<usize>,
    current_order_idx: usize,
    settings: Option<Arc<PlaybackSettings>>,
    current_row: usize,
    samples_per_tick: f64,
    tick_sample_counter: f64,
    tick_in_row: u16,
    speed: u16,
    receiver: mpsc::Receiver<Command>,
    playback_row: Arc<AtomicUsize>,
    playback_order: Arc<AtomicUsize>,
    master_volume: f32,
}

impl TrackerSource {
    pub fn new(
        receiver: mpsc::Receiver<Command>,
        playback_row: Arc<AtomicUsize>,
        playback_order: Arc<AtomicUsize>,
    ) -> Self {
        Self {
            channels: Vec::new(),
            preview_channel: Channel::new(),
            playing: false,
            patterns: Vec::new(),
            order: Vec::new(),
            current_order_idx: 0,
            settings: None,
            current_row: 0,
            samples_per_tick: 882.0,
            tick_sample_counter: 0.0,
            tick_in_row: 0,
            speed: DEFAULT_SPEED,
            receiver,
            playback_row,
            playback_order,
            master_volume: 1.0,
        }
    }

    fn process_commands(&mut self) {
        while let Ok(cmd) = self.receiver.try_recv() {
            match cmd {
                Command::Play {
                    start_row,
                    start_order,
                    patterns,
                    order,
                    settings,
                } => self.start_playback(start_row, start_order, patterns, order, settings),
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
                Command::UpdatePatterns { patterns, order } => {
                    if self.playing {
                        self.patterns = patterns;
                        self.order = order;
                        if self.current_order_idx >= self.order.len() {
                            self.current_order_idx = 0;
                            self.current_row = 0;
                        }
                        let pat_idx = self.order[self.current_order_idx];
                        if self.current_row >= self.patterns[pat_idx].rows {
                            self.current_row = 0;
                        }
                    }
                }
                Command::PreviewNote {
                    frequency,
                    volume,
                    envelope,
                    sample_data,
                    master_volume,
                } => {
                    let total = (PREVIEW_DURATION_SECS * SAMPLE_RATE_F).round() as u32;
                    self.preview_channel.trigger(
                        frequency,
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
        start_order: usize,
        patterns: Vec<Arc<PatternSnapshot>>,
        order: Vec<usize>,
        settings: Arc<PlaybackSettings>,
    ) {
        let pat_idx = order[start_order];
        while self.channels.len() < patterns[pat_idx].channels {
            self.channels.push(Channel::new());
        }
        for ch in &mut self.channels {
            *ch = Channel::new();
        }

        self.samples_per_tick = f64::from(SAMPLE_RATE) * 5.0 / (f64::from(settings.bpm) * 2.0);
        self.master_volume = settings.master_volume;
        self.current_row = start_row;
        self.current_order_idx = start_order;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.patterns = patterns;
        self.order = order;
        self.settings = Some(settings);
        self.playing = true;

        self.process_row();
        self.playback_row.store(self.current_row, Ordering::Relaxed);
        self.playback_order
            .store(self.current_order_idx, Ordering::Relaxed);
    }

    fn process_row(&mut self) {
        let pat_idx = self.order[self.current_order_idx];
        let pattern = match self.patterns.get(pat_idx) {
            Some(p) => p.clone(),
            None => return,
        };
        let Some(settings) = self.settings.as_ref() else {
            return;
        };

        let samples_per_row = (self.samples_per_tick * f64::from(self.speed)).round() as u32;

        for ch_idx in 0..pattern.channels.min(self.channels.len()) {
            let inst_num = pattern.instruments[ch_idx][self.current_row];
            if let Some(n) = inst_num {
                self.channels[ch_idx].current_instrument = n as usize;
            }
            let ci = self.channels[ch_idx].current_instrument;
            let inst = &settings.instruments[ci % settings.instruments.len()];
            let effect = pattern.effects[ch_idx][self.current_row];
            let volume = pattern.volumes[ch_idx][self.current_row];
            let cell = pattern.data[ch_idx][self.current_row];
            let channel = &mut self.channels[ch_idx];

            match cell {
                Cell::NoteOn(note) => {
                    let gate_rows = pattern.gate_rows(ch_idx, self.current_row);
                    let gate_samples = samples_per_row * gate_rows as u32;
                    let release_samples = (inst.envelope.release * SAMPLE_RATE_F).round() as u32;
                    let vol = volume.map_or(1.0, |v| v.min(64) as f32 / 64.0);

                    channel.trigger(
                        note.frequency(),
                        vol,
                        inst.envelope,
                        &inst.sample_data,
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
            if !self.patterns.is_empty() {
                let pat_idx = self.order[self.current_order_idx];
                let rows = self.patterns[pat_idx].rows;
                self.current_row += 1;
                if self.current_row >= rows {
                    self.current_row = 0;
                    self.current_order_idx = (self.current_order_idx + 1) % self.order.len();
                    self.playback_order
                        .store(self.current_order_idx, Ordering::Relaxed);
                }
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
    patterns: &[crate::project::Pattern],
    order: &[usize],
    bpm: u16,
    instruments: &[Instrument],
    master_volume: f32,
) -> (TrackerSource, usize) {
    let (sender, receiver) = mpsc::channel();
    let playback_row = Arc::new(AtomicUsize::new(0));
    let playback_order = Arc::new(AtomicUsize::new(0));

    let snapshots: Vec<Arc<PatternSnapshot>> = patterns
        .iter()
        .map(|p| Arc::new(PatternSnapshot::from_pattern(p)))
        .collect();
    let settings = Arc::new(PlaybackSettings {
        bpm,
        master_volume,
        instruments: instruments.to_vec(),
    });

    let samples_per_tick = f64::from(SAMPLE_RATE) * 5.0 / (f64::from(bpm) * 2.0);
    let samples_per_row = (samples_per_tick * f64::from(DEFAULT_SPEED)).round() as usize;
    let total_samples: usize = order
        .iter()
        .map(|&idx| samples_per_row * patterns[idx].rows)
        .sum();

    let _ = sender.send(Command::Play {
        start_row: 0,
        start_order: 0,
        patterns: snapshots,
        order: order.to_vec(),
        settings,
    });
    drop(sender);

    let source = TrackerSource::new(receiver, playback_row, playback_order);
    (source, total_samples)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source() -> (mpsc::Sender<Command>, TrackerSource, Arc<AtomicUsize>) {
        let (tx, rx) = mpsc::channel();
        let row = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(AtomicUsize::new(0));
        let source = TrackerSource::new(rx, row.clone(), order);
        (tx, source, row)
    }

    fn play_cmd(pattern: &crate::project::Pattern) -> Command {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm: 125,
            master_volume: 1.0,
            instruments: Instrument::defaults(),
        });
        Command::Play {
            start_row: 0,
            start_order: 0,
            patterns: vec![snapshot],
            order: vec![0],
            settings,
        }
    }

    #[test]
    fn tick_timing_125bpm() {
        let (tx, mut source, row) = make_source();

        let mut pattern = crate::project::Pattern::new(1, 4);
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        tx.send(play_cmd(&pattern)).unwrap();

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

        tx.send(play_cmd(&pattern)).unwrap();

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
