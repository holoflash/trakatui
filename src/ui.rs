use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, Stroke, Vec2};

use crate::app::{App, Mode, SettingsField};
use crate::pattern::Cell;
use crate::scale::root_name;
use crate::synth::CHANNEL_INSTRUMENTS;

const COLOR_LAYOUT_BG_DARK: Color32 = Color32::from_rgb(65, 65, 86);
const COLOR_LAYOUT_BG_PANEL: Color32 = Color32::from_rgb(170, 170, 170);

const COLOR_LAYOUT_BORDER: Color32 = Color32::from_rgb(145, 145, 161);
const COLOR_LAYOUT_BORDER_ACTIVE: Color32 = Color32::from_rgb(255, 255, 255);

const COLOR_TEXT_DIM: Color32 = Color32::from_rgb(145, 145, 161);
const COLOR_TEXT: Color32 = Color32::from_rgb(0, 0, 0);

const COLOR_MODE_EDIT: Color32 = Color32::from_rgb(0, 0, 0);
const COLOR_MODE_SETTINGS: Color32 = Color32::from_rgb(0, 0, 0);
const COLOR_MODE_PLAYING: Color32 = Color32::from_rgb(0, 0, 0);

const COLOR_ERROR: Color32 = Color32::from_rgb(255, 86, 85);
const COLOR_PATTERN_NOTE: Color32 = Color32::from_rgb(255, 255, 85);
const COLOR_PATTERN_NOTE_OFF: Color32 = Color32::from_rgb(255, 86, 85);
const COLOR_PATTERN_CURSOR_BG: Color32 = Color32::from_rgb(93, 93, 143);
const COLOR_PATTERN_CURSOR_TEXT: Color32 = Color32::from_rgb(255, 255, 255);

const COLOR_PATTERN_PLAYBACK_HIGHLIGHT: Color32 = Color32::from_rgb(93, 93, 143);
const COLOR_PATTERN_PLAYBACK_TEXT: Color32 = Color32::from_rgb(255, 255, 255);

const COLOR_PATTERN_SELECTION_BG: Color32 = Color32::from_rgb(80, 80, 140);
const COLOR_PATTERN_SELECTION_TEXT: Color32 = Color32::from_rgb(220, 220, 255);

const INST_COLORS: [Color32; 4] = [
    COLOR_PATTERN_NOTE,
    COLOR_PATTERN_NOTE,
    COLOR_PATTERN_NOTE,
    COLOR_PATTERN_NOTE,
];

pub fn draw(ctx: &egui::Context, app: &mut App) {
    draw_header(ctx, app);
    draw_footer(ctx, app);
    draw_settings_panel(ctx, app);
    draw_pattern(ctx, app);
}

fn draw_header(ctx: &egui::Context, app: &App) {
    egui::TopBottomPanel::top("header")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.add(
                    egui::Image::new(egui::include_image!("../psikat.png"))
                        .fit_to_exact_size(egui::Vec2::new(48.0, 48.0)),
                );
                ui.add_space(4.0);
                let (mode_str, mode_color) = if app.playing {
                    ("PLAYING", COLOR_MODE_PLAYING)
                } else {
                    match app.mode {
                        Mode::Edit => ("EDIT", COLOR_MODE_EDIT),
                        Mode::Settings => ("SETTINGS", COLOR_MODE_SETTINGS),
                    }
                };
                ui.label(
                    RichText::new(format!("[{}]", mode_str))
                        .font(FontId::monospace(14.0))
                        .color(mode_color)
                        .strong(),
                );
                ui.add_space(16.0);
                ui.label(
                    RichText::new(format!("Oct:{}", app.octave))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_MODE_SETTINGS),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("BPM:{}", app.bpm))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_TEXT),
                );
                ui.add_space(12.0);

                let root = root_name(app.transpose);
                let scale_name = app.scale_index.scale().name;
                ui.label(
                    RichText::new(format!("{} {}", root, scale_name))
                        .font(FontId::monospace(13.0))
                        .color(COLOR_MODE_PLAYING),
                );
            });
        });
}

fn draw_footer(ctx: &egui::Context, app: &App) {
    egui::TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .stroke(Stroke::new(1.0, COLOR_LAYOUT_BORDER)),
        )
        .show(ctx, |ui| {
            let help_text = match app.mode {
                Mode::Edit => {
                    "Z..M/Q..U:note  TAB:off  DEL:clear  ,/.:oct  ALT+\u{2190}\u{2191}\u{2192}\u{2193}:select  ENTER:play  2:settings"
                }
                _ if app.playing => "ENTER:stop  ESC:stop",
                Mode::Settings => {
                    "\u{2191}\u{2193}:select  \u{2190}\u{2192}:adjust  ENTER:confirm  1:pattern  ESC:back"
                }
            };
            ui.horizontal(|ui| {
                ui.centered_and_justified(|ui| {
                    ui.label(
                        RichText::new(help_text)
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_DIM),
                    );
                });
            });
        });
}

