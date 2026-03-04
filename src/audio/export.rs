use std::path::Path;

use hound::{SampleFormat, WavSpec, WavWriter};

use super::mixer::{self, SAMPLE_RATE};

pub fn export_wav(
    patterns: &[crate::project::Pattern],
    order: &[usize],
    bpm: u16,
    path: &Path,
    instruments: &[crate::project::Instrument],
    master_volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut source = mixer::export_source(patterns, order, bpm, instruments, master_volume);

    let spec = WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    while let Some(left) = source.next() {
        let right = source.next().unwrap_or(0.0);
        let l = (left.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        let r = (right.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16;
        writer.write_sample(l)?;
        writer.write_sample(r)?;
    }

    writer.finalize()?;
    Ok(())
}
