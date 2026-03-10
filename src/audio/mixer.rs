use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::Source;

use crate::project::channel::{Track, VolEnvelope};
use crate::project::sample::LoopType;
use crate::project::{Cell, SampleData};

pub const SAMPLE_RATE: u32 = 44100;
const TICKS_PER_ROW: u16 = 6;

pub const SCOPE_SIZE: usize = 256;
const SCOPE_DOWNSAMPLE: usize = 4;

#[inline]
fn hermite_interpolate(s0: f32, s1: f32, s2: f32, s3: f32, t: f32) -> f32 {
    let c0 = s1;
    let c1 = 0.5 * (s2 - s0);
    let c2 = s0 - 2.5 * s1 + 2.0 * s2 - 0.5 * s3;
    let c3 = 0.5 * (s3 - s0) + 1.5 * (s1 - s2);
    ((c3 * t + c2) * t + c1) * t + c0
}

pub struct ScopeBuffer {
    pub samples: Box<[AtomicU32; SCOPE_SIZE]>,
    pub write_pos: AtomicUsize,
}

impl ScopeBuffer {
    pub fn new() -> Self {
        Self {
            samples: Box::new(std::array::from_fn(|_| AtomicU32::new(0))),
            write_pos: AtomicUsize::new(0),
        }
    }

    pub fn push(&self, val: f32) {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) % SCOPE_SIZE;
        self.samples[pos].store(val.to_bits(), Ordering::Relaxed);
    }

    pub fn read_all(&self) -> [f32; SCOPE_SIZE] {
        let wp = self.write_pos.load(Ordering::Relaxed);
        let mut out = [0.0f32; SCOPE_SIZE];
        for i in 0..SCOPE_SIZE {
            let idx = (wp + i) % SCOPE_SIZE;
            out[i] = f32::from_bits(self.samples[idx].load(Ordering::Relaxed));
        }
        out
    }

    pub fn clear(&self) {
        for s in self.samples.iter() {
            s.store(0u32, Ordering::Relaxed);
        }
    }
}

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
        panning: f32,
        vol_envelope: VolEnvelope,
        sample_data: Arc<SampleData>,
        master_volume: f32,
        vol_fadeout: u16,
    },
}

pub struct PlaybackSettings {
    pub bpm: u16,
    pub rows_per_beat: usize,
    pub master_volume: f32,
    pub tracks: Vec<Track>,
    pub muted_channels: Vec<bool>,
}

pub struct PatternSnapshot {
    pub channels: usize,
    pub rows: usize,
    data: Vec<Vec<Cell>>,
    panning: Vec<Vec<Option<u8>>>,
    volumes: Vec<Vec<Option<u8>>>,
}

impl PatternSnapshot {
    pub fn from_pattern(pattern: &crate::project::Pattern) -> Self {
        Self {
            channels: pattern.channels,
            rows: pattern.rows,
            data: pattern.data.clone(),
            panning: pattern.panning.clone(),
            volumes: pattern.volumes.clone(),
        }
    }
}

struct Channel {
    active: bool,
    sample_data: Arc<SampleData>,
    sample_position: f64,
    sample_step: f64,
    sample_direction: f64,
    vol_envelope: VolEnvelope,
    volume: f32,
    pan_l: f32,
    pan_r: f32,
    last_volume: Option<u8>,
    last_panning: Option<u8>,

    env_tick: u16,
    cached_env_amp: f32,
    note_released: bool,
    fadeout_vol: u32,
    vol_fadeout_per_tick: u16,
}

impl Channel {
    fn new() -> Self {
        Self {
            active: false,
            sample_data: SampleData::silent(),
            sample_position: 0.0,
            sample_step: 0.0,
            sample_direction: 1.0,
            vol_envelope: VolEnvelope::disabled(),
            volume: 1.0,
            pan_l: 0.5_f32.sqrt(),
            pan_r: 0.5_f32.sqrt(),
            last_volume: None,
            last_panning: None,
            env_tick: 0,
            cached_env_amp: 1.0,
            note_released: false,
            fadeout_vol: 65536,
            vol_fadeout_per_tick: 0,
        }
    }

