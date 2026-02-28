use eframe::egui::{self, FontId, RichText};

use crate::app::App;

use super::*;

pub fn draw_footer(ctx: &egui::Context, app: &App) {
    let _ = app;
    egui::TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 6))
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new("1:pattern  2:synth  3:settings  Z..P:note  TAB:off  DEL:clear  ,/.:oct  ENTER:play/stop SPACE:play from cursor")
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_DIM),
                    );
                });
            });
        });
}