fn draw_settings_panel(ctx: &egui::Context, app: &mut App) {
    let border_color = if app.mode == Mode::Settings {
        COLOR_LAYOUT_BORDER_ACTIVE
    } else {
        COLOR_LAYOUT_BORDER
    };

    egui::SidePanel::right("settings")
        .resizable(false)
        .exact_width(280.0)
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .stroke(Stroke::new(1.0, border_color)),
        )
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Settings")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_MODE_SETTINGS)
                    .strong(),
            );
            ui.add_space(2.0);
            let sep_color = COLOR_TEXT_DIM;
            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    ui.cursor().left_top() + Vec2::new(240.0, 0.0),
                ],
                Stroke::new(1.0, sep_color),
            );
            ui.add_space(10.0);

            settings_row(
                ui,
                "BPM",
                &format!("{:>3}", app.bpm),
                app.settings_field == SettingsField::Bpm,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Length",
                &format!("{:>3}", app.pattern.rows),
                app.settings_field == SettingsField::PatternLength,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Scale",
                &format!("{:>9}", app.scale_index.scale().name),
                app.settings_field == SettingsField::Scale,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Transpose",
                &format!("{:>3}", app.transpose),
                app.settings_field == SettingsField::Transpose,
            );
            ui.add_space(12.0);

            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    ui.cursor().left_top() + Vec2::new(240.0, 0.0),
                ],
                Stroke::new(1.0, sep_color),
            );
            ui.add_space(12.0);

            let is_export = app.settings_field == SettingsField::ExportWav;
            let cursor_str = if is_export { " ▸ " } else { "   " };
            ui.horizontal(|ui| {
                if is_export {
                    ui.label(
                        RichText::new(cursor_str)
                            .font(FontId::monospace(13.0))
                            .color(COLOR_MODE_SETTINGS)
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new(cursor_str)
                            .font(FontId::monospace(13.0))
                            .color(COLOR_TEXT_DIM),
                    );
                }
                let export_text = RichText::new(" Export WAV ")
                    .font(FontId::monospace(13.0))
                    .strong();
                if is_export {
                    ui.label(
                        export_text
                            .color(COLOR_PATTERN_CURSOR_TEXT)
                            .background_color(COLOR_MODE_PLAYING),
                    );
                } else {
                    ui.label(export_text.color(COLOR_MODE_PLAYING));
                }
            });

            if let Some(ref msg) = app.status_message {
                ui.add_space(8.0);
                let color = if msg.starts_with("Exported") {
                    COLOR_MODE_PLAYING
                } else {
                    COLOR_ERROR
                };
                ui.label(
                    RichText::new(msg.as_str())
                        .font(FontId::monospace(11.0))
                        .color(color),
                );
            }
        });
}

fn settings_row(ui: &mut egui::Ui, label: &str, value: &str, active: bool) {
    ui.horizontal(|ui| {
        let cursor_str = if active { " ▸ " } else { "   " };
        let cursor_color = if active {
            COLOR_MODE_SETTINGS
        } else {
            COLOR_TEXT_DIM
        };
        ui.label(
            RichText::new(cursor_str)
                .font(FontId::monospace(13.0))
                .color(cursor_color)
                .strong(),
        );
        ui.label(
            RichText::new(format!("{:<10}", label))
                .font(FontId::monospace(13.0))
                .color(COLOR_TEXT),
        );
        if active {
            ui.label(
                RichText::new("◄")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT_DIM),
            );
            ui.label(
                RichText::new(value)
                    .font(FontId::monospace(13.0))
                    .color(COLOR_MODE_SETTINGS)
                    .strong(),
            );
            ui.label(
                RichText::new("►")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT_DIM),
            );
        } else {
            ui.label(RichText::new(" ").font(FontId::monospace(12.0)));
            ui.label(
                RichText::new(value)
                    .font(FontId::monospace(13.0))
                    .color(COLOR_TEXT)
                    .strong(),
            );
        }
    });
}

fn draw_pattern(ctx: &egui::Context, app: &mut App) {
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
                            let waveform = CHANNEL_INSTRUMENTS[ch % CHANNEL_INSTRUMENTS.len()];
                            let color = INST_COLORS[ch % INST_COLORS.len()];

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
                            let row_text_color = if is_playback_row {
                                COLOR_PATTERN_PLAYBACK_TEXT
                            } else {
                                COLOR_PATTERN_NOTE
                            };
                            let row_bg_color = if is_playback_row {
                                COLOR_PATTERN_PLAYBACK_HIGHLIGHT
                            } else {
                                egui::Color32::TRANSPARENT
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
                                            .color(COLOR_PATTERN_NOTE),
                                        Cell::NoteOff => RichText::new(&cell_text)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_PATTERN_NOTE_OFF),
                                        Cell::Empty => RichText::new(&cell_text)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_PATTERN_NOTE),
                                    }
                                };

                                ui.horizontal(|ui| {
                                    ui.label(
                                        RichText::new(left)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_TEXT_DIM),
                                    );
                                    let response = ui.label(note_text);
                                    ui.label(
                                        RichText::new(right)
                                            .font(FontId::monospace(13.0))
                                            .color(COLOR_TEXT_DIM),
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
