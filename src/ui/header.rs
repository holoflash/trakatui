use std::sync::atomic::Ordering;

use eframe::egui::{self, FontId, RichText, Stroke};

use crate::app::{App, Mode};
use crate::scale::root_name;

use super::*;

pub fn draw_header(ctx: &egui::Context, app: &mut App) {
    let raw_peak = f32::from_bits(app.peak_level.swap(0, Ordering::Relaxed));
    let target = raw_peak.min(1.5);
    if target > app.display_peak {
        app.display_peak = target;
    } else {
        app.display_peak *= 0.88;
        if app.display_peak < 0.001 {
            app.display_peak = 0.0;
        }
    }

    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 8)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../../psikat.png"))
                        .fit_to_exact_size(egui::Vec2::new(48.0, 48.0)),
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

                ui.add_space(16.0);

                draw_volume_control(ui, app);

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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let export_btn = ui.add(
                        egui::Button::new(
                            RichText::new(" Export WAV ")
                                .font(FontId::monospace(12.0))
                                .color(COLOR_PATTERN_CURSOR_TEXT),
                        )
                        .fill(COLOR_LAYOUT_BORDER)
                        .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE)),
                    );
                    export_btn.surrender_focus();
                    if export_btn.clicked() {
                        app.do_export();
                    }

                    ui.add_space(4.0);

                    let ctrl_btn = ui.add(
                        egui::Button::new(
                            RichText::new(" Controls ")
                                .font(FontId::monospace(12.0))
                                .color(COLOR_PATTERN_CURSOR_TEXT),
                        )
                        .fill(COLOR_LAYOUT_BORDER)
                        .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE)),
                    );
                    ctrl_btn.surrender_focus();
                    if ctrl_btn.clicked() {
                        app.show_controls_modal = !app.show_controls_modal;
                    }
                });
            });
        });
}

fn draw_volume_control(ui: &mut egui::Ui, app: &mut App) {
    ui.label(
        RichText::new("VOL")
            .font(FontId::monospace(10.0))
            .color(COLOR_TEXT_DIM),
    );

    let slider_response = ui.add(
        egui::Slider::new(&mut app.master_volume_db, -60.0..=6.0)
            .step_by(0.1)
            .clamping(egui::SliderClamping::Always),
    );
    slider_response.surrender_focus();
    if slider_response.double_clicked() {
        app.master_volume_db = 0.0;
    }

    ui.add_space(4.0);

    let meter_width = 80.0;
    let meter_height = 13.0;
    let (rect, _response) = ui.allocate_exact_size(
        egui::Vec2::new(meter_width, meter_height),
        egui::Sense::hover(),
    );

    let painter = ui.painter();

    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(20, 18, 32));

    let peak = app.display_peak;
    if peak > 0.001 {
        let peak_db = 20.0 * peak.log10();
        let meter_min_db = -60.0_f32;
        let meter_max_db = 6.0_f32;
        let normalized = ((peak_db - meter_min_db) / (meter_max_db - meter_min_db)).clamp(0.0, 1.0);

        let fill_width = rect.width() * normalized;
        let fill_rect =
            egui::Rect::from_min_size(rect.min, egui::Vec2::new(fill_width, meter_height));

        let color = if peak_db < -12.0 {
            egui::Color32::from_rgb(60, 190, 80)
        } else if peak_db < -3.0 {
            let t = ((peak_db + 12.0) / 9.0).clamp(0.0, 1.0);
            egui::Color32::from_rgb(
                (60.0 + t * 180.0) as u8,
                (190.0 + t * 30.0) as u8,
                (80.0 - t * 50.0) as u8,
            )
        } else if peak_db < 0.0 {
            let t = ((peak_db + 3.0) / 3.0).clamp(0.0, 1.0);
            egui::Color32::from_rgb((240.0 + t * 15.0) as u8, (220.0 - t * 120.0) as u8, 30)
        } else {
            egui::Color32::from_rgb(255, 60, 50)
        };

        painter.rect_filled(fill_rect, 2.0, color);

        let zero_db_x =
            rect.min.x + rect.width() * ((0.0 - meter_min_db) / (meter_max_db - meter_min_db));
        painter.line_segment(
            [
                egui::Pos2::new(zero_db_x, rect.min.y),
                egui::Pos2::new(zero_db_x, rect.max.y),
            ],
            Stroke::new(
                1.0,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 80),
            ),
        );
    }
}
