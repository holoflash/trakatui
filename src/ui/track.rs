use std::sync::Arc;

use eframe::egui::{self, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::{App, WaveformDrag};
use crate::audio::mixer::SCOPE_SIZE;
use crate::project::SampleData;
use crate::project::channel::{FilterType, VolEnvelope};
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

fn section_header(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .font(FontId::monospace(11.0))
            .color(COLOR_TEXT_DIM),
    );
}

pub fn draw_track(ui: &mut egui::Ui, app: &mut App) {
    handle_sample_drop(ui, app);

    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(ui.available_height());
            let inst_idx = app.current_track;
            let selected_label = format!("{:02X}: {}", inst_idx, app.project.tracks[inst_idx].name);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("track_combo")
                    .selected_text(RichText::new(&selected_label).font(FontId::monospace(12.0)))
                    .width(ui.available_width() - 4.0)
                    .show_ui(ui, |ui| {
                        for (i, trk) in app.project.tracks.iter().enumerate() {
                            let label = format!("{:02X}: {}", i, trk.name);
                            let color = if i == inst_idx {
                                COLOR_ACCENT
                            } else {
                                COLOR_TEXT_ACTIVE
                            };
                            if ui
                                .selectable_value(
                                    &mut app.current_track,
                                    i,
                                    RichText::new(label).color(color),
                                )
                                .changed()
                            {
                                app.envelope_point_idx = 0;
                                app.cursor.channel =
                                    i.min(app.project.current_pattern().channels.saturating_sub(1));
                            }
                        }
                        ui.separator();
                        if ui.button("+ Add Track").clicked() {
                            app.project.add_track();
                            let new_idx = app.project.tracks.len() - 1;
                            app.current_track = new_idx;
                            app.cursor.channel = new_idx;
                            app.envelope_point_idx = 0;
                        }
                    });
            });

            if app.project.tracks.len() > 1 {
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Delete Track")
                                    .font(FontId::monospace(11.0))
                                    .color(egui::Color32::from_rgb(200, 130, 120)),
                            )
                            .frame(false),
                        )
                        .clicked()
                    {
                        let idx = app.current_track;
                        app.project.delete_track(idx);
                        if app.current_track >= app.project.tracks.len() {
                            app.current_track = app.project.tracks.len() - 1;
                        }
                        app.cursor.channel = app.current_track;
                        app.envelope_point_idx = 0;
                    }
                });
                ui.add_space(8.0);
            }

            let inst_idx = app.current_track;

            egui::ScrollArea::vertical()
                .id_salt("track_scroll")
                .show(ui, |ui| {
                    ui.add_space(8.0);
                    draw_scope(ui, app, inst_idx);
                    ui.add_space(6.0);
                    draw_interactive_waveform(ui, app, inst_idx);
                    ui.add_space(6.0);

                    draw_basic_fields(ui, app, inst_idx);
                    ui.add_space(4.0);
                    separator(ui);

                    draw_loop_controls(ui, app, inst_idx);
                    ui.add_space(4.0);
                    separator(ui);

                    draw_vol_envelope_section(ui, app, inst_idx);
                    ui.add_space(4.0);
                    separator(ui);

                    draw_pitch_section(ui, app, inst_idx);
                    ui.add_space(4.0);
                    separator(ui);

                    draw_filter_section(ui, app, inst_idx);
                });
        });
}

