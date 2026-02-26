use eframe::egui::{self, FontId, RichText, Stroke, Vec2};

use crate::app::{App, Mode, SynthSettingsField};

use super::widgets::settings_row;
use super::*;

pub fn draw_instrument(ui: &mut egui::Ui, app: &App) {
    let synth_border = if app.mode == Mode::SynthEdit {
        COLOR_LAYOUT_BORDER_ACTIVE
    } else {
        COLOR_LAYOUT_BORDER
    };
    egui::Frame::new()
        .fill(COLOR_LAYOUT_BG_PANEL)
        .inner_margin(egui::Margin::symmetric(12, 10))
        .stroke(Stroke::new(1.0, synth_border))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            let ch = app.synth_channel;
            let cs = &app.channel_settings[ch];
            let synth_active = app.mode == Mode::SynthEdit;

            ui.label(
                RichText::new("Instrument")
                    .font(FontId::monospace(15.0))
                    .color(COLOR_MODE_SETTINGS)
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

            settings_row(
                ui,
                "Channel",
                &format!("{} ─ {}", ch + 1, cs.waveform.name()),
                synth_active && app.synth_field == SynthSettingsField::Channel,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Waveform",
                cs.waveform.name(),
                synth_active && app.synth_field == SynthSettingsField::Waveform,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Attack",
                &format!("{:.3}", cs.envelope.attack),
                synth_active && app.synth_field == SynthSettingsField::Attack,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Decay",
                &format!("{:.3}", cs.envelope.decay),
                synth_active && app.synth_field == SynthSettingsField::Decay,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Sustain",
                &format!("{:.2}", cs.envelope.sustain),
                synth_active && app.synth_field == SynthSettingsField::Sustain,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Release",
                &format!("{:.3}", cs.envelope.release),
                synth_active && app.synth_field == SynthSettingsField::Release,
            );
            ui.add_space(6.0);
            settings_row(
                ui,
                "Volume",
                &format!("{:.2}", cs.volume),
                synth_active && app.synth_field == SynthSettingsField::Volume,
            );
        });
}
