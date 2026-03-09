use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, RichText, Sense, Stroke};

use crate::app::{App, Mode};
use crate::ui::{
    COLOR_LAYOUT_BG_DARK, COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT,
    COLOR_PATTERN_PLAYBACK_HIGHLIGHT, COLOR_PATTERN_PLAYBACK_TEXT, COLOR_PATTERN_SUBDIVISION,
    COLOR_TEXT, COLOR_TEXT_DIM,
};

const FONT: egui::FontId = egui::FontId::monospace(14.0);
const BTN_FONT: egui::FontId = egui::FontId::monospace(14.0);
const CELL_W: f32 = 28.0;
const CELL_H: f32 = 18.0;
const COLS: usize = 3;

enum OrderAction {
    Select(usize),
    SetPattern(usize, usize),
    NewPattern,
    Delete(usize),
}

pub fn draw_order_bar(ctx: &egui::Context, app: &mut App) {
    let order_len = app.project.order.len();
    let pat_count = app.project.patterns.len();

    let mut actions: Vec<OrderAction> = Vec::new();

    let panel_w = COLS as f32 * CELL_W + (COLS - 1) as f32;
    let grid_rows = order_len.div_ceil(COLS);

    egui::SidePanel::left("order_bar")
        .resizable(false)
        .exact_width(panel_w)
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_DARK)
                .inner_margin(egui::Margin::ZERO)
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            egui::ScrollArea::vertical()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let grid_h = grid_rows as f32 * CELL_H + (grid_rows.max(1) - 1) as f32;
                    let total_h = CELL_H + grid_h;
                    let (area, _) =
                        ui.allocate_exact_size(egui::vec2(panel_w, total_h), Sense::hover());
                    let top = area.min;

                    let btn_rect = egui::Rect::from_min_size(top, egui::vec2(panel_w, CELL_H));
                    let btn_resp =
                        ui.interact(btn_rect, ui.id().with("order_new_btn"), Sense::click());
                    if btn_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    ui.painter()
                        .rect_filled(btn_rect, 0.0, COLOR_LAYOUT_BG_DARK);
                    ui.painter().text(
                        btn_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        BTN_FONT,
                        COLOR_TEXT,
                    );
                    if btn_resp.clicked() {
                        actions.push(OrderAction::NewPattern);
                    }

                    let grid_top = top.y + CELL_H;

                    for i in 0..order_len {
                        let col = i % COLS;
                        let row = i / COLS;
                        let rect = egui::Rect::from_min_size(
                            egui::pos2(top.x + col as f32 * CELL_W, grid_top + row as f32 * CELL_H),
                            egui::vec2(CELL_W, CELL_H),
                        );

                        let pat_idx = app.project.order[i];
                        let is_current = i == app.project.current_order_idx;
                        let is_playing = app.playback.playing && i == app.playback_order_display;

                        let (bg, fg) = if is_playing {
                            (
                                COLOR_PATTERN_PLAYBACK_HIGHLIGHT,
                                COLOR_PATTERN_PLAYBACK_TEXT,
                            )
                        } else if is_current && app.mode == Mode::Edit {
                            (COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT)
                        } else if is_current {
                            (COLOR_PATTERN_SUBDIVISION, COLOR_TEXT)
                        } else {
                            (COLOR_LAYOUT_BG_DARK, COLOR_TEXT_DIM)
                        };

                        let cell_resp =
                            ui.interact(rect, ui.id().with(("order_cell", i)), Sense::click());
                        if cell_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        ui.painter().rect_filled(rect, 0.0, bg);

                        let stroke_color = COLOR_PATTERN_SUBDIVISION;
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(1.0, stroke_color),
                            egui::StrokeKind::Outside,
                        );

                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{:02X}", pat_idx),
                            FONT,
                            fg,
                        );

                        if cell_resp.double_clicked() && order_len > 1 {
                            actions.push(OrderAction::Delete(i));
                        } else if cell_resp.clicked() {
                            actions.push(OrderAction::Select(i));
                        }

                        cell_resp.context_menu(|ui| {
                            for p in 0..pat_count {
                                let label = format!("{:02X}", p);
                                if ui
                                    .button(RichText::new(label).font(FONT).color(COLOR_TEXT))
                                    .clicked()
                                {
                                    actions.push(OrderAction::SetPattern(i, p));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
        });

    for action in actions {
        match action {
            OrderAction::Select(i) => {
                app.project.current_order_idx = i;
                app.cursor.row = 0;
                if app.playback.playing {
                    app.start_playback(false);
                }
            }
            OrderAction::SetPattern(i, p) => {
                app.project.order[i] = p;
                app.cursor.row = 0;
            }
            OrderAction::NewPattern => {
                let channels = app.project.current_pattern().channels;
                let rows = app.project.current_pattern().rows;
                let new_idx = find_unused_pattern(app).unwrap_or_else(|| {
                    let idx = app.project.patterns.len();
                    app.project
                        .patterns
                        .push(crate::project::Pattern::new(channels, rows));
                    idx
                });
                let insert_pos = app.project.order.len();
                app.project.order.insert(insert_pos, new_idx);
                app.project.current_order_idx = insert_pos;
                app.cursor.row = 0;
            }
            OrderAction::Delete(i) => {
                remove_order_entry(app, i);
            }
        }
    }
}

fn remove_order_entry(app: &mut App, idx: usize) {
    let removed_pat = app.project.order[idx];
    app.project.order.remove(idx);
    if app.project.current_order_idx >= app.project.order.len() {
        app.project.current_order_idx = app.project.order.len() - 1;
    }
    app.cursor.row = 0;

    if !app.project.order.contains(&removed_pat) {
        app.project.patterns.remove(removed_pat);
        for o in &mut app.project.order {
            if *o > removed_pat {
                *o -= 1;
            }
        }
    }
}

fn find_unused_pattern(app: &App) -> Option<usize> {
    (0..app.project.patterns.len()).find(|idx| !app.project.order.contains(idx))
}
