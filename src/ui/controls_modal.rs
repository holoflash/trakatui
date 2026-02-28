use eframe::egui::{self, FontId, RichText};

use crate::app::App;

use super::{
    COLOR_LAYOUT_BG_PANEL, COLOR_LAYOUT_BORDER_ACTIVE, COLOR_MODE_PLAYING, COLOR_TEXT,
    COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM,
};

pub fn draw_controls_modal(ctx: &egui::Context, app: &mut App) {
    if !app.show_controls_modal {
        return;
    }

    egui::Window::new("Controls")
        .open(&mut app.show_controls_modal)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([460.0, 420.0])
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .stroke(egui::Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE))
                .inner_margin(egui::Margin::same(16))
                .corner_radius(4.0),
        )
        .show(ctx, |ui| {
            let category_order = ["Global", "Mode", "Edit", "Settings"];

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 0,
                        right: 16,
                        top: 0,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        for &cat in &category_order {
                            let entries: Vec<_> = app
                                .keybindings
                                .bindings
                                .iter()
                                .filter(|b| b.category == cat)
                                .collect();

                            if entries.is_empty() {
                                continue;
                            }

                            ui.add_space(12.0);
                            ui.label(
                                RichText::new(cat)
                                    .font(FontId::monospace(13.0))
                                    .color(COLOR_MODE_PLAYING)
                                    .strong(),
                            );
                            ui.add_space(4.0);

                            egui::Grid::new(format!("controls_grid_{cat}"))
                                .num_columns(3)
                                .striped(true)
                                .spacing([12.0, 4.0])
                                .show(ui, |ui| {
                                    for binding in &entries {
                                        ui.label(
                                            RichText::new(binding.combo.label())
                                                .font(FontId::monospace(12.0))
                                                .color(COLOR_TEXT_ACTIVE),
                                        );
                                        ui.label(
                                            RichText::new(binding.title)
                                                .font(FontId::monospace(12.0))
                                                .color(COLOR_TEXT),
                                        );
                                        ui.label(
                                            RichText::new(binding.description)
                                                .font(FontId::monospace(11.0))
                                                .color(COLOR_TEXT_DIM),
                                        );
                                        ui.end_row();
                                    }
                                });
                        }

                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Notes")
                                .font(FontId::monospace(13.0))
                                .color(COLOR_MODE_PLAYING)
                                .strong(),
                        );
                        ui.add_space(4.0);

                        egui::Grid::new("controls_grid_notes")
                            .num_columns(3)
                            .spacing([12.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("Z .. P")
                                        .font(FontId::monospace(12.0))
                                        .color(COLOR_TEXT_ACTIVE),
                                );
                                ui.label(
                                    RichText::new("Insert note")
                                        .font(FontId::monospace(12.0))
                                        .color(COLOR_TEXT),
                                );
                                ui.label(
                                    RichText::new("Insert note at cursor using keyboard layout")
                                        .font(FontId::monospace(11.0))
                                        .color(COLOR_TEXT_DIM),
                                );
                                ui.end_row();
                            });
                    });
            });
        });
}
