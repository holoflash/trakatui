use std::num::NonZero;
use std::time::Duration;

use rodio::Source;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Waveform {
    Sine,
    Triangle,
    Square,
    Saw,
    Noise,
}

impl Waveform {
    pub fn name(&self) -> &'static str {
        match self {
            Waveform::Sine => "SIN",
            Waveform::Triangle => "TRI",
            Waveform::Square => "SQR",
            Waveform::Saw => "SAW",
            Waveform::Noise => "NOS",
        }
    }

    fn sample(&self, phase: f32) -> f32 {
        match self {
            Waveform::Sine => (std::f32::consts::TAU * phase).sin(),
            Waveform::Triangle => 4.0 * (phase - (phase + 0.5).floor()).abs() - 1.0,
            Waveform::Square => {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Saw => 2.0 * phase - 1.0,
            Waveform::Noise => fastrand::f32() * 2.0 - 1.0,
        }
    }

    pub fn default_envelope(&self) -> Envelope {
        match self {
            Waveform::Sine => Envelope {
                attack: 0.01,
                decay: 0.05,
                sustain: 0.9,
                release: 0.05,
            },
            Waveform::Triangle => Envelope {
                attack: 0.01,
                decay: 0.06,
                sustain: 0.9,
                release: 0.05,
            },
            Waveform::Square => Envelope {
                attack: 0.005,
                decay: 0.1,
                sustain: 0.8,
                release: 0.03,
            },
            Waveform::Saw => Envelope {
                attack: 0.005,
                decay: 0.08,
                sustain: 0.6,
                release: 0.04,
            },
            Waveform::Noise => Envelope {
                attack: 0.001,
                decay: 0.05,
                sustain: 0.3,
                release: 0.02,
            },
        }
    }
}

pub const CHANNEL_INSTRUMENTS: [Waveform; 8] = [
    Waveform::Sine,
    Waveform::Triangle,
    Waveform::Square,
    Waveform::Square,
    Waveform::Saw,
    Waveform::Saw,
    Waveform::Noise,
    Waveform::Noise,
];

#[derive(Debug, Clone, Copy)]
pub struct Envelope {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Envelope {
    fn amplitude(&self, time: f32, note_duration: f32) -> f32 {
        let release_start = note_duration - self.release;

        if time < self.attack {
            time / self.attack
        } else if time < self.attack + self.decay {
            let decay_progress = (time - self.attack) / self.decay;
            1.0 - (1.0 - self.sustain) * decay_progress
        } else if time < release_start {
            self.sustain
        } else if time < note_duration {
            let release_progress = (time - release_start) / self.release;
            self.sustain * (1.0 - release_progress)
        } else {
            0.0
        }
    }
}

pub struct SynthSource {
    waveform: Waveform,
    frequency: f32,
    envelope: Envelope,
    sample_rate: u32,
    phase: f32,
    elapsed_samples: u32,
    total_samples: u32,
    note_duration: f32,
    amplitude: f32,
    noise_held: f32,
}

impl SynthSource {
    pub fn new(waveform: Waveform, frequency: f32, duration: Duration, amplitude: f32) -> Self {
        let sample_rate = 44100;
        let note_duration = duration.as_secs_f32();
        let total_samples = (note_duration * sample_rate as f32) as u32;
        Self {
            waveform,
            frequency,
            envelope: waveform.default_envelope(),
            sample_rate,
            phase: 0.0,
            elapsed_samples: 0,
            total_samples,
            note_duration,
            amplitude,
            noise_held: fastrand::f32() * 2.0 - 1.0,
        }
    }
}

impl Iterator for SynthSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.elapsed_samples >= self.total_samples {
            return None;
        }

        let time = self.elapsed_samples as f32 / self.sample_rate as f32;
        let env_amp = self.envelope.amplitude(time, self.note_duration);
        let sample = if self.waveform == Waveform::Noise {
            self.noise_held
        } else {
            self.waveform.sample(self.phase)
        };

        self.phase += self.frequency / self.sample_rate as f32;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            if self.waveform == Waveform::Noise {
                self.noise_held = fastrand::f32() * 2.0 - 1.0;
            }
        }

        self.elapsed_samples += 1;
        Some(sample * env_amp * self.amplitude)
    }
}

impl Source for SynthSource {
    fn current_span_len(&self) -> Option<usize> {
        Some((self.total_samples - self.elapsed_samples) as usize)
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(self.sample_rate).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.note_duration))
    }
}
