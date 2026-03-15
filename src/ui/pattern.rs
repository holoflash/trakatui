use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, FontId, Stroke};

use crate::app::{App, Mode};
use crate::project::Cell;

use super::{
    COLOR_LAYOUT_BG_DARK, COLOR_PATTERN_BEATMARKER, COLOR_PATTERN_CURSOR_BG,
    COLOR_PATTERN_CURSOR_TEXT, COLOR_PATTERN_NOTE, COLOR_PATTERN_NOTE_OFF,
    COLOR_PATTERN_PLAYBACK_HIGHLIGHT, COLOR_PATTERN_PLAYBACK_TEXT, COLOR_PATTERN_SELECTION_BG,
    COLOR_PATTERN_SELECTION_TEXT, COLOR_PATTERN_SUBDIVISION, COLOR_TEXT_DIM,
};

const FONT: FontId = FontId::monospace(14.0);
const ROW_HEIGHT: f32 = 18.0;
const CELL_PAD: f32 = 8.0;
const ROW_NUM_WIDTH: f32 = 40.0;
const VOICE_COL_WIDTH: f32 = 42.0;

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

            let channels = app.project.channels;
            let voice_counts: Vec<usize> =
                (0..channels).map(|ch| app.voices_for_channel(ch)).collect();
            let pat = app.project.current_pattern();
            let max_rows = pat.rows;

            let total_voice_cols: usize = voice_counts.iter().sum();
            let content_width = ROW_NUM_WIDTH + total_voice_cols as f32 * VOICE_COL_WIDTH;
            let total_height = (max_rows as f32 * ROW_HEIGHT) + ROW_HEIGHT;

            egui::ScrollArea::both()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let visible_height = ui.available_height();

                    if app.playback.playing {
                        let target_y = (app.playback_row_display as f32 * ROW_HEIGHT + ROW_HEIGHT
                            - visible_height / 2.0
                            + ROW_HEIGHT / 2.0)
                            .max(0.0);
                        let diff = target_y - app.follow_scroll_offset;
                        if diff < -ROW_HEIGHT * 2.0 || diff.abs() < 0.5 {
                            app.follow_scroll_offset = target_y;
                        } else {
                            app.follow_scroll_offset += diff * 0.15;
                        }
                        ui.scroll_with_delta(egui::vec2(0.0, 0.0));
                    }

                    let (response, painter) = ui.allocate_painter(
                        egui::vec2(content_width, total_height),
                        egui::Sense::click_and_drag(),
                    );
                    let origin = response.rect.min;

                    draw_header(&painter, origin, channels, &voice_counts);

                    let body_origin = origin + egui::vec2(0.0, ROW_HEIGHT);

                    let cur_ch = app.cursor.channel;
                    let pat = app.project.current_pattern();
                    let cur_track_rows = pat.track_rows(cur_ch);
                    let cur_primary = pat.primary_row_group_for_track(cur_ch);
                    let cur_secondary = pat.secondary_row_group_for_track(cur_ch);
                    let playback_row = if app.playback.playing {
                        Some(app.playback_row_display)
                    } else {
                        None
                    };
                    draw_row_numbers(
                        &painter,
                        body_origin,
                        max_rows,
                        cur_track_rows,
                        playback_row,
                        cur_primary,
                        cur_secondary,
                    );

                    let sel_bounds = app.selection_bounds();
                    let has_selection = app.cursor.selection_anchor.is_some();

                    let mut col_x = ROW_NUM_WIDTH;
                    for (ch, &voices) in voice_counts.iter().enumerate().take(channels) {
                        let track_rows = app.project.current_pattern().track_rows(ch);
                        let cell_h = if track_rows > 0 {
                            (max_rows as f32 * ROW_HEIGHT) / track_rows as f32
                        } else {
                            ROW_HEIGHT
                        };

                        let ch_primary = app
                            .project
                            .current_pattern()
                            .primary_row_group_for_track(ch);
                        let ch_secondary = app
                            .project
                            .current_pattern()
                            .secondary_row_group_for_track(ch);

                        for v in 0..voices {
                            let is_first_voice = v == 0;
                            let vx = col_x + v as f32 * VOICE_COL_WIDTH;

                            if is_first_voice {
                                let top = body_origin + egui::vec2(vx, 0.0);
                                let bottom = top + egui::vec2(0.0, max_rows as f32 * ROW_HEIGHT);
                                painter
                                    .line_segment([top, bottom], Stroke::new(1.0, COLOR_TEXT_DIM));
                            }

                            for t in 0..track_rows {
                                let y = body_origin.y + t as f32 * cell_h;
                                let cell_rect = egui::Rect::from_min_size(
                                    egui::pos2(body_origin.x + vx, y),
                                    egui::vec2(VOICE_COL_WIDTH, cell_h),
                                );

                                let is_playback_cell = app.playback.playing && {
                                    let row_top = t as f32 / track_rows as f32;
                                    let row_bot = (t + 1) as f32 / track_rows as f32;
                                    let pb_frac = app.playback_row_display as f32 / max_rows as f32;
                                    pb_frac >= row_top && pb_frac < row_bot
                                };

                                let ch_is_beat = ch_primary > 0 && t.is_multiple_of(ch_primary);
                                let ch_is_subdivision = ch_secondary > 0
                                    && !ch_is_beat
                                    && t.is_multiple_of(ch_secondary);

                                let row_bg = if is_playback_cell {
                                    COLOR_PATTERN_PLAYBACK_HIGHLIGHT
                                } else if ch_is_beat {
                                    COLOR_PATTERN_BEATMARKER
                                } else if ch_is_subdivision {
                                    COLOR_PATTERN_SUBDIVISION
                                } else {
                                    egui::Color32::TRANSPARENT
                                };

                                let is_cursor_cell = !app.playback.playing
                                    && app.mode == Mode::Edit
                                    && ch == app.cursor.channel
                                    && v == app.cursor.voice
                                    && t == app.cursor.row
                                    && !has_selection;

                                let is_selected = sel_bounds.is_some_and(
                                    |(min_ch, max_ch, min_v, max_v, min_row, max_row)| {
                                        if t < min_row || t > max_row || ch < min_ch || ch > max_ch
                                        {
                                            return false;
                                        }
                                        if min_ch == max_ch {
                                            v >= min_v && v <= max_v
                                        } else if ch == min_ch {
                                            v >= min_v
                                        } else if ch == max_ch {
                                            v <= max_v
                                        } else {
                                            true
                                        }
                                    },
                                );

                                let pat = app.project.current_pattern();
                                let mut cell = if v < pat.voice_count(ch) && t < pat.track_rows(ch)
                                {
                                    pat.get(ch, v, t)
                                } else {
                                    Cell::Empty
                                };

                                if let Some(ref preview) = app.move_preview
                                    && let Some((min_ch, _, min_v, _, min_row, _)) = sel_bounds
                                {
                                    let base_flat = app.flat_col(min_ch, min_v);
                                    let cur_flat = app.flat_col(ch, v);
                                    let col_off = cur_flat.wrapping_sub(base_flat);
                                    let row_off = t.wrapping_sub(min_row);
                                    if is_selected
                                        && let Some((_, _, p_cell)) = preview
                                            .cells
                                            .iter()
                                            .find(|(co, ro, _)| *co == col_off && *ro == row_off)
                                    {
                                        cell = *p_cell;
                                    }
                                }

                                let bg = if is_cursor_cell {
                                    COLOR_PATTERN_CURSOR_BG
                                } else if is_selected {
                                    COLOR_PATTERN_SELECTION_BG
                                } else {
                                    row_bg
                                };

                                if bg != egui::Color32::TRANSPARENT {
                                    let fill_rect = egui::Rect::from_min_size(
                                        cell_rect.left_top(),
                                        egui::vec2(VOICE_COL_WIDTH, ROW_HEIGHT.min(cell_h)),
                                    );
                                    painter.rect_filled(fill_rect, 0.0, bg);
                                }

                                let is_muted = app.muted_channels.get(ch).copied().unwrap_or(false);
                                let note_text = match cell {
                                    Cell::NoteOn(note) => note.name(),
                                    Cell::NoteOff => "OFF".to_string(),
                                    Cell::Empty => "\u{00b7}\u{00b7}\u{00b7}".to_string(),
                                };

                                let text_color = if is_cursor_cell {
                                    COLOR_PATTERN_CURSOR_TEXT
                                } else if is_selected {
                                    COLOR_PATTERN_SELECTION_TEXT
                                } else if matches!(cell, Cell::Empty) || is_muted {
                                    COLOR_TEXT_DIM
                                } else if is_playback_cell {
                                    COLOR_PATTERN_PLAYBACK_TEXT
                                } else if matches!(cell, Cell::NoteOff) {
                                    COLOR_PATTERN_NOTE_OFF
                                } else {
                                    COLOR_PATTERN_NOTE
                                };

                                let text_pos = cell_rect.left_top() + egui::vec2(CELL_PAD, 1.0);
                                painter.text(
                                    text_pos,
                                    egui::Align2::LEFT_TOP,
                                    &note_text,
                                    FONT,
                                    text_color,
                                );
                            }
                        }
                        col_x += voices as f32 * VOICE_COL_WIDTH;
                    }

                    if let Some(pos) = response.interact_pointer_pos() {
                        let rel = pos - body_origin;
                        if rel.y >= 0.0 {
                            let mut cx = 0.0;
                            let mut found_ch = None;
                            let mut found_v = None;
                            for (ch, &voices) in voice_counts.iter().enumerate().take(channels) {
                                for v in 0..voices {
                                    let vx = ROW_NUM_WIDTH + cx;
                                    if rel.x >= vx && rel.x < vx + VOICE_COL_WIDTH {
                                        found_ch = Some(ch);
                                        found_v = Some(v);
                                    }
                                    cx += VOICE_COL_WIDTH;
                                }
                            }
                            if let (Some(ch), Some(v)) = (found_ch, found_v) {
                                let track_rows = app.project.current_pattern().track_rows(ch);
                                let cell_h = if track_rows > 0 {
                                    (max_rows as f32 * ROW_HEIGHT) / track_rows as f32
                                } else {
                                    ROW_HEIGHT
                                };
                                let track_row = (rel.y / cell_h).floor() as usize;
                                let track_row = track_row.min(track_rows.saturating_sub(1));
                                if response.clicked() {
                                    app.clear_selection();
                                    app.set_cursor(ch, v, track_row);
                                    if app.mode != Mode::Edit {
                                        app.mode = Mode::Edit;
                                    }
                                }
                                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                            }
                        }
                    }
                });
        });
}

