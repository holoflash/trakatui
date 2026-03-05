use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use rodio::Source;

use crate::project::channel::{Instrument, VolEnvelope};
use crate::project::sample::LoopType;
use crate::project::{Cell, Effect, SampleData};

pub const SAMPLE_RATE: u32 = 44100;
const DEFAULT_SPEED: u16 = 6;

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
        vol_envelope: VolEnvelope,
        sample_data: Arc<SampleData>,
        master_volume: f32,
        vibrato_type: u8,
        vibrato_sweep: u8,
        vibrato_depth: u8,
        vibrato_rate: u8,
    },
}

pub struct PlaybackSettings {
    pub bpm: u16,
    pub speed: u16,
    pub master_volume: f32,
    pub instruments: Vec<Instrument>,
    pub muted_channels: Vec<bool>,
    pub channel_panning: Vec<f32>,
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
}

struct Channel {
    active: bool,
    sample_data: Arc<SampleData>,
    sample_position: f64,
    sample_step: f64,
    sample_direction: f64,
    vol_envelope: VolEnvelope,
    volume: f32,
    panning: f32,
    elapsed_samples: u32,
    period: f32,
    base_period: f32,
    current_instrument: usize,
    current_effect: Option<Effect>,

    porta_target: f32,
    tone_porta_speed: u8,

    vibrato_speed: u8,
    vibrato_depth: u8,
    vibrato_pos: u8,

    tremolo_speed: u8,
    tremolo_depth: u8,
    tremolo_pos: u8,

    arpeggio_x: u8,
    arpeggio_y: u8,

    auto_vib_type: u8,
    auto_vib_sweep: u8,
    auto_vib_depth: u8,
    auto_vib_rate: u8,
    auto_vib_pos: u8,
    auto_vib_sweep_pos: u16,

    last_porta_up: u8,
    last_porta_down: u8,
    last_vol_slide: u8,
    last_sample_offset: u8,
    vol_column: u8,

    note_delay_tick: u8,
    delayed_frequency: f32,
    delayed_volume: f32,
    delayed_vol_envelope: VolEnvelope,
    delayed_sample_data: Arc<SampleData>,
    has_delayed_note: bool,

