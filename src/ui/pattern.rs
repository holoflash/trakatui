use eframe::egui::{self, FontId, RichText, ScrollArea, Stroke};

use crate::app::{App, Mode};
use crate::pattern::Cell;

use super::*;

pub fn draw_pattern(ctx: &egui::Context, app: &mut App) {
    let border_color = if app.mode == Mode::Edit {
        COLOR_LAYOUT_BORDER_ACTIVE
    } else {
        COLOR_LAYOUT_BORDER
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_DARK)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .stroke(Stroke::new(1.0, border_color)),
        )
        .show(ctx, |ui| {
            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;

                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("     ")
                                .font(FontId::monospace(13.0))
                                .color(COLOR_TEXT_DIM),
                        );
                        for ch in 0..app.pattern.channels {
                            let waveform = app.channel_settings[ch].waveform;
                            let is_synth_channel =
                                app.mode == Mode::SynthEdit && ch == app.synth_channel;
                            let color = if is_synth_channel {
                                COLOR_PATTERN_CURSOR_TEXT
                            } else {
                                INST_COLORS[ch % INST_COLORS.len()]
                            };
                            let bg = if is_synth_channel {
                                COLOR_PATTERN_CURSOR_BG
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("│ ")
                                        .font(FontId::monospace(13.0))
                                        .color(COLOR_TEXT_DIM),
                                );
                                ui.label(
                                    RichText::new(waveform.name())
                                        .font(FontId::monospace(13.0))
                                        .color(color)
                                        .background_color(bg)
                                        .strong(),
                                );
                                ui.label(
                                    RichText::new(" ")
                                        .font(FontId::monospace(13.0))
                                        .color(COLOR_TEXT_DIM),
                                );
                            });
                        }
                    });

                    ui.add_space(2.0);

                    for row in 0..app.pattern.rows {
                        ui.horizontal(|ui| {
                            let is_playback_row = app.playing && row == app.playback_row;
                            let is_subdivision = row.is_multiple_of(app.subdivision);

                            let row_bg_color = if is_playback_row {
                                COLOR_PATTERN_PLAYBACK_HIGHLIGHT
                            } else if is_subdivision {
                                COLOR_PATTERN_SUBDIVISION
                            } else {
                                egui::Color32::TRANSPARENT
                            };

                            let row_text_color = if is_playback_row {
                                COLOR_PATTERN_PLAYBACK_TEXT
                            } else {
                                COLOR_PATTERN_NOTE
                            };

                            ui.label(
                                RichText::new(format!("  {:02} ", row + 1))
                                    .font(FontId::monospace(13.0))
                                    .color(row_text_color)
                                    .background_color(row_bg_color)
                                    .strong(),
                            );

                            let sel_bounds = app.selection_bounds();

                            for ch in 0..app.pattern.channels {
                                let is_cursor = app.mode == Mode::Edit
                                    && ch == app.cursor_channel
                                    && row == app.cursor_row;
                                let is_playback = app.playing && row == app.playback_row;
                                let is_selected =
                                    sel_bounds.is_some_and(|(min_ch, max_ch, min_row, max_row)| {
                                        ch >= min_ch
                                            && ch <= max_ch
                                            && row >= min_row
                                            && row <= max_row
                                    });
                                let cell = app.pattern.get(ch, row);
                                let cell_text = match cell {
                                    Cell::NoteOn(note) => note.name(),
                                    Cell::NoteOff => "OFF".to_string(),
                                    Cell::Empty => "···".to_string(),
                                };

                                let left = "│ ";
                                let right = " ";

                                let note_text = if is_cursor {
                                    RichText::new(&cell_text)
                                        .font(FontId::monospace(13.0))
                                        .color(COLOR_PATTERN_CURSOR_TEXT)
                                        .background_color(COLOR_PATTERN_CURSOR_BG)
                                        .strong()
                                } else if is_selected {
                                    RichText::new(&cell_text)
                                        .font(FontId::monospace(13.0))
                                        .color(COLOR_PATTERN_SELECTION_TEXT)
                                        .background_color(COLOR_PATTERN_SELECTION_BG)
                                } else if is_playback {
                                    RichText::new(&cell_text)
                                        .font(FontId::monospace(13.0))
                                        .color(COLOR_PATTERN_PLAYBACK_TEXT)
                                        .background_color(COLOR_PATTERN_PLAYBACK_HIGHLIGHT)
                                } else {
                                    match cell {
                                        Cell::NoteOn(_) => RichText::new(&cell_text)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_PATTERN_NOTE)
                                            .background_color(row_bg_color),
                                        Cell::NoteOff => RichText::new(&cell_text)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_PATTERN_NOTE_OFF)
                                            .background_color(row_bg_color),
                                        Cell::Empty => RichText::new(&cell_text)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_PATTERN_NOTE)
                                            .background_color(row_bg_color),
                                    }
                                };

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(left)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_TEXT_DIM)
                                            .background_color(row_bg_color),
                                    );
                                    let response = ui.label(note_text);
                                    ui.label(
                                        RichText::new(right)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_TEXT_DIM)
                                            .background_color(row_bg_color),
                                    );

                                    if response.clicked() {
                                        app.set_cursor(ch, row);
                                        if app.mode != Mode::Edit {
                                            app.mode = Mode::Edit;
                                        }
                                    }
                                });
                            }
                        });
                    }
                });
        });
}
