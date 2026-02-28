use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use hound::{SampleFormat, WavSpec, WavWriter};

use crate::project::{Cell, ChannelSettings, Pattern, parse_pitch_bend};

use super::synth::{PitchBendControl, SynthSource};

const SAMPLE_RATE: u32 = 44100;
const BITS_PER_SAMPLE: u16 = 16;

struct ExportChannel {
    source: Option<SynthSource>,
    bend_control: Arc<PitchBendControl>,
    base_freq: f32,
    note_start_row: Option<usize>,
}

impl ExportChannel {
    fn new() -> Self {
        Self {
            source: None,
            bend_control: Arc::new(PitchBendControl::new()),
            base_freq: 0.0,
            note_start_row: None,
        }
    }
}

pub fn export_wav(
    pattern: &Pattern,
    bpm: u16,
    path: &Path,
    channel_settings: &[ChannelSettings],
    master_volume: f32,
) -> Result<(), Box<dyn std::error::Error>> {
    let step_duration_secs = 60.0 / f64::from(bpm) / 4.0;
    let step_duration = Duration::from_secs_f64(step_duration_secs);
    let samples_per_step = (step_duration_secs * f64::from(SAMPLE_RATE)).round() as usize;
    let total_samples = samples_per_step * pattern.rows;

    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: BITS_PER_SAMPLE,
        sample_format: SampleFormat::Int,
    };

    let mut buffer = vec![0.0_f32; total_samples];

    let mut channels: Vec<ExportChannel> = (0..pattern.channels)
        .map(|_| ExportChannel::new())
        .collect();

    for row in 0..pattern.rows {
        let row_offset = row * samples_per_step;

        for ch in 0..pattern.channels {
            let effect = pattern.get_effect(ch, row);
            let state = &mut channels[ch];

            match pattern.get(ch, row) {
                Cell::NoteOn(note) => {
                    let cs = &channel_settings[ch % channel_settings.len()];
                    let gate_f64 = pattern.gate_rows(ch, row) as f64;
                    let gate_duration = step_duration.mul_f64(gate_f64);
                    let note_duration =
                        gate_duration + Duration::from_secs_f32(cs.envelope.release);

                    let bend = Arc::new(PitchBendControl::new());

                    if let Some(cmd) = effect
                        && let Some((semitones, steps)) = parse_pitch_bend(cmd)
                        && semitones != 0
                        && steps > 0
                    {
                        let target = note.frequency() * (f32::from(semitones) / 12.0).exp2();
                        let dur = step_duration.as_secs_f32() * f32::from(steps);
                        bend.set(target, 0.0, dur);
                    }

                    state.source = Some(SynthSource::new(
                        cs.waveform,
                        note.frequency(),
                        note_duration,
                        cs.volume,
                        cs.envelope,
                        bend.clone(),
                    ));
                    state.bend_control = bend;
                    state.base_freq = note.frequency();
                    state.note_start_row = Some(row);
                }
                Cell::NoteOff => {
                    state.source = None;
                    state.note_start_row = None;
                }
                Cell::Empty => {
                    if let (Some(start_row), Some(cmd)) = (state.note_start_row, effect)
                        && let Some((semitones, steps)) = parse_pitch_bend(cmd)
                    {
                        if semitones != 0 && steps > 0 {
                            let offset = step_duration.as_secs_f32() * (row - start_row) as f32;
                            let target = state.base_freq * (f32::from(semitones) / 12.0).exp2();
                            let dur = step_duration.as_secs_f32() * f32::from(steps);
                            state.bend_control.set(target, offset, dur);
                        } else {
                            state.bend_control.reset();
                        }
                    }
                }
            }
        }

        for state in &mut channels {
            if let Some(ref mut source) = state.source {
                for i in 0..samples_per_step {
                    let pos = row_offset + i;
                    if pos >= total_samples {
                        break;
                    }
                    if let Some(sample) = source.next() {
                        buffer[pos] += sample;
                    } else {
                        break;
                    }
                }
            }
        }
    }

    let mut writer = WavWriter::create(path, spec)?;
    for &sample in &buffer {
        let scaled = sample * master_volume;
        let clamped = scaled.clamp(-1.0, 1.0);
        let value = (clamped * f32::from(i16::MAX)).round() as i16;
        writer.write_sample(value)?;
    }

    writer.finalize()?;
    Ok(())
}
