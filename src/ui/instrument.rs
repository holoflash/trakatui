use eframe::egui::{self, FontId, Pos2, RichText, Stroke, Vec2};

use crate::app::{App, Mode, SynthSettingsField};
use crate::project::{SampleData, Waveform};

use super::widgets::settings_row;
use super::{COLOR_LAYOUT_BG_PANEL, COLOR_MODE_SETTINGS, COLOR_TEXT_DIM};

pub fn draw_instrument(ui: &mut egui::Ui, app: &mut App) {
    handle_sample_drop(ui, app);

    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let inst_idx = app.current_instrument;
            let cs = &app.project.instruments[inst_idx];
            let synth_active = app.mode == Mode::SynthEdit;

            ui.label(
                RichText::new("Instrument")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_MODE_SETTINGS)
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
                "Waveform",
                cs.waveform.name(),
                synth_active && app.synth_field == SynthSettingsField::Waveform,
            );
            ui.add_space(6.0);

            let sample_display = if cs.waveform == Waveform::Sampler {
                if let Some(ref sd) = cs.sample_data {
                    truncate_name(&sd.name, 18).to_string()
                } else {
                    "No sample".to_string()
                }
            } else {
                "─".to_string()
            };
            settings_row(
                ui,
                "Sample",
                &sample_display,
                synth_active && app.synth_field == SynthSettingsField::Sample,
            );
            ui.add_space(6.0);

            if cs.waveform == Waveform::Sampler
                && let Some(ref sd) = cs.sample_data
            {
                draw_waveform_preview(ui, &sd.samples_i16);
                ui.add_space(6.0);
            }

            settings_row(
                ui,
                "Attack",
                &format!("{:.3}", cs.envelope.attack),
                synth_active && app.synth_field == SynthSettingsField::Attack,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Decay",
                &format!("{:.3}", cs.envelope.decay),
                synth_active && app.synth_field == SynthSettingsField::Decay,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Sustain",
                &format!("{:.2}", cs.envelope.sustain),
                synth_active && app.synth_field == SynthSettingsField::Sustain,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Release",
                &format!("{:.3}", cs.envelope.release),
                synth_active && app.synth_field == SynthSettingsField::Release,
            );
        });
}

fn truncate_name(name: &str, max_len: usize) -> &str {
    if name.len() <= max_len {
        name
    } else {
        &name[..max_len]
    }
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

    let waveform_color = super::COLOR_MODE_SETTINGS;
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

        if app.project.instruments[idx].waveform != Waveform::Sampler {
            app.project.instruments[idx].waveform = Waveform::Sampler;
        }

        if let Ok(data) = SampleData::load_from_path(&path) {
            app.project.instruments[idx].sample_data = Some(data);
        }
    }
}
