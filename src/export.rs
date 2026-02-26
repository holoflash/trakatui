use std::path::Path;
use std::time::Duration;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::pattern::{Cell, Pattern};
use crate::synth::{ChannelSettings, SynthSource};

const SAMPLE_RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 16;

pub fn export_wav(
    pattern: &Pattern,
    bpm: u16,
    path: &Path,
    channel_settings: &[ChannelSettings],
    master_volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_duration_secs = 60.0 / bpm as f64 / 4.0;
    let step_duration = Duration::from_secs_f64(step_duration_secs);
    let samples_per_step = (step_duration_secs * SAMPLE_RATE as f64) as usize;
    let total_samples = samples_per_step * pattern.rows;

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: SampleFormat::Int,
    };

    let mut buffer = vec![0.0_f32; total_samples];

    for row in 0..pattern.rows {
        let row_offset = row * samples_per_step;
        for ch in 0..pattern.channels {
            if let Cell::NoteOn(note) = pattern.get(ch, row) {
                let cs = &channel_settings[ch % channel_settings.len()];
                let gate = pattern.gate_rows(ch, row);
                let gate_duration = step_duration.mul_f32(gate as f32);
                let note_duration = gate_duration + Duration::from_secs_f32(cs.envelope.release);
                let source = SynthSource::new(
                    cs.waveform,
                    note.frequency(),
                    note_duration,
                    cs.volume,
                    cs.envelope,
                );

                for (i, sample) in source.enumerate() {
                    let pos = row_offset + i;
                    if pos >= total_samples {
                        break;
                    }
                    buffer[pos] += sample;
                }
            }
        }
    }

    let mut writer = WavWriter::create(path, spec)?;
    for &sample in &buffer {
        let scaled = sample * master_volume;
        let clamped = scaled.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(value)?;
    }

    writer.finalize()?;
    Ok(())
}
