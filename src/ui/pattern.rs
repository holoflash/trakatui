use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, FontId, Pos2, RichText, Sense, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::app::{App, Mode, SubColumn};
use crate::audio::mixer::SCOPE_SIZE;
use crate::project::{Cell, effect_display, panning_display, volume_display};

use super::{
    COLOR_LAYOUT_BG_DARK, COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT, COLOR_PATTERN_EFFECT,
    COLOR_PATTERN_PANNING, COLOR_PATTERN_NOTE, COLOR_PATTERN_NOTE_OFF,
    COLOR_PATTERN_PLAYBACK_HIGHLIGHT, COLOR_PATTERN_PLAYBACK_TEXT, COLOR_PATTERN_SELECTION_BG,
    COLOR_PATTERN_SELECTION_TEXT, COLOR_PATTERN_SUBDIVISION, COLOR_PATTERN_VOLUME, COLOR_TEXT_DIM,
};

const COLOR_MUTED: egui::Color32 = egui::Color32::from_rgb(180, 80, 70);

const FONT: FontId = FontId::monospace(14.0);
const ROW_HEIGHT: f32 = 18.0;
const CELL_PAD: f32 = 8.0;
const CELL_PAD_HALF: f32 = 4.0;

fn fill_cell(ui: &egui::Ui, color: egui::Color32) {
    if color != egui::Color32::TRANSPARENT {
        ui.painter().rect_filled(ui.max_rect(), 0.0, color);
    }
}

fn draw_left_border(ui: &egui::Ui) {
    let rect = ui.max_rect();
    ui.painter().line_segment(
        [rect.left_top(), rect.left_bottom()],
        Stroke::new(1.0, COLOR_TEXT_DIM),
    );
}

pub fn draw_pattern(ctx: &egui::Context, app: &mut App) {
    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_DARK)
                .inner_margin(egui::Margin {
                    left: 0,
                    right: 0,
                    top: 0,
                    bottom: 12,
                }),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
            ui.style_mut().interaction.selectable_labels = false;

            let channels = app.project.current_pattern().channels;
            let col = Column::auto().at_least(0.0);

            egui::ScrollArea::horizontal()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let visible_height = ui.available_height();
                    let mut table = TableBuilder::new(ui)
                        .striped(false)
                        .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .sense(Sense::hover())
                        .drag_to_scroll(false)
                        .column(col);

                    for _ in 0..channels {
                        table = table.column(col).column(col).column(col).column(col);
                    }

                    if app.playback.playing {
                        let target_y = (app.playback_row_display as f32 * ROW_HEIGHT
                            - visible_height / 2.0
                            + ROW_HEIGHT / 2.0)
                            .max(0.0);
                        let diff = target_y - app.follow_scroll_offset;
                        if diff < -ROW_HEIGHT * 2.0 || diff.abs() < 0.5 {
                            app.follow_scroll_offset = target_y;
                        } else {
                            app.follow_scroll_offset += diff * 0.15;
                        }
                        table = table.vertical_scroll_offset(app.follow_scroll_offset);
                    }

                    table
                        .header(SCOPE_HEIGHT + ROW_HEIGHT, |mut header| {
                            draw_header_row(
                                &mut header,
                                channels,
                                &mut app.muted_channels,
                                &app.display_scopes,
                                app.playback.playing,
                            );
                        })
                        .body(|body| {
                            body.rows(ROW_HEIGHT, app.project.current_pattern().rows, |mut row| {
                                draw_body_row(&mut row, app, channels);
                            });
                        });
                });
        });
}

const SCOPE_HEIGHT: f32 = 40.0;

