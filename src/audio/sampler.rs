use std::num::NonZero;
use std::sync::Arc;
use std::time::Duration;

use rodio::Source;

use crate::project::{Envelope, SampleData};

pub struct SamplerSource {
    data: Arc<SampleData>,
    step: f64,
    position: f64,
    envelope: Envelope,
    amplitude: f32,
    inv_output_rate: f32,
    elapsed_samples: u32,
    total_samples: u32,
    note_duration: f32,
}

impl SamplerSource {
    pub fn new(
        data: Arc<SampleData>,
        target_frequency: f32,
        duration: Duration,
        amplitude: f32,
        envelope: Envelope,
    ) -> Self {
        let output_rate: u32 = 44100;
        let note_duration = duration.as_secs_f32();
        let total_samples = (note_duration * output_rate as f32).round() as u32;

        let base_freq = 440.0 * ((f32::from(data.base_note) - 69.0) / 12.0).exp2();
        let playback_rate = f64::from(target_frequency) / f64::from(base_freq);
        let step = (f64::from(data.sample_rate) / f64::from(output_rate)) * playback_rate;

        Self {
            data,
            step,
            position: 0.0,
            envelope,
            amplitude,
            inv_output_rate: 1.0 / output_rate as f32,
            elapsed_samples: 0,
            total_samples,
            note_duration,
        }
    }
}

impl Iterator for SamplerSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.elapsed_samples >= self.total_samples {
            return None;
        }

        let time = self.elapsed_samples as f32 * self.inv_output_rate;
        let env_amp = self.envelope.amplitude(time, self.note_duration);

        let idx = self.position as usize;
        let frac = (self.position - idx as f64) as f32;
        let samples = &self.data.samples_f32;

        let sample = if idx >= samples.len() {
            0.0
        } else if idx + 1 < samples.len() {
            samples[idx] + (samples[idx + 1] - samples[idx]) * frac
        } else {
            samples[idx]
        };

        self.position += self.step;
        self.elapsed_samples += 1;
        Some(sample * env_amp * self.amplitude)
    }
}

impl Source for SamplerSource {
    fn current_span_len(&self) -> Option<usize> {
        Some((self.total_samples - self.elapsed_samples) as usize)
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        NonZero::new(44100).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.note_duration))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_sample() -> Arc<SampleData> {
        Arc::new(SampleData {
            name: "test.wav".to_string(),
            samples_i16: vec![0i16; 44100],
            samples_f32: vec![0.0f32; 44100],
            sample_rate: 44100,
            base_note: 60,
        })
    }

    #[test]
    fn correct_sample_count() {
        let data = make_test_sample();
        let source = SamplerSource::new(
            data,
            440.0,
            Duration::from_millis(100),
            1.0,
            Envelope {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.0,
            },
        );
        let count = source.count();
        assert_eq!(count, 4410);
    }

    #[test]
    fn pitch_ratio() {
        let data = make_test_sample();
        let source = SamplerSource::new(
            data,
            523.25,
            Duration::from_millis(100),
            1.0,
            Envelope {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.0,
            },
        );
        let expected_step = (44100.0_f64 / 44100.0) * (523.25_f64 / 261.63);
        assert!((source.step - expected_step).abs() < 0.01);
    }
}
