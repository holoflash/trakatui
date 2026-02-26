use eframe::egui::{self, FontId, RichText, Stroke};

use crate::app::{App, Mode};

use super::*;

pub fn draw_footer(ctx: &egui::Context, app: &App) {
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
                    "Z..M/Q..U:note  TAB:off  DEL:clear  ,/.:oct  ALT+\u{2190}\u{2191}\u{2192}\u{2193}:select  ENTER:play  2:synth  3:settings"
                }
                _ if app.playing => "ENTER:stop  ESC:stop",
                Mode::SynthEdit => {
                    "\u{2191}\u{2193}:select  \u{2190}\u{2192}:adjust  1:pattern  3:settings  ESC:back"
                }
                Mode::Settings => {
                    "\u{2191}\u{2193}:select  \u{2190}\u{2192}:adjust  ENTER:confirm  1:pattern  2:synth  ESC:back"
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
