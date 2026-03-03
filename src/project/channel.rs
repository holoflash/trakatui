use std::sync::Arc;

use super::sample::SampleData;

#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Envelope {
    pub fn amplitude(&self, time: f32, note_duration: f32) -> f32 {
        let release_start = note_duration - self.release;

        if self.attack > 0.0 && time < self.attack {
            time / self.attack
        } else if self.decay > 0.0 && time < self.attack + self.decay {
            let decay_progress = (time - self.attack) / self.decay;
            (1.0 - self.sustain).mul_add(-decay_progress, 1.0)
        } else if time < release_start {
            self.sustain
        } else if self.release > 0.0 && time < note_duration {
            let release_progress = (time - release_start) / self.release;
            self.sustain * (1.0 - release_progress)
        } else if time >= note_duration {
            0.0
        } else {
            self.sustain
        }
    }
}

const DEFAULT_ENVELOPE: Envelope = Envelope {
    attack: 0.005,
    decay: 0.08,
    sustain: 0.8,
    release: 0.05,
};

const NOISE_ENVELOPE: Envelope = Envelope {
    attack: 0.001,
    decay: 0.05,
    sustain: 0.3,
    release: 0.02,
};

#[derive(Debug, Clone)]
pub struct Instrument {
    pub name: String,
    pub envelope: Envelope,
    pub sample_data: Arc<SampleData>,
}

impl Instrument {
    pub fn defaults() -> Vec<Self> {
        vec![
            Self {
                name: "Square".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::square(),
            },
            Self {
                name: "Saw".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::saw(),
            },
            Self {
                name: "Triangle".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::triangle(),
            },
            Self {
                name: "Sine".into(),
                envelope: Envelope {
                    attack: 0.01,
                    decay: 0.05,
                    sustain: 0.9,
                    release: 0.05,
                },
                sample_data: SampleData::sine(),
            },
            Self {
                name: "Noise".into(),
                envelope: NOISE_ENVELOPE,
                sample_data: SampleData::noise(),
            },
            Self {
                name: "Empty 1".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::silent(),
            },
            Self {
                name: "Empty 2".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::silent(),
            },
            Self {
                name: "Empty 3".into(),
                envelope: DEFAULT_ENVELOPE,
                sample_data: SampleData::silent(),
            },
        ]
    }
}
