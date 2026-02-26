use eframe::egui::{self, FontId, RichText, Stroke};

use crate::app::{App, Mode};
use crate::scale::root_name;

use super::*;

pub fn draw_header(ctx: &egui::Context, app: &mut App) {
    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../../psikat.png"))
                        .fit_to_exact_size(egui::Vec2::new(48.0, 48.0)),
                );
                ui.add_space(4.0);
                let (mode_str, mode_color) = if app.playing {
                    ("PLAYING", COLOR_MODE_PLAYING)
                } else {
                    match app.mode {
                        Mode::Edit => ("EDIT", COLOR_MODE_EDIT),
                        Mode::Settings => ("SETTINGS", COLOR_MODE_SETTINGS),
                        Mode::SynthEdit => ("SYNTH", COLOR_MODE_SETTINGS),
                    }
                };
                ui.label(
                    RichText::new(format!("[{}]", mode_str))
                        .font(FontId::monospace(14.0))
                        .color(mode_color)
                        .strong(),
                );
                ui.add_space(16.0);
                ui.label(
                    RichText::new(format!("Oct:{}", app.octave))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_MODE_SETTINGS),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("BPM:{}", app.bpm))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_TEXT),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("Division:{}", app.subdivision))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_TEXT),
                );
                ui.add_space(12.0);

                let root = root_name(app.transpose);
                let scale_name = app.scale_index.scale().name;
                ui.label(
                    RichText::new(format!("{} {}", root, scale_name))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_MODE_PLAYING),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = ui.add(
                        egui::Button::new(
                            RichText::new(" Export WAV ")
                                .font(FontId::monospace(12.0))
                                .color(COLOR_PATTERN_CURSOR_TEXT),
                        )
                        .fill(COLOR_LAYOUT_BORDER)
                        .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE)),
                    );
                    if btn.clicked() {
                        app.do_export();
                    }
                });
            });
        });
}
