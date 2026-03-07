use eframe::egui::{self, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::{App, Mode, SynthSettingsField};
use crate::project::{SampleData, VolEnvelope};

use super::widgets::settings_row;
use super::{
    COLOR_LAYOUT_BG_DARK, COLOR_LAYOUT_BG_PANEL, COLOR_ACCENT, COLOR_PATTERN_CURSOR_BG,
    COLOR_PATTERN_CURSOR_TEXT, COLOR_TEXT, COLOR_TEXT_DIM,
};

pub fn draw_instrument(ui: &mut egui::Ui, app: &mut App) {
    handle_sample_drop(ui, app);

    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let inst_idx = app.current_instrument;
            let synth_active = app.mode == Mode::SynthEdit;

            ui.label(
                RichText::new("Instrument")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_ACCENT)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    ui.cursor().left_top() + Vec2::new(ui.available_width(), 0.0),
                ],
                Stroke::new(1.0, COLOR_TEXT_DIM),
            );
            ui.add_space(8.0);

            settings_row(
                ui,
                "Instrument",
                &format!("{:02X}", inst_idx),
                synth_active && app.synth_field == SynthSettingsField::Instrument,
            );
            ui.add_space(6.0);

            settings_row(
                ui,
                "Envelope",
                if app.project.instruments[inst_idx].vol_envelope.enabled {
                    "On"
                } else {
                    "Off"
                },
                synth_active && app.synth_field == SynthSettingsField::Envelope,
            );
            ui.add_space(6.0);

            let mut inst_name = app.project.instruments[inst_idx].name.clone();
            let te_has_focus = ui
                .horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{:<10}", "Name"))
                            .font(FontId::monospace(13.0))
                            .color(COLOR_TEXT),
                    );
                    ui.add_space(8.0);
                    let te = egui::TextEdit::singleline(&mut inst_name)
                        .font(FontId::monospace(12.0))
                        .text_color(COLOR_TEXT)
                        .desired_width(ui.available_width())
                        .frame(false);
                    ui.add(te).has_focus()
                })
                .inner;
            app.text_editing = te_has_focus;
            app.project.instruments[inst_idx].name = inst_name;
            ui.add_space(6.0);

            let cs = &app.project.instruments[inst_idx];

            settings_row(
                ui,
                "Loop",
                cs.sample_data.loop_type.label(),
                synth_active && app.synth_field == SynthSettingsField::LoopType,
            );
            ui.add_space(6.0);

            settings_row(
                ui,
                "Loop Start",
                &format!("{}", cs.sample_data.loop_start),
                synth_active && app.synth_field == SynthSettingsField::LoopStart,
            );
            ui.add_space(6.0);

            settings_row(
                ui,
                "Loop Len",
                &format!("{}", cs.sample_data.loop_length),
                synth_active && app.synth_field == SynthSettingsField::LoopLength,
            );
            ui.add_space(6.0);

            draw_waveform_preview(ui, &cs.sample_data.samples_i16);
            ui.add_space(6.0);

            draw_envelope_preview(ui, &cs.vol_envelope);
            ui.add_space(6.0);

            settings_row(
                ui,
                "Env Points",
                &format!("{}", cs.vol_envelope.points.len()),
                synth_active && app.synth_field == SynthSettingsField::EnvPoints,
            );
            ui.add_space(6.0);

            let pt_idx = app
                .envelope_point_idx
                .min(cs.vol_envelope.points.len().saturating_sub(1));
            settings_row(
                ui,
                "Env Point",
                &format!("{}", pt_idx),
                synth_active && app.synth_field == SynthSettingsField::EnvPoint,
            );
            ui.add_space(6.0);

            if let Some(&(tick, val)) = cs.vol_envelope.points.get(pt_idx) {
                settings_row(
                    ui,
                    "  Tick",
                    &format!("{}", tick),
                    synth_active && app.synth_field == SynthSettingsField::EnvTick,
                );
                ui.add_space(6.0);

                settings_row(
                    ui,
                    "  Value",
                    &format!("{}", val),
                    synth_active && app.synth_field == SynthSettingsField::EnvValue,
                );
                ui.add_space(6.0);
            }

            let sustain_str = match cs.vol_envelope.sustain_point {
                Some(sp) => format!("{}", sp),
                None => "Off".to_string(),
            };
            settings_row(
                ui,
                "Sustain Pt",
                &sustain_str,
                synth_active && app.synth_field == SynthSettingsField::EnvSustain,
            );
            ui.add_space(6.0);

            let (loop_start_str, loop_end_str) = match cs.vol_envelope.loop_range {
                Some((ls, le)) => (format!("{}", ls), format!("{}", le)),
                None => ("Off".to_string(), "Off".to_string()),
            };
            settings_row(
                ui,
                "Loop Start",
                &loop_start_str,
                synth_active && app.synth_field == SynthSettingsField::EnvLoopStart,
            );
            ui.add_space(6.0);

            settings_row(
                ui,
                "Loop End",
                &loop_end_str,
                synth_active && app.synth_field == SynthSettingsField::EnvLoopEnd,
            );
            ui.add_space(6.0);

            settings_row(
                ui,
                "Fadeout",
                &format!("{}", cs.vol_fadeout),
                synth_active && app.synth_field == SynthSettingsField::Fadeout,
            );
            ui.add_space(6.0);
            let vib_type_name = match cs.vibrato_type {
                0 => "Sine",
                1 => "Square",
                2 => "RampDn",
                3 => "RampUp",
                _ => "?",
            };
            settings_row(
                ui,
                "Vib Type",
                vib_type_name,
                synth_active && app.synth_field == SynthSettingsField::VibratoType,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Vib Sweep",
                &format!("{}", cs.vibrato_sweep),
                synth_active && app.synth_field == SynthSettingsField::VibratoSweep,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Vib Depth",
                &format!("{}", cs.vibrato_depth),
                synth_active && app.synth_field == SynthSettingsField::VibratoDepth,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Vib Rate",
                &format!("{}", cs.vibrato_rate),
                synth_active && app.synth_field == SynthSettingsField::VibratoRate,
            );
        });
}

