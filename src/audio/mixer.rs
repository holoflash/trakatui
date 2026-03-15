use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::Source;

use crate::project::channel::{FilterSettings, FilterType, Track, VolEnvelope};
use crate::project::sample::LoopType;
use crate::project::{Cell, SampleData};

pub const SAMPLE_RATE: u32 = 44100;
const TICKS_PER_ROW: u16 = 6;

pub const SCOPE_SIZE: usize = 256;
const SCOPE_DOWNSAMPLE: usize = 4;
const COMMAND_CHECK_INTERVAL: usize = 32;

#[inline]
fn hermite_interpolate(prev: f32, current: f32, next: f32, next2: f32, fraction: f32) -> f32 {
    let c0 = current;
    let c1 = 0.5 * (next - prev);
    let c2 = prev - 2.5 * current + 2.0 * next - 0.5 * next2;
    let c3 = 0.5 * (next2 - prev) + 1.5 * (current - next);
    ((c3 * fraction + c2) * fraction + c1) * fraction + c0
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

    pub fn push(&self, sample: f32) {
        let pos = self.write_pos.fetch_add(1, Ordering::Relaxed) % SCOPE_SIZE;
        self.samples[pos].store(sample.to_bits(), Ordering::Relaxed);
    }

    pub fn read_all(&self) -> [f32; SCOPE_SIZE] {
        let write_position = self.write_pos.load(Ordering::Relaxed);
        let mut out = [0.0f32; SCOPE_SIZE];
        for (i, slot) in out.iter_mut().enumerate() {
            let idx = (write_position + i) % SCOPE_SIZE;
            *slot = f32::from_bits(self.samples[idx].load(Ordering::Relaxed));
        }
        out
    }

    pub fn clear(&self) {
        for sample in self.samples.iter() {
            sample.store(0u32, Ordering::Relaxed);
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
        stop_at_end: bool,
    },
    Stop,
    UpdateSettings {
        settings: Arc<PlaybackSettings>,
    },
    UpdatePatterns {
        patterns: Vec<Arc<PatternSnapshot>>,
        order: Vec<usize>,
    },
    PreviewNotes {
        frequencies: Vec<f32>,
        volume: f32,
        panning: f32,
        vol_envelope: VolEnvelope,
        sample_data: Arc<SampleData>,
        master_volume: f32,
        vol_fadeout: u16,
        coarse_tune: i8,
        fine_tune: i8,
        pitch_env_enabled: bool,
        pitch_env_depth: f32,
        pitch_envelope: VolEnvelope,
        filter: Box<FilterSettings>,
    },
}

pub struct PlaybackSettings {
    pub master_volume: f32,
    pub tracks: Vec<Track>,
    pub muted_channels: Vec<bool>,
}

pub struct PatternSnapshot {
    pub rows: usize,
    pub bpm: u16,
    pub time_sig_denominator: u8,
    pub note_value: u8,
    pub track_rows: Vec<usize>,
    pub track_note_values: Vec<u8>,
    pub data: Vec<Vec<Vec<Cell>>>,
}

impl PatternSnapshot {
    pub fn from_pattern(pattern: &crate::project::Pattern) -> Self {
        let track_rows: Vec<usize> = (0..pattern.data.len())
            .map(|ch| pattern.track_rows(ch))
            .collect();
        Self {
            rows: pattern.rows,
            bpm: pattern.bpm,
            time_sig_denominator: pattern.time_sig_denominator,
            note_value: pattern.note_value,
            track_rows,
            track_note_values: pattern.track_note_values.clone(),
            data: pattern.data.clone(),
        }
    }
}

struct TrackTiming {
    total_rows: usize,
    current_row: usize,
    last_triggered_row: Option<usize>,
    samples_per_row: f64,
    sample_counter: f64,
}

const RELEASE_FADE_MIN_SAMPLES: u32 = 220;

struct Channel {
    active: bool,
    sample_data: Arc<SampleData>,
    sample_position: f64,
    base_sample_step: f64,
    sample_step: f64,
    sample_direction: f64,
    vol_envelope: VolEnvelope,
    volume: f32,
    pan_l: f32,
    pan_r: f32,

    env_tick: u16,
    cached_env_amp: f32,
    note_released: bool,
    fadeout_vol: u32,
    vol_fadeout_per_tick: u16,
    release_fade_remaining: u32,
    release_fade_total: u32,

    coarse_tune: i8,
    fine_tune: i8,
    pitch_env_enabled: bool,
    pitch_env_depth: f32,
    pitch_envelope: VolEnvelope,
    pitch_env_tick: u16,
    cached_pitch_env: f32,

    filter: FilterSettings,
    filter_env_tick: u16,
    cached_filter_env: f32,
    filter_state_1: f32,
    filter_state_2: f32,
    filter_b0: f32,
    filter_b1: f32,
    filter_b2: f32,
    filter_a1: f32,
    filter_a2: f32,

    region_start: usize,
    region_end: usize,
}

impl Channel {
    fn new() -> Self {
        Self {
            active: false,
            sample_data: SampleData::silent(),
            sample_position: 0.0,
            base_sample_step: 0.0,
            sample_step: 0.0,
            sample_direction: 1.0,
            vol_envelope: VolEnvelope::disabled(),
            volume: 1.0,
            pan_l: 0.5_f32.sqrt(),
            pan_r: 0.5_f32.sqrt(),
            env_tick: 0,
            cached_env_amp: 1.0,
            note_released: false,
            fadeout_vol: 65536,
            vol_fadeout_per_tick: 0,
            release_fade_remaining: 0,
            release_fade_total: 0,

            coarse_tune: 0,
            fine_tune: 0,
            pitch_env_enabled: false,
            pitch_env_depth: 12.0,
            pitch_envelope: VolEnvelope::disabled(),
            pitch_env_tick: 0,
            cached_pitch_env: 0.5,

            filter: FilterSettings::default(),
            filter_env_tick: 0,
            cached_filter_env: 1.0,
            filter_state_1: 0.0,
            filter_state_2: 0.0,
            filter_b0: 1.0,
            filter_b1: 0.0,
            filter_b2: 0.0,
            filter_a1: 0.0,
            filter_a2: 0.0,

            region_start: 0,
            region_end: 0,
        }
    }

    #[inline]
    fn set_panning(&mut self, panning: f32) {
        self.pan_l = (1.0 - panning).sqrt();
        self.pan_r = panning.sqrt();
    }

    fn compute_sample_step(
        frequency: f32,
        data: &SampleData,
        coarse_tune: i8,
        fine_tune: i8,
    ) -> f64 {
        let tune_semitones = f64::from(coarse_tune) + f64::from(fine_tune) / 100.0;
        let tune_ratio = (tune_semitones / 12.0).exp2();
        let base_freq = 440.0 * ((f32::from(data.base_note) - 69.0) / 12.0).exp2();
        let rate = f64::from(frequency) / f64::from(base_freq);
        (f64::from(data.sample_rate) / f64::from(SAMPLE_RATE)) * rate * tune_ratio
    }

    #[allow(clippy::too_many_arguments)]
    fn trigger(
        &mut self,
        frequency: f32,
        volume: f32,
        vol_envelope: VolEnvelope,
        sample_data: &Arc<SampleData>,
        vol_fadeout_per_tick: u16,
        coarse_tune: i8,
        fine_tune: i8,
        pitch_env_enabled: bool,
        pitch_env_depth: f32,
        pitch_envelope: VolEnvelope,
        filter: FilterSettings,
    ) {
        self.active = true;
        self.volume = volume;
        self.vol_envelope = vol_envelope;
        self.sample_data = Arc::clone(sample_data);

        self.coarse_tune = coarse_tune;
        self.fine_tune = fine_tune;
        self.base_sample_step =
            Self::compute_sample_step(frequency, sample_data, coarse_tune, fine_tune);
        self.sample_step = self.base_sample_step;

        self.region_start = sample_data.region_start;
        self.region_end = if sample_data.region_end == 0 {
            sample_data.samples_f32.len()
        } else {
            sample_data.region_end
        };
        self.sample_position = if sample_data.reverse {
            (self.region_end as f64 - 1.0).max(self.region_start as f64)
        } else {
            self.region_start as f64
        };

        self.sample_direction = if sample_data.reverse { -1.0 } else { 1.0 };
        self.env_tick = 0;
        self.cached_env_amp = self.vol_envelope.amplitude_at_tick(0);
        self.note_released = false;
        self.fadeout_vol = 65536;
        self.vol_fadeout_per_tick = vol_fadeout_per_tick;
        self.release_fade_remaining = 0;
        self.release_fade_total = 0;

        self.pitch_env_enabled = pitch_env_enabled;
        self.pitch_env_depth = pitch_env_depth;
        self.pitch_envelope = pitch_envelope;
        self.pitch_env_tick = 0;
        self.cached_pitch_env = if self.pitch_env_enabled {
            self.pitch_envelope.amplitude_at_tick(0)
        } else {
            0.5
        };

        self.filter = filter;
        self.filter_env_tick = 0;
        self.cached_filter_env = if self.filter.enabled && self.filter.envelope.enabled {
            self.filter.envelope.amplitude_at_tick(0)
        } else {
            1.0
        };
        self.filter_state_1 = 0.0;
        self.filter_state_2 = 0.0;
        if self.filter.enabled {
            self.update_filter_coeffs();
        }
    }

    fn note_off(&mut self, fade_samples: u32) {
        self.note_released = true;
        if !self.vol_envelope.enabled {
            let total = fade_samples.max(RELEASE_FADE_MIN_SAMPLES);
            self.release_fade_remaining = total;
            self.release_fade_total = total;
        }
    }

    fn update_filter_coeffs(&mut self) {
        let envelope_mod = if self.filter.envelope.enabled {
            let mod_octaves = self.filter.env_depth * (self.cached_filter_env * 2.0 - 1.0) * 4.0;
            mod_octaves.exp2()
        } else {
            1.0
        };

        let cutoff = (self.filter.cutoff * envelope_mod).clamp(20.0, 20000.0);
        let quality_factor = 0.5 + self.filter.resonance * 9.5;

        let omega = std::f32::consts::TAU * cutoff / SAMPLE_RATE as f32;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let alpha = sin_omega / (2.0 * quality_factor);

        let (b0, b1, b2, a0, a1, a2) = match self.filter.filter_type {
            FilterType::LowPass => {
                let b1 = 1.0 - cos_omega;
                let b0 = b1 / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::HighPass => {
                let b1 = -(1.0 + cos_omega);
                let b0 = (1.0 + cos_omega) / 2.0;
                let b2 = b0;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
            FilterType::BandPass => {
                let b0 = alpha;
                let b1 = 0.0;
                let b2 = -alpha;
                let a0 = 1.0 + alpha;
                let a1 = -2.0 * cos_omega;
                let a2 = 1.0 - alpha;
                (b0, b1, b2, a0, a1, a2)
            }
        };

        let inv_a0 = 1.0 / a0;
        self.filter_b0 = b0 * inv_a0;
        self.filter_b1 = b1 * inv_a0;
        self.filter_b2 = b2 * inv_a0;
        self.filter_a1 = a1 * inv_a0;
        self.filter_a2 = a2 * inv_a0;
    }

    #[inline]
    fn apply_filter(&mut self, input: f32) -> f32 {
        let output = self.filter_b0 * input + self.filter_state_1;
        self.filter_state_1 = self.filter_b1 * input - self.filter_a1 * output + self.filter_state_2;
        self.filter_state_2 = self.filter_b2 * input - self.filter_a2 * output;
        output
    }

    #[inline]
    fn read_interpolated_sample(&self) -> f32 {
        let samples = &self.sample_data.samples_f32;
        let len = samples.len();

        if len == 0 {
            return 0.0;
        }

        let sample_index = self.sample_position as usize;
        if sample_index >= len {
            return 0.0;
        }

        let fractional_position = (self.sample_position - sample_index as f64) as f32;
        let prev = if sample_index > 0 { samples[sample_index - 1] } else { samples[sample_index] };
        let current = samples[sample_index];
        let next = if sample_index + 1 < len { samples[sample_index + 1] } else { current };
        let next2 = if sample_index + 2 < len { samples[sample_index + 2] } else { next };

        hermite_interpolate(prev, current, next, next2, fractional_position)
    }

    #[inline]
    fn advance_sample_position(&mut self) -> bool {
        self.sample_position += self.sample_step * self.sample_direction;

        let data = &self.sample_data;
        let region_start = self.region_start as f64;
        let region_end = self.region_end as f64;

        if data.loop_length > 0 {
            let loop_start = (data.loop_start as f64).max(region_start);
            let loop_end = (data.loop_end() as f64).min(region_end);
            let loop_len = loop_end - loop_start;

            if loop_len > 0.0 {
                match data.loop_type {
                    LoopType::Forward => {
                        if self.sample_direction > 0.0 && self.sample_position >= loop_end {
                            self.sample_position -= loop_len;
                        } else if self.sample_direction < 0.0 && self.sample_position < loop_start {
                            self.sample_position += loop_len;
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
                        if self.sample_position >= region_end || self.sample_position < region_start {
                            return false;
                        }
                    }
                }
            } else if self.sample_position >= region_end || self.sample_position < region_start {
                return false;
            }
        } else if self.sample_position >= region_end || self.sample_position < region_start {
            return false;
        }

        true
    }

    #[inline]
    fn compute_volume_gain(&mut self) -> Option<f32> {
        let envelope_amplitude = self.cached_env_amp;
        if self.note_released && envelope_amplitude <= 0.0 && self.fadeout_vol == 0 {
            return None;
        }

        let fadeout_factor = if self.vol_envelope.enabled {
            self.fadeout_vol as f32 / 65536.0
        } else {
            1.0
        };

        let release_factor = if self.release_fade_remaining > 0 {
            self.release_fade_remaining -= 1;
            let factor = self.release_fade_remaining as f32 / self.release_fade_total as f32;
            if self.release_fade_remaining == 0 {
                self.active = false;
            }
            factor
        } else {
            1.0
        };

        Some(envelope_amplitude * self.volume * fadeout_factor * release_factor)
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let volume_gain = match self.compute_volume_gain() {
            Some(gain) => gain,
            None => {
                self.active = false;
                return 0.0;
            }
        };

        let raw_sample = self.read_interpolated_sample();

        if !self.advance_sample_position() {
            self.active = false;
            return 0.0;
        }

        if self.filter.enabled {
            self.apply_filter(raw_sample) * volume_gain
        } else {
            raw_sample * volume_gain
        }
    }

    #[inline]
    fn tick_envelopes(&mut self) {
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

        if self.pitch_env_enabled && self.pitch_envelope.enabled {
            self.pitch_env_tick = self
                .pitch_envelope
                .advance_tick(self.pitch_env_tick, self.note_released);
            self.cached_pitch_env = self.pitch_envelope.amplitude_at_tick(self.pitch_env_tick);
            let pitch_mod_semitones = (self.cached_pitch_env - 0.5) * 2.0 * self.pitch_env_depth;
            let pitch_ratio = (f64::from(pitch_mod_semitones) / 12.0).exp2();
            self.sample_step = self.base_sample_step * pitch_ratio;
        }

        if self.filter.enabled && self.filter.envelope.enabled {
            self.filter_env_tick = self
                .filter
                .envelope
                .advance_tick(self.filter_env_tick, self.note_released);
            self.cached_filter_env = self.filter.envelope.amplitude_at_tick(self.filter_env_tick);
            self.update_filter_coeffs();
        }
    }
}

const PREVIEW_RELEASE_TICKS: u16 = 10;

pub struct TrackerSource {
    channels: Vec<Vec<Channel>>,
    preview_channels: Vec<Channel>,
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
    track_timings: Vec<TrackTiming>,
    receiver: mpsc::Receiver<Command>,
    playback_row: Arc<AtomicUsize>,
    playback_order: Arc<AtomicUsize>,
    playback_ended: Arc<AtomicBool>,
    master_volume: f32,
    stereo_phase: bool,
    right_sample: f32,
    muted_channels: Vec<bool>,
    pub stop_at_end: bool,
    channel_scopes: Arc<Vec<ScopeBuffer>>,
    scope_counter: usize,
    command_check_counter: usize,
}

fn compute_samples_per_tick(bpm: u16, note_value: u8, time_sig_denominator: u8) -> f64 {
    f64::from(SAMPLE_RATE) * 60.0 * f64::from(time_sig_denominator)
        / (f64::from(bpm) * f64::from(TICKS_PER_ROW) * f64::from(note_value))
}

fn compute_samples_per_row(bpm: u16, note_value: u8, time_sig_denominator: u8) -> f64 {
    f64::from(SAMPLE_RATE) * 60.0 * f64::from(time_sig_denominator)
        / (f64::from(bpm) * f64::from(note_value))
}

impl TrackerSource {
    pub fn new(
        receiver: mpsc::Receiver<Command>,
        playback_row: Arc<AtomicUsize>,
        playback_order: Arc<AtomicUsize>,
        playback_ended: Arc<AtomicBool>,
        channel_scopes: Arc<Vec<ScopeBuffer>>,
    ) -> Self {
        Self {
            channels: Vec::new(),
            preview_channels: Vec::new(),
            preview_ticks_remaining: 0,
            playing: false,
            patterns: Vec::new(),
            order: Vec::new(),
            current_order_idx: 0,
            settings: None,
            current_row: 0,
            samples_per_tick: compute_samples_per_tick(120, 16, 4),
            tick_sample_counter: 0.0,
            tick_in_row: 0,
            track_timings: Vec::new(),
            receiver,
            playback_row,
            playback_order,
            playback_ended,
            master_volume: 1.0,
            stereo_phase: false,
            right_sample: 0.0,
            muted_channels: vec![false; 32],
            stop_at_end: false,
            channel_scopes,
            scope_counter: 0,
            command_check_counter: COMMAND_CHECK_INTERVAL,
        }
    }

    fn reset_state(&mut self) {
        self.playing = false;
        for track_voices in &mut self.channels {
            for voice in track_voices {
                *voice = Channel::new();
            }
        }
        self.current_row = 0;
        self.current_order_idx = 0;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.track_timings.clear();
        self.samples_per_tick = compute_samples_per_tick(120, 16, 4);
        self.master_volume = 1.0;
        self.stereo_phase = false;
        self.right_sample = 0.0;
        self.muted_channels.clear();
        self.stop_at_end = false;
        self.patterns.clear();
        self.order.clear();
        self.settings = None;
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
                    stop_at_end,
                } => self.start_playback(start_row, start_order, patterns, order, settings, stop_at_end),
                Command::Stop => self.reset_state(),
                Command::UpdateSettings { settings } => {
                    if self.playing {
                        self.master_volume = settings.master_volume;
                        self.muted_channels = settings.muted_channels.clone();

                        for (track_index, track) in settings.tracks.iter().enumerate() {
                            let needed = track.polyphony.max(1) as usize;
                            if track_index < self.channels.len() {
                                while self.channels[track_index].len() < needed {
                                    let mut voice = Channel::new();
                                    voice.set_panning(track.default_panning);
                                    self.channels[track_index].push(voice);
                                }
                            }
                        }

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
                        let pattern_index = self.order[self.current_order_idx];
                        if self.current_row >= self.patterns[pattern_index].rows {
                            self.current_row = 0;
                        }
                    }
                }
                Command::PreviewNotes {
                    frequencies,
                    volume,
                    panning,
                    vol_envelope,
                    sample_data,
                    master_volume,
                    vol_fadeout,
                    coarse_tune,
                    fine_tune,
                    pitch_env_enabled,
                    pitch_env_depth,
                    pitch_envelope,
                    filter,
                } => {
                    let effective_fadeout = if vol_envelope.enabled && vol_fadeout == 0 {
                        256
                    } else {
                        vol_fadeout
                    };
                    self.preview_channels.clear();
                    for &freq in &frequencies {
                        let mut voice = Channel::new();
                        voice.trigger(
                            freq,
                            volume,
                            vol_envelope.clone(),
                            &sample_data,
                            effective_fadeout,
                            coarse_tune,
                            fine_tune,
                            pitch_env_enabled,
                            pitch_env_depth,
                            pitch_envelope.clone(),
                            (*filter).clone(),
                        );
                        voice.set_panning(panning);
                        self.preview_channels.push(voice);
                    }
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
        stop_at_end: bool,
    ) {
        self.channels.clear();
        for track in &settings.tracks {
            let voice_count = track.polyphony.max(1) as usize;
            let mut voices = Vec::with_capacity(voice_count);
            for _ in 0..voice_count {
                let mut voice = Channel::new();
                voice.set_panning(track.default_panning);
                voices.push(voice);
            }
            self.channels.push(voices);
        }

        self.preview_channels.clear();
        self.preview_ticks_remaining = 0;

        let pattern_index = order[start_order];
        let initial_pattern = &patterns[pattern_index];
        self.samples_per_tick = compute_samples_per_tick(initial_pattern.bpm, initial_pattern.note_value, initial_pattern.time_sig_denominator);
        self.master_volume = settings.master_volume;
        self.current_row = start_row;
        self.current_order_idx = start_order;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.stop_at_end = stop_at_end;
        self.muted_channels = settings.muted_channels.clone();

        self.track_timings.clear();
        for (track_index, _) in settings.tracks.iter().enumerate() {
            let track_rows = initial_pattern.track_rows.get(track_index).copied().unwrap_or(initial_pattern.rows);
            let track_nv = initial_pattern.track_note_values.get(track_index).copied().unwrap_or(initial_pattern.note_value);
            let spr = compute_samples_per_row(initial_pattern.bpm, track_nv, initial_pattern.time_sig_denominator);
            let initial_track_row = if track_rows == initial_pattern.rows {
                start_row
            } else {
                start_row * track_rows / initial_pattern.rows
            };
            self.track_timings.push(TrackTiming {
                total_rows: track_rows,
                current_row: initial_track_row,
                last_triggered_row: None,
                samples_per_row: spr,
                sample_counter: 0.0,
            });
        }

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
        if self.patterns.is_empty() || !self.playing {
            return;
        }
        let settings = match self.settings.as_ref() {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let pattern_index = self.order[self.current_order_idx];
        let pattern = &self.patterns[pattern_index];

        for (track_index, track_voices) in self.channels.iter_mut().enumerate() {
            if track_index >= settings.tracks.len() {
                continue;
            }
            let track_row = self.track_timings.get(track_index)
                .map(|t| t.current_row)
                .unwrap_or(self.current_row);
            if let Some(timing) = self.track_timings.get_mut(track_index) {
                timing.last_triggered_row = Some(track_row);
            }
            Self::trigger_track_row(&settings, pattern, track_index, track_row, track_voices, self.samples_per_tick);
        }
    }

    fn trigger_track_row(
        settings: &PlaybackSettings,
        pattern: &PatternSnapshot,
        track_index: usize,
        track_row: usize,
        track_voices: &mut [Channel],
        samples_per_tick: f64,
    ) {
        let track = &settings.tracks[track_index];
        let voices_in_pattern = pattern.data.get(track_index).map(|d| d.len()).unwrap_or(0);
        let voices_in_mixer = track_voices.len();

        for (voice_index, voice) in track_voices.iter_mut().enumerate().take(voices_in_pattern.min(voices_in_mixer)) {
            let ch_data = &pattern.data[track_index][voice_index];
            if track_row >= ch_data.len() {
                continue;
            }
            let cell = ch_data[track_row];

            match cell {
                Cell::NoteOn(note) => {
                    let (sample_data, sample_vol) = track.sample_for_note(note.pitch);
                    voice.trigger(
                        note.frequency(),
                        sample_vol,
                        track.vol_envelope.clone(),
                        sample_data,
                        track.vol_fadeout,
                        track.coarse_tune,
                        track.fine_tune,
                        track.pitch_env_enabled,
                        track.pitch_env_depth,
                        track.pitch_envelope.clone(),
                        track.filter.clone(),
                    );
                    voice.set_panning(track.default_panning);
                }
                Cell::NoteOff => voice.note_off(samples_per_tick as u32),
                Cell::Empty => {}
            }
        }
    }

    fn advance_tracks(&mut self) {
        if self.patterns.is_empty() || !self.playing {
            return;
        }
        let settings = match self.settings.as_ref() {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let pattern_index = self.order[self.current_order_idx];
        let pattern = &self.patterns[pattern_index];
        let max_rows = pattern.rows;

        let mut densest_row = self.current_row;
        let mut pattern_ended = false;

        for (track_index, timing) in self.track_timings.iter_mut().enumerate() {
            timing.sample_counter += 1.0;
            if timing.sample_counter >= timing.samples_per_row {
                timing.sample_counter -= timing.samples_per_row;
                timing.current_row += 1;

                if timing.current_row >= timing.total_rows {
                    timing.current_row = 0;
                    if timing.total_rows == max_rows {
                        pattern_ended = true;
                    }
                }

                if Some(timing.current_row) != timing.last_triggered_row {
                    timing.last_triggered_row = Some(timing.current_row);
                    if track_index < self.channels.len() && track_index < settings.tracks.len() {
                        Self::trigger_track_row(
                            &settings,
                            pattern,
                            track_index,
                            timing.current_row,
                            &mut self.channels[track_index],
                            self.samples_per_tick,
                        );
                    }
                }
            }

            if timing.total_rows == max_rows {
                densest_row = timing.current_row;
            }
        }

        self.current_row = densest_row;
        self.playback_row.store(self.current_row, Ordering::Relaxed);

        if pattern_ended {
            let next_order = self.current_order_idx + 1;
            if next_order >= self.order.len() && self.stop_at_end {
                self.playing = false;
                self.playback_ended.store(true, Ordering::Relaxed);
                return;
            }
            self.current_order_idx = next_order % self.order.len();
            self.playback_order
                .store(self.current_order_idx, Ordering::Relaxed);
            let new_pattern_index = self.order[self.current_order_idx];
            if let Some(new_pattern) = self.patterns.get(new_pattern_index) {
                let new_samples_per_tick =
                    compute_samples_per_tick(new_pattern.bpm, new_pattern.note_value, new_pattern.time_sig_denominator);
                if (new_samples_per_tick - self.samples_per_tick).abs() > f64::EPSILON {
                    self.samples_per_tick = new_samples_per_tick;
                }
                for (i, timing) in self.track_timings.iter_mut().enumerate() {
                    timing.total_rows = new_pattern.track_rows.get(i).copied().unwrap_or(new_pattern.rows);
                    timing.current_row = 0;
                    timing.last_triggered_row = None;
                    timing.sample_counter = 0.0;
                    let nv = new_pattern.track_note_values.get(i).copied().unwrap_or(new_pattern.note_value);
                    timing.samples_per_row = compute_samples_per_row(new_pattern.bpm, nv, new_pattern.time_sig_denominator);
                }
            }
        }
    }


    fn tick(&mut self) {
        if self.playing {
            self.tick_in_row += 1;
            if self.tick_in_row >= TICKS_PER_ROW {
                self.tick_in_row = 0;
            }
        }

        for track_voices in &mut self.channels {
            for voice in track_voices.iter_mut() {
                voice.tick_envelopes();
            }
        }

        for preview_voice in &mut self.preview_channels {
            preview_voice.tick_envelopes();
        }
        if self.preview_ticks_remaining > 0 {
            self.preview_ticks_remaining -= 1;
            if self.preview_ticks_remaining == 0 {
                for preview_voice in &mut self.preview_channels {
                    preview_voice.note_off(RELEASE_FADE_MIN_SAMPLES);
                }
            }
        }
    }

    #[inline]
    fn check_commands(&mut self) {
        self.command_check_counter += 1;
        if self.command_check_counter >= COMMAND_CHECK_INTERVAL {
            self.command_check_counter = 0;
            self.process_commands();
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

        self.check_commands();


        let mut left_mix = 0.0_f32;
        let mut right_mix = 0.0_f32;

        self.scope_counter += 1;
        let write_scope = self.scope_counter.is_multiple_of(SCOPE_DOWNSAMPLE);

        self.tick_sample_counter += 1.0;
        if self.tick_sample_counter >= self.samples_per_tick {
            self.tick_sample_counter -= self.samples_per_tick;
            self.tick();
        }

        self.advance_tracks();

        if self.playing {
            for (track_index, track_voices) in self.channels.iter_mut().enumerate() {
                let muted = self.muted_channels.get(track_index).copied().unwrap_or(false);
                let mut track_sample_sum = 0.0_f32;

                for voice in track_voices.iter_mut() {
                    if !voice.active {
                        continue;
                    }
                    if muted {
                        voice.next_sample();
                        continue;
                    }
                    let sample = voice.next_sample();
                    track_sample_sum += sample;
                    left_mix += sample * voice.pan_l;
                    right_mix += sample * voice.pan_r;
                }

                if write_scope
                    && let Some(scope) = self.channel_scopes.get(track_index)
                {
                    scope.push(if muted { 0.0 } else { track_sample_sum });
                }
            }
        }

        let mut preview_sample_sum = 0.0_f32;
        for preview_voice in &mut self.preview_channels {
            let sample = preview_voice.next_sample();
            preview_sample_sum += sample;
            left_mix += sample * preview_voice.pan_l;
            right_mix += sample * preview_voice.pan_r;
        }

        if !self.playing
            && preview_sample_sum.abs() > 0.0
            && write_scope
            && let Some(scope) = self.channel_scopes.first()
        {
            scope.push(preview_sample_sum);
        }

        self.right_sample = right_mix;
        self.stereo_phase = true;

        Some(left_mix * self.master_volume)
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
        stop_at_end: true,
    });
    drop(sender);

    let dummy_scopes: Arc<Vec<ScopeBuffer>> = Arc::new(Vec::new());
    let dummy_ended = Arc::new(AtomicBool::new(false));
    let mut source = TrackerSource::new(receiver, playback_row, playback_order, dummy_ended, dummy_scopes);
    source.stop_at_end = true;
    source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[allow(dead_code)]
    struct TestHarness {
        sender: mpsc::Sender<Command>,
        source: TrackerSource,
        playback_row: Arc<AtomicUsize>,
        playback_order: Arc<AtomicUsize>,
        playback_ended: Arc<AtomicBool>,
    }

    fn make_test_harness() -> TestHarness {
        let (sender, receiver) = mpsc::channel();
        let playback_row = Arc::new(AtomicUsize::new(0));
        let playback_order = Arc::new(AtomicUsize::new(0));
        let playback_ended = Arc::new(AtomicBool::new(false));
        let scopes: Arc<Vec<ScopeBuffer>> = Arc::new(Vec::new());
        let source = TrackerSource::new(
            receiver,
            playback_row.clone(),
            playback_order.clone(),
            playback_ended.clone(),
            scopes,
        );
        TestHarness {
            sender,
            source,
            playback_row,
            playback_order,
            playback_ended,
        }
    }

    fn play_cmd(pattern: &crate::project::Pattern, stop_at_end: bool) -> Command {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let settings = Arc::new(PlaybackSettings {
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
            stop_at_end,
        }
    }

    fn advance_samples(source: &mut TrackerSource, count: usize) {
        for _ in 0..count {
            source.next();
        }
    }

    #[test]
    fn hermite_interpolation_passthrough() {
        assert_eq!(hermite_interpolate(0.0, 1.0, 2.0, 3.0, 0.0), 1.0);
        assert_eq!(hermite_interpolate(5.0, 10.0, 15.0, 20.0, 0.0), 10.0);
    }

    #[test]
    fn hermite_interpolation_linear_midpoint() {
        let result = hermite_interpolate(0.0, 1.0, 2.0, 3.0, 0.5);
        assert!((result - 1.5).abs() < 1e-6);
    }

    #[test]
    fn scope_buffer_write_and_read() {
        let scope = ScopeBuffer::new();
        scope.push(1.0);
        scope.push(2.0);
        scope.push(3.0);
        let data = scope.read_all();
        assert_eq!(data[SCOPE_SIZE - 3], 1.0);
        assert_eq!(data[SCOPE_SIZE - 2], 2.0);
        assert_eq!(data[SCOPE_SIZE - 1], 3.0);
    }

    #[test]
    fn scope_buffer_wraps_around() {
        let scope = ScopeBuffer::new();
        for i in 0..(SCOPE_SIZE + 5) {
            scope.push(i as f32);
        }
        let data = scope.read_all();
        assert_eq!(data[0], 5.0);
        assert_eq!(data[SCOPE_SIZE - 1], (SCOPE_SIZE + 4) as f32);
    }

    #[test]
    fn channel_panning_center() {
        let mut channel = Channel::new();
        channel.set_panning(0.5);
        assert!((channel.pan_l - channel.pan_r).abs() < 1e-6);
    }

    #[test]
    fn channel_panning_hard_left() {
        let mut channel = Channel::new();
        channel.set_panning(0.0);
        assert!((channel.pan_l - 1.0).abs() < 1e-6);
        assert!(channel.pan_r.abs() < 1e-6);
    }

    #[test]
    fn channel_panning_hard_right() {
        let mut channel = Channel::new();
        channel.set_panning(1.0);
        assert!(channel.pan_l.abs() < 1e-6);
        assert!((channel.pan_r - 1.0).abs() < 1e-6);
    }

    #[test]
    fn channel_inactive_produces_silence() {
        let mut channel = Channel::new();
        assert!(!channel.active);
        assert_eq!(channel.next_sample(), 0.0);
    }

    #[test]
    fn channel_note_off_deactivates() {
        let mut channel = Channel::new();
        let sample_data = SampleData::sine();
        channel.trigger(
            440.0,
            1.0,
            VolEnvelope::disabled(),
            &sample_data,
            0,
            0,
            0,
            false,
            12.0,
            VolEnvelope::disabled(),
            FilterSettings::default(),
        );
        assert!(channel.active);

        channel.note_off(100);
        for _ in 0..300 {
            channel.next_sample();
        }
        assert!(!channel.active);
    }

    #[test]
    fn compute_samples_per_tick_values() {
        let spt_120 = compute_samples_per_tick(120, 16, 4);
        assert!((spt_120 - 918.75).abs() < 0.01);

        let spt_240 = compute_samples_per_tick(240, 16, 4);
        assert!((spt_240 - 918.75 / 2.0).abs() < 0.01);
    }

    #[test]
    fn tick_timing_120bpm() {
        let mut harness = make_test_harness();

        let mut pattern = crate::project::Pattern::new("test".into(), 1, 4);
        pattern.set(0, 0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        harness.sender.send(play_cmd(&pattern, false)).unwrap();

        advance_samples(&mut harness.source, 11025);
        harness.source.next();
        harness.source.next();
        assert_eq!(harness.playback_row.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn stop_silences_output() {
        let mut harness = make_test_harness();

        let mut pattern = crate::project::Pattern::new("test".into(), 1, 4);
        pattern.set(0, 0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        harness.sender.send(play_cmd(&pattern, false)).unwrap();

        advance_samples(&mut harness.source, 100);

        harness.sender.send(Command::Stop).unwrap();

        advance_samples(&mut harness.source, 128);

        for _ in 0..100 {
            assert_eq!(harness.source.next(), Some(0.0));
        }
    }

    #[test]
    fn playback_loops_to_beginning() {
        let mut harness = make_test_harness();
        let pattern = crate::project::Pattern::new("test".into(), 1, 2);

        harness.sender.send(play_cmd(&pattern, false)).unwrap();

        advance_samples(&mut harness.source, 22060);

        assert_eq!(harness.playback_row.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn playback_stops_at_end() {
        let mut harness = make_test_harness();
        let pattern = crate::project::Pattern::new("test".into(), 1, 2);

        harness
            .sender
            .send(play_cmd(&pattern, true))
            .unwrap();

        advance_samples(&mut harness.source, 22060);

        assert!(harness.playback_ended.load(Ordering::Relaxed));
    }

    #[test]
    fn muted_channel_produces_silence() {
        let mut harness = make_test_harness();

        let mut pattern = crate::project::Pattern::new("test".into(), 1, 4);
        pattern.set(0, 0, 0, Cell::NoteOn(crate::project::Note::new(69)));

        let snapshot = Arc::new(PatternSnapshot::from_pattern(&pattern));
        let mut muted = vec![false; 1];
        muted[0] = true;
        let settings = Arc::new(PlaybackSettings {
            master_volume: 1.0,
            tracks: Track::defaults(),
            muted_channels: muted,
        });
        harness
            .sender
            .send(Command::Play {
                start_row: 0,
                start_order: 0,
                patterns: vec![snapshot],
                order: vec![0],
                settings,
                stop_at_end: false,
            })
            .unwrap();

        advance_samples(&mut harness.source, 64);

        for _ in 0..100 {
            assert_eq!(harness.source.next(), Some(0.0));
        }
    }
}