fn draw_basic_fields(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    let mut inst_name = app.project.tracks[inst_idx].name.clone();
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
    app.project.tracks[inst_idx].name = inst_name;
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "VOLUME");
        let mut vol_hex = (app.project.tracks[inst_idx].default_volume * 255.0).round() as u32;
        let r = ui
            .add(
                egui::DragValue::new(&mut vol_hex)
                    .range(0..=255)
                    .speed(0.2)
                    .custom_formatter(|v, _| format!("{:02X}", v as u32))
                    .custom_parser(|s| u32::from_str_radix(s.trim(), 16).ok().map(|v| v as f64)),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            app.text_editing = true;
        }
        app.project.tracks[inst_idx].default_volume = (vol_hex as f32 / 255.0).clamp(0.0, 1.0);
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "PANNING");
        let mut pan_hex = (app.project.tracks[inst_idx].default_panning * 255.0).round() as u32;
        let r = ui
            .add(
                egui::DragValue::new(&mut pan_hex)
                    .range(0..=255)
                    .speed(0.2)
                    .custom_formatter(|v, _| {
                        let val = v as u32;
                        if val == 0 {
                            "L".to_string()
                        } else if val == 128 {
                            "C".to_string()
                        } else if val == 255 {
                            "R".to_string()
                        } else {
                            format!("{:02X}", val)
                        }
                    })
                    .custom_parser(|s| {
                        let s = s.trim();
                        match s {
                            "L" | "l" => Some(0.0),
                            "C" | "c" => Some(128.0),
                            "R" | "r" => Some(255.0),
                            _ => u32::from_str_radix(s, 16).ok().map(|v| v as f64),
                        }
                    }),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            app.text_editing = true;
        }
        app.project.tracks[inst_idx].default_panning = (pan_hex as f32 / 255.0).clamp(0.0, 1.0);
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        use crate::project::channel::WaveformKind;
        field_label(ui, "WAVEFORM");
        let current = app.project.tracks[inst_idx].waveform;
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
            app.project.tracks[inst_idx].waveform = selected;
            app.project.tracks[inst_idx].sample_data = selected.generate();
        }
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "POLYPHONY");
        let mut poly = app.project.tracks[inst_idx].polyphony;
        let drag = egui::DragValue::new(&mut poly).range(1..=8).speed(0.05);
        let response = ui.add(drag);
        if response.changed() {
            app.project.tracks[inst_idx].polyphony = poly;
            for pat in &mut app.project.patterns {
                if inst_idx < pat.channels {
                    pat.set_voice_count(inst_idx, poly as usize);
                }
            }
        }
    });
}

fn draw_loop_controls(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    ui.horizontal(|ui| {
        field_label(ui, "LOOP");
        let sd = &app.project.tracks[inst_idx].sample_data;
        let current_type = sd.loop_type;
        let mut selected_idx = match current_type {
            LoopType::None => 0usize,
            LoopType::Forward => 1,
            LoopType::PingPong => 2,
        };
        let labels = ["Off", "Forward", "Ping-Pong"];
        egui::ComboBox::from_id_salt("loop_type_combo")
            .selected_text(RichText::new(labels[selected_idx]).font(FontId::monospace(12.0)))
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
            let sd = Arc::make_mut(&mut app.project.tracks[inst_idx].sample_data);
            sd.loop_type = new_type;
            if new_type != LoopType::None && sd.loop_length == 0 {
                sd.loop_start = sd.region_start;
                sd.loop_length = sd.region_end.saturating_sub(sd.region_start);
            }
        }
    });
}

fn draw_scope(ui: &mut egui::Ui, app: &App, inst_idx: usize) {
    let width = ui.available_width();
    let height = 48.0;

    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 2.0, COLOR_LAYOUT_BG_DARK);

    let is_muted = app.muted_channels.get(inst_idx).copied().unwrap_or(false);
    let color = if is_muted {
        egui::Color32::from_rgb(180, 80, 70)
    } else if app.playback.playing {
        COLOR_PATTERN_PLAYBACK_TEXT
    } else {
        COLOR_TEXT_DIM
    };

    if let Some(data) = app.display_scopes.get(inst_idx) {
        let w = rect.width();
        let h = rect.height();
        let mid_y = rect.min.y + h * 0.5;

        let step = SCOPE_SIZE as f32 / w;
        let points: Vec<Pos2> = (0..w as usize)
            .map(|px| {
                let idx = ((px as f32) * step) as usize;
                let sample = data[idx.min(SCOPE_SIZE - 1)];
                let y = mid_y - sample.clamp(-1.0, 1.0) * h * 0.45;
                Pos2::new(rect.min.x + px as f32, y)
            })
            .collect();

        if points.len() >= 2 {
            painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
        }
    }

    let mid_y = rect.min.y + height * 0.5;
    painter.line_segment(
        [
            Pos2::new(rect.left(), mid_y),
            Pos2::new(rect.right(), mid_y),
        ],
        Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(80, 70, 90, 40)),
    );
}

