use std::io::{Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::Arc;

use crate::app::scale::ScaleIndex;
use crate::project::Project;
use crate::project::channel::{Instrument, VolEnvelope};
use crate::project::pattern::{Cell, Effect, Note, Pattern};
use crate::project::sample::{LoopType, SampleData};

const XM_HEADER_ID: &[u8] = b"Extended Module: ";

fn err(cur: &Cursor<&[u8]>, ctx: &str, e: impl std::fmt::Display) -> String {
    format!("{ctx} at offset {:#x}: {e}", cur.position())
}

fn read_u8(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<u8, String> {
    let mut buf = [0u8; 1];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(buf[0])
}

fn read_u16(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<u16, String> {
    let mut buf = [0u8; 2];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(u16::from_le_bytes(buf))
}

fn read_u32(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<u32, String> {
    let mut buf = [0u8; 4];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(u32::from_le_bytes(buf))
}

fn read_i8(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<i8, String> {
    Ok(read_u8(cur, ctx)? as i8)
}

fn read_bytes(cur: &mut Cursor<&[u8]>, n: usize, ctx: &str) -> Result<Vec<u8>, String> {
    let mut buf = vec![0u8; n];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(buf)
}

fn read_string(cur: &mut Cursor<&[u8]>, n: usize, ctx: &str) -> Result<String, String> {
    let bytes = read_bytes(cur, n, ctx)?;
    Ok(String::from_utf8_lossy(&bytes)
        .trim_end_matches('\0')
        .trim()
        .to_string())
}

pub fn load_xm(path: &Path) -> Result<Project, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    let file_len = data.len() as u64;
    let cur = &mut Cursor::new(data.as_slice());

    let id = read_bytes(cur, 17, "header ID")?;
    if id != XM_HEADER_ID {
        return Err("Not a valid XM file".into());
    }
    let _module_name = read_string(cur, 20, "module name")?;
    let _0x1a = read_u8(cur, "header 0x1a")?;
    let _tracker_name = read_string(cur, 20, "tracker name")?;
    let version = read_u16(cur, "version")?;
    if version < 0x0104 {
        return Err(format!(
            "XM version {version:#06x} not supported (need >= 0x0104)"
        ));
    }

    let header_start = cur.position();
    let header_size = read_u32(cur, "header size")?;

    let song_length = read_u16(cur, "song length")? as usize;
    let _restart_pos = read_u16(cur, "restart pos")?;
    let num_channels = read_u16(cur, "num channels")? as usize;
    let num_patterns = read_u16(cur, "num patterns")? as usize;
    let num_instruments = read_u16(cur, "num instruments")? as usize;
    let _flags = read_u16(cur, "flags")?;
    let default_speed = read_u16(cur, "default speed")?;
    let default_bpm = read_u16(cur, "default bpm")?;

    let order_table = read_bytes(cur, 256, "order table")?;
    let order: Vec<usize> = order_table[..song_length]
        .iter()
        .map(|&x| x as usize)
        .collect();

    cur.seek(SeekFrom::Start(header_start + u64::from(header_size)))
        .map_err(|e| format!("Seek past header: {e}"))?;

    let mut patterns: Vec<Pattern> = Vec::with_capacity(num_patterns);

    for pat_i in 0..num_patterns {
        let pat_start = cur.position();
        let pat_header_len = read_u32(cur, &format!("pattern {pat_i} header len"))?;
        let _packing_type = read_u8(cur, &format!("pattern {pat_i} packing"))?;
        let num_rows = read_u16(cur, &format!("pattern {pat_i} rows"))? as usize;
        let packed_size = read_u16(cur, &format!("pattern {pat_i} packed size"))?;

        cur.seek(SeekFrom::Start(pat_start + u64::from(pat_header_len)))
            .map_err(|e| format!("Seek past pattern {pat_i} header: {e}"))?;

        let mut pattern = Pattern::new(num_channels, num_rows);

        if packed_size > 0 {
            let pat_data = read_bytes(cur, packed_size as usize, &format!("pattern {pat_i} data"))?;
            let pcur = &mut Cursor::new(pat_data.as_slice());

            for row in 0..num_rows {
                for ch in 0..num_channels {
                    let mut note = 0u8;
                    let mut instrument = 0u8;
                    let mut volume = 0u8;
                    let mut effect_type = 0u8;
                    let mut effect_param = 0u8;

                    let ctx = &format!("pat {pat_i} r{row} ch{ch}");
                    let first = read_u8(pcur, ctx)?;
                    if first & 0x80 != 0 {
                        if first & 0x01 != 0 {
                            note = read_u8(pcur, ctx)?;
                        }
                        if first & 0x02 != 0 {
                            instrument = read_u8(pcur, ctx)?;
                        }
                        if first & 0x04 != 0 {
                            volume = read_u8(pcur, ctx)?;
                        }
                        if first & 0x08 != 0 {
                            effect_type = read_u8(pcur, ctx)?;
                        }
                        if first & 0x10 != 0 {
                            effect_param = read_u8(pcur, ctx)?;
                        }
                    } else {
                        note = first;
                        instrument = read_u8(pcur, ctx)?;
                        volume = read_u8(pcur, ctx)?;
                        effect_type = read_u8(pcur, ctx)?;
                        effect_param = read_u8(pcur, ctx)?;
                    }

                    pattern.data[ch][row] = match note {
                        1..=96 => Cell::NoteOn(Note { pitch: note }),
                        97 => Cell::NoteOff,
                        _ => Cell::Empty,
                    };

                    pattern.instruments[ch][row] = if instrument > 0 {
                        Some(instrument - 1)
                    } else {
                        None
                    };

                    pattern.volumes[ch][row] = if volume >= 0x10 { Some(volume) } else { None };

                    pattern.effects[ch][row] = if effect_type != 0 || effect_param != 0 {
                        Some(Effect {
                            kind: effect_type,
                            param: effect_param,
                        })
                    } else {
                        None
                    };
                }
            }
        }

        patterns.push(pattern);
    }

    let mut instruments: Vec<Instrument> = Vec::with_capacity(num_instruments);

    for inst_i in 0..num_instruments {
        let inst_start = cur.position();
        let inst_header_size = read_u32(cur, &format!("inst {inst_i} header size"))?;
        let inst_name = read_string(cur, 22, &format!("inst {inst_i} name"))?;
        let _inst_type = read_u8(cur, &format!("inst {inst_i} type"))?;
        let num_samples = read_u16(cur, &format!("inst {inst_i} num_samples"))? as usize;

        if num_samples == 0 {
            cur.seek(SeekFrom::Start(inst_start + u64::from(inst_header_size)))
                .map_err(|e| format!("Seek past empty inst {inst_i}: {e}"))?;

            instruments.push(Instrument {
                name: if inst_name.is_empty() {
                    format!("Inst {}", inst_i + 1)
                } else {
                    inst_name
                },
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::silent(),
                default_volume: 1.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            });
            continue;
        }

        let _sample_header_size = read_u32(cur, &format!("inst {inst_i} sample hdr size"))?;
        let note_to_sample = read_bytes(cur, 96, &format!("inst {inst_i} note map"))?;
        let vol_env_raw = read_bytes(cur, 48, &format!("inst {inst_i} vol env"))?;
        let _pan_env_raw = read_bytes(cur, 48, &format!("inst {inst_i} pan env"))?;

        let num_vol_points = read_u8(cur, &format!("inst {inst_i} num vol pts"))?;
        let _num_pan_points = read_u8(cur, "num pan pts")?;
        let vol_sustain = read_u8(cur, "vol sustain")?;
        let vol_loop_start = read_u8(cur, "vol loop start")?;
        let vol_loop_end = read_u8(cur, "vol loop end")?;
        let _pan_sustain = read_u8(cur, "pan sustain")?;
        let _pan_loop_start = read_u8(cur, "pan loop start")?;
        let _pan_loop_end = read_u8(cur, "pan loop end")?;
        let vol_type = read_u8(cur, "vol type")?;
        let _pan_type = read_u8(cur, "pan type")?;
        let vibrato_type = read_u8(cur, "vib type")?;
        let vibrato_sweep = read_u8(cur, "vib sweep")?;
        let vibrato_depth = read_u8(cur, "vib depth")?;
        let vibrato_rate = read_u8(cur, "vib rate")?;
        let vol_fadeout = read_u16(cur, "vol fadeout")?;
        let _reserved = read_u16(cur, "inst reserved")?;

        let vol_env_enabled = vol_type & 1 != 0;
        let vol_env_sustain = vol_type & 2 != 0;
        let vol_env_loop = vol_type & 4 != 0;

        let vol_envelope = if vol_env_enabled && num_vol_points >= 2 {
            let mut points = Vec::with_capacity(num_vol_points as usize);
            for i in 0..num_vol_points.min(12) as usize {
                let offset = i * 4;
                if offset + 3 < vol_env_raw.len() {
                    let tick =
                        u16::from(vol_env_raw[offset]) | (u16::from(vol_env_raw[offset + 1]) << 8);
                    let value = u16::from(vol_env_raw[offset + 2])
                        | (u16::from(vol_env_raw[offset + 3]) << 8);
                    points.push((tick, value.min(64)));
                }
            }

            let sustain_point = if vol_env_sustain {
                Some(vol_sustain as usize)
            } else {
                None
            };

            let loop_range = if vol_env_loop {
                Some((vol_loop_start as usize, vol_loop_end as usize))
            } else {
                None
            };

            VolEnvelope {
                points,
                sustain_point,
                loop_range,
                enabled: true,
            }
        } else {
            VolEnvelope::disabled()
        };

        cur.seek(SeekFrom::Start(inst_start + u64::from(inst_header_size)))
            .map_err(|e| format!("Seek past inst {inst_i} header: {e}"))?;

        let mut sample_headers: Vec<XmSampleHeader> = Vec::with_capacity(num_samples);
        for s in 0..num_samples {
            let ctx = &format!("inst {inst_i} sample {s}");
            let length = read_u32(cur, &format!("{ctx} length"))?;
            let loop_start = read_u32(cur, &format!("{ctx} loop_start"))?;
            let loop_length = read_u32(cur, &format!("{ctx} loop_len"))?;
            let volume = read_u8(cur, &format!("{ctx} vol"))?;
            let finetune = read_i8(cur, &format!("{ctx} finetune"))?;
            let sample_type = read_u8(cur, &format!("{ctx} type"))?;
            let panning = read_u8(cur, &format!("{ctx} pan"))?;
            let relative_note = read_i8(cur, &format!("{ctx} relnote"))?;
            let _reserved = read_u8(cur, &format!("{ctx} reserved"))?;
            let name = read_string(cur, 22, &format!("{ctx} name"))?;

            sample_headers.push(XmSampleHeader {
                length,
                loop_start,
                loop_length,
                volume,
                finetune,
                sample_type,
                _panning: panning,
                relative_note,
                name,
            });
        }

        let mut all_samples: Vec<(Arc<SampleData>, f32)> = Vec::with_capacity(num_samples);

        for (s, sh) in sample_headers.iter().enumerate() {
            if sh.length == 0 {
                all_samples.push((SampleData::silent(), 1.0));
                continue;
            }

            if cur.position() + u64::from(sh.length) > file_len {
                return Err(format!(
                    "inst {inst_i} sample {s}: data length {} exceeds file (pos {:#x}, file len {:#x})",
                    sh.length,
                    cur.position(),
                    file_len
                ));
            }

            let is_16bit = sh.sample_type & 0x10 != 0;
            let loop_type = match sh.sample_type & 0x03 {
                1 => LoopType::Forward,
                2 => LoopType::PingPong,
                _ => LoopType::None,
            };

            let ctx = &format!("inst {inst_i} sample {s} data");
            let (samples_i16, samples_f32) = if is_16bit {
                decode_16bit_sample(cur, sh.length as usize, ctx)?
            } else {
                decode_8bit_sample(cur, sh.length as usize, ctx)?
            };

            let base_note = (60i16 - i16::from(sh.relative_note)).clamp(0, 127) as u8;

            let effective_rate =
                (8363.0_f64 * (f64::from(sh.finetune) / 128.0 / 12.0).exp2()) as u32;

            let (ls, ll) = if is_16bit {
                ((sh.loop_start / 2) as usize, (sh.loop_length / 2) as usize)
            } else {
                (sh.loop_start as usize, sh.loop_length as usize)
            };

            let sd = Arc::new(SampleData {
                name: sh.name.clone(),
                samples_i16,
                samples_f32,
                sample_rate: effective_rate,
                base_note,
                loop_type,
                loop_start: ls,
                loop_length: ll,
            });

            let vol = sh.volume.min(64) as f32 / 64.0;
            all_samples.push((sd, vol));
        }

        let (first_sd, first_vol) = all_samples
            .first()
            .cloned()
            .unwrap_or_else(|| (SampleData::silent(), 1.0));

        instruments.push(Instrument {
            name: if inst_name.is_empty() {
                format!("Inst {}", inst_i + 1)
            } else {
                inst_name
            },
            vol_envelope,
            sample_data: first_sd,
            default_volume: first_vol,
            samples: all_samples,
            note_to_sample,
            vol_fadeout,
            vibrato_type,
            vibrato_sweep,
            vibrato_depth,
            vibrato_rate,
        });
    }

    while instruments.len() < 8 {
        instruments.push(Instrument {
            name: format!("Empty {}", instruments.len() + 1),
            vol_envelope: VolEnvelope::disabled(),
            sample_data: SampleData::silent(),
            default_volume: 1.0,
            samples: Vec::new(),
            note_to_sample: Vec::new(),
            vol_fadeout: 0,
            vibrato_type: 0,
            vibrato_sweep: 0,
            vibrato_depth: 0,
            vibrato_rate: 0,
        });
    }

    Ok(Project {
        patterns,
        order,
        current_order_idx: 0,
        instruments,
        bpm: default_bpm,
        subdivision: default_speed as usize,
        step: 1,
        scale_index: ScaleIndex::default(),
        transpose: 0,
        master_volume_db: 0.0,
    })
}

fn decode_8bit_sample(
    cur: &mut Cursor<&[u8]>,
    byte_len: usize,
    ctx: &str,
) -> Result<(Vec<i16>, Vec<f32>), String> {
    let raw = read_bytes(cur, byte_len, ctx)?;
    let mut samples_i16 = Vec::with_capacity(raw.len());
    let mut samples_f32 = Vec::with_capacity(raw.len());

    let mut old: i8 = 0;
    for &byte in &raw {
        old = old.wrapping_add(byte as i8);
        samples_i16.push(i16::from(old) * 256);
        samples_f32.push(f32::from(old) / 128.0);
    }

    Ok((samples_i16, samples_f32))
}

fn decode_16bit_sample(
    cur: &mut Cursor<&[u8]>,
    byte_len: usize,
    ctx: &str,
) -> Result<(Vec<i16>, Vec<f32>), String> {
    let raw = read_bytes(cur, byte_len, ctx)?;
    let num_samples = raw.len() / 2;
    let mut samples_i16 = Vec::with_capacity(num_samples);
    let mut samples_f32 = Vec::with_capacity(num_samples);

    let mut old: i16 = 0;
    for chunk in raw.chunks_exact(2) {
        let delta = i16::from_le_bytes([chunk[0], chunk[1]]);
        old = old.wrapping_add(delta);
        samples_i16.push(old);
        samples_f32.push(f32::from(old) / 32768.0);
    }

    Ok((samples_i16, samples_f32))
}

struct XmSampleHeader {
    length: u32,
    loop_start: u32,
    loop_length: u32,
    #[allow(dead_code)]
    volume: u8,
    #[allow(dead_code)]
    finetune: i8,
    sample_type: u8,
    #[allow(dead_code)]
    _panning: u8,
    relative_note: i8,
    name: String,
}
