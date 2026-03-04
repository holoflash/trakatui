use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, FontId, RichText, Sense, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::app::{App, Mode, SubColumn};
use crate::project::{Cell, effect_display, instrument_display, volume_display};

use super::{
    COLOR_LAYOUT_BG_DARK, COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT, COLOR_PATTERN_EFFECT,
    COLOR_PATTERN_INSTRUMENT, COLOR_PATTERN_NOTE, COLOR_PATTERN_NOTE_OFF,
    COLOR_PATTERN_PLAYBACK_HIGHLIGHT, COLOR_PATTERN_PLAYBACK_TEXT, COLOR_PATTERN_SELECTION_BG,
    COLOR_PATTERN_SELECTION_TEXT, COLOR_PATTERN_SUBDIVISION, COLOR_PATTERN_VOLUME, COLOR_TEXT,
    COLOR_TEXT_DIM,
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
                        .column(col);

                    for _ in 0..channels {
                        table = table.column(col).column(col).column(col).column(col);
                    }

                    if app.follow_playback && app.playback.playing {
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
                        .header(ROW_HEIGHT, |mut header| {
                            draw_header_row(&mut header, channels, &mut app.muted_channels);
                        })
                        .body(|body| {
                            body.rows(ROW_HEIGHT, app.project.current_pattern().rows, |mut row| {
                                draw_body_row(&mut row, app, channels);
                            });
                        });
                });
        });
}