fn draw_interactive_waveform(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    let width = ui.available_width();
    let height = 120.0;

    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 2.0, COLOR_LAYOUT_BG_DARK);

    let sd = &app.project.tracks[inst_idx].sample_data;
    let samples = &sd.samples_i16;
    let total_len = samples.len();

    if total_len == 0 {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Double-click or drag a WAV to load",
            FontId::monospace(10.0),
            COLOR_TEXT_DIM,
        );
        if response.double_clicked() {
            load_sample_dialog(app, inst_idx);
        }
        return;
    }

    let center_y = rect.center().y;
    painter.line_segment(
        [
            Pos2::new(rect.left(), center_y),
            Pos2::new(rect.right(), center_y),
        ],
        Stroke::new(0.5, egui::Color32::from_rgba_premultiplied(80, 70, 90, 60)),
    );

    let num_points = width as usize;
    let samples_per_point = total_len / num_points.max(1);
    if samples_per_point > 0 {
        let waveform_color = egui::Color32::from_rgba_premultiplied(160, 145, 100, 100);
        for i in 0..num_points {
            let start = i * samples_per_point;
            let end = (start + samples_per_point).min(total_len);
            let mut peak_pos: i32 = 0;
            let mut peak_neg: i32 = 0;
            for &s in &samples[start..end] {
                let v = i32::from(s);
                if v > peak_pos {
                    peak_pos = v;
                }
                if v < peak_neg {
                    peak_neg = v;
                }
            }
            let x = rect.left() + (i as f32 / num_points as f32) * width;
            let h_half = height * 0.45;
            let y_top = center_y - (peak_pos as f32 / f32::from(i16::MAX)) * h_half;
            let y_bot = center_y - (peak_neg as f32 / f32::from(i16::MAX)) * h_half;
            painter.line_segment(
                [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                Stroke::new(1.0, waveform_color),
            );
        }
    }

    let sd = &app.project.tracks[inst_idx].sample_data;
    let region_start = sd.region_start;
    let region_end = if sd.region_end == 0 {
        total_len
    } else {
        sd.region_end
    };

    let sample_to_x = |s: usize| -> f32 { rect.left() + (s as f32 / total_len as f32) * width };
    let x_to_sample = |x: f32| -> usize {
        let frac = ((x - rect.left()) / width).clamp(0.0, 1.0);
        (frac * total_len as f32) as usize
    };

    let rs_x = sample_to_x(region_start);
    let re_x = sample_to_x(region_end);

    let dim_color = egui::Color32::from_rgba_premultiplied(18, 16, 28, 180);
    if region_start > 0 {
        painter.rect_filled(
            egui::Rect::from_min_max(rect.left_top(), Pos2::new(rs_x, rect.bottom())),
            0.0,
            dim_color,
        );
    }
    if region_end < total_len {
        painter.rect_filled(
            egui::Rect::from_min_max(Pos2::new(re_x, rect.top()), rect.right_bottom()),
            0.0,
            dim_color,
        );
    }

    let region_rect =
        egui::Rect::from_min_max(Pos2::new(rs_x, rect.top()), Pos2::new(re_x, rect.bottom()));
    painter.rect_stroke(
        region_rect,
        0.0,
        Stroke::new(1.0, COLOR_ACCENT),
        egui::StrokeKind::Outside,
    );

    let handle_color = COLOR_ACCENT;
    let handle_w = 3.0;

    painter.rect_filled(
        egui::Rect::from_center_size(
            Pos2::new(rs_x, rect.center().y),
            Vec2::new(handle_w, height),
        ),
        1.0,
        handle_color,
    );
    painter.rect_filled(
        egui::Rect::from_center_size(
            Pos2::new(re_x, rect.center().y),
            Vec2::new(handle_w, height),
        ),
        1.0,
        handle_color,
    );

    let handle_grab_radius = 8.0;

    if response.drag_started() {
        let origin = ui.input(|i| i.pointer.press_origin());
        if let Some(pointer) = origin {
            let candidates = [
                ((pointer.x - rs_x).abs(), WaveformDrag::RegionStart),
                ((pointer.x - re_x).abs(), WaveformDrag::RegionEnd),
            ];
            if let Some((dist, drag_kind)) = candidates
                .iter()
                .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
                && *dist <= handle_grab_radius
            {
                app.dragging_waveform = Some(*drag_kind);
            }
        }
    }

    if response.dragged()
        && let Some(drag_kind) = app.dragging_waveform
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let sample_pos = x_to_sample(pointer.x);
        let sd = Arc::make_mut(&mut app.project.tracks[inst_idx].sample_data);
        let total = sd.samples_i16.len();
        match drag_kind {
            WaveformDrag::RegionStart => {
                let new_start = sample_pos.min(sd.region_end.saturating_sub(64));
                sd.region_start = new_start;
            }
            WaveformDrag::RegionEnd => {
                let new_end = sample_pos.clamp(sd.region_start + 64, total);
                sd.region_end = new_end;
            }
        }
    }

    if response.drag_stopped() {
        app.dragging_waveform = None;
    }

    if app.dragging_waveform.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
    } else if let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
        && rect.contains(pointer)
    {
        let near_handle = [(pointer.x - rs_x).abs(), (pointer.x - re_x).abs()]
            .iter()
            .any(|d| *d <= handle_grab_radius);
        if near_handle {
            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
        }
    }

    if response.double_clicked() {
        load_sample_dialog(app, inst_idx);
    }
}

