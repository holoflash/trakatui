use std::sync::Arc;

use super::sample::SampleData;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Saw,
    Noise,
    Sampler,
}

impl Waveform {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Sine => "SINE",
            Self::Triangle => "TRIANGLE",
            Self::Square => "SQUARE",
            Self::Saw => "SAW",
            Self::Noise => "NOISE",
            Self::Sampler => "SAMPLE",
        }
    }

    pub const fn next(self) -> Self {
        match self {
            Self::Sine => Self::Triangle,
            Self::Triangle => Self::Square,
            Self::Square => Self::Saw,
            Self::Saw => Self::Noise,
            Self::Noise => Self::Sampler,
            Self::Sampler => Self::Sine,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Sine => Self::Sampler,
            Self::Triangle => Self::Sine,
            Self::Square => Self::Triangle,
            Self::Saw => Self::Square,
            Self::Noise => Self::Saw,
            Self::Sampler => Self::Noise,
        }
    }

    pub const fn default_envelope(self) -> Envelope {
        match self {
            Self::Sine => Envelope {
                attack: 0.01,
                decay: 0.05,
                sustain: 0.9,
                release: 0.05,
            },
            Self::Triangle => Envelope {
                attack: 0.01,
                decay: 0.06,
                sustain: 0.9,
                release: 0.05,
            },
            Self::Square => Envelope {
                attack: 0.005,
                decay: 0.1,
                sustain: 0.8,
                release: 0.03,
            },
            Self::Saw => Envelope {
                attack: 0.005,
                decay: 0.08,
                sustain: 0.6,
                release: 0.04,
            },
            Self::Noise => Envelope {
                attack: 0.001,
                decay: 0.05,
                sustain: 0.3,
                release: 0.02,
            },
            Self::Sampler => Envelope {
                attack: 0.000,
                decay: 0.1,
                sustain: 0.8,
                release: 0.05,
            },
        }
    }
}

impl Waveform {
    pub fn sample(self, phase: f32) -> f32 {
        match self {
            Self::Sine => (std::f32::consts::TAU * phase).sin(),
            Self::Triangle => 4.0f32.mul_add((phase - (phase + 0.5).floor()).abs(), -1.0),
            Self::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Self::Saw => 2.0f32.mul_add(phase, -1.0),
            Self::Noise => fastrand::f32().mul_add(2.0, -1.0),
            Self::Sampler => 0.0,
        }
    }
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

pub const DEFAULT_INSTRUMENTS: [Waveform; 8] = [
    Waveform::Square,
    Waveform::Saw,
    Waveform::Triangle,
    Waveform::Sine,
    Waveform::Noise,
    Waveform::Sampler,
    Waveform::Sampler,
    Waveform::Sampler,
];

#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

#[derive(Debug, Clone)]
pub struct ChannelSettings {
    pub waveform: Waveform,
    pub envelope: Envelope,
    pub volume: f32,
    pub sample_data: Option<Arc<SampleData>>,
}

impl ChannelSettings {
    pub fn default_for(waveform: Waveform) -> Self {
        let volume = if waveform == Waveform::Sampler {
            1.0
        } else {
            0.5
        };
        Self {
            envelope: waveform.default_envelope(),
            waveform,
            volume,
            sample_data: None,
        }
    }

    pub fn defaults() -> Vec<Self> {
        DEFAULT_INSTRUMENTS
            .iter()
            .map(|w| Self::default_for(*w))
            .collect()
    }
}
