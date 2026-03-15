use std::sync::atomic::Ordering;

use eframe::egui::{self, FontId, RichText, Stroke, Vec2};

use crate::app::App;
use crate::ui::COLOR_TEXT;

use super::{COLOR_ACCENT, COLOR_LAYOUT_BG_PANEL, COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM};

const fn clamp_to_u8(v: f32) -> u8 {
    v.round().clamp(0.0, 255.0) as u8
}

pub fn draw_header(ctx: &egui::Context, app: &mut App) {
    let status = app.project_status();
    let title = if status.is_empty() {
        format!("psikat — {}", app.project_name())
    } else {
        format!("psikat — {} {}", app.project_name(), status)
    };
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(title));

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
                .inner_margin(egui::Margin::symmetric(8, 8)),
        )
        .show(ctx, |ui| {
            ui.style_mut().drag_value_text_style = egui::TextStyle::Monospace;

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;

                app.text_editing = false;
                // Hack to allow TAB to be used for note-off and not steal focus
                // Combo box blinks when using surrender_focus but an empty element seems to work
                // TODO: look for a better way
                let barrier = ui.allocate_response(
                    egui::vec2(0.0, 0.0),
                    egui::Sense::focusable_noninteractive(),
                );
                barrier.surrender_focus();

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
                                RichText::new("New Project").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close();
                            app.do_new_project();
                        }
                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Open Project").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close();
                            app.do_open();
                        }
                        ui.separator();

                        let has_path = app.project_path.is_some();
                        let save_color = if has_path {
                            COLOR_TEXT_ACTIVE
                        } else {
                            COLOR_TEXT_DIM
                        };
                        let save_resp =
                            ui.selectable_label(false, RichText::new("Save").color(save_color));
                        if save_resp.clicked() && has_path {
                            ui.close();
                            app.do_quick_save();
                        }

                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Save Project").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close();
                            app.do_save_as();
                        }

                        ui.separator();
                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Export WAV").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close();
                            app.do_export();
                        }
                        ui.separator();
                        if ui
                            .selectable_label(
                                false,
                                RichText::new("Controls").color(COLOR_TEXT_ACTIVE),
                            )
                            .clicked()
                        {
                            ui.close();
                            app.show_controls_modal = !app.show_controls_modal;
                        }
                    });

                ui.add_space(100.0);
                ui.separator();
                ui.add_space(8.0);

                draw_field(ui, "BPM");
                let mut bpm = app.project.current_pattern().bpm;
                let r = ui
                    .add(egui::DragValue::new(&mut bpm).range(20..=666).speed(0.5))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().bpm = bpm;
                }
                ui.add_space(8.0);
                draw_field(ui, "SIGNATURE");
                let mut num = app.project.current_pattern().time_sig_numerator;
                let r = ui
                    .add(egui::DragValue::new(&mut num).range(1..=32).speed(0.2))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().time_sig_numerator = num;
                    let new_rows = app.project.current_pattern().computed_rows();
                    app.project.current_pattern_mut().resize(new_rows);
                    if app.cursor.row >= new_rows {
                        app.cursor.row = new_rows.saturating_sub(1);
                    }
                }
                ui.label(
                    RichText::new("/")
                        .font(FontId::monospace(8.0))
                        .color(COLOR_TEXT_DIM),
                );

                let mut den = app.project.current_pattern().time_sig_denominator;
                let r = ui
                    .add(egui::DragValue::new(&mut den).range(1..=32).speed(0.2))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().time_sig_denominator = den;
                    let new_rows = app.project.current_pattern().computed_rows();
                    app.project.current_pattern_mut().resize(new_rows);
                    if app.cursor.row >= new_rows {
                        app.cursor.row = new_rows.saturating_sub(1);
                    }
                }
                ui.add_space(8.0);

                const SUBDIVISIONS: [(u8, &str); 26] = [
                    (1, "1/1"),
                    (2, "1/2"),
                    (3, "1/2T"),
                    (4, "1/4"),
                    (5, "1/4·5"),
                    (6, "1/4T"),
                    (7, "1/4·7"),
                    (8, "1/8"),
                    (9, "1/8·9"),
                    (10, "1/8·5"),
                    (11, "1/8·11"),
                    (12, "1/8T"),
                    (14, "1/8·7"),
                    (16, "1/16"),
                    (18, "1/16·9"),
                    (20, "1/16·5"),
                    (22, "1/16·11"),
                    (24, "1/16T"),
                    (28, "1/16·7"),
                    (32, "1/32"),
                    (36, "1/32·9"),
                    (40, "1/32·5"),
                    (44, "1/32·11"),
                    (48, "1/32T"),
                    (56, "1/32·7"),
                    (64, "1/64"),
                ];

                draw_field(ui, "NOTE VALUE");
                let ch = app.cursor.channel;
                let current_nv = app
                    .project
                    .current_pattern()
                    .track_note_values
                    .get(ch)
                    .copied()
                    .unwrap_or(app.project.current_pattern().note_value);
                let current_label = SUBDIVISIONS
                    .iter()
                    .find(|(v, _)| *v == current_nv)
                    .map_or_else(|| format!("{current_nv}"), |(_, l)| l.to_string());
                egui::ComboBox::from_id_salt("note_value")
                    .selected_text(
                        RichText::new(&current_label)
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_ACTIVE),
                    )
                    .width(60.0)
                    .show_ui(ui, |ui| {
                        for &(nv, label) in &SUBDIVISIONS {
                            let color = if nv == current_nv {
                                COLOR_ACCENT
                            } else {
                                COLOR_TEXT_ACTIVE
                            };
                            let pat = app.project.current_pattern_mut();
                            if ch < pat.track_note_values.len()
                                && ui
                                    .selectable_value(
                                        &mut pat.track_note_values[ch],
                                        nv,
                                        RichText::new(label).color(color),
                                    )
                                    .changed()
                            {
                                app.project.current_pattern_mut().resize_track(ch);
                                let track_rows = app.project.current_pattern().track_rows(ch);
                                if app.cursor.row >= track_rows {
                                    app.cursor.row = track_rows.saturating_sub(1);
                                }
                            }
                        }
                    });
                ui.add_space(8.0);
                draw_field(ui, "BARS");
                let mut bars = app.project.current_pattern().measures;
                let r = ui
                    .add(egui::DragValue::new(&mut bars).range(1..=32).speed(0.2))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().measures = bars;
                    let new_rows = app.project.current_pattern().computed_rows();
                    app.project.current_pattern_mut().resize(new_rows);
                    if app.cursor.row >= new_rows {
                        app.cursor.row = new_rows.saturating_sub(1);
                    }
                }
                ui.add_space(8.0);
                draw_field(ui, "REPEAT");
                let mut rep = app.project.current_pattern().repeat;
                let r = ui
                    .add(egui::DragValue::new(&mut rep).range(1..=999).speed(0.2))
                    .on_hover_cursor(egui::CursorIcon::ResizeHorizontal);
                if r.has_focus() {
                    app.text_editing = true;
                }
                if r.changed() {
                    app.project.current_pattern_mut().repeat = rep;
                }
                ui.add_space(8.0);
                ui.separator();

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;

                    ui.add_space(6.0);
                    draw_toggle_btn(ui, "ARRANGER", app.show_arranger, &mut app.show_arranger);
                    ui.add_space(6.0);
                    draw_toggle_btn(ui, "EDIT", app.show_sidebar, &mut app.show_sidebar);
                    ui.add_space(6.0);
                    draw_toggle_btn(ui, "MIXER", app.show_mixer, &mut app.show_mixer);
                    ui.add_space(6.0);

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
                    ui.add_space(6.0);
                    draw_peak_meter(ui, app);
                });
            });
        });
}

fn draw_toggle_btn(ui: &mut egui::Ui, label: &str, active: bool, state: &mut bool) {
    let color = if active { COLOR_TEXT } else { COLOR_TEXT_DIM };
    let btn = ui
        .add(
            egui::Button::new(
                RichText::new(label)
                    .font(FontId::monospace(12.0))
                    .color(color),
            )
            .fill(COLOR_LAYOUT_BG_PANEL)
            .stroke(Stroke::new(1.0, color)),
        )
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    btn.surrender_focus();
    if btn.clicked() {
        *state = !*state;
    }
}

fn draw_field(ui: &mut egui::Ui, label: &str) {
    ui.label(
        RichText::new(label)
            .font(FontId::monospace(12.0))
            .color(COLOR_TEXT_DIM),
    );
}

fn draw_peak_meter(ui: &mut egui::Ui, app: &App) {
    let meter_width = 60.0;
    let meter_height = 12.0;
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
