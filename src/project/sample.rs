use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rodio::Source;

const MAX_SAMPLE_LEN: usize = 131_072;
const INV_I16_MAX: f32 = 1.0 / i16::MAX as f32;

#[derive(Debug, Clone)]
pub struct SampleData {
    pub name: String,
    pub samples_i16: Vec<i16>,
    pub samples_f32: Vec<f32>,
    pub sample_rate: u32,
    pub base_note: u8,
}

impl SampleData {
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

        let name = path
            .file_name()
            .map_or_else(|| "sample".to_string(), |n| n.to_string_lossy().to_string());

        Ok(Arc::new(Self {
            name,
            samples_i16,
            samples_f32,
            sample_rate,
            base_note: 60,
        }))
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
            name: "test.wav".to_string(),
            samples_i16: vec![0i16; 44100],
            samples_f32: vec![0.0f32; 44100],
            sample_rate: 44100,
            base_note: 60,
        };
        assert_eq!(data.samples_i16.len(), 44100);
        assert_eq!(data.samples_f32.len(), 44100);
    }

    #[test]
    fn load_nonexistent_file() {
        let result = SampleData::load_from_path(Path::new("/nonexistent/file.wav"));
        assert!(result.is_err());
    }
}
