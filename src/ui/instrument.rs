use std::sync::Arc;

use eframe::egui::{self, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::App;
use crate::project::SampleData;
use crate::project::sample::LoopType;

use super::{
    COLOR_ACCENT, COLOR_LAYOUT_BG_DARK, COLOR_LAYOUT_BG_PANEL, COLOR_PATTERN_PLAYBACK_TEXT,
    COLOR_TEXT, COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM,
};

fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(format!("{:<11}", label))
            .font(FontId::monospace(12.0))
            .color(COLOR_TEXT),
    );
}

fn separator(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.painter().line_segment(
        [
            ui.cursor().left_top(),
            ui.cursor().left_top() + Vec2::new(ui.available_width(), 0.0),
        ],
        Stroke::new(1.0, COLOR_TEXT_DIM),
    );
    ui.add_space(6.0);
}

fn toggle_checkbox(ui: &mut egui::Ui, checked: &mut bool) {
    let size = Vec2::new(14.0, 14.0);
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        2.0,
        if *checked {
            COLOR_PATTERN_PLAYBACK_TEXT
        } else {
            egui::Color32::TRANSPARENT
        },
    );
    painter.rect_stroke(
        rect,
        2.0,
        Stroke::new(1.0, COLOR_TEXT_DIM),
        egui::StrokeKind::Outside,
    );
    if response.clicked() {
        *checked = !*checked;
    }
}

