mod controls_modal;
mod header;
mod instrument;
mod pattern;
mod settings_panel;
mod sidebar;
mod widgets;

use eframe::egui::{self, Color32};

use crate::app::App;

pub const COLOR_LAYOUT_BG_DARK: Color32 = Color32::from_rgb(18, 16, 28);
pub const COLOR_LAYOUT_BG_PANEL: Color32 = Color32::from_rgb(26, 23, 38);

pub const COLOR_LAYOUT_BORDER: Color32 = Color32::from_rgb(120, 100, 60);
pub const COLOR_LAYOUT_BORDER_ACTIVE: Color32 = Color32::from_rgb(210, 185, 120);

pub const COLOR_TEXT_DIM: Color32 = Color32::from_rgb(110, 95, 70);
pub const COLOR_TEXT: Color32 = Color32::from_rgb(210, 190, 140);
pub const COLOR_TEXT_ACTIVE: Color32 = Color32::from_rgb(255, 250, 235);

pub const COLOR_MODE_EDIT: Color32 = Color32::from_rgb(210, 190, 140);
pub const COLOR_MODE_SETTINGS: Color32 = Color32::from_rgb(190, 170, 120);
pub const COLOR_MODE_PLAYING: Color32 = Color32::from_rgb(230, 205, 140);

pub const COLOR_ERROR: Color32 = Color32::from_rgb(220, 80, 70);
pub const COLOR_PATTERN_NOTE: Color32 = Color32::from_rgb(210, 190, 130);
pub const COLOR_PATTERN_NOTE_OFF: Color32 = Color32::from_rgb(200, 130, 120);
pub const COLOR_PATTERN_SUBDIVISION: Color32 = Color32::from_rgb(32, 28, 48);
pub const COLOR_PATTERN_CURSOR_BG: Color32 = Color32::from_rgb(140, 115, 60);
pub const COLOR_PATTERN_CURSOR_TEXT: Color32 = Color32::from_rgb(255, 250, 235);

pub const COLOR_PATTERN_PLAYBACK_HIGHLIGHT: Color32 = Color32::from_rgb(90, 75, 40);
pub const COLOR_PATTERN_PLAYBACK_TEXT: Color32 = Color32::from_rgb(255, 245, 220);

pub const COLOR_PATTERN_SELECTION_BG: Color32 = Color32::from_rgb(100, 85, 50);
pub const COLOR_PATTERN_SELECTION_TEXT: Color32 = Color32::from_rgb(245, 235, 200);

pub fn draw(ctx: &egui::Context, app: &mut App) {
    header::draw_header(ctx, app);
    sidebar::draw_sidebar(ctx, app);
    pattern::draw_pattern(ctx, app);
    controls_modal::draw_controls_modal(ctx, app);
}
