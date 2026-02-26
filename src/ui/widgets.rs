use eframe::egui::{self, FontId, RichText};

use super::*;

pub fn settings_row(ui: &mut egui::Ui, label: &str, value: &str, active: bool) {
    ui.horizontal(|ui| {
        let cursor_str = if active { " ▸ " } else { "   " };
        let cursor_color = if active {
            COLOR_MODE_SETTINGS
        } else {
            COLOR_TEXT_DIM
        };
        ui.label(
            RichText::new(cursor_str)
                .font(FontId::monospace(13.0))
                .color(cursor_color)
                .strong(),
        );
        ui.label(
            RichText::new(format!("{:<10}", label))
                .font(FontId::monospace(13.0))
                .color(COLOR_TEXT),
        );
        let centered = format!("{:^9}", value);
        if active {
            ui.label(
                RichText::new("◄")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT_DIM),
            );
            ui.label(
                RichText::new(&centered)
                    .font(FontId::monospace(13.0))
                    .color(COLOR_MODE_SETTINGS)
                    .strong(),
            );
            ui.label(
                RichText::new("►")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT_DIM),
            );
        } else {
            ui.label(RichText::new(" ").font(FontId::monospace(12.0)));
            ui.label(
                RichText::new(&centered)
                    .font(FontId::monospace(13.0))
                    .color(COLOR_TEXT)
                    .strong(),
            );
        }
    });
}
