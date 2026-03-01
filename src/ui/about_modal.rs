use eframe::egui::{self, FontId, RichText};

use crate::app::App;

use super::{
    COLOR_LAYOUT_BG_PANEL, COLOR_LAYOUT_BORDER_ACTIVE, COLOR_MODE_PLAYING, COLOR_TEXT_ACTIVE,
    COLOR_TEXT_DIM,
};

const VERSION: &str = env!("CARGO_PKG_VERSION");
const ISSUES_URL: &str = "https://github.com/holoflash/psikat/issues";

pub fn draw_about_modal(ctx: &egui::Context, app: &mut App) {
    if !app.show_about_modal {
        return;
    }

    egui::Window::new("")
        .open(&mut app.show_about_modal)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([300.0, 260.0])
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .stroke(egui::Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE))
                .inner_margin(egui::Margin::same(16))
                .corner_radius(4.0),
        )
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add(
                    egui::Image::new(egui::include_image!("../../website/psikat_full.png"))
                        .fit_to_exact_size(egui::Vec2::new(200.0, 200.0))
                        .texture_options(egui::TextureOptions::NEAREST),
                );

                ui.add_space(12.0);

                ui.label(
                    RichText::new(format!("v{VERSION}"))
                        .font(FontId::monospace(14.0))
                        .color(COLOR_MODE_PLAYING)
                        .strong(),
                );

                ui.add_space(12.0);

                if ui
                    .link(
                        RichText::new("Report an issue")
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_ACTIVE),
                    )
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab(ISSUES_URL));
                }

                ui.add_space(4.0);

                ui.label(
                    RichText::new("Made by holoflash")
                        .font(FontId::monospace(10.0))
                        .color(COLOR_TEXT_DIM),
                );
            });
        });
}