fn draw_header_row(
    header: &mut egui_extras::TableRow<'_, '_>,
    channels: usize,
    muted: &mut Vec<bool>,
    scopes: &[[f32; SCOPE_SIZE]],
    playing: bool,
) {
    header.col(|ui| {
        let full = ui.max_rect();
        ui.painter().rect_filled(full, 0.0, COLOR_LAYOUT_BG_DARK);
    });

    for ch in 0..channels {
        let is_muted = muted.get(ch).copied().unwrap_or(false);
        let label = format!("{}", ch + 1);

        let cell_bg = if is_muted {
            COLOR_MUTED
        } else {
            egui::Color32::TRANSPARENT
        };
        let text_color = if is_muted {
            COLOR_PATTERN_CURSOR_TEXT
        } else {
            COLOR_TEXT_DIM
        };

        let ch_start_x = std::cell::Cell::new(0.0_f32);
        let scope_top = std::cell::Cell::new(0.0_f32);
        let scope_bottom = std::cell::Cell::new(0.0_f32);

        header.col(|ui| {
            let full = ui.max_rect();
            let scope_rect =
                egui::Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - ROW_HEIGHT));
            let label_rect =
                egui::Rect::from_min_max(Pos2::new(full.min.x, full.max.y - ROW_HEIGHT), full.max);

            ch_start_x.set(full.min.x);
            scope_top.set(scope_rect.min.y);
            scope_bottom.set(scope_rect.max.y);

            ui.painter()
                .rect_filled(scope_rect, 0.0, COLOR_LAYOUT_BG_DARK);
            ui.painter().rect_filled(label_rect, 0.0, cell_bg);
            draw_left_border(ui);

            let response = ui.interact(label_rect, ui.id().with(("ch_lbl", ch)), Sense::click());

            ui.painter().text(
                label_rect.left_center() + egui::vec2(CELL_PAD, 0.0),
                egui::Align2::LEFT_CENTER,
                &label,
                FONT,
                text_color,
            );

            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if response.secondary_clicked() {
                toggle_solo(muted, ch, channels);
            } else if response.clicked() {
                if ch >= muted.len() {
                    muted.resize(ch + 1, false);
                }
                muted[ch] = !muted[ch];
            }
        });

        for sub in 0..3 {
            header.col(|ui| {
                let full = ui.max_rect();
                let scope_rect = egui::Rect::from_min_max(
                    full.min,
                    Pos2::new(full.max.x, full.max.y - ROW_HEIGHT),
                );
                let label_rect = egui::Rect::from_min_max(
                    Pos2::new(full.min.x, full.max.y - ROW_HEIGHT),
                    full.max,
                );

                ui.painter()
                    .rect_filled(scope_rect, 0.0, COLOR_LAYOUT_BG_DARK);
                ui.painter().rect_filled(label_rect, 0.0, cell_bg);

                if sub == 2
                    && let Some(scope_data) = scopes.get(ch) {
                        let wide_rect = egui::Rect::from_min_max(
                            Pos2::new(ch_start_x.get(), scope_top.get()),
                            Pos2::new(full.max.x, scope_bottom.get()),
                        );
                        let clip = wide_rect.intersect(ui.clip_rect());
                        let wide_painter =
                            egui::Painter::new(ui.ctx().clone(), ui.layer_id(), clip);
                        draw_scope_with_painter(
                            &wide_painter,
                            wide_rect,
                            scope_data,
                            is_muted,
                            playing,
                        );

                        wide_painter.line_segment(
                            [wide_rect.left_bottom(), wide_rect.right_bottom()],
                            Stroke::new(1.0, COLOR_TEXT_DIM),
                        );

                        let label_bottom_y = scope_bottom.get() + ROW_HEIGHT;
                        let bottom_line_rect = egui::Rect::from_min_max(
                            Pos2::new(ch_start_x.get(), label_bottom_y - 1.0),
                            Pos2::new(full.max.x, label_bottom_y),
                        );
                        let bottom_clip = bottom_line_rect.intersect(ui.clip_rect());
                        let bottom_painter =
                            egui::Painter::new(ui.ctx().clone(), ui.layer_id(), bottom_clip);
                        bottom_painter.line_segment(
                            [
                                Pos2::new(ch_start_x.get(), label_bottom_y),
                                Pos2::new(full.max.x, label_bottom_y),
                            ],
                            Stroke::new(1.0, COLOR_TEXT_DIM),
                        );
                    }

                let response = ui.interact(
                    label_rect,
                    ui.id().with(("ch_lbl_sub", ch, sub)),
                    Sense::click(),
                );

                if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                }
                if response.secondary_clicked() {
                    toggle_solo(muted, ch, channels);
                } else if response.clicked() {
                    if ch >= muted.len() {
                        muted.resize(ch + 1, false);
                    }
                    muted[ch] = !muted[ch];
                }
            });
        }
    }
}

