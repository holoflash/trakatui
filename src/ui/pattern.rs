use eframe::egui::{self, FontId, RichText, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::app::{App, Mode, SubColumn};
use crate::project::{Cell, effect_display};

use super::{
    COLOR_LAYOUT_BG_DARK, COLOR_PATTERN_CURSOR_BG, COLOR_PATTERN_CURSOR_TEXT, COLOR_PATTERN_EFFECT,
    COLOR_PATTERN_NOTE, COLOR_PATTERN_NOTE_OFF, COLOR_PATTERN_PLAYBACK_HIGHLIGHT,
    COLOR_PATTERN_PLAYBACK_TEXT, COLOR_PATTERN_SELECTION_BG, COLOR_PATTERN_SELECTION_TEXT,
    COLOR_PATTERN_SUBDIVISION, COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM,
};

const FONT: FontId = FontId::monospace(14.0);
const ROW_HEIGHT: f32 = 18.0;
const CELL_PAD: f32 = 8.0;

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
                .inner_margin(egui::Margin::symmetric(16, 12)),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            let channels = app.project.pattern.channels;
            let col = Column::auto().at_least(0.0);

            let mut table = TableBuilder::new(ui)
                .striped(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(col);

            for _ in 0..channels {
                table = table.column(col).column(col);
            }

            table
                .header(ROW_HEIGHT, |mut header| {
                    draw_header_row(&mut header, app, channels);
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, app.project.pattern.rows, |mut row| {
                        draw_body_row(&mut row, app, channels);
                    });
                });
        });
}

fn draw_header_row(header: &mut egui_extras::TableRow<'_, '_>, app: &App, channels: usize) {
    header.col(|ui| {
        ui.add_space(CELL_PAD);
    });

    for ch in 0..channels {
        header.col(|ui| {
            draw_left_border(ui);

            let waveform = app.project.channel_settings[ch].waveform;
            let is_synth_channel = app.mode == Mode::SynthEdit && ch == app.cursor.synth_channel;
            if is_synth_channel {
                fill_cell(ui, COLOR_PATTERN_CURSOR_BG);
            }

            ui.add_space(CELL_PAD);
            ui.label(
                RichText::new(waveform.name())
                    .font(FONT)
                    .color(if is_synth_channel {
                        COLOR_TEXT_ACTIVE
                    } else {
                        COLOR_TEXT_DIM
                    }),
            );
        });
        header.col(|ui| {
            let is_synth_channel = app.mode == Mode::SynthEdit && ch == app.cursor.synth_channel;
            if is_synth_channel {
                fill_cell(ui, COLOR_PATTERN_CURSOR_BG);
            }
            ui.add_space(CELL_PAD);
        });
    }
}

fn draw_body_row(row: &mut egui_extras::TableRow<'_, '_>, app: &mut App, channels: usize) {
    let row_idx = row.index();
    let is_playback_row = app.playback.playing && row_idx == app.playback.row;
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

    for ch in 0..channels {
        let is_cursor_ch_row =
            app.mode == Mode::Edit && ch == app.cursor.channel && row_idx == app.cursor.row;
        let is_cursor_note = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Note;
        let is_cursor_effect = is_cursor_ch_row && app.cursor.sub_column == SubColumn::Effect;
        let in_selection = sel_bounds.is_some_and(|(min_ch, max_ch, min_row, max_row)| {
            ch >= min_ch && ch <= max_ch && row_idx >= min_row && row_idx <= max_row
        });
        let is_note_selected = in_selection && app.cursor.sub_column == SubColumn::Note;
        let is_fx_selected = in_selection && app.cursor.sub_column == SubColumn::Effect;

        let cell = app.project.pattern.get(ch, row_idx);
        let effect_cmd = app.project.pattern.get_effect(ch, row_idx);

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

            if response.clicked() {
                app.cursor.sub_column = SubColumn::Note;
                app.set_cursor(ch, row_idx);
                if app.mode != Mode::Edit {
                    app.mode = Mode::Edit;
                }
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
            } else if is_playback_row {
                COLOR_PATTERN_PLAYBACK_TEXT
            } else {
                COLOR_PATTERN_EFFECT
            };

            let mut fx_rt = RichText::new(&effect_text).font(FONT).color(fx_color);
            if is_cursor_effect {
                fx_rt = fx_rt.strong();
            }

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
        });
    }
}