    #[inline]
    fn set_panning(&mut self, pan: f32) {
        self.pan_l = (1.0 - pan).sqrt();
        self.pan_r = pan.sqrt();
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
        vol_envelope: VolEnvelope,
        sample_data: &Arc<SampleData>,
        vol_fadeout_per_tick: u16,
    ) {
        self.active = true;
        self.volume = volume;
        self.vol_envelope = vol_envelope;
        self.sample_data = Arc::clone(sample_data);
        self.sample_step = Self::compute_sample_step(frequency, sample_data);
        self.sample_position = 0.0;
        self.sample_direction = 1.0;
        self.env_tick = 0;
        self.cached_env_amp = self.vol_envelope.amplitude_at_tick(0);
        self.note_released = false;
        self.fadeout_vol = 65536;
        self.vol_fadeout_per_tick = vol_fadeout_per_tick;
    }

    fn note_off(&mut self) {
        self.note_released = true;
        if !self.vol_envelope.enabled {
            self.active = false;
        }
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let env = self.cached_env_amp;
        if self.note_released && env <= 0.0 && self.fadeout_vol == 0 {
            self.active = false;
            return 0.0;
        }

        let fadeout_factor = if self.vol_envelope.enabled {
            self.fadeout_vol as f32 / 65536.0
        } else {
            1.0
        };

        let data = &self.sample_data;
        let samples = &data.samples_f32;
        let len = samples.len();

        if len == 0 {
            return 0.0;
        }

        let idx = self.sample_position as usize;
        let frac = (self.sample_position - idx as f64) as f32;

        let sample = if idx >= len {
            0.0
        } else {
            let s0 = if idx > 0 {
                samples[idx - 1]
            } else {
                samples[idx]
            };
            let s1 = samples[idx];
            let s2 = if idx + 1 < len { samples[idx + 1] } else { s1 };
            let s3 = if idx + 2 < len { samples[idx + 2] } else { s2 };
            hermite_interpolate(s0, s1, s2, s3, frac)
        };

        self.sample_position += self.sample_step * self.sample_direction;

        if data.loop_length > 0 {
            let loop_end = data.loop_end() as f64;
            let loop_start = data.loop_start as f64;
            match data.loop_type {
                LoopType::Forward => {
                    if self.sample_position >= loop_end {
                        self.sample_position -= data.loop_length as f64;
                    }
                }
                LoopType::PingPong => {
                    if self.sample_direction > 0.0 && self.sample_position >= loop_end {
                        self.sample_position = loop_end - (self.sample_position - loop_end);
                        self.sample_direction = -1.0;
                    } else if self.sample_direction < 0.0 && self.sample_position < loop_start {
                        self.sample_position = loop_start + (loop_start - self.sample_position);
                        self.sample_direction = 1.0;
                    }
                }
                LoopType::None => {
                    if self.sample_position >= len as f64 {
                        self.active = false;
                        return 0.0;
                    }
                }
            }
        } else if self.sample_position >= len as f64 {
            self.active = false;
            return 0.0;
        }

        sample * env * self.volume * fadeout_factor
    }

    #[inline]
    fn tick_update(&mut self) {
        if !self.active {
            return;
        }

        if self.vol_envelope.enabled {
            self.env_tick = self
                .vol_envelope
                .advance_tick(self.env_tick, self.note_released);
            self.cached_env_amp = self.vol_envelope.amplitude_at_tick(self.env_tick);
        }

        if self.note_released && self.vol_envelope.enabled && self.vol_fadeout_per_tick > 0 {
            self.fadeout_vol = self
                .fadeout_vol
                .saturating_sub(u32::from(self.vol_fadeout_per_tick));
            if self.fadeout_vol == 0 {
                self.active = false;
            }
        }
    }
}