fn toggle_solo(muted: &mut Vec<bool>, ch: usize, channels: usize) {
    if muted.len() < channels {
        muted.resize(channels, false);
    }

    let is_soloed =
        !muted[ch] && (0..channels).all(|c| c == ch || muted.get(c).copied().unwrap_or(false));

    if is_soloed {
        for m in muted.iter_mut() {
            *m = false;
        }
    } else {
        for (c, m) in muted.iter_mut().enumerate() {
            *m = c != ch;
        }
    }
}

fn draw_scope_with_painter(
    painter: &egui::Painter,
    rect: egui::Rect,
    data: &[f32; SCOPE_SIZE],
    muted: bool,
    playing: bool,
) {
    let w = rect.width();
    let h = rect.height();
    let mid_y = rect.min.y + h * 0.5;
    let color = if muted {
        COLOR_MUTED
    } else if playing {
        COLOR_PATTERN_PLAYBACK_TEXT
    } else {
        COLOR_TEXT_DIM
    };

    let step = SCOPE_SIZE as f32 / w;
    let points: Vec<Pos2> = (0..w as usize)
        .map(|px| {
            let idx = ((px as f32) * step) as usize;
            let sample = data[idx.min(SCOPE_SIZE - 1)];
            let y = mid_y - sample.clamp(-1.0, 1.0) * h * 0.45;
            Pos2::new(rect.min.x + px as f32, y)
        })
        .collect();

    if points.len() >= 2 {
        painter.add(egui::Shape::line(points, Stroke::new(1.0, color)));
    }
}