fn draw_waveform_preview(ui: &mut egui::Ui, samples: &[i16]) {
    let width = ui.available_width();
    let height = 32.0;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, super::COLOR_LAYOUT_BG_DARK);

    let center_y = rect.center().y;
    painter.line_segment(
        [
            Pos2::new(rect.left(), center_y),
            Pos2::new(rect.right(), center_y),
        ],
        Stroke::new(0.5, COLOR_TEXT_DIM),
    );

    if samples.is_empty() {
        return;
    }

    let num_points = width as usize;
    let samples_per_point = samples.len() / num_points.max(1);

    if samples_per_point == 0 {
        return;
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

    let waveform_color = super::COLOR_ACCENT;
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

fn draw_envelope_preview(ui: &mut egui::Ui, env: &VolEnvelope) {
    let width = ui.available_width();
    let height = 60.0;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(width, height), egui::Sense::hover());
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
    let points_pos: Vec<Pos2> = env.points.iter().map(|&(t, v)| to_pos(t, v)).collect();
    for window in points_pos.windows(2) {
        painter.line_segment([window[0], window[1]], Stroke::new(1.5, line_color));
    }

    let dot_color = COLOR_TEXT;
    for (i, &pos) in points_pos.iter().enumerate() {
        let r = if Some(i) == env.sustain_point {
            3.5
        } else {
            2.5
        };
        painter.circle_filled(pos, r, dot_color);
    }
}

pub fn draw_instrument_list(ui: &mut egui::Ui, app: &mut App) {
    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.set_min_height(ui.available_height());

            ui.label(
                RichText::new("Instruments")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_ACCENT)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    ui.cursor().left_top() + Vec2::new(ui.available_width(), 0.0),
                ],
                Stroke::new(1.0, COLOR_TEXT_DIM),
            );
            ui.add_space(4.0);

            egui::ScrollArea::vertical().show(ui, |ui| {
                for (i, inst) in app.project.instruments.iter().enumerate() {
                    let is_current = i == app.current_instrument;
                    let label = format!("{:02X}: {}", i, inst.name);

                    let (bg, fg) = if is_current {
                        (COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT)
                    } else {
                        (egui::Color32::TRANSPARENT, COLOR_TEXT_DIM)
                    };

                    let (rect, response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width(), 14.0),
                        egui::Sense::click(),
                    );

                    if bg != egui::Color32::TRANSPARENT {
                        ui.painter().rect_filled(rect, 2.0, bg);
                    }

                    if response.hovered() && !is_current {
                        ui.painter().rect_filled(
                            rect,
                            2.0,
                            egui::Color32::from_rgba_premultiplied(80, 70, 90, 40),
                        );
                    }

                    ui.painter().text(
                        Pos2::new(rect.left() + 4.0, rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        &label,
                        FontId::monospace(11.0),
                        fg,
                    );

                    if response.clicked() {
                        app.current_instrument = i;
                    }
                }
            });
        });
}
