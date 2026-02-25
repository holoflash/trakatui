use eframe::egui::{self, Color32, FontId, RichText, ScrollArea, Stroke, Vec2};

use crate::app::{App, Mode, SettingsField};
use crate::pattern::Cell;
use crate::scale::root_name;
use crate::synth::CHANNEL_INSTRUMENTS;

const BG_DARK: Color32 = Color32::from_rgb(0, 0, 0);
const BG_PANEL: Color32 = Color32::from_rgb(150, 150, 150);
const BG_HEADER: Color32 = Color32::from_rgb(150, 150, 150);
const BORDER: Color32 = Color32::from_rgb(80, 80, 80);
const BORDER_ACTIVE: Color32 = Color32::from_rgb(255, 255, 255);
const DIM: Color32 = Color32::from_rgb(80, 80, 80);
const TEXT: Color32 = Color32::from_rgb(0, 0, 0);
const CYAN: Color32 = Color32::from_rgb(0, 0, 0);
const YELLOW: Color32 = Color32::from_rgb(255, 255, 255);
const GREEN: Color32 = Color32::from_rgb(0, 0, 0);
const MAGENTA: Color32 = Color32::from_rgb(0, 0, 0);
const RED: Color32 = Color32::from_rgb(220, 80, 80);
const NOTE_BLUE: Color32 = Color32::from_rgb(80, 80, 255);
const CURSOR_BG: Color32 = Color32::from_rgb(255, 50, 120);
const CURSOR_TEXT: Color32 = Color32::from_rgb(255, 255, 255);
const PLAYBACK_BG: Color32 = Color32::from_rgb(160, 160, 160);
const PLAYBACK_TEXT: Color32 = Color32::from_rgb(0, 0, 0);

const INST_COLORS: [Color32; 4] = [NOTE_BLUE, NOTE_BLUE, NOTE_BLUE, NOTE_BLUE];

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
                .fill(BG_HEADER)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .stroke(Stroke::new(1.0, BORDER)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(4.0);
                ui.label(
                    RichText::new("PSIKAT")
                        .font(FontId::monospace(24.0))
                        .color(CYAN)
                        .strong(),
                );
                ui.add_space(16.0);

                let (mode_str, mode_color) = if app.playing {
                    ("PLAYING", GREEN)
                } else {
                    match app.mode {
                        Mode::Edit => ("EDIT", CYAN),
                        Mode::Settings => ("SETTINGS", YELLOW),
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
                        .color(YELLOW),
                );
                ui.add_space(12.0);
                ui.label(
                    RichText::new(format!("BPM:{}", app.bpm))
                        .font(FontId::monospace(13.0))
                        .color(MAGENTA),
                );
                ui.add_space(12.0);

                let root = root_name(app.transpose);
                let scale_name = app.scale_index.scale().name;
                ui.label(
                    RichText::new(format!("{} {}", root, scale_name))
                        .font(FontId::monospace(13.0))
                        .color(GREEN),
                );
            });
        });
}

fn draw_footer(ctx: &egui::Context, app: &App) {
    egui::TopBottomPanel::bottom("footer")
        .frame(
            egui::Frame::new()
                .fill(BG_HEADER)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .stroke(Stroke::new(1.0, BORDER)),
        )
        .show(ctx, |ui| {
            let help_text = match app.mode {
                Mode::Edit => {
                    "Z..M/Q..U:note  TAB:off  DEL:clear  ,/.:oct  ENTER:play  2:settings  ESC:quit"
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
                            .color(DIM),
                    );
                });
            });
        });
}

