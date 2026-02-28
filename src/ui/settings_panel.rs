use eframe::egui::{self, FontId, RichText, Stroke, Vec2};

use crate::app::{App, Mode, SettingsField};

use super::widgets::settings_row;
use super::{
    COLOR_ERROR, COLOR_LAYOUT_BG_PANEL, COLOR_MODE_PLAYING, COLOR_MODE_SETTINGS, COLOR_TEXT_DIM,
};

pub fn draw_settings(ui: &mut egui::Ui, app: &App) {
    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("Settings")
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

            let sa = app.mode == Mode::Settings;
            settings_row(
                ui,
                "Scale",
                app.project.scale_index.scale().name,
                sa && app.settings_field == SettingsField::Scale,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "BPM",
                &app.project.bpm.to_string(),
                sa && app.settings_field == SettingsField::Bpm,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Division",
                &app.project.subdivision.to_string(),
                sa && app.settings_field == SettingsField::Subdivision,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Step",
                &app.project.step.to_string(),
                sa && app.settings_field == SettingsField::Step,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Length",
                &app.project.pattern.rows.to_string(),
                sa && app.settings_field == SettingsField::PatternLength,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Transpose",
                &app.project.transpose.to_string(),
                sa && app.settings_field == SettingsField::Transpose,
            );
            if let Some(ref msg) = app.status_message {
                ui.add_space(8.0);
                let color = if msg.starts_with("Exported") {
                    COLOR_MODE_PLAYING
                } else {
                    COLOR_ERROR
                };
                ui.label(
                    RichText::new(msg.as_str())
                        .font(FontId::monospace(11.0))
                        .color(color),
                );
            }
        });
}