fn draw_header(
    painter: &egui::Painter,
    origin: egui::Pos2,
    channels: usize,
    voice_counts: &[usize],
) {
    let header_rect = egui::Rect::from_min_size(origin, egui::vec2(ROW_NUM_WIDTH, ROW_HEIGHT));
    painter.rect_filled(header_rect, 0.0, COLOR_LAYOUT_BG_DARK);

    let mut col_x = ROW_NUM_WIDTH;
    for (ch, &voices) in voice_counts.iter().enumerate().take(channels) {
        let x = origin.x + col_x;
        let top = egui::pos2(x, origin.y);
        let bottom = egui::pos2(x, origin.y + ROW_HEIGHT);
        painter.line_segment([top, bottom], Stroke::new(1.0, COLOR_TEXT_DIM));

        let text_pos = egui::pos2(x + CELL_PAD, origin.y + ROW_HEIGHT / 2.0);
        painter.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            format!("{}", ch + 1),
            FONT,
            COLOR_TEXT_DIM,
        );

        col_x += voices as f32 * VOICE_COL_WIDTH;
    }
}

fn draw_row_numbers(
    painter: &egui::Painter,
    body_origin: egui::Pos2,
    max_rows: usize,
    track_rows: usize,
    playback_row: Option<usize>,
    primary: usize,
    secondary: usize,
) {
    let cell_h = if track_rows > 0 {
        (max_rows as f32 * ROW_HEIGHT) / track_rows as f32
    } else {
        ROW_HEIGHT
    };

    for r in 0..track_rows {
        let y = body_origin.y + r as f32 * cell_h;
        let rect = egui::Rect::from_min_size(
            egui::pos2(body_origin.x, y),
            egui::vec2(ROW_NUM_WIDTH, cell_h),
        );

        let is_playback = playback_row.is_some_and(|pb| {
            let row_top = r as f32 / track_rows as f32;
            let row_bot = (r + 1) as f32 / track_rows as f32;
            let pb_frac = pb as f32 / max_rows as f32;
            pb_frac >= row_top && pb_frac < row_bot
        });
        let is_beat = primary > 0 && r.is_multiple_of(primary);
        let is_sub = secondary > 0 && !is_beat && r.is_multiple_of(secondary);

        let bg = if is_playback {
            COLOR_PATTERN_PLAYBACK_HIGHLIGHT
        } else if is_beat {
            COLOR_PATTERN_BEATMARKER
        } else if is_sub {
            COLOR_PATTERN_SUBDIVISION
        } else {
            egui::Color32::TRANSPARENT
        };
        if bg != egui::Color32::TRANSPARENT {
            let fill_rect = egui::Rect::from_min_size(
                rect.left_top(),
                egui::vec2(ROW_NUM_WIDTH, ROW_HEIGHT.min(cell_h)),
            );
            painter.rect_filled(fill_rect, 0.0, bg);
        }

        let text_color = if is_playback {
            COLOR_PATTERN_PLAYBACK_TEXT
        } else {
            COLOR_TEXT_DIM
        };
        painter.text(
            rect.left_top() + egui::vec2(CELL_PAD, 2.0),
            egui::Align2::LEFT_TOP,
            format!("{:02}", r + 1),
            FONT,
            text_color,
        );
    }
}
