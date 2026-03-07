use std::sync::atomic::Ordering;

use eframe::egui::{self, FontId, RichText, Stroke, Vec2};

use crate::app::{App, scale::SCALES};

use super::{
    COLOR_ACCENT, COLOR_LAYOUT_BG_PANEL, COLOR_LAYOUT_BORDER, COLOR_PATTERN_CURSOR_TEXT,
    COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM,
};

const fn clamp_to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

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
            ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;

            ui.horizontal(|ui| {
                ui.add_space(4.0);
                let logo_btn = ui
                    .add(
                        egui::ImageButton::new(
                            egui::Image::new(egui::include_image!("../../psikat.png"))
                                .fit_to_exact_size(Vec2::new(48.0, 48.0))
                                .texture_options(egui::TextureOptions::NEAREST),
                        )
                        .frame(false),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand);
                logo_btn.surrender_focus();
                if logo_btn.clicked() {
                    app.show_about_modal = !app.show_about_modal;
                }
                ui.add_space(8.0);

                egui::ComboBox::from_id_salt("file_menu")
                    .selected_text(
                        RichText::new("File")
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_ACTIVE),
                    )
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Open Module").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close_menu();
                            app.open_module_file();
                        }
                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Export WAV").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close_menu();
                            app.do_export();
                        }
                        ui.separator();
                        if ui
                            .selectable_label(false, RichText::new("Help").color(COLOR_TEXT_ACTIVE))
                            .clicked()
                        {
                            ui.close_menu();
                            app.show_controls_modal = !app.show_controls_modal;
                        }
                    });

                ui.add_space(24.0);

                app.text_editing = false;

                draw_field(ui, "BPM");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.project.bpm)
                            .range(20..=666)
                            .speed(0.5),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }

                draw_field(ui, "DIV");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.project.subdivision)
                            .range(2..=64)
                            .speed(0.2),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }

                draw_field(ui, "LEN");
                let mut len = app.project.current_pattern().rows;
                let r = ui
                    .add(egui::DragValue::new(&mut len).range(1..=128).speed(0.3))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().resize(len);
                    if app.cursor.row >= len {
                        app.cursor.row = len - 1;
                    }
                }

                draw_field(ui, "STEP");
                let r = ui
                    .add(
                        egui::DragValue::new(&mut app.project.step)
                            .range(1..=64)
                            .speed(0.2),
                    )
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }

                ui.add_space(24.0);
                draw_field(ui, "INPUT");
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

                let current_name = app.project.scale_index.scale().name;
                egui::ComboBox::from_id_salt("scale_combo")
                    .selected_text(RichText::new(current_name).font(FontId::monospace(12.0)))
                    .width(160.0)
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

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = ui
                        .add(
                            egui::Button::new(
                                RichText::new("INSTRUMENT")
                                    .font(FontId::monospace(12.0))
                                    .color(if app.show_sidebar {
                                        COLOR_PATTERN_CURSOR_TEXT
                                    } else {
                                        COLOR_TEXT_DIM
                                    }),
                            )
                            .fill(if app.show_sidebar {
                                COLOR_LAYOUT_BORDER
                            } else {
                                COLOR_LAYOUT_BG_PANEL
                            })
                            .stroke(Stroke::new(
                                1.0,
                                if app.show_sidebar {
                                    COLOR_PATTERN_CURSOR_TEXT
                                } else {
                                    COLOR_TEXT_DIM
                                },
                            )),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    btn.surrender_focus();
                    if btn.clicked() {
                        app.show_sidebar = !app.show_sidebar;
                    }

                    ui.add_space(8.0);
                    draw_peak_meter(ui, app);
                    ui.add_space(4.0);

                    draw_field(ui, "VOL");
                    let r = ui
                        .add(
                            egui::DragValue::new(&mut app.project.master_volume_db)
                                .range(-60.0..=6.0)
                                .speed(0.2)
                                .custom_formatter(|v, _| {
                                    let i = v.round() as i32;
                                    let s = if i > 0 {
                                        format!("+{i}")
                                    } else {
                                        format!("{i}")
                                    };
                                    format!("{s:>3} dB")
                                })
                                .fixed_decimals(1),
                        )
                        .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                    if r.has_focus() {
                        app.text_editing = true;
                    }
                    if r.double_clicked() {
                        app.project.master_volume_db = 0.0;
                    }

                    ui.add_space(8.0);

                    let follow_color = if app.follow_playback {
                        COLOR_PATTERN_CURSOR_TEXT
                    } else {
                        COLOR_TEXT_DIM
                    };
                    let btn = ui
                        .add(
                            egui::Button::new(
                                RichText::new(if app.follow_playback {
                                    "Follow"
                                } else {
                                    "Follow"
                                })
                                .font(FontId::monospace(12.0))
                                .color(follow_color),
                            )
                            .fill(COLOR_LAYOUT_BG_PANEL)
                            .stroke(Stroke::new(1.0, follow_color)),
                        )
                        .on_hover_cursor(egui::CursorIcon::PointingHand);
                    btn.surrender_focus();
                    if btn.clicked() {
                        app.follow_playback = !app.follow_playback;
                    }
                });
            });
        });
}

fn draw_field(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .font(FontId::monospace(10.0))
            .color(COLOR_TEXT_DIM),
    );
}

fn draw_peak_meter(ui: &mut egui::Ui, app: &App) {
    let meter_width = 60.0;
    let meter_height = 10.0;
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(meter_width, meter_height), egui::Sense::hover());

    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, egui::Color32::from_rgb(20, 18, 32));

    let peak = app.display_peak;
    if peak > 0.001 {
        let peak_db = 20.0 * peak.log10();
        let min_db = -60.0_f32;
        let max_db = 6.0_f32;
        let norm = ((peak_db - min_db) / (max_db - min_db)).clamp(0.0, 1.0);
        let fill_rect =
            egui::Rect::from_min_size(rect.min, Vec2::new(rect.width() * norm, meter_height));

        let color = if peak_db < -12.0 {
            egui::Color32::from_rgb(60, 190, 80)
        } else if peak_db < -3.0 {
            let t = ((peak_db + 12.0) / 9.0).clamp(0.0, 1.0);
            egui::Color32::from_rgb(
                clamp_to_u8(60.0 + t * 180.0),
                clamp_to_u8(190.0 + t * 30.0),
                clamp_to_u8(80.0 - t * 50.0),
            )
        } else if peak_db < 0.0 {
            let t = ((peak_db + 3.0) / 3.0).clamp(0.0, 1.0);
            egui::Color32::from_rgb(
                clamp_to_u8(240.0 + t * 15.0),
                clamp_to_u8(220.0 - t * 120.0),
                30,
            )
        } else {
            egui::Color32::from_rgb(255, 60, 50)
        };

        painter.rect_filled(fill_rect, 2.0, color);

        let zero_x = rect
            .width()
            .mul_add((0.0 - min_db) / (max_db - min_db), rect.min.x);
        painter.line_segment(
            [
                egui::Pos2::new(zero_x, rect.min.y),
                egui::Pos2::new(zero_x, rect.max.y),
            ],
            Stroke::new(
                1.0,
                egui::Color32::from_rgba_premultiplied(255, 255, 255, 80),
            ),
        );
    }
}
