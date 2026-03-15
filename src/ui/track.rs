use std::sync::Arc;

use eframe::egui::{self, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::{App, WaveformDrag};
use crate::project::SampleData;
use crate::project::channel::FilterType;
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

pub fn draw_track_sidebar(ctx: &egui::Context, app: &mut App) {
    if !app.show_sidebar {
        return;
    }
    egui::SidePanel::right("sidebar")
        .resizable(false)
        .max_width(286.0)
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_DARK)
                .inner_margin(egui::Margin::ZERO)
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;

            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());

                    ui.horizontal(|ui| {
                        if ui
                            .add(egui::Button::new(
                                RichText::new("Add Track")
                                    .font(FontId::monospace(11.0))
                                    .color(COLOR_TEXT_ACTIVE),
                            ))
                            .clicked()
                        {
                            app.project.add_track();
                            let new_idx = app.project.tracks.len() - 1;
                            app.current_track = new_idx;
                            app.cursor.channel = new_idx;
                        }

                        if app.project.tracks.len() > 1
                            && ui
                                .add(egui::Button::new(
                                    RichText::new("Delete Track")
                                        .font(FontId::monospace(11.0))
                                        .color(egui::Color32::from_rgb(200, 130, 120)),
                                ))
                                .clicked()
                        {
                            let idx = app.current_track;
                            app.project.delete_track(idx);
                            if app.current_track >= app.project.tracks.len() {
                                app.current_track = app.project.tracks.len() - 1;
                            }
                            app.cursor.channel = app.current_track;
                        }
                    });
                    ui.add_space(8.0);

                    let inst_idx = app.current_track;
                    let selected_label =
                        format!("{:02X}: {}", inst_idx, app.project.tracks[inst_idx].name);
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("track_combo")
                            .selected_text(
                                RichText::new(&selected_label).font(FontId::monospace(12.0)),
                            )
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
                                        app.cursor.channel =
                                            i.min(app.project.channels.saturating_sub(1));
                                    }
                                }
                            });
                    });
                });

            let inst_idx = app.current_track;

            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 0))
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    ui.set_min_height(ui.available_height());

                    egui::ScrollArea::vertical()
                        .id_salt("track_scroll")
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysHidden,
                        )
                        .show(ui, |ui| {

                            ui.add_space(8.0);
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
                            ui.add_space(8.0);
                        });
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
            if selected == WaveformKind::Sample {
                load_sample_dialog(app, inst_idx);
            } else {
                app.project.tracks[inst_idx].sample_data = selected.generate();
            }
        }
    });
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        field_label(ui, "POLYPHONY");
        let current = app.project.tracks[inst_idx].polyphony;
        for n in 1u8..=8 {
            let selected = n == current;
            let color = if selected {
                COLOR_TEXT_ACTIVE
            } else {
                COLOR_TEXT_DIM
            };
            let btn = ui.selectable_label(
                selected,
                RichText::new(format!("{n}"))
                    .font(FontId::monospace(12.0))
                    .color(color),
            );
            if btn.clicked() && n != current {
                app.project.tracks[inst_idx].polyphony = n;
                for pat in &mut app.project.patterns {
                    if inst_idx < pat.data.len() {
                        pat.set_voice_count(inst_idx, n as usize);
                    }
                }
            }
        }
    });
}

