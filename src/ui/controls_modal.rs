use eframe::egui::{self, FontId, RichText};

use crate::app::App;
use crate::app::keybindings::KeyBindings;

use super::{
    COLOR_LAYOUT_BG_PANEL, COLOR_LAYOUT_BORDER_ACTIVE, COLOR_MODE_PLAYING, COLOR_PATTERN_EFFECT,
    COLOR_TEXT, COLOR_TEXT_ACTIVE, COLOR_TEXT_DIM,
};

pub fn draw_controls_modal(ctx: &egui::Context, app: &mut App) {
    if !app.show_controls_modal {
        return;
    }

    egui::Window::new("Controls")
        .open(&mut app.show_controls_modal)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .fixed_size([520.0, 540.0])
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .stroke(egui::Stroke::new(1.0, COLOR_LAYOUT_BORDER_ACTIVE))
                .inner_margin(egui::Margin::same(16))
                .corner_radius(4.0),
        )
        .show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                draw_keybinding_categories(ui, &app.keybindings);
                draw_notes_section(ui);
                draw_commands_section(ui);
            });
        });
}

fn draw_keybinding_categories(ui: &mut egui::Ui, keybindings: &KeyBindings) {
    let category_order = ["Global", "Mode", "Edit", "Synth"];

    for &cat in &category_order {
        let entries: Vec<_> = keybindings
            .bindings
            .iter()
            .filter(|b| b.category == cat)
            .collect();

        if entries.is_empty() {
            continue;
        }

        ui.add_space(12.0);
        ui.label(
            RichText::new(cat)
                .font(FontId::monospace(13.0))
                .color(COLOR_MODE_PLAYING)
                .strong(),
        );
        ui.add_space(4.0);

        egui::Grid::new(format!("controls_grid_{cat}"))
            .num_columns(3)
            .striped(true)
            .spacing([12.0, 4.0])
            .show(ui, |ui| {
                for binding in &entries {
                    ui.label(
                        RichText::new(binding.combo.label())
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT_ACTIVE),
                    );
                    ui.label(
                        RichText::new(binding.title)
                            .font(FontId::monospace(12.0))
                            .color(COLOR_TEXT),
                    );
                    ui.label(
                        RichText::new(binding.description)
                            .font(FontId::monospace(11.0))
                            .color(COLOR_TEXT_DIM),
                    );
                    ui.end_row();
                }
            });
    }
}

fn draw_notes_section(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.label(
        RichText::new("Notes")
            .font(FontId::monospace(13.0))
            .color(COLOR_MODE_PLAYING)
            .strong(),
    );
    ui.add_space(4.0);

    egui::Grid::new("controls_grid_notes")
        .num_columns(3)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label(
                RichText::new("Z .. P")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT_ACTIVE),
            );
            ui.label(
                RichText::new("Insert note")
                    .font(FontId::monospace(12.0))
                    .color(COLOR_TEXT),
            );
            ui.label(
                RichText::new("Insert note at cursor using keyboard layout")
                    .font(FontId::monospace(11.0))
                    .color(COLOR_TEXT_DIM),
            );
            ui.end_row();
        });
}

struct CmdEntry {
    code: &'static str,
    description: &'static str,
}

const COMMANDS: &[CmdEntry] = &[
    CmdEntry {
        code: "0xy",
        description: "Arpeggio (x,y semitones)",
    },
    CmdEntry {
        code: "1xx",
        description: "Portamento up",
    },
    CmdEntry {
        code: "2xx",
        description: "Portamento down",
    },
    CmdEntry {
        code: "3xx",
        description: "Tone portamento",
    },
    CmdEntry {
        code: "4xy",
        description: "Vibrato (x=speed y=depth)",
    },
    CmdEntry {
        code: "5xy",
        description: "Tone porta + vol slide",
    },
    CmdEntry {
        code: "6xy",
        description: "Vibrato + vol slide",
    },
    CmdEntry {
        code: "7xy",
        description: "Tremolo (x=speed y=depth)",
    },
    CmdEntry {
        code: "8xx",
        description: "Set panning (00-FF)",
    },
    CmdEntry {
        code: "9xx",
        description: "Sample offset (xx*256)",
    },
    CmdEntry {
        code: "Axy",
        description: "Volume slide (x=up y=down)",
    },
    CmdEntry {
        code: "Bxx",
        description: "Position jump",
    },
    CmdEntry {
        code: "Cxx",
        description: "Set volume (00-40)",
    },
    CmdEntry {
        code: "Dxx",
        description: "Pattern break (BCD row)",
    },
    CmdEntry {
        code: "E1x",
        description: "Fine porta up",
    },
    CmdEntry {
        code: "E2x",
        description: "Fine porta down",
    },
    CmdEntry {
        code: "E9x",
        description: "Retrigger note",
    },
    CmdEntry {
        code: "EAx",
        description: "Fine vol slide up",
    },
    CmdEntry {
        code: "EBx",
        description: "Fine vol slide down",
    },
    CmdEntry {
        code: "ECx",
        description: "Note cut at tick x",
    },
    CmdEntry {
        code: "EDx",
        description: "Note delay to tick x",
    },
    CmdEntry {
        code: "Fxx",
        description: "Set speed/tempo",
    },
];

fn draw_commands_section(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.label(
        RichText::new("Commands")
            .font(FontId::monospace(13.0))
            .color(COLOR_PATTERN_EFFECT)
            .strong(),
    );
    ui.add_space(4.0);

    egui::Grid::new("controls_grid_commands")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for cmd in COMMANDS {
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
                ui.end_row();
            }
        });
}