pub fn draw_instrument(ui: &mut egui::Ui, app: &mut App) {
    handle_sample_drop(ui, app);

    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(ui.available_height());
            let inst_idx = app.current_instrument;
            let selected_label = format!(
                "{:02X}: {}",
                inst_idx + 1,
                app.project.instruments[inst_idx].name
            );
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("instrument_combo")
                    .selected_text(RichText::new(&selected_label).font(FontId::monospace(12.0)))
                    .width(ui.available_width() - 4.0)
                    .show_ui(ui, |ui| {
                        for (i, inst) in app.project.instruments.iter().enumerate() {
                            let label = format!("{:02X}: {}", i + 1, inst.name);
                            let color = if i == inst_idx {
                                COLOR_ACCENT
                            } else {
                                COLOR_TEXT_ACTIVE
                            };
                            if ui
                                .selectable_value(
                                    &mut app.current_instrument,
                                    i,
                                    RichText::new(label).color(color),
                                )
                                .changed()
                            {
                                app.envelope_point_idx = 0;
                            }
                        }
                        ui.separator();
                        if ui.button("+ Add new").clicked() {
                            let idx = app.project.instruments.len();
                            let name = format!("Instrument {:02X}", idx + 1);
                            app.project
                                .instruments
                                .push(crate::project::channel::Instrument::new_empty(&name));
                            app.current_instrument = idx;
                            app.envelope_point_idx = 0;
                        }
                    });
            });

            ui.add_space(12.0);

            {
                let samples = app.project.instruments[inst_idx]
                    .sample_data
                    .samples_i16
                    .clone();
                let resp = draw_waveform_preview(ui, &samples);
                if resp.double_clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Audio", &["wav", "WAV"])
                        .pick_file()
                    {
                        if let Ok(data) = SampleData::load_from_path(&path) {
                            app.project.instruments[inst_idx].sample_data = data;
                            app.project.instruments[inst_idx].waveform =
                                crate::project::channel::WaveformKind::Sample;
                            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                app.project.instruments[inst_idx].name = stem.to_string();
                            }
                        }
                    }
                }
                ui.add_space(6.0);
            }

            let mut inst_name = app.project.instruments[inst_idx].name.clone();
            let te_has_focus = ui
                .horizontal(|ui| {
                    field_label(ui, "NAME");
                    let te = egui::TextEdit::singleline(&mut inst_name)
                        .font(FontId::monospace(12.0))
                        .desired_width(160.0);
                    ui.add(te).has_focus()
                })
                .inner;
            app.text_editing = te_has_focus;
            app.project.instruments[inst_idx].name = inst_name;
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                field_label(ui, "VOLUME");
                let mut vol_hex =
                    (app.project.instruments[inst_idx].default_volume * 255.0).round() as u32;
                let r = ui
                    .add(
                        egui::DragValue::new(&mut vol_hex)
                            .range(0..=255)
                            .speed(0.2)
                            .custom_formatter(|v, _| format!("{:02X}", v as u32))
                            .custom_parser(|s| {
                                u32::from_str_radix(s.trim(), 16).ok().map(|v| v as f64)
                            }),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                app.project.instruments[inst_idx].default_volume =
                    (vol_hex as f32 / 255.0).clamp(0.0, 1.0);
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                use crate::project::channel::WaveformKind;
                field_label(ui, "WAVEFORM");
                let current = app.project.instruments[inst_idx].waveform;
                let mut selected = current;
                egui::ComboBox::from_id_salt("waveform_combo")
                    .selected_text(RichText::new(current.label()).font(FontId::monospace(12.0)))
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for &kind in WaveformKind::ALL {
                            ui.selectable_value(
                                &mut selected,
                                kind,
                                RichText::new(kind.label()).color(if kind == current {
                                    COLOR_ACCENT
                                } else {
                                    COLOR_TEXT
                                }),
                            );
                        }
                    });
                if selected != current {
                    app.project.instruments[inst_idx].waveform = selected;
                    app.project.instruments[inst_idx].sample_data = selected.generate();
                }
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                field_label(ui, "LOOP");
                let sd = &app.project.instruments[inst_idx].sample_data;
                let current_type = sd.loop_type;
                let mut selected_idx = match current_type {
                    LoopType::None => 0usize,
                    LoopType::Forward => 1,
                    LoopType::PingPong => 2,
                };
                let labels = ["Off", "Forward", "Ping-Pong"];
                egui::ComboBox::from_id_salt("loop_type_combo")
                    .selected_text(
                        RichText::new(labels[selected_idx]).font(FontId::monospace(12.0)),
                    )
                    .width(100.0)
                    .show_ui(ui, |ui| {
                        let cur = selected_idx;
                        for (i, label) in labels.iter().enumerate() {
                            ui.selectable_value(
                                &mut selected_idx,
                                i,
                                RichText::new(*label).color(if i == cur {
                                    COLOR_ACCENT
                                } else {
                                    COLOR_TEXT
                                }),
                            );
                        }
                    });
                let new_type = match selected_idx {
                    0 => LoopType::None,
                    1 => LoopType::Forward,
                    _ => LoopType::PingPong,
                };
                if new_type != current_type {
                    let sd = Arc::make_mut(&mut app.project.instruments[inst_idx].sample_data);
                    sd.loop_type = new_type;
                }
            });
            ui.add_space(4.0);

            {
                let sample_len = app.project.instruments[inst_idx]
                    .sample_data
                    .samples_f32
                    .len();
                let max_start = sample_len
                    .saturating_sub(app.project.instruments[inst_idx].sample_data.loop_length);
                let max_len = sample_len
                    .saturating_sub(app.project.instruments[inst_idx].sample_data.loop_start);

                ui.horizontal(|ui| {
                    field_label(ui, "LOOP START");
                    let sd = Arc::make_mut(&mut app.project.instruments[inst_idx].sample_data);
                    let mut v = sd.loop_start as f64;
                    let r = ui
                        .add(
                            egui::DragValue::new(&mut v)
                                .range(0..=max_start)
                                .speed((sample_len as f64 / 500.0).max(1.0)),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    sd.loop_start = v as usize;
                });
                ui.add_space(2.0);

                ui.horizontal(|ui| {
                    field_label(ui, "LOOP LEN");
                    let sd = Arc::make_mut(&mut app.project.instruments[inst_idx].sample_data);
                    let mut v = sd.loop_length as f64;
                    let r = ui
                        .add(
                            egui::DragValue::new(&mut v)
                                .range(0..=max_len)
                                .speed((sample_len as f64 / 500.0).max(1.0)),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    sd.loop_length = v as usize;
                });
            }

            ui.add_space(12.0);
            separator(ui);
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                field_label(ui, "ENVELOPE");
                let mut enabled = app.project.instruments[inst_idx].vol_envelope.enabled;
                toggle_checkbox(ui, &mut enabled);
                if enabled != app.project.instruments[inst_idx].vol_envelope.enabled {
                    app.project.instruments[inst_idx].vol_envelope.enabled = enabled;
                    if enabled && app.project.instruments[inst_idx].vol_envelope.points.len() < 2 {
                        app.project.instruments[inst_idx].vol_envelope.points =
                            vec![(0, 64), (16, 48), (96, 0)];
                        app.project.instruments[inst_idx].vol_envelope.sustain_point = Some(1);
                    }
                }
            });
            if app.project.instruments[inst_idx].vol_envelope.enabled {
                ui.add_space(6.0);
                draw_envelope_preview(ui, app, inst_idx);
                ui.add_space(6.0);

                ui.add_space(4.0);

                let num_points = app.project.instruments[inst_idx].vol_envelope.points.len();
                ui.horizontal(|ui| {
                    field_label(ui, "POINTS");
                    let mut v = num_points as f64;
                    let r = ui
                        .add(egui::DragValue::new(&mut v).range(2..=32).speed(0.15))
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    if r.changed() {
                        let new_len = v as usize;
                        let env = &mut app.project.instruments[inst_idx].vol_envelope;
                        while env.points.len() < new_len {
                            let last_tick = env.points.last().map(|p| p.0).unwrap_or(0);
                            env.points.push((last_tick + 16, 32));
                        }
                        while env.points.len() > new_len && env.points.len() > 2 {
                            env.points.pop();
                        }
                        if app.envelope_point_idx >= env.points.len() {
                            app.envelope_point_idx = env.points.len().saturating_sub(1);
                        }
                        if let Some(sp) = env.sustain_point {
                            if sp >= env.points.len() {
                                env.sustain_point = None;
                            }
                        }
                        if let Some((ls, le)) = env.loop_range {
                            if ls >= env.points.len() || le >= env.points.len() {
                                env.loop_range = None;
                            }
                        }
                    }
                });
                ui.add_space(2.0);

                let max_pt = num_points.saturating_sub(1);
                ui.horizontal(|ui| {
                    field_label(ui, "POINT");
                    let mut v = app.envelope_point_idx as f64;
                    let r = ui
                        .add(egui::DragValue::new(&mut v).range(0..=max_pt).speed(0.1))
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    app.envelope_point_idx = v as usize;
                });
                ui.add_space(2.0);

                let pt_idx = app.envelope_point_idx.min(max_pt);
                if pt_idx < app.project.instruments[inst_idx].vol_envelope.points.len() {
                    let env = &mut app.project.instruments[inst_idx].vol_envelope;

                    let min_tick = if pt_idx > 0 {
                        env.points[pt_idx - 1].0 + 1
                    } else {
                        0
                    };
                    let max_tick = if pt_idx + 1 < env.points.len() {
                        env.points[pt_idx + 1].0 - 1
                    } else {
                        9999
                    };

                    ui.horizontal(|ui| {
                        field_label(ui, "TICK");
                        let mut v = env.points[pt_idx].0 as f64;
                        let r = ui
                            .add(
                                egui::DragValue::new(&mut v)
                                    .range(min_tick..=max_tick)
                                    .speed(0.3),
                            )
                            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                        if r.has_focus() {
                            app.text_editing = true;
                        }
                        env.points[pt_idx].0 = v as u16;
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        field_label(ui, "VALUE");
                        let mut v = env.points[pt_idx].1 as f64;
                        let r = ui
                            .add(egui::DragValue::new(&mut v).range(0..=64).speed(0.15))
                            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                        if r.has_focus() {
                            app.text_editing = true;
                        }
                        env.points[pt_idx].1 = v as u16;
                    });
                    ui.add_space(2.0);
                }

                {
                    let env = &mut app.project.instruments[inst_idx].vol_envelope;
                    let max_idx = env.points.len().saturating_sub(1);

                    ui.horizontal(|ui| {
                        field_label(ui, "SUSTAIN PT");
                        let mut has_sustain = env.sustain_point.is_some();
                        toggle_checkbox(ui, &mut has_sustain);
                        if has_sustain {
                            if env.sustain_point.is_none() {
                                env.sustain_point = Some(0);
                            }
                            let mut v = env.sustain_point.unwrap() as f64;
                            let r = ui
                                .add(egui::DragValue::new(&mut v).range(0..=max_idx).speed(0.1))
                                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                            if r.has_focus() {
                                app.text_editing = true;
                            }
                            env.sustain_point = Some(v as usize);
                        } else {
                            env.sustain_point = None;
                        }
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        field_label(ui, "LOOP START");
                        let mut has_loop = env.loop_range.is_some();
                        toggle_checkbox(ui, &mut has_loop);
                        if has_loop {
                            if env.loop_range.is_none() {
                                env.loop_range = Some((0, max_idx));
                            }
                            let mut v = env.loop_range.unwrap().0 as f64;
                            let r = ui
                                .add(egui::DragValue::new(&mut v).range(0..=max_idx).speed(0.1))
                                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                            if r.has_focus() {
                                app.text_editing = true;
                            }
                            let e = env.loop_range.unwrap().1;
                            let s = (v as usize).min(e).min(max_idx);
                            env.loop_range = Some((s, e));
                        } else {
                            env.loop_range = None;
                        }
                    });
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        field_label(ui, "LOOP END");
                        let mut has_loop = env.loop_range.is_some();
                        toggle_checkbox(ui, &mut has_loop);
                        if has_loop {
                            if env.loop_range.is_none() {
                                env.loop_range = Some((0, max_idx));
                            }
                            let mut v = env.loop_range.unwrap().1 as f64;
                            let r = ui
                                .add(egui::DragValue::new(&mut v).range(0..=max_idx).speed(0.1))
                                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                            if r.has_focus() {
                                app.text_editing = true;
                            }
                            let s = env.loop_range.unwrap().0;
                            let e = (v as usize).max(s).min(max_idx);
                            env.loop_range = Some((s, e));
                        } else {
                            env.loop_range = None;
                        }
                    });
                }

                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    field_label(ui, "FADEOUT");
                    let mut v = app.project.instruments[inst_idx].vol_fadeout as f64;
                    let r = ui
                        .add(egui::DragValue::new(&mut v).range(0..=4095).speed(2.0))
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    app.project.instruments[inst_idx].vol_fadeout = v as u16;
                });
            }

            ui.add_space(12.0);
            separator(ui);
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                field_label(ui, "VIB TYPE");
                let vib_labels = ["Sine", "Square", "RampDn", "RampUp"];
                let mut vib_type = app.project.instruments[inst_idx].vibrato_type as usize;
                egui::ComboBox::from_id_salt("vibrato_type_combo")
                    .selected_text(
                        RichText::new(vib_labels[vib_type.min(3)]).font(FontId::monospace(12.0)),
                    )
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        let cur = vib_type;
                        for (i, label) in vib_labels.iter().enumerate() {
                            ui.selectable_value(
                                &mut vib_type,
                                i,
                                RichText::new(*label).color(if i == cur {
                                    COLOR_ACCENT
                                } else {
                                    COLOR_TEXT
                                }),
                            );
                        }
                    });
                app.project.instruments[inst_idx].vibrato_type = vib_type as u8;
            });
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                field_label(ui, "VIB SWEEP");
                let mut v = app.project.instruments[inst_idx].vibrato_sweep as f64;
                let r = ui
                    .add(egui::DragValue::new(&mut v).range(0..=255).speed(0.3))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                app.project.instruments[inst_idx].vibrato_sweep = v as u8;
            });
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                field_label(ui, "VIB DEPTH");
                let mut v = app.project.instruments[inst_idx].vibrato_depth as f64;
                let r = ui
                    .add(egui::DragValue::new(&mut v).range(0..=15).speed(0.1))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                app.project.instruments[inst_idx].vibrato_depth = v as u8;
            });
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                field_label(ui, "VIB RATE");
                let mut v = app.project.instruments[inst_idx].vibrato_rate as f64;
                let r = ui
                    .add(egui::DragValue::new(&mut v).range(0..=63).speed(0.15))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                app.project.instruments[inst_idx].vibrato_rate = v as u8;
            });
        });
}