    env_tick: u16,
    note_released: bool,
    fadeout_vol: u32,
    vol_fadeout_per_tick: u16,
    delayed_fadeout_per_tick: u16,
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
            panning: 0.5,
            elapsed_samples: 0,
            period: 0.0,
            base_period: 0.0,
            current_instrument: 0,
            current_effect: None,
            porta_target: 0.0,
            tone_porta_speed: 0,
            vibrato_speed: 0,
            vibrato_depth: 0,
            vibrato_pos: 0,
            tremolo_speed: 0,
            tremolo_depth: 0,
            tremolo_pos: 0,
            arpeggio_x: 0,
            arpeggio_y: 0,
            auto_vib_type: 0,
            auto_vib_sweep: 0,
            auto_vib_depth: 0,
            auto_vib_rate: 0,
            auto_vib_pos: 0,
            auto_vib_sweep_pos: 0,
            last_porta_up: 0,
            last_porta_down: 0,
            last_vol_slide: 0,
            last_sample_offset: 0,
            vol_column: 0,
            note_delay_tick: 0,
            delayed_frequency: 0.0,
            delayed_volume: 0.0,
            delayed_vol_envelope: VolEnvelope::disabled(),
            delayed_sample_data: SampleData::silent(),
            has_delayed_note: false,
            env_tick: 0,
            note_released: false,
            fadeout_vol: 65536,
            vol_fadeout_per_tick: 0,
            delayed_fadeout_per_tick: 0,
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
        vol_envelope: VolEnvelope,
        sample_data: &Arc<SampleData>,
        vol_fadeout_per_tick: u16,
    ) {
        self.active = true;
        self.period = Self::freq_to_period(frequency);
        self.base_period = self.period;
        self.volume = volume;
        self.vol_envelope = vol_envelope;
        self.elapsed_samples = 0;
        self.sample_data = Arc::clone(sample_data);
        self.sample_step = Self::compute_sample_step(frequency, sample_data);
        self.sample_position = 0.0;
        self.sample_direction = 1.0;
        self.vibrato_pos = 0;
        self.env_tick = 0;
        self.note_released = false;
        self.fadeout_vol = 65536;
        self.vol_fadeout_per_tick = vol_fadeout_per_tick;
        self.auto_vib_pos = 0;
        self.auto_vib_sweep_pos = 0;
    }

    fn note_off(&mut self) {
        self.note_released = true;
        if !self.vol_envelope.enabled {
            self.active = false;
        }
    }

    fn next_sample(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }

        let env = {
            let amp = self.vol_envelope.amplitude_at_tick(self.env_tick);
            if self.note_released && amp <= 0.0 && self.fadeout_vol == 0 {
                self.active = false;
                return 0.0;
            }
            amp
        };

        let fadeout_factor = if self.vol_envelope.enabled {
            self.fadeout_vol as f32 / 65536.0
        } else {
            1.0
        };

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

        self.sample_position += self.sample_step * self.sample_direction;

        if data.loop_length > 0 {
            let loop_end = (data.loop_start + data.loop_length) as f64;
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

        self.elapsed_samples += 1;

        sample * env * self.volume * fadeout_factor
    }

    fn tick_update(&mut self, tick: u16) {
        if self.has_delayed_note && tick == u16::from(self.note_delay_tick) {
            let vol_env =
                std::mem::replace(&mut self.delayed_vol_envelope, VolEnvelope::disabled());
            let fadeout = self.delayed_fadeout_per_tick;
            self.trigger(
                self.delayed_frequency,
                self.delayed_volume,
                vol_env,
                &self.delayed_sample_data.clone(),
                fadeout,
            );
            self.has_delayed_note = false;
        }

        if !self.active {
            return;
        }

        if let Some(effect) = self.current_effect {
            match effect.kind {
                0 if effect.param != 0 => {
                    let semitone_offset = match tick % 3 {
                        0 => 0,
                        1 => self.arpeggio_x,
                        _ => self.arpeggio_y,
                    };
                    let period = self.base_period - f32::from(semitone_offset) * 64.0;
                    let freq = Self::period_to_freq(period.max(50.0));
                    self.sample_step = Self::compute_sample_step(freq, &self.sample_data);
                }
                1 => {
                    self.period = (self.period - f32::from(self.last_porta_up) * 4.0).max(50.0);
                    self.update_freq_from_period();
                }
                2 => {
                    self.period = (self.period + f32::from(self.last_porta_down) * 4.0).min(7680.0);
                    self.update_freq_from_period();
                }
                3 => self.do_tone_porta(),
                4 => self.do_vibrato(),
                5 => {
                    self.do_tone_porta();
                    self.do_vol_slide(self.last_vol_slide);
                }
                6 => {
                    self.do_vibrato();
                    self.do_vol_slide(self.last_vol_slide);
                }
                7 => self.do_tremolo(),
                0xA => {
                    self.do_vol_slide(self.last_vol_slide);
                }
                0xE => {
                    let sub = effect.param >> 4;
                    let val = effect.param & 0x0F;
                    match sub {
                        0xC => {
                            if tick == u16::from(val) {
                                self.volume = 0.0;
                            }
                        }
                        0x9 => {
                            if val != 0 && tick.is_multiple_of(u16::from(val)) {
                                self.sample_position = 0.0;
                                self.sample_direction = 1.0;
                                self.elapsed_samples = 0;
                            }
                        }
                        _ => {}
                    }
                }
                0x14 => {
                    if tick == u16::from(effect.param) {
                        self.note_off();
                    }
                }
                _ => {}
            }
        }

        match self.vol_column >> 4 {
            6 => {
                self.volume = (self.volume - (self.vol_column & 0x0F) as f32 / 64.0).max(0.0);
            }
            7 => {
                self.volume = (self.volume + (self.vol_column & 0x0F) as f32 / 64.0).min(1.0);
            }
            0xB => {
                self.do_vibrato();
            }
            0xF => {
                self.do_tone_porta();
            }
            _ => {}
        }

        if self.auto_vib_depth > 0 && self.auto_vib_rate > 0 {
            let sweep_factor = if self.auto_vib_sweep > 0 {
                (self.auto_vib_sweep_pos as f32 / f32::from(self.auto_vib_sweep)).min(1.0)
            } else {
                1.0
            };
            let pos = self.auto_vib_pos;
            let wave = match self.auto_vib_type {
                1 => {
                    if pos < 128 {
                        1.0_f32
                    } else {
                        -1.0
                    }
                }
                2 => 1.0 - (f32::from(pos) / 128.0),
                3 => (f32::from(pos) / 128.0) - 1.0,
                _ => (f32::from(pos) * std::f32::consts::TAU / 256.0).sin(),
            };
            let delta = wave * f32::from(self.auto_vib_depth) * sweep_factor * 2.0;
            let period = (self.base_period + delta).clamp(50.0, 7680.0);
            let freq = Self::period_to_freq(period);
            self.sample_step = Self::compute_sample_step(freq, &self.sample_data);
            self.auto_vib_pos = self.auto_vib_pos.wrapping_add(self.auto_vib_rate);
            self.auto_vib_sweep_pos = self.auto_vib_sweep_pos.saturating_add(1);
        }

        if self.vol_envelope.enabled {
            self.env_tick = self
                .vol_envelope
                .advance_tick(self.env_tick, self.note_released);
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

    fn update_freq_from_period(&mut self) {
        let freq = Self::period_to_freq(self.period);
        self.sample_step = Self::compute_sample_step(freq, &self.sample_data);
    }

    fn do_tone_porta(&mut self) {
        if self.porta_target == 0.0 || self.tone_porta_speed == 0 {
            return;
        }
        let speed = f32::from(self.tone_porta_speed) * 4.0;
        if self.period > self.porta_target {
            self.period = (self.period - speed).max(self.porta_target);
        } else if self.period < self.porta_target {
            self.period = (self.period + speed).min(self.porta_target);
        }
        self.base_period = self.period;
        self.update_freq_from_period();
    }

    fn do_vibrato(&mut self) {
        // Sine-based vibrato
        let pos = self.vibrato_pos & 63;
        let sine = (f32::from(pos) * std::f32::consts::TAU / 64.0).sin();
        let delta = sine * f32::from(self.vibrato_depth) * 4.0;
        let period = (self.base_period + delta).clamp(50.0, 7680.0);
        let freq = Self::period_to_freq(period);
        self.sample_step = Self::compute_sample_step(freq, &self.sample_data);
        self.vibrato_pos = self.vibrato_pos.wrapping_add(self.vibrato_speed);
    }

    fn do_vol_slide(&mut self, param: u8) {
        let up = (param >> 4) as f32;
        let down = (param & 0x0F) as f32;
        if up > 0.0 {
            self.volume = (self.volume + up / 64.0).min(1.0);
        } else {
            self.volume = (self.volume - down / 64.0).max(0.0);
        }
    }

    fn do_tremolo(&mut self) {
        let pos = f32::from(self.tremolo_pos) * std::f32::consts::TAU / 64.0;
        let delta = pos.sin() * f32::from(self.tremolo_depth) / 64.0;
        self.volume = (self.volume + delta).clamp(0.0, 1.0);
        self.tremolo_pos = self.tremolo_pos.wrapping_add(self.tremolo_speed);
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
    bpm: u16,
    receiver: mpsc::Receiver<Command>,
    playback_row: Arc<AtomicUsize>,
    playback_order: Arc<AtomicUsize>,
    master_volume: f32,
    jump_to_order: Option<usize>,
    break_to_row: Option<usize>,
    stereo_phase: bool,
    left_sample: f32,
    right_sample: f32,
    preview_tick_counter: f64,
    preview_tick: u16,
    muted_channels: Vec<bool>,
    stop_at_end: bool,
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
            bpm: 125,
            receiver,
            playback_row,
            playback_order,
            master_volume: 1.0,
            jump_to_order: None,
            break_to_row: None,
            stereo_phase: false,
            left_sample: 0.0,
            right_sample: 0.0,
            preview_tick_counter: 0.0,
            preview_tick: 0,
            muted_channels: vec![false; 32],
            stop_at_end: false,
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
                    self.speed = 6;
                    self.bpm = 125;
                    self.samples_per_tick =
                        f64::from(SAMPLE_RATE) * 5.0 / (f64::from(125u16) * 2.0);
                    self.master_volume = 1.0;
                    self.jump_to_order = None;
                    self.break_to_row = None;
                    self.stereo_phase = false;
                    self.left_sample = 0.0;
                    self.right_sample = 0.0;
                    self.muted_channels.clear();
                    self.stop_at_end = false;
                    self.patterns.clear();
                    self.order.clear();
                    self.settings = None;
                }
                Command::UpdateSettings { settings } => {
                    if self.playing {
                        if settings.bpm != self.bpm {
                            self.bpm = settings.bpm;
                            self.samples_per_tick =
                                f64::from(SAMPLE_RATE) * 5.0 / (f64::from(self.bpm) * 2.0);
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
                    vol_envelope,
                    sample_data,
                    master_volume,
                    vibrato_type,
                    vibrato_sweep,
                    vibrato_depth,
                    vibrato_rate,
                } => {
                    self.preview_channel.trigger(
                        frequency,
                        volume * 0.8,
                        vol_envelope,
                        &sample_data,
                        0,
                    );
                    self.preview_channel.auto_vib_type = vibrato_type;
                    self.preview_channel.auto_vib_sweep = vibrato_sweep;
                    self.preview_channel.auto_vib_depth = vibrato_depth;
                    self.preview_channel.auto_vib_rate = vibrato_rate;
                    self.preview_tick_counter = 0.0;
                    self.preview_tick = 0;
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
        for (i, ch) in self.channels.iter_mut().enumerate() {
            if let Some(&pan) = settings.channel_panning.get(i) {
                ch.panning = pan;
            }
        }

        self.samples_per_tick = f64::from(SAMPLE_RATE) * 5.0 / (f64::from(settings.bpm) * 2.0);
        self.speed = settings.speed;
        self.bpm = settings.bpm;
        self.master_volume = settings.master_volume;
        self.current_row = start_row;
        self.current_order_idx = start_order;
        self.tick_sample_counter = 0.0;
        self.tick_in_row = 0;
        self.jump_to_order = None;
        self.break_to_row = None;
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
            Some(p) => p.clone(),
            None => return,
        };
        let Some(settings) = self.settings.as_ref() else {
            return;
        };

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

            let effect = effect.map(|e| {
                let mut e = e;
                match e.kind {
                    1 => {
                        if e.param != 0 {
                            channel.last_porta_up = e.param;
                        } else {
                            e.param = channel.last_porta_up;
                        }
                    }
                    2 => {
                        if e.param != 0 {
                            channel.last_porta_down = e.param;
                        } else {
                            e.param = channel.last_porta_down;
                        }
                    }
                    3 => {
                        if e.param != 0 {
                            channel.tone_porta_speed = e.param;
                        }
                    }
                    5 | 6 | 0xA => {
                        if e.param != 0 {
                            channel.last_vol_slide = e.param;
                        } else {
                            e.param = channel.last_vol_slide;
                        }
                    }
                    9 => {
                        if e.param != 0 {
                            channel.last_sample_offset = e.param;
                        } else {
                            e.param = channel.last_sample_offset;
                        }
                    }
                    _ => {}
                }
                e
            });

            let is_tone_porta = matches!(effect, Some(Effect { kind: 3 | 5, .. }));
            let is_note_delay =
                matches!(effect, Some(Effect { kind: 0xE, param }) if param >> 4 == 0xD);

            match cell {
                Cell::NoteOn(note) => {
                    let vol_from_col = volume.and_then(|v| {
                        if (0x10..=0x50).contains(&v) {
                            Some((v - 0x10).min(64) as f32 / 64.0)
                        } else {
                            None
                        }
                    });

                    let (sample_data, sample_vol) = inst.sample_for_note(note.pitch);

                    if is_tone_porta {
                        channel.porta_target = Channel::freq_to_period(note.frequency());
                    } else if is_note_delay {
                        let delay_tick = effect.unwrap().param & 0x0F;
                        let vol = vol_from_col.unwrap_or(sample_vol);

                        channel.note_delay_tick = delay_tick;
                        channel.delayed_frequency = note.frequency();
                        channel.delayed_volume = vol;
                        channel.delayed_vol_envelope = inst.vol_envelope.clone();
                        channel.delayed_sample_data = Arc::clone(sample_data);
                        channel.delayed_fadeout_per_tick = inst.vol_fadeout;
                        channel.has_delayed_note = true;
                        channel.auto_vib_type = inst.vibrato_type;
                        channel.auto_vib_sweep = inst.vibrato_sweep;
                        channel.auto_vib_depth = inst.vibrato_depth;
                        channel.auto_vib_rate = inst.vibrato_rate;
                        channel.panning = inst.default_panning;
                    } else {
                        let vol = vol_from_col.unwrap_or(sample_vol);

                        channel.trigger(
                            note.frequency(),
                            vol,
                            inst.vol_envelope.clone(),
                            sample_data,
                            inst.vol_fadeout,
                        );
                        channel.auto_vib_type = inst.vibrato_type;
                        channel.auto_vib_sweep = inst.vibrato_sweep;
                        channel.auto_vib_depth = inst.vibrato_depth;
                        channel.auto_vib_rate = inst.vibrato_rate;
                        channel.panning = inst.default_panning;
                    }
                }
                Cell::NoteOff => channel.note_off(),
                Cell::Empty => {}
            }

            channel.vol_column = volume.unwrap_or(0);
            if let Some(v) = volume {
                match v >> 4 {
                    // 0x10-0x50: Set volume (already handled above for note trigger)
                    1..=4 => {
                        channel.volume = (v - 0x10).min(64) as f32 / 64.0;
                    }
                    5 if v <= 0x50 => {
                        channel.volume = (v - 0x10).min(64) as f32 / 64.0;
                    }
                    // 0x80-0x8F: Fine volume slide down (tick 0 only)
                    8 => {
                        channel.volume = (channel.volume - (v & 0x0F) as f32 / 64.0).max(0.0);
                    }
                    // 0x90-0x9F: Fine volume slide up (tick 0 only)
                    9 => {
                        channel.volume = (channel.volume + (v & 0x0F) as f32 / 64.0).min(1.0);
                    }
                    // 0xC0-0xCF: Set panning
                    0xC => {
                        channel.panning = (v & 0x0F) as f32 / 15.0;
                    }
                    // 0xF0-0xFF: Tone portamento
                    0xF => {
                        let speed = (v & 0x0F) * 16;
                        if speed != 0 {
                            channel.tone_porta_speed = speed;
                        }
                    }
                    _ => {}
                }
            }

            channel.current_effect = effect;

            if let Some(e) = effect {
                match e.kind {
                    // 0xy — Arpeggio (store nibbles)
                    0 if e.param != 0 => {
                        channel.arpeggio_x = e.param >> 4;
                        channel.arpeggio_y = e.param & 0x0F;
                    }
                    // 4xy — Vibrato
                    4 => {
                        let x = e.param >> 4;
                        let y = e.param & 0x0F;
                        if x != 0 {
                            channel.vibrato_speed = x;
                        }
                        if y != 0 {
                            channel.vibrato_depth = y;
                        }
                    }
                    // 7xy — Tremolo (store params)
                    7 => {
                        let x = e.param >> 4;
                        let y = e.param & 0x0F;
                        if x != 0 {
                            channel.tremolo_speed = x;
                        }
                        if y != 0 {
                            channel.tremolo_depth = y;
                        }
                    }
                    // 8xx — Set panning
                    8 => {
                        channel.panning = f32::from(e.param) / 255.0;
                    }
                    // 9xx — Sample offset
                    9 => {
                        if inst_num.is_some() {
                            channel.sample_position = f64::from(e.param) * 256.0;
                        }
                    }
                    // Bxx — Position jump
                    0xB => {
                        self.jump_to_order = Some(e.param as usize);
                    }
                    // Cxx — Set volume
                    0xC => {
                        channel.volume = (e.param.min(64) as f32) / 64.0;
                    }
                    // Dxx — Pattern break (BCD)
                    0xD => {
                        let hi = e.param >> 4;
                        let lo = e.param & 0x0F;
                        self.break_to_row = Some((hi * 10 + lo) as usize);
                    }
                    // Exx — Extended effects (tick-0 sub-effects)
                    0xE => {
                        let sub = e.param >> 4;
                        let val = e.param & 0x0F;
                        match sub {
                            // E1x — Fine portamento up
                            1 => {
                                channel.period = (channel.period - f32::from(val) * 4.0).max(50.0);
                                channel.update_freq_from_period();
                            }
                            // E2x — Fine portamento down
                            2 => {
                                channel.period =
                                    (channel.period + f32::from(val) * 4.0).min(7680.0);
                                channel.update_freq_from_period();
                            }
                            // E5x — Set finetune
                            5 => {
                                let ft = if val > 7 { val as i8 - 16 } else { val as i8 };
                                channel.period =
                                    (channel.period - f32::from(ft) * 4.0).clamp(50.0, 7680.0);
                                channel.base_period = channel.period;
                                channel.update_freq_from_period();
                            }
                            // EAx — Fine volume slide up
                            0xA => {
                                channel.volume = (channel.volume + f32::from(val) / 64.0).min(1.0);
                            }
                            // EBx — Fine volume slide down
                            0xB => {
                                channel.volume = (channel.volume - f32::from(val) / 64.0).max(0.0);
                            }
                            _ => {}
                        }
                    }
                    0xF => {
                        if e.param > 0 {
                            if e.param <= 0x1F {
                                self.speed = u16::from(e.param);
                            } else {
                                self.bpm = u16::from(e.param);
                                self.samples_per_tick =
                                    f64::from(SAMPLE_RATE) * 60.0 / f64::from(self.bpm) / 24.0;
                            }
                        }
                    }
                    // Gxx — Set global volume (0-40h)
                    0x10 => {
                        self.master_volume = (e.param.min(0x40) as f32) / 64.0;
                    }
                    // Kxx — Key off (tick 0: immediate key off)
                    0x14 => {
                        if e.param == 0 {
                            channel.note_off();
                        }
                        // Non-zero param handled in tick_update
                    }
                    // Lxx — Set envelope position
                    0x15 => {
                        channel.env_tick = u16::from(e.param);
                    }
                    _ => {}
                }
            }
        }
    }

    fn tick(&mut self) {
        self.tick_in_row += 1;
        if self.tick_in_row >= self.speed {
            self.tick_in_row = 0;
            if !self.patterns.is_empty() {
                if let Some(order) = self.jump_to_order.take() {
                    let row = self.break_to_row.take().unwrap_or(0);
                    self.current_order_idx = order.min(self.order.len() - 1);
                    self.current_row = row;
                    self.playback_order
                        .store(self.current_order_idx, Ordering::Relaxed);
                } else if let Some(row) = self.break_to_row.take() {
                    let next_order = self.current_order_idx + 1;
                    if next_order >= self.order.len() && self.stop_at_end {
                        self.playing = false;
                        return;
                    }
                    self.current_order_idx = next_order % self.order.len();
                    self.current_row = row;
                    self.playback_order
                        .store(self.current_order_idx, Ordering::Relaxed);
                } else {
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
                }
                self.playback_row.store(self.current_row, Ordering::Relaxed);
            }
            self.process_row();

            for ch in &mut self.channels {
                ch.tick_update(0);
            }
        } else {
            let tick = self.tick_in_row;
            for ch in &mut self.channels {
                ch.tick_update(tick);
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

        self.process_commands();

        if self.stop_at_end && !self.playing {
            return None;
        }

        let mut mix_l = 0.0_f32;
        let mut mix_r = 0.0_f32;

        if self.playing {
            self.tick_sample_counter += 1.0;
            if self.tick_sample_counter >= self.samples_per_tick {
                self.tick_sample_counter -= self.samples_per_tick;
                self.tick();
            }
            for (ch_idx, ch) in self.channels.iter_mut().enumerate() {
                if self.muted_channels.get(ch_idx).copied().unwrap_or(false) {
                    ch.next_sample();
                    continue;
                }
                let sample = ch.next_sample();
                let pan = ch.panning;
                mix_l += sample * (1.0 - pan).sqrt();
                mix_r += sample * pan.sqrt();
            }
        }

        let preview = self.preview_channel.next_sample();
        if self.preview_channel.active {
            self.preview_tick_counter += 1.0;
            if self.preview_tick_counter >= self.samples_per_tick {
                self.preview_tick_counter -= self.samples_per_tick;
                self.preview_tick += 1;
                self.preview_channel.tick_update(self.preview_tick);
                if self.preview_tick == 10 {
                    self.preview_channel.note_off();
                }
            }
        }
        mix_l += preview * 0.707;
        mix_r += preview * 0.707;

        self.left_sample = mix_l;
        self.right_sample = mix_r;
        self.stereo_phase = true;

        Some(self.left_sample * self.master_volume)
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
    instruments: &[Instrument],
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
        speed: DEFAULT_SPEED,
        master_volume,
        instruments: instruments.to_vec(),
        muted_channels: Vec::new(),
        channel_panning: Vec::new(),
    });

    let _ = sender.send(Command::Play {
        start_row: 0,
        start_order: 0,
        patterns: snapshots,
        order: order.to_vec(),
        settings,
    });
    drop(sender);

    let mut source = TrackerSource::new(receiver, playback_row, playback_order);
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
        let source = TrackerSource::new(rx, row.clone(), order);
        (tx, source, row)
    }

    fn play_cmd(pattern: &crate::project::Pattern) -> Command {
        let snapshot = Arc::new(PatternSnapshot::from_pattern(pattern));
        let settings = Arc::new(PlaybackSettings {
            bpm: 125,
            speed: DEFAULT_SPEED,
            master_volume: 1.0,
            instruments: Instrument::defaults(),
            muted_channels: Vec::new(),
            channel_panning: Vec::new(),
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
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(58)));

        tx.send(play_cmd(&pattern)).unwrap();

        for _ in 0..10584 {
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
        pattern.set(0, 0, Cell::NoteOn(crate::project::Note::new(58)));

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
