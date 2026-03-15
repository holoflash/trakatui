use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rodio::Source;
use serde::{Deserialize, Serialize};

const MAX_SAMPLE_LEN: usize = 220_500;
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
    pub samples_f32_right: Vec<f32>,
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

    pub fn load_from_path(path: &Path) -> Result<Arc<Self>, String> {
        let file = File::open(path).map_err(|e| format!("Failed to open file: {e}"))?;
        let reader = BufReader::new(file);
        let decoder =
            rodio::Decoder::new(reader).map_err(|e| format!("Failed to decode audio: {e}"))?;

        let sample_rate = decoder.sample_rate().get();
        let channels = decoder.channels().get() as usize;

        let raw: Vec<f32> = decoder.collect();

        let (left, right) = if channels == 1 {
            (raw.clone(), raw)
        } else {
            let mut l = Vec::with_capacity(raw.len() / channels);
            let mut r = Vec::with_capacity(raw.len() / channels);
            for frame in raw.chunks_exact(channels) {
                l.push(frame[0]);
                r.push(frame[1 % channels]);
            }
            (l, r)
        };

        let threshold: f32 = 0.01;
        let find_start = |buf: &[f32]| buf.iter().position(|&s| s.abs() >= threshold).unwrap_or(0);
        let find_end = |buf: &[f32], fallback: usize| {
            buf.iter().rposition(|&s| s.abs() >= threshold).map_or(fallback, |p| p + 1)
        };
        let start = find_start(&left).min(find_start(&right));
        let end = find_end(&left, start).max(find_end(&right, start));

        let cap = |buf: &[f32]| -> Vec<f32> {
            let trimmed = &buf[start..end.min(buf.len())];
            if trimmed.len() > MAX_SAMPLE_LEN {
                trimmed[..MAX_SAMPLE_LEN].to_vec()
            } else {
                trimmed.to_vec()
            }
        };
        let mut samples_f32 = cap(&left);
        let mut samples_f32_right = cap(&right);

        let fade_len = 64.min(samples_f32.len());
        for i in 0..fade_len {
            let gain = 1.0 - i as f32 / fade_len as f32;
            let idx = samples_f32.len() - fade_len + i;
            samples_f32[idx] *= gain;
            samples_f32_right[idx] *= gain;
        }

        let samples_i16: Vec<i16> = samples_f32
            .iter()
            .zip(samples_f32_right.iter())
            .map(|(&l, &r)| {
                let mono = (l + r) * 0.5;
                (mono.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16
            })
            .collect();

        let total_len = samples_f32.len();
        Ok(Arc::new(Self {
            samples_i16,
            samples_f32,
            samples_f32_right,
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
        let samples_f32_right = samples_f32.clone();
        let len = samples_f32.len();
        Arc::new(Self {
            samples_i16,
            samples_f32,
            samples_f32_right,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_data_basics() {
        let data = SampleData {
            samples_i16: vec![0i16; 44100],
            samples_f32: vec![0.0f32; 44100],
            samples_f32_right: vec![0.0f32; 44100],
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
        assert_eq!(data.samples_f32_right.len(), 44100);
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