fn load_sample_dialog(app: &mut App, inst_idx: usize) {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Audio", &["wav", "WAV"])
        .pick_file()
        && let Ok(data) = SampleData::load_from_path(&path)
    {
        app.project.tracks[inst_idx].sample_data = data;
        app.project.tracks[inst_idx].waveform = crate::project::channel::WaveformKind::Sample;
        if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
            app.project.tracks[inst_idx].name = stem.to_string();
        }
    }
}

fn draw_vol_envelope_section(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        field_label(ui, "ENVELOPE");
        let mut enabled = app.project.tracks[inst_idx].vol_envelope.enabled;
        toggle_checkbox(ui, &mut enabled);
        if enabled != app.project.tracks[inst_idx].vol_envelope.enabled {
            app.project.tracks[inst_idx].vol_envelope.enabled = enabled;
            if enabled && app.project.tracks[inst_idx].vol_envelope.points.len() < 2 {
                app.project.tracks[inst_idx].vol_envelope.points = vec![(0, 64), (16, 48), (96, 0)];
                app.project.tracks[inst_idx].vol_envelope.sustain_point = Some(1);
            }
        }
    });

    if app.project.tracks[inst_idx].vol_envelope.enabled {
        ui.add_space(6.0);

        draw_envelope_editor(
            ui,
            &mut app.project.tracks[inst_idx].vol_envelope,
            &mut app.envelope_point_idx,
            &mut app.dragging_envelope_point,
            &mut app.text_editing,
            "vol_env",
        );

        ui.add_space(8.0);
        ui.horizontal(|ui| {
            field_label(ui, "FADEOUT");
            let mut v = app.project.tracks[inst_idx].vol_fadeout as f64;
            let r = ui
                .add(egui::DragValue::new(&mut v).range(0..=4095).speed(2.0))
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if r.has_focus() {
                app.text_editing = true;
            }
            app.project.tracks[inst_idx].vol_fadeout = v as u16;
        });
    }
}

fn draw_pitch_section(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    section_header(ui, "PITCH");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "COARSE");
        let mut v = app.project.tracks[inst_idx].coarse_tune as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(-48..=48)
                    .speed(0.15)
                    .suffix(" st"),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            app.text_editing = true;
        }
        app.project.tracks[inst_idx].coarse_tune = v as i8;
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "FINE");
        let mut v = app.project.tracks[inst_idx].fine_tune as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(-100..=100)
                    .speed(0.3)
                    .suffix(" ct"),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            app.text_editing = true;
        }
        app.project.tracks[inst_idx].fine_tune = v as i8;
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "PITCH ENV");
        let mut enabled = app.project.tracks[inst_idx].pitch_env_enabled;
        toggle_checkbox(ui, &mut enabled);
        if enabled != app.project.tracks[inst_idx].pitch_env_enabled {
            app.project.tracks[inst_idx].pitch_env_enabled = enabled;
            if enabled && app.project.tracks[inst_idx].pitch_envelope.points.len() < 2 {
                app.project.tracks[inst_idx].pitch_envelope.points =
                    vec![(0, 64), (16, 32), (48, 32)];
                app.project.tracks[inst_idx].pitch_envelope.sustain_point = Some(2);
                app.project.tracks[inst_idx].pitch_envelope.enabled = true;
            }
        }
    });

    if app.project.tracks[inst_idx].pitch_env_enabled {
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            field_label(ui, "DEPTH");
            let mut v = app.project.tracks[inst_idx].pitch_env_depth as f64;
            let r = ui
                .add(
                    egui::DragValue::new(&mut v)
                        .range(0.0..=48.0)
                        .speed(0.15)
                        .suffix(" st"),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if r.has_focus() {
                app.text_editing = true;
            }
            app.project.tracks[inst_idx].pitch_env_depth = v as f32;
        });
        ui.add_space(6.0);

        draw_envelope_editor(
            ui,
            &mut app.project.tracks[inst_idx].pitch_envelope,
            &mut app.pitch_envelope_point_idx,
            &mut app.dragging_pitch_env_point,
            &mut app.text_editing,
            "pitch_env",
        );
    }
}

