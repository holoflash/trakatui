use eframe::egui::{self, FontId, RichText, Stroke};
use egui_extras::{Column, TableBuilder};

use crate::app::{App, Mode};
use crate::pattern::Cell;

use super::*;

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

            let channels = app.pattern.channels;

            let col = Column::auto().at_least(0.0);

            let mut table = TableBuilder::new(ui)
                .striped(false)
                .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                .column(col);

            for _ in 0..channels {
                table = table.column(col);
            }

            table
                .header(ROW_HEIGHT, |mut header| {
                    header.col(|ui| {
                        ui.add_space(CELL_PAD);
                    });

                    for ch in 0..channels {
                        header.col(|ui| {
                            draw_left_border(ui);

                            let waveform = app.channel_settings[ch].waveform;
                            let is_synth_channel =
                                app.mode == Mode::SynthEdit && ch == app.synth_channel;
                            if is_synth_channel {
                                fill_cell(ui, COLOR_PATTERN_CURSOR_BG);
                            }

                            ui.add_space(CELL_PAD);
                            ui.label(RichText::new(waveform.name()).font(FONT).color(
                                if is_synth_channel {
                                    COLOR_TEXT_ACTIVE
                                } else {
                                    COLOR_TEXT_DIM
                                },
                            ));
                            ui.add_space(CELL_PAD);
                        });
                    }
                })
                .body(|body| {
                    body.rows(ROW_HEIGHT, app.pattern.rows, |mut row| {
                        let row_idx = row.index();
                        let is_playback_row = app.playing && row_idx == app.playback_row;
                        let is_subdivision = row_idx.is_multiple_of(app.subdivision);

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

                        // Row number cell
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

                        // Channel cells
                        for ch in 0..channels {
                            row.col(|ui| {
                                let is_cursor = app.mode == Mode::Edit
                                    && ch == app.cursor_channel
                                    && row_idx == app.cursor_row;
                                let is_selected =
                                    sel_bounds.is_some_and(|(min_ch, max_ch, min_row, max_row)| {
                                        ch >= min_ch
                                            && ch <= max_ch
                                            && row_idx >= min_row
                                            && row_idx <= max_row
                                    });

                                let cell = app.pattern.get(ch, row_idx);
                                let cell_text = match cell {
                                    Cell::NoteOn(note) => note.name(),
                                    Cell::NoteOff => "OFF".to_string(),
                                    Cell::Empty => "···".to_string(),
                                };

                                let cell_bg = if is_cursor {
                                    COLOR_PATTERN_CURSOR_BG
                                } else if is_selected {
                                    COLOR_PATTERN_SELECTION_BG
                                } else {
                                    row_bg
                                };

                                fill_cell(ui, cell_bg);
                                draw_left_border(ui);

                                let text_color = if is_cursor {
                                    COLOR_PATTERN_CURSOR_TEXT
                                } else if is_selected {
                                    COLOR_PATTERN_SELECTION_TEXT
                                } else if is_playback_row {
                                    COLOR_PATTERN_PLAYBACK_TEXT
                                } else if matches!(cell, Cell::NoteOff) {
                                    COLOR_PATTERN_NOTE_OFF
                                } else {
                                    COLOR_PATTERN_NOTE
                                };

                                let mut text =
                                    RichText::new(&cell_text).font(FONT).color(text_color);
                                if is_cursor {
                                    text = text.strong();
                                }

                                ui.add_space(CELL_PAD);
                                let response = ui.label(text);
                                ui.add_space(CELL_PAD);

                                if response.clicked() {
                                    app.set_cursor(ch, row_idx);
                                    if app.mode != Mode::Edit {
                                        app.mode = Mode::Edit;
                                    }
                                }
                            });
                        }
                    });
                });
        });
}
