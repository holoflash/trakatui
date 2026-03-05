use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Arc;

use crate::app::scale::ScaleIndex;
use crate::project::Project;
use crate::project::channel::{Instrument, VolEnvelope};
use crate::project::pattern::{Cell, Effect, Note, Pattern};
use crate::project::sample::{LoopType, SampleData};

const PERIOD_TABLE: [(u16, u8); 36] = [
    (856, 13),
    (808, 14),
    (762, 15),
    (720, 16),
    (678, 17),
    (640, 18),
    (604, 19),
    (570, 20),
    (538, 21),
    (508, 22),
    (480, 23),
    (453, 24),
    (428, 25),
    (404, 26),
    (381, 27),
    (360, 28),
    (339, 29),
    (320, 30),
    (302, 31),
    (285, 32),
    (269, 33),
    (254, 34),
    (240, 35),
    (226, 36),
    (214, 37),
    (202, 38),
    (190, 39),
    (180, 40),
    (170, 41),
    (160, 42),
    (151, 43),
    (143, 44),
    (135, 45),
    (127, 46),
    (120, 47),
    (113, 48),
];

fn period_to_pitch(period: u16) -> Option<u8> {
    if period == 0 {
        return None;
    }
    let mut best_pitch = None;
    let mut best_dist = u16::MAX;
    for &(p, pitch) in &PERIOD_TABLE {
        let dist = p.abs_diff(period);
        if dist < best_dist {
            best_dist = dist;
            best_pitch = Some(pitch);
        }
    }
    best_pitch
}

fn err(cur: &Cursor<&[u8]>, ctx: &str, e: impl std::fmt::Display) -> String {
    format!("{ctx} at offset {:#x}: {e}", cur.position())
}

