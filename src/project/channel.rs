use std::sync::Arc;

use super::sample::SampleData;

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
            let map_idx = (pitch.saturating_sub(1) as usize).min(self.note_to_sample.len() - 1);
            let sample_idx = self.note_to_sample[map_idx] as usize;
            if sample_idx < self.samples.len() {
                let (ref sd, vol) = self.samples[sample_idx];
                return (sd, vol);
            }
        }
        (&self.sample_data, self.default_volume)
    }

    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                name: "Square".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::square(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Saw".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::saw(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Triangle".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::triangle(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Sine".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::sine(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Noise".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::noise(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Empty 1".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::silent(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Empty 2".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::silent(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
            Self {
                name: "Empty 3".into(),
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::silent(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            },
        ]
    }
}
