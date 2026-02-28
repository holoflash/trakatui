use eframe::egui::{self, Stroke};

use crate::app::App;

use super::{COLOR_LAYOUT_BG_DARK, commands_ref, instrument, settings_panel};

pub fn draw_sidebar(ctx: &egui::Context, app: &App) {
    egui::SidePanel::right("sidebar")
        .resizable(false)
        .exact_width(280.0)
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_DARK)
                .inner_margin(egui::Margin::ZERO)
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            instrument::draw_instrument(ui, app);
            settings_panel::draw_settings(ui, app);
            commands_ref::draw_commands_ref(ui);
        });
}