fn draw_filter_section(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    section_header(ui, "FILTER");
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "FILTER");
        let mut enabled = app.project.tracks[inst_idx].filter.enabled;
        toggle_checkbox(ui, &mut enabled);
        app.project.tracks[inst_idx].filter.enabled = enabled;
    });

    if app.project.tracks[inst_idx].filter.enabled {
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            field_label(ui, "TYPE");
            let current = app.project.tracks[inst_idx].filter.filter_type;
            let mut selected = current;
            egui::ComboBox::from_id_salt("filter_type_combo")
                .selected_text(RichText::new(current.label()).font(FontId::monospace(12.0)))
                .width(80.0)
                .show_ui(ui, |ui| {
                    for &kind in FilterType::ALL {
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
            app.project.tracks[inst_idx].filter.filter_type = selected;
        });
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            field_label(ui, "CUTOFF");
            let mut v = app.project.tracks[inst_idx].filter.cutoff as f64;
            let r = ui
                .add(
                    egui::DragValue::new(&mut v)
                        .range(20.0..=20000.0)
                        .speed(20.0)
                        .suffix(" Hz"),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if r.has_focus() {
                app.text_editing = true;
            }
            app.project.tracks[inst_idx].filter.cutoff = v as f32;
        });
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            field_label(ui, "RESONANCE");
            let mut v = (app.project.tracks[inst_idx].filter.resonance * 100.0) as f64;
            let r = ui
                .add(
                    egui::DragValue::new(&mut v)
                        .range(0.0..=100.0)
                        .speed(0.3)
                        .suffix("%"),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if r.has_focus() {
                app.text_editing = true;
            }
            app.project.tracks[inst_idx].filter.resonance = (v as f32 / 100.0).clamp(0.0, 1.0);
        });
        ui.add_space(2.0);

        ui.horizontal(|ui| {
            field_label(ui, "ENV DEPTH");
            let mut v = (app.project.tracks[inst_idx].filter.env_depth * 100.0) as f64;
            let r = ui
                .add(
                    egui::DragValue::new(&mut v)
                        .range(-100.0..=100.0)
                        .speed(0.3)
                        .suffix("%"),
                )
                .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
            if r.has_focus() {
                app.text_editing = true;
            }
            app.project.tracks[inst_idx].filter.env_depth = (v as f32 / 100.0).clamp(-1.0, 1.0);
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            field_label(ui, "FILT ENV");
            let mut env_enabled = app.project.tracks[inst_idx].filter.envelope.enabled;
            toggle_checkbox(ui, &mut env_enabled);
            if env_enabled != app.project.tracks[inst_idx].filter.envelope.enabled {
                app.project.tracks[inst_idx].filter.envelope.enabled = env_enabled;
                if env_enabled && app.project.tracks[inst_idx].filter.envelope.points.len() < 2 {
                    app.project.tracks[inst_idx].filter.envelope.points =
                        vec![(0, 64), (16, 48), (96, 0)];
                    app.project.tracks[inst_idx].filter.envelope.sustain_point = Some(1);
                }
            }
        });

        if app.project.tracks[inst_idx].filter.envelope.enabled {
            ui.add_space(6.0);

            draw_envelope_editor(
                ui,
                &mut app.project.tracks[inst_idx].filter.envelope,
                &mut app.filter_envelope_point_idx,
                &mut app.dragging_filter_env_point,
                &mut app.text_editing,
                "filter_env",
            );
        }
    }
}