fn draw_loop_controls(ui: &mut egui::Ui, app: &mut App, inst_idx: usize) {
    ui.horizontal(|ui| {
        field_label(ui, "PLAYBACK");
        let sd = &app.project.tracks[inst_idx].sample_data;
        let current_reverse = sd.reverse;
        let mut selected_idx = if current_reverse { 1usize } else { 0 };
        let labels = ["Forward", "Reverse"];
        egui::ComboBox::from_id_salt("playback_dir_combo")
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
        let new_reverse = selected_idx == 1;
        if new_reverse != current_reverse {
            let sd = Arc::make_mut(&mut app.project.tracks[inst_idx].sample_data);
            sd.reverse = new_reverse;
        }
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "LOOP");
        let sd = &app.project.tracks[inst_idx].sample_data;
        let current_type = sd.loop_type;
        let mut selected_idx = match current_type {
            LoopType::None => 0usize,
            LoopType::Forward => 1,
            LoopType::PingPong => 2,
        };
        let labels = ["No Loop", "Forward", "Ping-Pong"];
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
    let is_reversed = sd.reverse;
    if samples_per_point > 0 {
        let waveform_color = egui::Color32::from_rgba_premultiplied(160, 145, 100, 100);
        for i in 0..num_points {
            let src_i = if is_reversed { num_points - 1 - i } else { i };
            let start = src_i * samples_per_point;
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

    let near_drag_handle = app.dragging_waveform.is_some()
        || ui.input(|i| i.pointer.hover_pos()).is_some_and(|pointer| {
            rect.contains(pointer)
                && [(pointer.x - rs_x).abs(), (pointer.x - re_x).abs()]
                    .iter()
                    .any(|d| *d <= handle_grab_radius)
        });

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

    response.context_menu(|ui| {
        let sd = &app.project.tracks[inst_idx].sample_data;
        let total = sd.samples_i16.len();
        let rs = sd.region_start;
        let re = if sd.region_end == 0 {
            total
        } else {
            sd.region_end
        };
        let has_selection = rs > 0 || re < total;
        if ui
            .add_enabled(has_selection, egui::Button::new("Trim to selection"))
            .clicked()
        {
            let sd = Arc::make_mut(&mut app.project.tracks[inst_idx].sample_data);
            let re_clamped = re.min(sd.samples_i16.len());
            let rs_clamped = rs.min(re_clamped);
            sd.samples_i16 = sd.samples_i16[rs_clamped..re_clamped].to_vec();
            sd.samples_f32 = sd.samples_f32[rs_clamped..re_clamped].to_vec();
            sd.samples_f32_right = sd.samples_f32_right[rs_clamped..re_clamped].to_vec();
            let new_len = sd.samples_i16.len();
            if sd.loop_start >= rs_clamped {
                sd.loop_start -= rs_clamped;
            } else {
                sd.loop_start = 0;
            }
            sd.loop_length = sd.loop_length.min(new_len.saturating_sub(sd.loop_start));
            sd.region_start = 0;
            sd.region_end = new_len;
            ui.close();
        }
    });

    if !near_drag_handle {
        response.on_hover_ui(|ui| {
            ui.label("Double-click or drag-and-drop a WAV to load sample");
        });
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
        app.project.tracks[inst_idx].vol_envelope.enabled = enabled;
    });

    if app.project.tracks[inst_idx].vol_envelope.enabled {
        draw_adsr_controls(
            ui,
            &mut app.project.tracks[inst_idx].vol_envelope,
            &mut app.text_editing,
            "vol",
        );
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
            if enabled {
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

        draw_adsr_controls(
            ui,
            &mut app.project.tracks[inst_idx].pitch_envelope,
            &mut app.text_editing,
            "pitch",
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
            app.project.tracks[inst_idx].filter.envelope.enabled = env_enabled;
        });

        if app.project.tracks[inst_idx].filter.envelope.enabled {
            draw_adsr_controls(
                ui,
                &mut app.project.tracks[inst_idx].filter.envelope,
                &mut app.text_editing,
                "filt",
            );
        }
    }
}

fn draw_adsr_controls(
    ui: &mut egui::Ui,
    env: &mut crate::project::channel::AdsrEnvelope,
    text_editing: &mut bool,
    _id_prefix: &str,
) {
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "ATTACK");
        let mut v = env.attack_ms as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(0.0..=5000.0)
                    .speed(1.0)
                    .suffix(" ms")
                    .custom_formatter(|v, _| format!("{:.0}", v)),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        env.attack_ms = v as f32;
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "DECAY");
        let mut v = env.decay_ms as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(0.0..=5000.0)
                    .speed(1.0)
                    .suffix(" ms")
                    .custom_formatter(|v, _| format!("{:.0}", v)),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        env.decay_ms = v as f32;
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "SUSTAIN");
        let mut v = (env.sustain * 100.0) as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(0.0..=100.0)
                    .speed(0.3)
                    .suffix("%")
                    .custom_formatter(|v, _| format!("{:.0}", v)),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        env.sustain = (v as f32 / 100.0).clamp(0.0, 1.0);
    });
    ui.add_space(2.0);

    ui.horizontal(|ui| {
        field_label(ui, "RELEASE");
        let mut v = env.release_ms as f64;
        let r = ui
            .add(
                egui::DragValue::new(&mut v)
                    .range(0.0..=5000.0)
                    .speed(1.0)
                    .suffix(" ms")
                    .custom_formatter(|v, _| format!("{:.0}", v)),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
        if r.has_focus() {
            *text_editing = true;
        }
        env.release_ms = v as f32;
    });
}