const PREVIEW_RELEASE_TICKS: u16 = 10;

pub struct TrackerSource {
    channels: Vec<Channel>,
    preview_channel: Channel,
    preview_ticks_remaining: u16,
    playing: bool,
    patterns: Vec<Arc<PatternSnapshot>>,
    order: Vec<usize>,
    current_order_idx: usize,
    settings: Option<Arc<PlaybackSettings>>,
    current_row: usize,
    samples_per_tick: f64,
    tick_sample_counter: f64,
    tick_in_row: u16,
    receiver: mpsc::Receiver<Command>,
    playback_row: Arc<AtomicUsize>,
    playback_order: Arc<AtomicUsize>,
    master_volume: f32,
    stereo_phase: bool,
    right_sample: f32,
    muted_channels: Vec<bool>,
    pub stop_at_end: bool,
    channel_scopes: Arc<Vec<ScopeBuffer>>,
    scope_counter: usize,
    command_check_counter: usize,
}

fn compute_samples_per_tick(bpm: u16, rows_per_beat: usize) -> f64 {
    f64::from(SAMPLE_RATE) * 60.0
        / (f64::from(bpm) * f64::from(TICKS_PER_ROW) * rows_per_beat as f64)
}

impl TrackerSource {
    pub fn new(
        receiver: mpsc::Receiver<Command>,
        playback_row: Arc<AtomicUsize>,
        playback_order: Arc<AtomicUsize>,
        channel_scopes: Arc<Vec<ScopeBuffer>>,
    ) -> Self {
        Self {
            channels: Vec::new(),
            preview_channel: Channel::new(),
            preview_ticks_remaining: 0,
            playing: false,
            patterns: Vec::new(),
            order: Vec::new(),
            current_order_idx: 0,
            settings: None,
            current_row: 0,
            samples_per_tick: compute_samples_per_tick(120, 4),
            tick_sample_counter: 0.0,
            tick_in_row: 0,
            receiver,
            playback_row,
            playback_order,
            master_volume: 1.0,
            stereo_phase: false,
            right_sample: 0.0,
            muted_channels: vec![false; 32],
            stop_at_end: false,
            channel_scopes,
            scope_counter: 0,
            command_check_counter: 32,
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
                        *ch = Channel::new();
                    }
                    self.current_row = 0;
                    self.current_order_idx = 0;
                    self.tick_sample_counter = 0.0;
                    self.tick_in_row = 0;
                    self.samples_per_tick = compute_samples_per_tick(120, 4);
                    self.master_volume = 1.0;
                    self.stereo_phase = false;
                    self.right_sample = 0.0;
                    self.muted_channels.clear();
                    self.stop_at_end = false;
                    self.patterns.clear();
                    self.order.clear();
                    self.settings = None;
                }
                Command::UpdateSettings { settings } => {
                    if self.playing {
                        let new_spt =
                            compute_samples_per_tick(settings.bpm, settings.rows_per_beat);
                        if (new_spt - self.samples_per_tick).abs() > f64::EPSILON {
                            self.samples_per_tick = new_spt;
                        }
                        self.master_volume = settings.master_volume;
                        self.muted_channels = settings.muted_channels.clone();
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
                    panning,
                    vol_envelope,
                    sample_data,
                    master_volume,
                    vol_fadeout,
                } => {
                    let fadeout = if vol_envelope.enabled && vol_fadeout == 0 {
                        256
                    } else {
                        vol_fadeout
                    };
                    self.preview_channel.trigger(
                        frequency,
                        volume,
                        vol_envelope,
                        &sample_data,
                        fadeout,
                    );
                    self.preview_channel.set_panning(panning);
                    self.preview_ticks_remaining = PREVIEW_RELEASE_TICKS;
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
        let max_channels = patterns.iter().map(|p| p.channels).max().unwrap_or(0);
        while self.channels.len() < max_channels {
            self.channels.push(Channel::new());
        }
        for ch in &mut self.channels {
            *ch = Channel::new();
        }
        self.preview_channel = Channel::new();
        self.preview_ticks_remaining = 0;
        for (i, ch) in self.channels.iter_mut().enumerate() {
            if let Some(track) = settings.tracks.get(i) {
                ch.set_panning(track.default_panning);
            }
        }

        self.samples_per_tick = compute_samples_per_tick(settings.bpm, settings.rows_per_beat);
        self.master_volume = settings.master_volume;
        self.current_row = start_row;
        self.current_order_idx = start_order;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.stop_at_end = false;
        self.muted_channels = settings.muted_channels.clone();
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
            Some(p) => Arc::clone(p),
            None => return,
        };
        let Some(settings) = self.settings.as_ref() else {
            return;
        };

        for ch_idx in 0..pattern.channels.min(self.channels.len()) {
            if ch_idx >= settings.tracks.len() {
                continue;
            }
            let track = &settings.tracks[ch_idx];
            let raw_volume = pattern.volumes[ch_idx][self.current_row];
            let raw_panning = pattern.panning[ch_idx][self.current_row];
            let cell = pattern.data[ch_idx][self.current_row];
            let channel = &mut self.channels[ch_idx];

            let volume = match raw_volume {
                Some(v) => {
                    channel.last_volume = Some(v);
                    Some(v)
                }
                None => channel.last_volume,
            };

            if let Some(p) = raw_panning {
                channel.last_panning = Some(p);
                channel.set_panning(p as f32 / 255.0);
            }

            match cell {
                Cell::NoteOn(note) => {
                    let vol_from_col = volume.map(|v| v as f32 / 255.0);
                    let (sample_data, sample_vol) = track.sample_for_note(note.pitch);
                    let vol = vol_from_col.unwrap_or(sample_vol);

                    channel.trigger(
                        note.frequency(),
                        vol,
                        track.vol_envelope.clone(),
                        sample_data,
                        track.vol_fadeout,
                    );
                    if raw_panning.is_none() && channel.last_panning.is_none() {
                        channel.set_panning(track.default_panning);
                    }
                }
                Cell::NoteOff => channel.note_off(),
                Cell::Empty => {}
            }

            if cell == Cell::Empty || cell == Cell::NoteOff {
                if let Some(v) = volume {
                    channel.volume = v as f32 / 255.0;
                }
            }
        }
    }

