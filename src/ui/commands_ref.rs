use eframe::egui::{self, FontId, RichText, Stroke, Vec2};

use super::{COLOR_LAYOUT_BG_PANEL, COLOR_PATTERN_EFFECT, COLOR_TEXT_DIM};

struct CmdEntry {
    code: &'static str,
    description: &'static str,
}

const COMMANDS: &[CmdEntry] = &[
    CmdEntry {
        code: "PUxy",
        description: "Pitch up x semitones in y steps",
    },
    CmdEntry {
        code: "PDxy",
        description: "Pitch down x semitones in y steps",
    },
    CmdEntry {
        code: "PU00",
        description: "Stop ongoing pitch bend",
    },
];

pub fn draw_commands_ref(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.label(
                RichText::new("Commands")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_PATTERN_EFFECT)
                    .strong(),
            );
            ui.add_space(2.0);
            ui.painter().line_segment(
                [
                    ui.cursor().left_top(),
                    ui.cursor().left_top() + Vec2::new(ui.available_width(), 0.0),
                ],
                Stroke::new(1.0, COLOR_TEXT_DIM),
            );
            ui.add_space(8.0);

            for cmd in COMMANDS {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(cmd.code)
                            .font(FontId::monospace(12.0))
                            .color(COLOR_PATTERN_EFFECT),
                    );
                    ui.label(
                        RichText::new(cmd.description)
                            .font(FontId::monospace(11.0))
                            .color(COLOR_TEXT_DIM),
                    );
                });
            }
            ui.allocate_space(ui.available_size());
        });
}
