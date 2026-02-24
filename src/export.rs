use std::path::Path;
use std::time::Duration;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::pattern::{Cell, Pattern};
use crate::synth::{CHANNEL_INSTRUMENTS, SynthSource};

const SAMPLE_RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 16;

pub fn export_wav(
    pattern: &Pattern,
    bpm: u16,
    path: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_duration_secs = 60.0 / bpm as f64 / 4.0;
    let step_duration = Duration::from_secs_f64(step_duration_secs);
    let samples_per_step = (step_duration_secs * SAMPLE_RATE as f64) as usize;

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: SampleFormat::Int,
    };

    let mut writer = WavWriter::create(path, spec)?;

    for row in 0..pattern.rows {
        let mut row_buffer = vec![0.0_f32; samples_per_step];

        for ch in 0..pattern.channels {
            if let Cell::NoteOn(note) = pattern.get(ch, row) {
                let waveform = CHANNEL_INSTRUMENTS[ch % CHANNEL_INSTRUMENTS.len()];
                let source = SynthSource::new(waveform, note.frequency(), step_duration, 0.3);

                for (i, sample) in source.enumerate() {
                    if i >= samples_per_step {
                        break;
                    }
                    row_buffer[i] += sample;
                }
            }
        }

        for &sample in &row_buffer {
            let clamped = sample.clamp(-1.0, 1.0);
            let value = (clamped * i16::MAX as f32) as i16;
            writer.write_sample(value)?;
        }
    }

    writer.finalize()?;
    Ok(())
}