    fn tick(&mut self) {
        self.tick_in_row += 1;
        if self.playing && self.tick_in_row >= TICKS_PER_ROW {
            self.tick_in_row = 0;
            if !self.patterns.is_empty() {
                let pat_idx = self.order[self.current_order_idx];
                let rows = self.patterns[pat_idx].rows;
                self.current_row += 1;
                if self.current_row >= rows {
                    self.current_row = 0;
                    let next_order = self.current_order_idx + 1;
                    if next_order >= self.order.len() && self.stop_at_end {
                        self.playing = false;
                        return;
                    }
                    self.current_order_idx = next_order % self.order.len();
                    self.playback_order
                        .store(self.current_order_idx, Ordering::Relaxed);
                }
                self.playback_row.store(self.current_row, Ordering::Relaxed);
            }
            self.process_row();
        }

        for ch in &mut self.channels {
            if ch.active {
                ch.tick_update();
            }
        }

        if self.preview_channel.active {
            self.preview_channel.tick_update();
            if self.preview_ticks_remaining > 0 {
                self.preview_ticks_remaining -= 1;
                if self.preview_ticks_remaining == 0 {
                    self.preview_channel.note_off();
                }
            }
        }
    }
}

impl Iterator for TrackerSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.stereo_phase {
            self.stereo_phase = false;
            return Some(self.right_sample * self.master_volume);
        }

        if self.stop_at_end && !self.playing {
            return None;
        }

        self.command_check_counter += 1;
        if self.command_check_counter >= 32 {
            self.command_check_counter = 0;
            self.process_commands();
        }

        let mut mix_l = 0.0_f32;
        let mut mix_r = 0.0_f32;

        self.scope_counter += 1;
        let write_scope = self.scope_counter % SCOPE_DOWNSAMPLE == 0;

        self.tick_sample_counter += 1.0;
        if self.tick_sample_counter >= self.samples_per_tick {
            self.tick_sample_counter -= self.samples_per_tick;
            self.tick();
        }

        if self.playing {
            for (ch_idx, ch) in self.channels.iter_mut().enumerate() {
                if !ch.active {
                    if write_scope {
                        if let Some(scope) = self.channel_scopes.get(ch_idx) {
                            scope.push(0.0);
                        }
                    }
                    continue;
                }
                if self.muted_channels.get(ch_idx).copied().unwrap_or(false) {
                    ch.next_sample();
                    if write_scope {
                        if let Some(scope) = self.channel_scopes.get(ch_idx) {
                            scope.push(0.0);
                        }
                    }
                    continue;
                }
                let sample = ch.next_sample();
                if write_scope {
                    if let Some(scope) = self.channel_scopes.get(ch_idx) {
                        scope.push(sample);
                    }
                }
                mix_l += sample * ch.pan_l;
                mix_r += sample * ch.pan_r;
            }
        }

        let preview = self.preview_channel.next_sample();
        mix_l += preview * self.preview_channel.pan_l;
        mix_r += preview * self.preview_channel.pan_r;

        if !self.playing && self.preview_channel.active && write_scope {
            if let Some(scope) = self.channel_scopes.first() {
                scope.push(preview);
            }
        }

        self.right_sample = mix_r;
        self.stereo_phase = true;

        Some(mix_l * self.master_volume)
    }
}

