use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::sample::SampleData;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WaveformKind {
    Sample,
    Sine,
    Triangle,
    Square,
    Saw,
    Noise,
}

impl WaveformKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Sample => "Sample",
            Self::Sine => "Sine",
            Self::Triangle => "Tri",
            Self::Square => "Sqr",
            Self::Saw => "Saw",
            Self::Noise => "Noise",
        }
    }

    pub fn generate(self) -> Arc<SampleData> {
        match self {
            Self::Sample => SampleData::silent(),
            Self::Sine => SampleData::sine(),
            Self::Triangle => SampleData::triangle(),
            Self::Square => SampleData::square(),
            Self::Saw => SampleData::saw(),
            Self::Noise => SampleData::noise(),
        }
    }

    pub const ALL: &'static [Self] = &[
        Self::Sample,
        Self::Sine,
        Self::Triangle,
        Self::Square,
        Self::Saw,
        Self::Noise,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[derive(Default)]
#[allow(clippy::enum_variant_names)]
pub enum FilterType {
    #[default]
    LowPass,
    HighPass,
    BandPass,
}

impl FilterType {
    pub fn label(self) -> &'static str {
        match self {
            Self::LowPass => "LP",
            Self::HighPass => "HP",
            Self::BandPass => "BP",
        }
    }
    pub const ALL: &'static [Self] = &[Self::LowPass, Self::HighPass, Self::BandPass];
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterSettings {
    pub enabled: bool,
    pub filter_type: FilterType,
    pub cutoff: f32,
    pub resonance: f32,
    pub env_depth: f32,
    pub envelope: VolEnvelope,
}

impl Default for FilterSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            filter_type: FilterType::LowPass,
            cutoff: 20000.0,
            resonance: 0.0,
            env_depth: 0.0,
            envelope: VolEnvelope::disabled(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolEnvelope {
    pub points: Vec<(u16, u16)>,
    pub sustain_point: Option<usize>,
    pub loop_range: Option<(usize, usize)>,
    pub enabled: bool,
}

impl VolEnvelope {
    pub fn disabled() -> Self {
        Self {
            points: vec![(0, 64)],
            sustain_point: None,
            loop_range: None,
            enabled: false,
        }
    }

    pub fn default_preset() -> Self {
        Self {
            points: vec![(0, 64), (9, 0), (96, 0)],
            sustain_point: Some(1),
            loop_range: None,
            enabled: false,
        }
    }

    pub fn amplitude_at_tick(&self, tick: u16) -> f32 {
        if !self.enabled || self.points.is_empty() {
            return 1.0;
        }

        let points = &self.points;

        if tick <= points[0].0 {
            return points[0].1 as f32 / 64.0;
        }

        if tick >= points[points.len() - 1].0 {
            return points[points.len() - 1].1 as f32 / 64.0;
        }

        for i in 0..points.len() - 1 {
            let (t0, v0) = points[i];
            let (t1, v1) = points[i + 1];
            if tick >= t0 && tick < t1 {
                if t1 == t0 {
                    return v0 as f32 / 64.0;
                }
                let frac = (tick - t0) as f32 / (t1 - t0) as f32;
                let vol = v0 as f32 + (v1 as f32 - v0 as f32) * frac;
                return vol / 64.0;
            }
        }

        points[points.len() - 1].1 as f32 / 64.0
    }

    pub fn advance_tick(&self, current_tick: u16, note_released: bool) -> u16 {
        let next = current_tick + 1;

        if !note_released
            && let Some(sus_idx) = self.sustain_point
            && sus_idx < self.points.len()
            && next >= self.points[sus_idx].0
        {
            return self.points[sus_idx].0;
        }

        if let Some((loop_start, loop_end)) = self.loop_range
            && loop_start < self.points.len()
            && loop_end < self.points.len()
        {
            let loop_end_tick = self.points[loop_end].0;
            if next > loop_end_tick {
                return self.points[loop_start].0;
            }
        }

        next
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub name: String,
    pub waveform: WaveformKind,
    pub vol_envelope: VolEnvelope,
    pub sample_data: Arc<SampleData>,
    pub default_volume: f32,
    pub samples: Vec<(Arc<SampleData>, f32)>,
    pub note_to_sample: Vec<u8>,
    pub vol_fadeout: u16,
    pub default_panning: f32,
    #[serde(default)]
    pub coarse_tune: i8,
    #[serde(default)]
    pub fine_tune: i8,
    #[serde(default)]
    pub pitch_env_enabled: bool,
    #[serde(default = "default_pitch_env_depth")]
    pub pitch_env_depth: f32,
    #[serde(default = "VolEnvelope::disabled")]
    pub pitch_envelope: VolEnvelope,
    #[serde(default)]
    pub filter: FilterSettings,
    #[serde(default = "default_polyphony")]
    pub polyphony: u8,
}

fn default_pitch_env_depth() -> f32 {
    12.0
}

fn default_polyphony() -> u8 {
    1
}

impl Track {
    pub fn sample_for_note(&self, pitch: u8) -> (&Arc<SampleData>, f32) {
        if !self.note_to_sample.is_empty() && !self.samples.is_empty() {
            let map_idx = (pitch as usize).min(self.note_to_sample.len() - 1);
            let sample_idx = self.note_to_sample[map_idx] as usize;
            if sample_idx < self.samples.len() {
                let (ref sd, vol) = self.samples[sample_idx];
                return (sd, vol);
            }
        }
        (&self.sample_data, self.default_volume)
    }

    pub fn defaults() -> Vec<Self> {
        vec![Self::new_empty("Track 00")]
    }

    pub fn new_empty(name: &str) -> Self {
        Self {
            name: name.into(),
            waveform: WaveformKind::Square,
            vol_envelope: VolEnvelope::default_preset(),
            sample_data: SampleData::square(),
            default_volume: 0.2,
            samples: Vec::new(),
            note_to_sample: Vec::new(),
            vol_fadeout: 0,
            default_panning: 0.5,
            coarse_tune: 0,
            fine_tune: 0,
            pitch_env_enabled: false,
            pitch_env_depth: 12.0,
            pitch_envelope: VolEnvelope::disabled(),
            filter: FilterSettings::default(),
            polyphony: 1,
        }
    }
}