fn draw_header_row(
    header: &mut egui_extras::TableRow<'_, '_>,
    channels: usize,
    muted: &mut Vec<bool>,
) {
    header.col(|ui| {
        ui.add_space(CELL_PAD);
    });

    for ch in 0..channels {
        header.col(|ui| {
            draw_left_border(ui);

            let is_muted = muted.get(ch).copied().unwrap_or(false);

            let label = if is_muted {
                format!("M{}", ch + 1)
            } else {
                format!("{}", ch + 1)
            };

            let color = if is_muted {
                COLOR_MUTED
            } else {
                COLOR_TEXT_DIM
            };

            ui.add_space(CELL_PAD);
            let response = ui.add(
                egui::Label::new(RichText::new(&label).font(FONT).color(color))
                    .sense(Sense::click()),
            );

            if response.hovered() && !is_muted {
                ui.painter().text(
                    response.rect.left_center() + egui::vec2(0.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    &label,
                    FONT,
                    COLOR_TEXT,
                );
            }

            if response.clicked() {
                if ch >= muted.len() {
                    muted.resize(ch + 1, false);
                }
                muted[ch] = !muted[ch];
            }
        });
        for _ in 0..3 {
            header.col(|ui| {
                ui.add_space(CELL_PAD);
            });
        }
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
            RichText::new(format!("{:02}", row_idx))
                .font(FONT)
                .color(row_text_color),
        );
        ui.add_space(CELL_PAD);
    });

    let sel_bounds = app.selection_bounds();

    for ch in 0..channels {
        let is_cursor_ch_row =
            app.mode == Mode::Edit && ch == app.cursor.channel && row_idx == app.cursor.row;
        let is_cursor_note = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Note;
        let is_cursor_inst = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Instrument;
        let is_cursor_volume = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Volume;
        let is_cursor_effect = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Effect;
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
        let is_note_selected = sub_selected(SubColumn::Note);
        let is_inst_selected = sub_selected(SubColumn::Instrument);
        let is_vol_selected = sub_selected(SubColumn::Volume);
        let is_fx_selected = sub_selected(SubColumn::Effect);

        let cell = app.project.current_pattern().get(ch, row_idx);
        let inst_val = app.project.current_pattern().get_instrument(ch, row_idx);
        let volume_val = app.project.current_pattern().get_volume(ch, row_idx);
        let effect_cmd = app.project.current_pattern().get_effect(ch, row_idx);

        row.col(|ui| {
            let note_bg = if is_cursor_note {
                COLOR_PATTERN_CURSOR_BG
            } else if is_note_selected {
                COLOR_PATTERN_SELECTION_BG
            } else {
                row_bg
            };

            fill_cell(ui, note_bg);
            draw_left_border(ui);

            let cell_text = match cell {
                Cell::NoteOn(note) => note.name(),
                Cell::NoteOff => "OFF".to_string(),
                Cell::Empty => "···".to_string(),
            };

            let note_color = if is_cursor_note {
                COLOR_PATTERN_CURSOR_TEXT
            } else if is_note_selected {
                COLOR_PATTERN_SELECTION_TEXT
            } else if matches!(cell, Cell::Empty) {
                COLOR_TEXT_DIM
            } else if is_playback_row {
                COLOR_PATTERN_PLAYBACK_TEXT
            } else if matches!(cell, Cell::NoteOff) {
                COLOR_PATTERN_NOTE_OFF
            } else {
                COLOR_PATTERN_NOTE
            };

            let mut note_rt = RichText::new(&cell_text).font(FONT).color(note_color);
            if is_cursor_note {
                note_rt = note_rt.strong();
            }

            ui.add_space(CELL_PAD);
            let response = ui.label(note_rt);
            ui.add_space(CELL_PAD_HALF);

            if response.clicked() {
                app.cursor.sub_column = SubColumn::Note;
                app.set_cursor(ch, row_idx);
                if app.mode != Mode::Edit {
                    app.mode = Mode::Edit;
                }
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });

        row.col(|ui| {
            let inst_bg = if is_cursor_inst {
                COLOR_PATTERN_CURSOR_BG
            } else if is_inst_selected {
                COLOR_PATTERN_SELECTION_BG
            } else {
                row_bg
            };

            fill_cell(ui, inst_bg);

            let inst_text = instrument_display(inst_val);

            let inst_color = if is_cursor_inst {
                COLOR_PATTERN_CURSOR_TEXT
            } else if is_inst_selected {
                COLOR_PATTERN_SELECTION_TEXT
            } else if inst_val.is_none() {
                COLOR_TEXT_DIM
            } else if is_playback_row {
                COLOR_PATTERN_PLAYBACK_TEXT
            } else {
                COLOR_PATTERN_INSTRUMENT
            };

            let mut inst_rt = RichText::new(&inst_text).font(FONT).color(inst_color);
            if is_cursor_inst {
                inst_rt = inst_rt.strong();
            }

            ui.add_space(CELL_PAD_HALF);
            let response = ui.label(inst_rt);
            ui.add_space(CELL_PAD_HALF);

            if response.clicked() {
                app.cursor.sub_column = SubColumn::Instrument;
                app.cursor.instrument_edit_pos = 0;
                app.set_cursor(ch, row_idx);
                if app.mode != Mode::Edit {
                    app.mode = Mode::Edit;
                }
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });

        row.col(|ui| {
            let vol_bg = if is_cursor_volume {
                COLOR_PATTERN_CURSOR_BG
            } else if is_vol_selected {
                COLOR_PATTERN_SELECTION_BG
            } else {
                row_bg
            };

            fill_cell(ui, vol_bg);

            let vol_text = volume_display(volume_val);

            let vol_color = if is_cursor_volume {
                COLOR_PATTERN_CURSOR_TEXT
            } else if is_vol_selected {
                COLOR_PATTERN_SELECTION_TEXT
            } else if volume_val.is_none() {
                COLOR_TEXT_DIM
            } else if is_playback_row {
                COLOR_PATTERN_PLAYBACK_TEXT
            } else {
                COLOR_PATTERN_VOLUME
            };

            let mut vol_rt = RichText::new(&vol_text).font(FONT).color(vol_color);
            if is_cursor_volume {
                vol_rt = vol_rt.strong();
            }

            ui.add_space(CELL_PAD_HALF);
            let response = ui.label(vol_rt);
            ui.add_space(CELL_PAD_HALF);

            if response.clicked() {
                app.cursor.sub_column = SubColumn::Volume;
                app.cursor.volume_edit_pos = 0;
                app.set_cursor(ch, row_idx);
                if app.mode != Mode::Edit {
                    app.mode = Mode::Edit;
                }
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });

        row.col(|ui| {
            let fx_bg = if is_cursor_effect {
                COLOR_PATTERN_CURSOR_BG
            } else if is_fx_selected {
                COLOR_PATTERN_SELECTION_BG
            } else {
                row_bg
            };

            fill_cell(ui, fx_bg);

            let effect_text = effect_display(effect_cmd);

            let fx_color = if is_cursor_effect {
                COLOR_PATTERN_CURSOR_TEXT
            } else if is_fx_selected {
                COLOR_PATTERN_SELECTION_TEXT
            } else if effect_cmd.is_none() {
                COLOR_TEXT_DIM
            } else if is_playback_row {
                COLOR_PATTERN_PLAYBACK_TEXT
            } else {
                COLOR_PATTERN_EFFECT
            };

            let mut fx_rt = RichText::new(&effect_text).font(FONT).color(fx_color);
            if is_cursor_effect {
                fx_rt = fx_rt.strong();
            }

            ui.add_space(CELL_PAD_HALF);
            let response = ui.label(fx_rt);
            ui.add_space(CELL_PAD);

            if response.clicked() {
                app.cursor.sub_column = SubColumn::Effect;
                app.cursor.effect_edit_pos = 0;
                app.set_cursor(ch, row_idx);
                if app.mode != Mode::Edit {
                    app.mode = Mode::Edit;
                }
            }
            if response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        });
    }
}
