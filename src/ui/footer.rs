use eframe::egui::{self, FontId, RichText};

use crate::app::{App, scale::SCALES};

use super::{COLOR_ACCENT, COLOR_LAYOUT_BG_PANEL, COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM};

fn draw_field(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .font(FontId::monospace(12.0))
            .color(COLOR_TEXT_DIM),
    );
}

pub fn draw_footer(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(8, 8)),
        )
        .show(ctx, |ui| {
            ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;

            ui.horizontal(|ui| {
                let total_width = ui.available_width();
                let controls_width = 520.0;
                let pad = ((total_width - controls_width) / 2.0).max(0.0);
                ui.add_space(pad);

                ui.spacing_mut().item_spacing.x = 4.0;

                ui.separator();
                ui.add_space(8.0);

                draw_field(ui, "PITCH");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.project.transpose)
                            .range(-12..=12)
                            .speed(0.15)
                            .custom_formatter(|v, _| {
                                let i = v as i32;
                                if i > 0 {
                                    format!("+{i}")
                                } else if i == 0 {
                                    " 0".to_string()
                                } else {
                                    format!("{i}")
                                }
                            }),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                ui.add_space(8.0);
                draw_field(ui, "OCTAVE");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.cursor.octave)
                            .range(0..=8)
                            .speed(0.15),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                ui.add_space(8.0);
                let current_name = app.project.scale_index.scale().name;
                egui::ComboBox::from_id_salt("scale_combo")
                    .selected_text(RichText::new(current_name).font(FontId::monospace(12.0)))
                    .width(140.0)
                    .show_ui(ui, |ui| {
                        for (i, scale) in SCALES.iter().enumerate() {
                            let color = if app.project.scale_index.0 == i {
                                COLOR_ACCENT
                            } else {
                                COLOR_TEXT_ACTIVE
                            };
                            ui.selectable_value(
                                &mut app.project.scale_index.0,
                                i,
                                RichText::new(scale.name).color(color),
                            );
                        }
                    });
                ui.add_space(8.0);
                draw_field(ui, "SKIP");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.project.step)
                            .range(0..=64)
                            .speed(0.2),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                ui.add_space(8.0);
                ui.separator();
            });
        });
}
