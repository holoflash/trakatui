use eframe::egui::{self, RichText, Sense};

use crate::app::{App, Mode};
use crate::ui::{
    COLOR_LAYOUT_BG_DARK, COLOR_LAYOUT_BORDER, COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT,
    COLOR_TEXT, COLOR_TEXT_DIM,
};

const FONT: egui::FontId = egui::FontId::monospace(11.0);
const ARROW_FONT: egui::FontId = egui::FontId::monospace(8.0);
const CELL_W: f32 = 22.0;
const CELL_H: f32 = 16.0;
const ARROW_H: f32 = 10.0;

pub fn draw_order_bar(ctx: &egui::Context, app: &mut App) {
    let order_len = app.project.order.len();
    let pat_count = app.project.patterns.len();

    let mut click_select: Option<usize> = None;
    let mut click_up: Option<usize> = None;
    let mut click_down: Option<usize> = None;

    egui::TopBottomPanel::top("order_bar").show(ctx, |ui| {
        ui.set_min_height(ARROW_H + CELL_H + ARROW_H + 4.0);
        let painter = ui.painter();
        painter.rect_filled(ui.max_rect(), 0.0, COLOR_LAYOUT_BG_DARK);

        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.spacing_mut().item_spacing.y = 0.0;

            ui.vertical(|ui| {
                ui.add_space(ARROW_H);
                ui.label(RichText::new("ORD").font(FONT).color(COLOR_TEXT_DIM));
            });
            ui.add_space(4.0);

            for i in 0..order_len {
                let pat_idx = app.project.order[i];
                let is_current = i == app.project.current_order_idx;
                let is_playing = app.playback.playing && i == app.playback_order_display;

                let (bg, fg) = if is_current && app.mode == Mode::Edit {
                    (COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT)
                } else if is_playing {
                    (
                        egui::Color32::from_rgb(90, 75, 40),
                        egui::Color32::from_rgb(255, 245, 220),
                    )
                } else if is_current {
                    (egui::Color32::from_rgb(40, 36, 56), COLOR_TEXT)
                } else {
                    (COLOR_LAYOUT_BG_DARK, COLOR_TEXT_DIM)
                };

                ui.vertical(|ui| {
                    ui.set_width(CELL_W);

                    let (up_rect, up_resp) =
                        ui.allocate_exact_size(egui::vec2(CELL_W, ARROW_H), Sense::click());
                    ui.painter().text(
                        up_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "▲",
                        ARROW_FONT,
                        if up_resp.hovered() {
                            COLOR_TEXT
                        } else {
                            COLOR_TEXT_DIM
                        },
                    );
                    if up_resp.clicked() {
                        click_up = Some(i);
                    }

                    let (rect, cell_resp) =
                        ui.allocate_exact_size(egui::vec2(CELL_W, CELL_H), Sense::click());
                    ui.painter().rect_filled(rect, 3.0, bg);
                    if is_current {
                        ui.painter().rect_stroke(
                            rect,
                            3.0,
                            egui::Stroke::new(1.0, COLOR_LAYOUT_BORDER),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let text = format!("{:02X}", pat_idx);
                    ui.painter()
                        .text(rect.center(), egui::Align2::CENTER_CENTER, text, FONT, fg);
                    if cell_resp.clicked() {
                        click_select = Some(i);
                    }

                    let (dn_rect, dn_resp) =
                        ui.allocate_exact_size(egui::vec2(CELL_W, ARROW_H), Sense::click());
                    ui.painter().text(
                        dn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "▼",
                        ARROW_FONT,
                        if dn_resp.hovered() {
                            COLOR_TEXT
                        } else {
                            COLOR_TEXT_DIM
                        },
                    );
                    if dn_resp.clicked() {
                        click_down = Some(i);
                    }
                });
            }

            ui.add_space(6.0);

            ui.vertical(|ui| {
                ui.add_space(ARROW_H);
                ui.horizontal(|ui| {
                    let btn = |ui: &mut egui::Ui, label: &str| -> bool {
                        ui.add(
                            egui::Button::new(RichText::new(label).font(FONT).color(COLOR_TEXT))
                                .min_size(egui::vec2(24.0, CELL_H))
                                .fill(egui::Color32::from_rgb(32, 28, 48))
                                .stroke(egui::Stroke::new(1.0, COLOR_LAYOUT_BORDER)),
                        )
                        .clicked()
                    };

                    if btn(ui, "+") {
                        let current_pat = app.project.order[app.project.current_order_idx];
                        let insert_pos = app.project.current_order_idx + 1;
                        app.project.order.insert(insert_pos, current_pat);
                        app.project.current_order_idx = insert_pos;
                        app.cursor.row = 0;
                    }

                    if app.project.order.len() > 1 && btn(ui, "−") {
                        remove_order_entry(app);
                    }

                    if btn(ui, "NEW") {
                        let channels = app.project.current_pattern().channels;
                        let rows = app.project.current_pattern().rows;
                        let new_idx = find_unused_pattern(app).unwrap_or_else(|| {
                            let idx = app.project.patterns.len();
                            app.project
                                .patterns
                                .push(crate::project::Pattern::new(channels, rows));
                            idx
                        });
                        let insert_pos = app.project.current_order_idx + 1;
                        app.project.order.insert(insert_pos, new_idx);
                        app.project.current_order_idx = insert_pos;
                        app.cursor.row = 0;
                    }
                });
            });
        });
    });

    if let Some(i) = click_select {
        app.project.current_order_idx = i;
        app.cursor.row = 0;
    }
    if let Some(i) = click_up {
        let current = app.project.order[i];
        app.project.order[i] = (current + 1) % pat_count;
        app.cursor.row = 0;
    }
    if let Some(i) = click_down {
        let current = app.project.order[i];
        app.project.order[i] = if current == 0 {
            pat_count - 1
        } else {
            current - 1
        };
        app.cursor.row = 0;
    }
}

fn remove_order_entry(app: &mut App) {
    let removed_pat = app.project.order[app.project.current_order_idx];
    app.project.order.remove(app.project.current_order_idx);
    if app.project.current_order_idx >= app.project.order.len() {
        app.project.current_order_idx = app.project.order.len() - 1;
    }
    app.cursor.row = 0;

    if !app.project.order.contains(&removed_pat) {
        app.project.patterns.remove(removed_pat);
        for idx in &mut app.project.order {
            if *idx > removed_pat {
                *idx -= 1;
            }
        }
    }
}

fn find_unused_pattern(app: &App) -> Option<usize> {
    (0..app.project.patterns.len()).find(|idx| !app.project.order.contains(idx))
}
