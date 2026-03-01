use eframe::egui::{self, FontId, RichText};

use super::{COLOR_MODE_SETTINGS, COLOR_TEXT, COLOR_TEXT_DIM};

pub fn settings_row(ui: &mut egui::Ui, label: &str, value: &str, active: bool) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{label:<10}"))
                .font(FontId::monospace(13.0))
                .color(COLOR_TEXT),
        );

        let arrow_color = if active {
            COLOR_TEXT_DIM
        } else {
            egui::Color32::TRANSPARENT
        };
        let value_color = if active {
            COLOR_MODE_SETTINGS
        } else {
            COLOR_TEXT
        };

        ui.label(
            RichText::new("◄")
                .font(FontId::monospace(12.0))
                .color(arrow_color),
        );
        ui.label(
            RichText::new(format!("{value:^9}"))
                .font(FontId::monospace(13.0))
                .color(value_color)
                .strong(),
        );
        ui.label(
            RichText::new("►")
                .font(FontId::monospace(12.0))
                .color(arrow_color),
        );
    });
}