impl Source for TrackerSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(2).unwrap()
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
    tracks: &[Track],
    master_volume: f32,
) -> TrackerSource {
    let (sender, receiver) = mpsc::channel();
    let playback_row = Arc::new(AtomicUsize::new(0));
    let playback_order = Arc::new(AtomicUsize::new(0));

    let snapshots: Vec<Arc<PatternSnapshot>> = patterns
        .iter()
        .map(|p| Arc::new(PatternSnapshot::from_pattern(p)))
        .collect();
    let settings = Arc::new(PlaybackSettings {
        bpm,
        rows_per_beat: 4,
        master_volume,
        tracks: tracks.to_vec(),
        muted_channels: Vec::new(),
    });

    let _ = sender.send(Command::Play {
        start_row: 0,
        start_order: 0,
        patterns: snapshots,
        order: order.to_vec(),
        settings,
    });
    drop(sender);

    let dummy_scopes: Arc<Vec<ScopeBuffer>> = Arc::new(Vec::new());
    let mut source = TrackerSource::new(receiver, playback_row, playback_order, dummy_scopes);
    source.stop_at_end = true;
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source() -> (mpsc::Sender<Command>, TrackerSource, Arc<AtomicUsize>) {
        let (tx, rx) = mpsc::channel();
        let row = Arc::new(AtomicUsize::new(0));
        let order = Arc::new(AtomicUsize::new(0));
        let scopes: Arc<Vec<ScopeBuffer>> = Arc::new(Vec::new());
        let source = TrackerSource::new(rx, row.clone(), order, scopes);
        (tx, source, row)
    }

    fn play_cmd(pattern: &crate::project::Pattern) -> Command {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm: 120,
            rows_per_beat: 4,
            master_volume: 1.0,
            tracks: Track::defaults(),
            muted_channels: Vec::new(),
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
    fn tick_timing_120bpm() {
        let (tx, mut source, row) = make_source();

        let mut pattern = crate::project::Pattern::new(1, 4);
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        tx.send(play_cmd(&pattern)).unwrap();

        for _ in 0..11025 {
            source.next();
        }
        source.next();
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

        for _ in 0..128 {
            source.next();
        }

        for _ in 0..100 {
            assert_eq!(source.next(), Some(0.0));
        }
    }
}