fn draw_waveform_preview(ui: &mut egui::Ui, samples: &[i16]) -> egui::Response {
    let width = ui.available_width();
    let height = 32.0;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, COLOR_LAYOUT_BG_DARK);

    let center_y = rect.center().y;
    painter.line_segment(
        [
            Pos2::new(rect.left(), center_y),
            Pos2::new(rect.right(), center_y),
        ],
        Stroke::new(0.5, COLOR_TEXT_DIM),
    );

    if samples.is_empty() {
        return response;
    }

    let num_points = width as usize;
    let samples_per_point = samples.len() / num_points.max(1);

    if samples_per_point == 0 {
        return response;
    }

    let points: Vec<Pos2> = (0..num_points)
        .map(|i| {
            let start = i * samples_per_point;
            let end = (start + samples_per_point).min(samples.len());
            let peak = samples[start..end]
                .iter()
                .map(|&s| i32::from(s).abs())
                .max()
                .unwrap_or(0);
            let normalized = peak as f32 / f32::from(i16::MAX);
            let x = rect.left() + (i as f32 / num_points as f32) * width;
            let y = center_y - normalized * (height * 0.45);
            Pos2::new(x, y)
        })
        .collect();

    let waveform_color = COLOR_ACCENT;
    for window in points.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(1.0, waveform_color));
    }

    let points_bottom: Vec<Pos2> = points
        .iter()
        .map(|p| Pos2::new(p.x, center_y + (center_y - p.y)))
        .collect();
    for window in points_bottom.windows(2) {
        painter.line_segment(
            [window[0], window[1]],
            Stroke::new(1.0, waveform_color.gamma_multiply(0.5)),
        );
    }

    response
}

