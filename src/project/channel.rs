use std::sync::Arc;

use super::sample::SampleData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
pub struct Instrument {
    pub name: String,
    pub waveform: WaveformKind,
    pub vol_envelope: VolEnvelope,
    pub sample_data: Arc<SampleData>,
    pub default_volume: f32,
    pub samples: Vec<(Arc<SampleData>, f32)>,
    pub note_to_sample: Vec<u8>,
    pub vol_fadeout: u16,
    pub default_panning: f32,
    pub vibrato_type: u8,
    pub vibrato_sweep: u8,
    pub vibrato_depth: u8,
    pub vibrato_rate: u8,
}

impl Instrument {
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
        vec![Self::new_empty("Instrument 01")]
    }

    pub fn new_empty(name: &str) -> Self {
        Self {
            name: name.into(),
            waveform: WaveformKind::Square,
            vol_envelope: VolEnvelope::default_preset(),
            sample_data: SampleData::square(),
            default_volume: 0.5,
            samples: Vec::new(),
            note_to_sample: Vec::new(),
            vol_fadeout: 0,
            default_panning: 0.5,
            vibrato_type: 0,
            vibrato_sweep: 0,
            vibrato_depth: 0,
            vibrato_rate: 0,
        }
    }
}
