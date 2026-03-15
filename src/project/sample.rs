use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rodio::Source;
use serde::{Deserialize, Serialize};

const MAX_SAMPLE_LEN: usize = 131_072;
const INV_I16_MAX: f32 = 1.0 / i16::MAX as f32;
const WAVE_LEN: usize = 256;
const WAVE_RATE: u32 = 440 * WAVE_LEN as u32;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoopType {
    None,
    Forward,
    PingPong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SampleData {
    pub samples_i16: Vec<i16>,
    pub samples_f32: Vec<f32>,
    pub sample_rate: u32,
    pub base_note: u8,
    pub loop_type: LoopType,
    pub loop_start: usize,
    pub loop_length: usize,
    pub region_start: usize,
    pub region_end: usize,
    pub reverse: bool,
}
impl SampleData {
    #[inline]
    pub fn loop_end(&self) -> usize {
        self.loop_start + self.loop_length
    }
    pub fn load_from_path(path: &Path) -> Result<Arc<Self>, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
        let reader = BufReader::new(file);
        let decoder =
            rodio::Decoder::new(reader).map_err(|e| format!("Failed to decode audio: {e}"))?;

        let sample_rate = decoder.sample_rate().get();
        let channels = decoder.channels().get() as usize;

        let raw_samples: Vec<i16> = decoder
            .map(|s| (s.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
            .collect();

        let mono: Vec<i16> = if channels == 1 {
            raw_samples
        } else {
            raw_samples
                .chunks_exact(channels)
                .map(|frame| {
                    let sum: i32 = frame.iter().map(|&s| i32::from(s)).sum();
                    (sum / channels as i32) as i16
                })
                .collect()
        };

        let threshold: i16 = 328;
        let trimmed = trim_silence(&mono, threshold);

        let samples_i16: Vec<i16> = if trimmed.len() > MAX_SAMPLE_LEN {
            trimmed[..MAX_SAMPLE_LEN].to_vec()
        } else {
            trimmed.to_vec()
        };

        let samples_f32: Vec<f32> = samples_i16
            .iter()
            .map(|&s| f32::from(s) * INV_I16_MAX)
            .collect();

        let total_len = samples_i16.len();
        Ok(Arc::new(Self {
            samples_i16,
            samples_f32,
            sample_rate,
            base_note: 60,
            loop_type: LoopType::None,
            loop_start: 0,
            loop_length: 0,
            region_start: 0,
            region_end: total_len,
            reverse: false,
        }))
    }

    fn generate(samples_f32: Vec<f32>, looped: bool) -> Arc<Self> {
        let samples_f32: Vec<f32> = samples_f32.iter().map(|&s| s * 0.5).collect();
        let samples_i16: Vec<i16> = samples_f32
            .iter()
            .map(|&s| (s * f32::from(i16::MAX)) as i16)
            .collect();
        let len = samples_f32.len();
        Arc::new(Self {
            samples_i16,
            samples_f32,
            sample_rate: WAVE_RATE,
            base_note: 69,
            loop_type: if looped {
                LoopType::Forward
            } else {
                LoopType::None
            },
            loop_start: 0,
            loop_length: if looped { len } else { 0 },
            region_start: 0,
            region_end: len,
            reverse: false,
        })
    }

    pub fn sine() -> Arc<Self> {
        let data: Vec<f32> = (0..WAVE_LEN)
            .map(|i| {
                let phase = i as f32 / WAVE_LEN as f32;
                (std::f32::consts::TAU * phase).sin()
            })
            .collect();
        Self::generate(data, true)
    }

    pub fn triangle() -> Arc<Self> {
        let data: Vec<f32> = (0..WAVE_LEN)
            .map(|i| {
                let phase = i as f32 / WAVE_LEN as f32;
                4.0f32.mul_add((phase - (phase + 0.5).floor()).abs(), -1.0)
            })
            .collect();
        Self::generate(data, true)
    }

    pub fn square() -> Arc<Self> {
        let data: Vec<f32> = (0..WAVE_LEN)
            .map(|i| {
                let phase = i as f32 / WAVE_LEN as f32;
                if phase < 0.5 { 1.0 } else { -1.0 }
            })
            .collect();
        Self::generate(data, true)
    }

    pub fn saw() -> Arc<Self> {
        let data: Vec<f32> = (0..WAVE_LEN)
            .map(|i| {
                let phase = i as f32 / WAVE_LEN as f32;
                2.0f32.mul_add(phase, -1.0)
            })
            .collect();
        Self::generate(data, true)
    }

    pub fn noise() -> Arc<Self> {
        let data: Vec<f32> = (0..4096)
            .map(|_| fastrand::f32().mul_add(2.0, -1.0))
            .collect();
        Self::generate(data, true)
    }

    pub fn silent() -> Arc<Self> {
        Self::generate(vec![0.0; WAVE_LEN], false)
    }
}

fn trim_silence(samples: &[i16], threshold: i16) -> &[i16] {
    let start = samples
        .iter()
        .position(|&s| s.abs() >= threshold)
        .unwrap_or(0);
    let end = samples
        .iter()
        .rposition(|&s| s.abs() >= threshold)
        .map_or(start, |p| p + 1);
    &samples[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_data_basics() {
        let data = SampleData {
            samples_i16: vec![0i16; 44100],
            samples_f32: vec![0.0f32; 44100],
            sample_rate: 44100,
            base_note: 60,
            loop_type: LoopType::None,
            loop_start: 0,
            loop_length: 0,
            region_start: 0,
            region_end: 44100,
            reverse: false,
        };
        assert_eq!(data.samples_i16.len(), 44100);
        assert_eq!(data.samples_f32.len(), 44100);
    }

    #[test]
    fn load_nonexistent_file() {
        let result = SampleData::load_from_path(Path::new("/nonexistent/file.wav"));
        assert!(result.is_err());
    }

    #[test]
    fn generated_waveforms() {
        let sine = SampleData::sine();
        assert_eq!(sine.samples_f32.len(), WAVE_LEN);
        assert_eq!(sine.loop_length, WAVE_LEN);
        assert!((sine.samples_f32[0]).abs() < 0.01);

        let square = SampleData::square();
        assert!((square.samples_f32[0] - 0.5).abs() < 0.01);
        assert!((square.samples_f32[WAVE_LEN / 2] + 0.5).abs() < 0.01);

        let noise = SampleData::noise();
        assert_eq!(noise.samples_f32.len(), 4096);
        assert_eq!(noise.loop_length, 4096);

        let silent = SampleData::silent();
        assert_eq!(silent.loop_length, 0);
    }
}
