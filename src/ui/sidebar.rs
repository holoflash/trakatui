use eframe::egui::{self, Stroke};

use crate::app::App;

use super::{COLOR_LAYOUT_BG_DARK, instrument};

pub fn draw_sidebar(ctx: &egui::Context, app: &mut App) {
    if !app.show_sidebar {
        return;
    }
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
            egui::ScrollArea::vertical()
                .id_salt("sidebar_scroll")
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    instrument::draw_instrument(ui, app);
                    instrument::draw_instrument_list(ui, app);
                });
        });
}