fn handle_sample_drop(ui: &mut egui::Ui, app: &mut App) {
    let dropped_files: Vec<egui::DroppedFile> = ui.input(|i| i.raw.dropped_files.clone());

    for file in &dropped_files {
        let path = if let Some(ref p) = file.path {
            p.clone()
        } else {
            continue;
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext != "wav" {
            continue;
        }

        let idx = app.current_instrument;

        if let Ok(data) = SampleData::load_from_path(&path) {
            app.project.instruments[idx].sample_data = data;
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                app.project.instruments[idx].name = stem.to_string();
            }
        }
    }
}

fn draw_envelope_preview(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    let width = ui.available_width();
    let height = 120.0;

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 2.0, COLOR_LAYOUT_BG_DARK);

    let grid_color = egui::Color32::from_rgba_premultiplied(80, 70, 90, 40);
    for frac in [0.25, 0.5, 0.75] {
        let y = rect.top() + height * (1.0 - frac);
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5, grid_color),
        );
    }

    let env = &app.project.instruments[inst_idx].vol_envelope;

    if !env.enabled || env.points.len() < 2 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            if env.enabled { "NO POINTS" } else { "DISABLED" },
            FontId::monospace(10.0),
            COLOR_TEXT_DIM,
        );
        return;
    }

    let max_tick = env.points.last().map(|(t, _)| *t).unwrap_or(1).max(1) as f32;
    let margin = 4.0;
    let draw_w = width - margin * 2.0;
    let draw_h = height - margin * 2.0;

    let to_pos = |tick: u16, val: u16| -> Pos2 {
        let x = rect.left() + margin + (tick as f32 / max_tick) * draw_w;
        let y = rect.top() + margin + draw_h * (1.0 - val as f32 / 64.0);
        Pos2::new(x, y)
    };

    let from_pos = |pos: Pos2| -> (u16, u16) {
        let t = ((pos.x - rect.left() - margin) / draw_w * max_tick)
            .round()
            .clamp(0.0, 9999.0) as u16;
        let v = ((1.0 - (pos.y - rect.top() - margin) / draw_h) * 64.0)
            .round()
            .clamp(0.0, 64.0) as u16;
        (t, v)
    };

    let points_pos: Vec<Pos2> = env.points.iter().map(|&(t, v)| to_pos(t, v)).collect();
    let num_points = env.points.len();

    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let mut best = None;
            let mut best_dist = f32::MAX;
            for (i, &pos) in points_pos.iter().enumerate() {
                let d = pos.distance(pointer);
                if d < best_dist {
                    best_dist = d;
                    best = Some(i);
                }
            }
            if best_dist <= 12.0 {
                app.dragging_envelope_point = best;
                if let Some(idx) = best {
                    app.envelope_point_idx = idx;
                }
            }
        }
    }

    if response.dragged() {
        if let Some(idx) = app.dragging_envelope_point {
            if let Some(pointer) = response.interact_pointer_pos() {
                let (raw_tick, raw_val) = from_pos(pointer);

                let env = &app.project.instruments[inst_idx].vol_envelope;

                let min_tick = if idx > 0 {
                    env.points[idx - 1].0 + 1
                } else {
                    0
                };
                let max_tick_pt = if idx + 1 < num_points {
                    env.points[idx + 1].0 - 1
                } else {
                    9999
                };

                let tick = if idx == 0 {
                    0
                } else {
                    raw_tick.clamp(min_tick, max_tick_pt)
                };
                let val = raw_val.min(64);

                app.project.instruments[inst_idx].vol_envelope.points[idx] = (tick, val);
            }
        }
    }

    if response.drag_stopped() {
        app.dragging_envelope_point = None;
    }

    if response.secondary_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let env = &app.project.instruments[inst_idx].vol_envelope;
            if env.points.len() > 2 {
                let pts_pos: Vec<Pos2> = env.points.iter().map(|&(t, v)| to_pos(t, v)).collect();
                if let Some(idx) = pts_pos
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.distance(pointer) <= 8.0)
                    .min_by(|(_, a), (_, b)| {
                        a.distance(pointer)
                            .partial_cmp(&b.distance(pointer))
                            .unwrap()
                    })
                    .map(|(i, _)| i)
                {
                    app.project.instruments[inst_idx]
                        .vol_envelope
                        .points
                        .remove(idx);
                    let new_len = app.project.instruments[inst_idx].vol_envelope.points.len();
                    app.envelope_point_idx = app.envelope_point_idx.min(new_len.saturating_sub(1));

                    if let Some(sp) = app.project.instruments[inst_idx].vol_envelope.sustain_point {
                        if sp == idx {
                            app.project.instruments[inst_idx].vol_envelope.sustain_point = None;
                        } else if sp > idx {
                            app.project.instruments[inst_idx].vol_envelope.sustain_point =
                                Some(sp - 1);
                        }
                    }
                    if let Some((ls, le)) =
                        app.project.instruments[inst_idx].vol_envelope.loop_range
                    {
                        let new_ls = if ls > idx { ls - 1 } else { ls };
                        let new_le = if le > idx { le - 1 } else { le };
                        if new_ls >= new_len || new_le >= new_len {
                            app.project.instruments[inst_idx].vol_envelope.loop_range = None;
                        } else {
                            app.project.instruments[inst_idx].vol_envelope.loop_range =
                                Some((new_ls, new_le));
                        }
                    }
                }
            }
        }
    }

    if response.double_clicked() {
        if let Some(pointer) = response.interact_pointer_pos() {
            let env = &app.project.instruments[inst_idx].vol_envelope;
            let pts = &env.points;

            let insert_after = pts
                .windows(2)
                .enumerate()
                .find(|(_, w)| {
                    let x0 = to_pos(w[0].0, w[0].1).x;
                    let x1 = to_pos(w[1].0, w[1].1).x;
                    pointer.x >= x0 && pointer.x <= x1
                })
                .map(|(i, _)| i)
                .unwrap_or(pts.len().saturating_sub(1));

            let (raw_tick, raw_val) = from_pos(pointer);
            let env = &app.project.instruments[inst_idx].vol_envelope;
            let min_tick = env.points[insert_after].0 + 1;
            let max_tick = if insert_after + 1 < env.points.len() {
                env.points[insert_after + 1].0.saturating_sub(1)
            } else {
                env.points[insert_after].0 + 16
            };
            let tick = raw_tick.clamp(min_tick, max_tick);

            let new_idx = insert_after + 1;
            app.project.instruments[inst_idx]
                .vol_envelope
                .points
                .insert(new_idx, (tick, raw_val.min(64)));
            app.envelope_point_idx = new_idx;

            if let Some(sp) = app.project.instruments[inst_idx].vol_envelope.sustain_point {
                if sp >= new_idx {
                    app.project.instruments[inst_idx].vol_envelope.sustain_point = Some(sp + 1);
                }
            }
            if let Some((ls, le)) = app.project.instruments[inst_idx].vol_envelope.loop_range {
                let new_ls = if ls >= new_idx { ls + 1 } else { ls };
                let new_le = if le >= new_idx { le + 1 } else { le };
                app.project.instruments[inst_idx].vol_envelope.loop_range = Some((new_ls, new_le));
            }
        }
    }

    let env = &app.project.instruments[inst_idx].vol_envelope;
    let points_pos: Vec<Pos2> = env.points.iter().map(|&(t, v)| to_pos(t, v)).collect();

    if let Some((ls, le)) = env.loop_range
        && ls < env.points.len()
        && le < env.points.len()
    {
        let x0 = to_pos(env.points[ls].0, 0).x;
        let x1 = to_pos(env.points[le].0, 0).x;
        let loop_rect =
            egui::Rect::from_min_max(Pos2::new(x0, rect.top()), Pos2::new(x1, rect.bottom()));
        painter.rect_filled(
            loop_rect,
            0.0,
            egui::Color32::from_rgba_premultiplied(120, 100, 60, 25),
        );
    }

    if let Some(si) = env.sustain_point
        && si < env.points.len()
    {
        let x = to_pos(env.points[si].0, 0).x;
        let dash_color = egui::Color32::from_rgba_premultiplied(200, 180, 120, 80);
        let mut y = rect.top();
        while y < rect.bottom() {
            let y_end = (y + 3.0).min(rect.bottom());
            painter.line_segment(
                [Pos2::new(x, y), Pos2::new(x, y_end)],
                Stroke::new(1.0, dash_color),
            );
            y += 6.0;
        }
    }

    let line_color = COLOR_ACCENT;
    for window in points_pos.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(1.5, line_color));
    }

    let mut hovered_point = None;
    if app.dragging_envelope_point.is_none() {
        if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
            if rect.contains(pointer) {
                for (i, &pos) in points_pos.iter().enumerate() {
                    if pos.distance(pointer) <= 8.0 {
                        hovered_point = Some(i);
                        break;
                    }
                }
            }
        }
    }

    for (i, &pos) in points_pos.iter().enumerate() {
        let is_selected = i == app.envelope_point_idx;
        let is_dragging = app.dragging_envelope_point == Some(i);
        let is_hovered = hovered_point == Some(i);

        let r = if is_dragging || is_selected {
            4.0
        } else if is_hovered {
            3.5
        } else if Some(i) == env.sustain_point {
            3.5
        } else {
            2.5
        };

        let color = if is_dragging {
            COLOR_ACCENT
        } else if is_selected {
            COLOR_ACCENT
        } else {
            COLOR_TEXT
        };

        painter.circle_filled(pos, r, color);
    }

    if app.dragging_envelope_point.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if hovered_point.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
}