fn draw_settings_panel(ctx: &egui::Context, app: &mut App) {
    let border_color = if app.mode == Mode::Settings {
        BORDER_ACTIVE
    } else {
        BORDER
    };

    egui::SidePanel::right("settings")
        .resizable(false)
        .exact_width(280.0)
        .frame(
            egui::Frame::new()
                .fill(BG_PANEL)
                .inner_margin(egui::Margin::symmetric(16, 12))
                .stroke(Stroke::new(1.0, border_color)),
        )
        .show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new("Settings")
                    .font(FontId::monospace(15.0))
                    .color(YELLOW)
                    .strong(),
            );
            ui.add_space(2.0);
            let sep_color = DIM;
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
                            .color(YELLOW)
                            .strong(),
                    );
                } else {
                    ui.label(
                        RichText::new(cursor_str)
                            .font(FontId::monospace(13.0))
                            .color(DIM),
                    );
                }
                let export_text = RichText::new(" Export WAV ")
                    .font(FontId::monospace(13.0))
                    .strong();
                if is_export {
                    ui.label(export_text.color(CURSOR_TEXT).background_color(GREEN));
                } else {
                    ui.label(export_text.color(GREEN));
                }
            });

            if let Some(ref msg) = app.status_message {
                ui.add_space(8.0);
                let color = if msg.starts_with("Exported") {
                    GREEN
                } else {
                    RED
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
        let cursor_color = if active { YELLOW } else { DIM };
        ui.label(
            RichText::new(cursor_str)
                .font(FontId::monospace(13.0))
                .color(cursor_color)
                .strong(),
        );
        ui.label(
            RichText::new(format!("{:<10}", label))
                .font(FontId::monospace(13.0))
                .color(TEXT),
        );
        if active {
            ui.label(RichText::new("◄").font(FontId::monospace(12.0)).color(DIM));
            ui.label(
                RichText::new(value)
                    .font(FontId::monospace(13.0))
                    .color(YELLOW)
                    .strong(),
            );
            ui.label(RichText::new("►").font(FontId::monospace(12.0)).color(DIM));
        } else {
            ui.label(RichText::new(" ").font(FontId::monospace(12.0)));
            ui.label(
                RichText::new(value)
                    .font(FontId::monospace(13.0))
                    .color(TEXT)
                    .strong(),
            );
        }
    });
}

fn draw_pattern(ctx: &egui::Context, app: &mut App) {
    let border_color = if app.mode == Mode::Edit {
        BORDER_ACTIVE
    } else {
        BORDER
    };

    egui::CentralPanel::default()
        .frame(
            egui::Frame::new()
                .fill(BG_DARK)
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
                                .color(DIM),
                        );
                        for ch in 0..app.pattern.channels {
                            let waveform = CHANNEL_INSTRUMENTS[ch % CHANNEL_INSTRUMENTS.len()];
                            let color = INST_COLORS[ch % INST_COLORS.len()];
                            ui.label(
                                RichText::new(format!("│ {} ", waveform.name()))
                                    .font(FontId::monospace(13.0))
                                    .color(color)
                                    .strong(),
                            );
                        }
                    });

                    ui.add_space(2.0);

                    for row in 0..app.pattern.rows {
                        ui.horizontal(|ui| {
                            let is_playback_row = app.playing && row == app.playback_row;
                            let row_text_color = if is_playback_row {
                                PLAYBACK_TEXT
                            } else {
                                NOTE_BLUE
                            };
                            let row_bg_color = if is_playback_row {
                                PLAYBACK_BG
                            } else {
                                egui::Color32::TRANSPARENT
                            };
                            ui.label(
                                RichText::new(format!("  {:02} ", row))
                                    .font(FontId::monospace(13.0))
                                    .color(row_text_color)
                                    .background_color(row_bg_color)
                                    .strong(),
                            );

                            for ch in 0..app.pattern.channels {
                                let is_cursor = !app.playing
                                    && app.mode == Mode::Edit
                                    && ch == app.cursor_channel
                                    && row == app.cursor_row;
                                let is_playback = app.playing && row == app.playback_row;
                                let cell = app.pattern.get(ch, row);
                                let cell_text = match cell {
                                    Cell::NoteOn(note) => note.name(),
                                    Cell::NoteOff => "OFF".to_string(),
                                    Cell::Empty => "···".to_string(),
                                };

                                let display = format!("│ {} ", cell_text);
                                let text = RichText::new(display).font(FontId::monospace(13.0));

                                let text = if is_cursor {
                                    text.color(CURSOR_TEXT).background_color(CURSOR_BG).strong()
                                } else if is_playback {
                                    text.color(PLAYBACK_TEXT).background_color(PLAYBACK_BG)
                                } else {
                                    match cell {
                                        Cell::NoteOn(_) => text.color(NOTE_BLUE),
                                        Cell::NoteOff => text.color(RED),
                                        Cell::Empty => text.color(NOTE_BLUE),
                                    }
                                };

                                let response = ui.label(text);

                                if response.clicked() {
                                    app.set_cursor(ch, row);
                                    if app.mode != Mode::Edit {
                                        app.mode = Mode::Edit;
                                    }
                                }
                            }
                        });
                    }
                });
        });
}
