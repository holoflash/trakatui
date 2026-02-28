use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rodio::Source;

use crate::project::{Envelope, Waveform};

impl Waveform {
    pub(crate) fn sample(self, phase: f32) -> f32 {
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
    pub(crate) fn amplitude(&self, time: f32, note_duration: f32) -> f32 {
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

pub struct PitchBendControl {
    target_freq: AtomicU32,
    start_secs: AtomicU32,
    duration_secs: AtomicU32,
}

impl PitchBendControl {
    pub const fn new() -> Self {
        Self {
            target_freq: AtomicU32::new(0f32.to_bits()),
            start_secs: AtomicU32::new(0f32.to_bits()),
            duration_secs: AtomicU32::new(0f32.to_bits()),
        }
    }

    pub fn reset(&self) {
        self.target_freq.store(0f32.to_bits(), Ordering::Relaxed);
    }

    pub fn set(&self, target_freq: f32, start_secs: f32, duration_secs: f32) {
        self.start_secs
            .store(start_secs.to_bits(), Ordering::Relaxed);
        self.duration_secs
            .store(duration_secs.to_bits(), Ordering::Relaxed);
        self.target_freq
            .store(target_freq.to_bits(), Ordering::Release);
    }
}

pub struct SynthSource {
    waveform: Waveform,
    base_frequency: f32,
    bend_control: Arc<PitchBendControl>,
    envelope: Envelope,
    sample_rate: f32,
    sample_rate_u32: u32,
    phase: f32,
    elapsed_samples: u32,
    total_samples: u32,
    note_duration: f32,
    amplitude: f32,
    noise_held: f32,
}

impl SynthSource {
    pub fn new(
        waveform: Waveform,
        frequency: f32,
        duration: Duration,
        amplitude: f32,
        envelope: Envelope,
        bend_control: Arc<PitchBendControl>,
    ) -> Self {
        let sample_rate_u32: u32 = 44100;
        let sample_rate = 44100.0_f32;
        let note_duration = duration.as_secs_f32();
        let total_samples = (note_duration * sample_rate).round() as u32;
        Self {
            waveform,
            base_frequency: frequency,
            bend_control,
            envelope,
            sample_rate,
            sample_rate_u32,
            phase: 0.0,
            elapsed_samples: 0,
            total_samples,
            note_duration,
            amplitude,
            noise_held: fastrand::f32().mul_add(2.0, -1.0),
        }
    }

    fn current_frequency(&self, time: f32) -> f32 {
        let target = f32::from_bits(self.bend_control.target_freq.load(Ordering::Acquire));
        if target == 0.0 {
            return self.base_frequency;
        }
        let start = f32::from_bits(self.bend_control.start_secs.load(Ordering::Relaxed));
        let dur = f32::from_bits(self.bend_control.duration_secs.load(Ordering::Relaxed));

        if dur <= 0.0 || time < start {
            return self.base_frequency;
        }

        let t = ((time - start) / dur).clamp(0.0, 1.0);
        (target - self.base_frequency).mul_add(t, self.base_frequency)
    }
}

impl Iterator for SynthSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.elapsed_samples >= self.total_samples {
            return None;
        }

        let time = f64::from(self.elapsed_samples) as f32 / self.sample_rate;
        let env_amp = self.envelope.amplitude(time, self.note_duration);
        let sample = if self.waveform == Waveform::Noise {
            self.noise_held
        } else {
            self.waveform.sample(self.phase)
        };

        let freq = self.current_frequency(time);
        self.phase += freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            if self.waveform == Waveform::Noise {
                self.noise_held = fastrand::f32().mul_add(2.0, -1.0);
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
        NonZero::new(self.sample_rate_u32).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.note_duration))
    }
}