fn draw_envelope_editor(
    ui: &mut egui::Ui,
    env: &mut VolEnvelope,
    point_idx: &mut usize,
    dragging_point: &mut Option<usize>,
    text_editing: &mut bool,
    _id_salt: &str,
) {
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

    if response.drag_started()
        && let Some(pointer) = response.interact_pointer_pos()
    {
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
            *dragging_point = best;
            if let Some(idx) = best {
                *point_idx = idx;
            }
        }
    }

    if response.dragged()
        && let Some(idx) = *dragging_point
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let (raw_tick, raw_val) = from_pos(pointer);

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

        env.points[idx] = (tick, val);
    }

    if response.drag_stopped() {
        *dragging_point = None;
    }

    if response.secondary_clicked()
        && let Some(pointer) = response.interact_pointer_pos()
        && env.points.len() > 2
    {
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
            env.points.remove(idx);
            let new_len = env.points.len();
            *point_idx = (*point_idx).min(new_len.saturating_sub(1));

            if let Some(sp) = env.sustain_point {
                if sp == idx {
                    env.sustain_point = None;
                } else if sp > idx {
                    env.sustain_point = Some(sp - 1);
                }
            }
            if let Some((ls, le)) = env.loop_range {
                let new_ls = if ls > idx { ls - 1 } else { ls };
                let new_le = if le > idx { le - 1 } else { le };
                if new_ls >= new_len || new_le >= new_len {
                    env.loop_range = None;
                } else {
                    env.loop_range = Some((new_ls, new_le));
                }
            }
        }
    }

    if response.double_clicked()
        && let Some(pointer) = response.interact_pointer_pos()
    {
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
        let min_tick = env.points[insert_after].0 + 1;
        let max_tick = if insert_after + 1 < env.points.len() {
            env.points[insert_after + 1].0.saturating_sub(1)
        } else {
            env.points[insert_after].0 + 16
        };
        let tick = raw_tick.clamp(min_tick, max_tick);

        let new_idx = insert_after + 1;
        env.points.insert(new_idx, (tick, raw_val.min(64)));
        *point_idx = new_idx;

        if let Some(sp) = env.sustain_point
            && sp >= new_idx
        {
            env.sustain_point = Some(sp + 1);
        }
        if let Some((ls, le)) = env.loop_range {
            let new_ls = if ls >= new_idx { ls + 1 } else { ls };
            let new_le = if le >= new_idx { le + 1 } else { le };
            env.loop_range = Some((new_ls, new_le));
        }
    }

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
    if dragging_point.is_none()
        && let Some(pointer) = ui.input(|i| i.pointer.hover_pos())
        && rect.contains(pointer)
    {
        for (i, &pos) in points_pos.iter().enumerate() {
            if pos.distance(pointer) <= 8.0 {
                hovered_point = Some(i);
                break;
            }
        }
    }

    for (i, &pos) in points_pos.iter().enumerate() {
        let is_selected = i == *point_idx;
        let is_dragging = *dragging_point == Some(i);
        let is_hovered = hovered_point == Some(i);

        let r = if is_dragging || is_selected {
            4.0
        } else if is_hovered || Some(i) == env.sustain_point {
            3.5
        } else {
            2.5
        };

        let color = if is_dragging || is_selected {
            COLOR_ACCENT
        } else {
            COLOR_TEXT
        };

        painter.circle_filled(pos, r, color);
    }

    if dragging_point.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
    } else if hovered_point.is_some() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }

    ui.add_space(4.0);

    let num_points = env.points.len();
    ui.horizontal(|ui| {
        field_label(ui, "POINTS");
        let mut v = num_points as f64;
        let r = ui
            .add(egui::DragValue::new(&mut v).range(2..=32).speed(0.15))
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        if r.changed() {
            let new_len = v as usize;
            while env.points.len() < new_len {
                let last_tick = env.points.last().map(|p| p.0).unwrap_or(0);
                env.points.push((last_tick + 16, 32));
            }
            while env.points.len() > new_len && env.points.len() > 2 {
                env.points.pop();
            }
            if *point_idx >= env.points.len() {
                *point_idx = env.points.len().saturating_sub(1);
            }
            if let Some(sp) = env.sustain_point
                && sp >= env.points.len()
            {
                env.sustain_point = None;
            }
            if let Some((ls, le)) = env.loop_range
                && (ls >= env.points.len() || le >= env.points.len())
            {
                env.loop_range = None;
            }
        }
    });
    ui.add_space(2.0);

    let max_pt = num_points.saturating_sub(1);
    ui.horizontal(|ui| {
        field_label(ui, "POINT");
        let mut v = *point_idx as f64;
        let r = ui
            .add(egui::DragValue::new(&mut v).range(0..=max_pt).speed(0.1))
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        *point_idx = v as usize;
    });
    ui.add_space(2.0);

    let pt_idx = (*point_idx).min(max_pt);
    if pt_idx < env.points.len() {
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
                *text_editing = true;
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
                *text_editing = true;
            }
            env.points[pt_idx].1 = v as u16;
        });
        ui.add_space(2.0);
    }

    {
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
                    *text_editing = true;
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
                    *text_editing = true;
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
                    *text_editing = true;
                }
                let s = env.loop_range.unwrap().0;
                let e = (v as usize).max(s).min(max_idx);
                env.loop_range = Some((s, e));
            } else {
                env.loop_range = None;
            }
        });
    }
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

        let idx = app.current_track;

        if let Ok(data) = SampleData::load_from_path(&path) {
            app.project.tracks[idx].sample_data = data;
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                app.project.tracks[idx].name = stem.to_string();
            }
        }
    }
}