fn draw_body_row(row: &mut egui_extras::TableRow<'_, '_>, app: &mut App, channels: usize) {
    let row_idx = row.index();
    let is_playback_row = app.playback.playing && row_idx == app.playback_row_display;
    let is_subdivision = row_idx.is_multiple_of(app.project.subdivision);

    let row_bg = if is_playback_row {
        COLOR_PATTERN_PLAYBACK_HIGHLIGHT
    } else if is_subdivision {
        COLOR_PATTERN_SUBDIVISION
    } else {
        egui::Color32::TRANSPARENT
    };

    let row_text_color = if is_playback_row {
        COLOR_PATTERN_PLAYBACK_TEXT
    } else {
        COLOR_TEXT_DIM
    };

    row.col(|ui| {
        fill_cell(ui, row_bg);
        ui.add_space(CELL_PAD);
        ui.label(
            RichText::new(format!("{:02}", row_idx + 1))
                .font(FONT)
                .color(row_text_color),
        );
        ui.add_space(CELL_PAD);
    });

    let sel_bounds = app.selection_bounds();
    let has_selection = app.cursor.selection_anchor.is_some();

    for ch in 0..channels {
        let is_cursor_ch_row = app.mode == Mode::Edit
            && ch == app.cursor.channel
            && row_idx == app.cursor.row
            && !has_selection;
        let in_selection =
            sel_bounds.is_some_and(|(min_ch, max_ch, min_row, max_row, _min_sub, _max_sub)| {
                if row_idx < min_row || row_idx > max_row {
                    return false;
                }
                if ch < min_ch || ch > max_ch {
                    return false;
                }
                true
            });
        let sub_selected = |sub: SubColumn| -> bool {
            if !in_selection {
                return false;
            }
            let Some((min_ch, max_ch, _min_row, _max_row, min_sub, max_sub)) = sel_bounds else {
                return false;
            };
            let flat = ch * 4 + sub as usize;
            let sel_start = min_ch * 4 + min_sub as usize;
            let sel_end = max_ch * 4 + max_sub as usize;
            flat >= sel_start && flat <= sel_end
        };
        let pat = app.project.current_pattern();
        let mut cell = pat.get(ch, row_idx);
        let mut inst_val = pat.get_panning(ch, row_idx);
        let mut volume_val = pat.get_volume(ch, row_idx);
        let mut effect_cmd = pat.get_effect(ch, row_idx);

        if let Some(ref preview) = app.move_preview
            && let Some((min_ch, _, min_row, _, _, _)) = sel_bounds {
                let ch_off = ch.wrapping_sub(min_ch);
                let row_off = row_idx.wrapping_sub(min_row);
                if in_selection
                    && let Some((_, _, p_cell, p_inst, p_vol, p_fx)) = preview
                        .cells
                        .iter()
                        .find(|(co, ro, _, _, _, _)| *co == ch_off && *ro == row_off)
                    {
                        if preview.move_notes {
                            cell = *p_cell;
                        }
                        if preview.move_pan {
                            inst_val = *p_inst;
                        }
                        if preview.move_vol {
                            volume_val = *p_vol;
                        }
                        if preview.move_fx {
                            effect_cmd = *p_fx;
                        }
                    }
            }

        let note_text = match cell {
            Cell::NoteOn(note) => note.name(),
            Cell::NoteOff => "OFF".to_string(),
            Cell::Empty => "\u{00b7}\u{00b7}\u{00b7}".to_string(),
        };
        let note_data_color = if matches!(cell, Cell::Empty) {
            COLOR_TEXT_DIM
        } else if matches!(cell, Cell::NoteOff) {
            COLOR_PATTERN_NOTE_OFF
        } else {
            COLOR_PATTERN_NOTE
        };

        row.col(|ui| {
            draw_left_border(ui);
            draw_sub_column(
                ui,
                app,
                ch,
                row_idx,
                SubColumn::Note,
                &note_text,
                matches!(cell, Cell::Empty),
                note_data_color,
                is_cursor_ch_row,
                sub_selected(SubColumn::Note),
                is_playback_row,
                row_bg,
                CELL_PAD,
                CELL_PAD_HALF,
            );
        });

        let inst_text = panning_display(inst_val);
        row.col(|ui| {
            draw_sub_column(
                ui,
                app,
                ch,
                row_idx,
                SubColumn::Panning,
                &inst_text,
                inst_val.is_none(),
                COLOR_PATTERN_PANNING,
                is_cursor_ch_row,
                sub_selected(SubColumn::Panning),
                is_playback_row,
                row_bg,
                CELL_PAD_HALF,
                CELL_PAD_HALF,
            );
        });

        let vol_text = volume_display(volume_val);
        row.col(|ui| {
            draw_sub_column(
                ui,
                app,
                ch,
                row_idx,
                SubColumn::Volume,
                &vol_text,
                volume_val.is_none(),
                COLOR_PATTERN_VOLUME,
                is_cursor_ch_row,
                sub_selected(SubColumn::Volume),
                is_playback_row,
                row_bg,
                CELL_PAD_HALF,
                CELL_PAD_HALF,
            );
        });

        let effect_text = effect_display(effect_cmd);
        row.col(|ui| {
            draw_sub_column(
                ui,
                app,
                ch,
                row_idx,
                SubColumn::Effect,
                &effect_text,
                effect_cmd.is_none(),
                COLOR_PATTERN_EFFECT,
                is_cursor_ch_row,
                sub_selected(SubColumn::Effect),
                is_playback_row,
                row_bg,
                CELL_PAD_HALF,
                CELL_PAD,
            );
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_sub_column(
    ui: &mut egui::Ui,
    app: &mut App,
    ch: usize,
    row_idx: usize,
    sub: SubColumn,
    text: &str,
    is_empty: bool,
    data_color: egui::Color32,
    is_cursor_ch_row: bool,
    is_selected: bool,
    is_playback_row: bool,
    row_bg: egui::Color32,
    pad_left: f32,
    pad_right: f32,
) {
    let is_cursor = is_cursor_ch_row && app.cursor.sub_column == sub;

    let bg = if is_cursor {
        COLOR_PATTERN_CURSOR_BG
    } else if is_selected {
        COLOR_PATTERN_SELECTION_BG
    } else {
        row_bg
    };
    fill_cell(ui, bg);

    let color = if is_cursor {
        COLOR_PATTERN_CURSOR_TEXT
    } else if is_selected {
        COLOR_PATTERN_SELECTION_TEXT
    } else if is_empty {
        COLOR_TEXT_DIM
    } else if is_playback_row {
        COLOR_PATTERN_PLAYBACK_TEXT
    } else {
        data_color
    };

    let mut rt = RichText::new(text).font(FONT).color(color);
    if is_cursor {
        rt = rt.strong();
    }

    ui.add_space(pad_left);
    ui.label(rt);
    ui.add_space(pad_right);

    let pointer_pos = ui.input(|i| i.pointer.hover_pos());
    if pointer_pos.is_some_and(|p| ui.max_rect().contains(p)) {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        if ui.input(|i| i.pointer.primary_pressed()) {
            app.clear_selection();
            app.cursor.sub_column = sub;
            app.set_cursor(ch, row_idx);
            if app.mode != Mode::Edit {
                app.mode = Mode::Edit;
            }
        }
    }
}