fn read_u8(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<u8, String> {
    let mut buf = [0u8; 1];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(buf[0])
}

fn read_u16_be(cur: &mut Cursor<&[u8]>, ctx: &str) -> Result<u16, String> {
    let mut buf = [0u8; 2];
    cur.read_exact(&mut buf).map_err(|e| err(cur, ctx, e))?;
    Ok(u16::from_be_bytes(buf))
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

struct ModSampleHeader {
    name: String,
    length: u32,
    #[allow(dead_code)]
    finetune: i8,
    volume: u8,
    loop_start: u32,
    loop_length: u32,
}

pub fn load_mod(path: &Path) -> Result<Project, String> {
    let data = std::fs::read(path).map_err(|e| format!("Failed to read file: {e}"))?;
    if data.len() < 1084 {
        return Err("File too small for MOD format".into());
    }
    let cur = &mut Cursor::new(data.as_slice());

    let _module_name = read_string(cur, 20, "module name")?;

    let mut sample_headers: Vec<ModSampleHeader> = Vec::with_capacity(31);
    for i in 0..31 {
        let name = read_string(cur, 22, &format!("sample {i} name"))?;
        let length = read_u16_be(cur, &format!("sample {i} length"))? as u32 * 2;
        let finetune_raw = read_u8(cur, &format!("sample {i} finetune"))? & 0x0F;
        let finetune = if finetune_raw > 7 {
            finetune_raw as i8 - 16
        } else {
            finetune_raw as i8
        };
        let volume = read_u8(cur, &format!("sample {i} volume"))?;
        let loop_start = read_u16_be(cur, &format!("sample {i} loop start"))? as u32 * 2;
        let loop_length = read_u16_be(cur, &format!("sample {i} loop length"))? as u32 * 2;

        sample_headers.push(ModSampleHeader {
            name,
            length,
            finetune,
            volume,
            loop_start,
            loop_length,
        });
    }

    let song_length = read_u8(cur, "song length")? as usize;
    let _restart_pos = read_u8(cur, "restart position")?;

    let order_table = read_bytes(cur, 128, "order table")?;
    let order: Vec<usize> = order_table[..song_length]
        .iter()
        .map(|&x| x as usize)
        .collect();

    let magic = read_bytes(cur, 4, "magic ID")?;
    let magic_str = String::from_utf8_lossy(&magic);

    let num_channels: usize = match magic_str.as_ref() {
        "M.K." | "M!K!" | "FLT4" | "4CHN" => 4,
        "6CHN" => 6,
        "8CHN" | "FLT8" | "OCTA" => 8,
        _ => {
            if let Some(ch_str) = magic_str.strip_suffix("CH") {
                ch_str.parse::<usize>().unwrap_or(4)
            } else {
                return Err(format!("Unknown MOD magic: {magic_str}"));
            }
        }
    };

    let num_patterns = order_table.iter().map(|&x| x as usize).max().unwrap_or(0) + 1;

    let mut patterns: Vec<Pattern> = Vec::with_capacity(num_patterns);

    for pat_i in 0..num_patterns {
        let mut pattern = Pattern::new(num_channels, 64);

        for row in 0..64 {
            for ch in 0..num_channels {
                let b = read_bytes(cur, 4, &format!("pat {pat_i} row {row} ch {ch}"))?;

                let sample_num = (b[0] & 0xF0) | (b[2] >> 4);
                let period = u16::from(b[0] & 0x0F) << 8 | u16::from(b[1]);
                let effect_type = b[2] & 0x0F;
                let effect_param = b[3];

                pattern.data[ch][row] = if period > 0 {
                    match period_to_pitch(period) {
                        Some(pitch) => Cell::NoteOn(Note { pitch }),
                        None => Cell::Empty,
                    }
                } else {
                    Cell::Empty
                };

                pattern.instruments[ch][row] = if sample_num > 0 {
                    Some(sample_num - 1)
                } else {
                    None
                };

                pattern.volumes[ch][row] = None;

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

        patterns.push(pattern);
    }

    let mut instruments: Vec<Instrument> = Vec::with_capacity(31);

    for (i, sh) in sample_headers.iter().enumerate() {
        if sh.length <= 2 {
            instruments.push(Instrument {
                name: if sh.name.is_empty() {
                    format!("Sample {:02X}", i + 1)
                } else {
                    sh.name.clone()
                },
                vol_envelope: VolEnvelope::disabled(),
                sample_data: SampleData::silent(),
                default_volume: sh.volume as f32 / 64.0,
                samples: Vec::new(),
                note_to_sample: Vec::new(),
                vol_fadeout: 0,
                default_panning: 0.5,
                vibrato_type: 0,
                vibrato_sweep: 0,
                vibrato_depth: 0,
                vibrato_rate: 0,
            });
            continue;
        }

        let sample_bytes = read_bytes(cur, sh.length as usize, &format!("sample {i} data"))?;

        let samples_i16: Vec<i16> = sample_bytes
            .iter()
            .map(|&b| (b as i8 as i16) * 256)
            .collect();
        let samples_f32: Vec<f32> = samples_i16.iter().map(|&s| s as f32 / 32768.0).collect();

        let loop_type = if sh.loop_length > 2 {
            LoopType::Forward
        } else {
            LoopType::None
        };

        let base_note: u8 = 48;

        let sd = Arc::new(SampleData {
            samples_i16,
            samples_f32,
            sample_rate: 16574,
            base_note,
            loop_start: sh.loop_start as usize,
            loop_length: sh.loop_length as usize,
            loop_type,
        });

        instruments.push(Instrument {
            name: if sh.name.is_empty() {
                format!("Sample {:02X}", i + 1)
            } else {
                sh.name.clone()
            },
            vol_envelope: VolEnvelope::disabled(),
            sample_data: sd,
            default_volume: sh.volume as f32 / 64.0,
            samples: Vec::new(),
            note_to_sample: Vec::new(),
            vol_fadeout: 0,
            default_panning: 0.5,
            vibrato_type: 0,
            vibrato_sweep: 0,
            vibrato_depth: 0,
            vibrato_rate: 0,
        });
    }

    while instruments.len() < 31 {
        instruments.push(Instrument {
            name: String::new(),
            vol_envelope: VolEnvelope::disabled(),
            sample_data: SampleData::silent(),
            default_volume: 0.0,
            samples: Vec::new(),
            note_to_sample: Vec::new(),
            vol_fadeout: 0,
            default_panning: 0.5,
            vibrato_type: 0,
            vibrato_sweep: 0,
            vibrato_depth: 0,
            vibrato_rate: 0,
        });
    }

    let channel_panning: Vec<f32> = (0..num_channels)
        .map(|ch| match ch % 4 {
            0 | 3 => 0.2,
            _ => 0.8,
        })
        .collect();

    Ok(Project {
        patterns,
        order,
        current_order_idx: 0,
        instruments,
        bpm: 125,
        initial_speed: 6,
        subdivision: 4,
        step: 1,
        scale_index: ScaleIndex::default(),
        transpose: 0,
        master_volume_db: 0.0,
        channel_panning,
    })
}
