use eframe::egui::scroll_area::ScrollBarVisibility;
use eframe::egui::{self, RichText, Sense, Stroke, UiBuilder};

use crate::app::App;
use crate::project::{ArrangerItem, PatternColor};
use crate::ui::{
    COLOR_ACCENT, COLOR_LAYOUT_BG_PANEL, COLOR_PATTERN_BEATMARKER, COLOR_TEXT, COLOR_TEXT_ACTIVE,
    COLOR_TEXT_DIM,
};

const FONT: egui::FontId = egui::FontId::monospace(12.0);
const FONT_SMALL: egui::FontId = egui::FontId::monospace(10.0);
const ITEM_HEIGHT: f32 = 36.0;
const SUB_ITEM_HEIGHT: f32 = 30.0;
const MIN_WIDTH: f32 = 180.0;
const COLOR_SWATCH_SIZE: f32 = 14.0;
const GROUP_INDENT: f32 = 14.0;

enum ArrangerAction {
    Select(usize),
    ShiftSelect(usize),
    StartRename(usize),
    CommitRename,
    AddPattern,
    DeleteItem(usize),
    DuplicateItem(usize),
    CloneItem(usize),
    GroupSelected,
    Ungroup(usize),
    SetItemColor(usize, Option<PatternColor>),
    SetGroupRepeat(usize, u16),
    DragStartItem(usize),
    DragStartSub(usize, usize),
    DragHoverItem(usize),
    DragHoverSub(usize, usize),
    DragEnd,
    SelectPatternInGroup(usize, usize),
    ToggleCollapse(usize),
    SubDuplicate(usize, usize),
    SubClone(usize, usize),
    SubDelete(usize, usize),
    RemoveFromGroup(usize, usize),
}

pub fn draw_arranger(ctx: &egui::Context, app: &mut App) {
    if !app.show_arranger {
        return;
    }

    let mut actions: Vec<ArrangerAction> = Vec::new();

    egui::SidePanel::left("arranger")
        .resizable(true)
        .min_width(MIN_WIDTH)
        .frame(
            egui::Frame::new()
                .fill(COLOR_LAYOUT_BG_PANEL)
                .inner_margin(egui::Margin::ZERO)
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

            egui::ScrollArea::vertical()
                .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    let item_count = app.project.arranger.len();
                    for i in 0..item_count {
                        draw_arranger_item(ui, app, i, &mut actions);
                    }

                    if app.arranger_renaming.is_some() {
                        app.text_editing = true;
                    }

                    let add_btn_rect =
                        ui.allocate_space(egui::vec2(ui.available_width(), ITEM_HEIGHT));
                    let add_resp =
                        ui.interact(add_btn_rect.1, ui.id().with("arranger_add"), Sense::click());
                    if add_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    ui.painter()
                        .rect_filled(add_btn_rect.1, 0.0, COLOR_LAYOUT_BG_PANEL);
                    ui.painter().text(
                        add_btn_rect.1.center(),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        FONT,
                        COLOR_TEXT,
                    );
                    if add_resp.clicked() {
                        actions.push(ArrangerAction::AddPattern);
                    }
                });
        });

    process_actions(app, actions);
}

fn draw_arranger_item(
    ui: &mut egui::Ui,
    app: &mut App,
    idx: usize,
    actions: &mut Vec<ArrangerAction>,
) {
    let is_current = idx == app.project.current_item_idx;
    let is_selected = app.arranger_selection.contains(&idx);
    let is_drag_target = app
        .arranger_drag
        .as_ref()
        .is_some_and(|d| matches!(d.current, crate::app::DragTarget::Item(i) if i == idx));

    match &app.project.arranger[idx] {
        ArrangerItem::Single { pattern_idx } => {
            let pat_idx = *pattern_idx;
            let name = app.project.patterns[pat_idx].name.clone();
            let color = app.project.patterns[pat_idx].color;

            let bg = item_bg(is_current, is_selected, is_drag_target);

            let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), ITEM_HEIGHT));
            ui.painter().rect_filled(rect, 0.0, bg);

            if let Some(c) = color {
                let swatch = egui::Rect::from_min_size(
                    rect.min + egui::vec2(4.0, (ITEM_HEIGHT - COLOR_SWATCH_SIZE) / 2.0),
                    egui::vec2(3.0, COLOR_SWATCH_SIZE),
                );
                ui.painter().rect_filled(swatch, 1.0, c.to_color32());
            }

            let text_left = rect.min.x + 12.0;

            if let Some((rename_idx, ref mut buf)) = app.arranger_renaming
                && rename_idx == idx
            {
                let finished = draw_rename_field(ui, rect, text_left, buf);
                if finished {
                    actions.push(ArrangerAction::CommitRename);
                }
                return;
            }

            let fg = if is_current {
                COLOR_TEXT_ACTIVE
            } else {
                COLOR_TEXT
            };
            ui.painter().text(
                egui::pos2(text_left, rect.center().y),
                egui::Align2::LEFT_CENTER,
                &name,
                FONT,
                fg,
            );

            let resp = ui.interact(
                rect,
                ui.id().with(("arr_item", idx)),
                Sense::click_and_drag(),
            );
            let is_dragging = app.arranger_drag.is_some();
            handle_item_interaction(ui, &resp, idx, is_dragging, actions);
            if is_dragging {
                let pointer_over = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|p| rect.contains(p));
                if pointer_over {
                    actions.push(ArrangerAction::DragHoverItem(idx));
                }
            }

            resp.context_menu(|ui| {
                single_context_menu(ui, idx, pat_idx, app, actions);
            });
        }
        ArrangerItem::Group {
            name,
            color,
            repeat,
            pattern_indices,
            collapsed,
            ..
        } => {
            let name = name.clone();
            let color = *color;
            let repeat = *repeat;
            let indices = pattern_indices.clone();
            let collapsed = *collapsed;

            let group_height = if collapsed {
                ITEM_HEIGHT
            } else {
                ITEM_HEIGHT + indices.len() as f32 * SUB_ITEM_HEIGHT
            };
            let (_, rect) = ui.allocate_space(egui::vec2(ui.available_width(), group_height));

            let bg = item_bg(is_current, is_selected, is_drag_target);
            ui.painter().rect_filled(rect, 0.0, bg);

            if let Some(c) = color {
                let bar = egui::Rect::from_min_size(
                    rect.min + egui::vec2(2.0, 2.0),
                    egui::vec2(3.0, rect.height() - 4.0),
                );
                ui.painter().rect_filled(bar, 1.0, c.to_color32());
            }

            let header_rect =
                egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), ITEM_HEIGHT));

            let arrow_size = 16.0;
            let arrow_rect = egui::Rect::from_min_size(
                egui::pos2(
                    header_rect.max.x - arrow_size - 4.0,
                    header_rect.min.y + (ITEM_HEIGHT - arrow_size) / 2.0,
                ),
                egui::vec2(arrow_size, arrow_size),
            );
            let arrow_glyph = if collapsed { "▶" } else { "▼" };
            ui.painter().text(
                arrow_rect.center(),
                egui::Align2::CENTER_CENTER,
                arrow_glyph,
                FONT_SMALL,
                COLOR_TEXT_DIM,
            );

            if let Some((rename_idx, ref mut buf)) = app.arranger_renaming {
                if rename_idx == idx {
                    let finished = draw_rename_field(ui, header_rect, rect.min.x + 12.0, buf);
                    if finished {
                        actions.push(ArrangerAction::CommitRename);
                    }
                } else {
                    draw_group_header_text(ui, header_rect, &name, repeat, is_current);
                }
            } else {
                draw_group_header_text(ui, header_rect, &name, repeat, is_current);
            }

            let header_resp = ui.interact(
                header_rect,
                ui.id().with(("arr_group", idx)),
                Sense::click_and_drag(),
            );
            let is_dragging = app.arranger_drag.is_some();
            handle_item_interaction(ui, &header_resp, idx, is_dragging, actions);
            if is_dragging {
                let pointer_over = ui
                    .ctx()
                    .pointer_hover_pos()
                    .is_some_and(|p| header_rect.contains(p));
                if pointer_over {
                    actions.push(ArrangerAction::DragHoverItem(idx));
                }
            }
            header_resp.context_menu(|ui| {
                group_context_menu(ui, idx, repeat, app, actions);
            });

            let arrow_resp = ui.interact(
                arrow_rect,
                ui.id().with(("arr_collapse", idx)),
                Sense::click(),
            );
            if arrow_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if arrow_resp.clicked() {
                actions.push(ArrangerAction::ToggleCollapse(idx));
            }

            if !collapsed {
                for (sub_i, &pat_idx) in indices.iter().enumerate() {
                    let sub_rect = egui::Rect::from_min_size(
                        rect.min
                            + egui::vec2(
                                GROUP_INDENT,
                                ITEM_HEIGHT + sub_i as f32 * SUB_ITEM_HEIGHT,
                            ),
                        egui::vec2(rect.width() - GROUP_INDENT, SUB_ITEM_HEIGHT),
                    );

                    let sub_is_current = is_current && app.project.current_pattern_idx() == pat_idx;
                    let sub_is_drag_target = app
                        .arranger_drag
                        .as_ref()
                        .is_some_and(|d| matches!(d.current, crate::app::DragTarget::SubPattern(g, s) if g == idx && s == sub_i));

                    let sub_bg = if sub_is_drag_target {
                        COLOR_ACCENT
                    } else if sub_is_current {
                        COLOR_PATTERN_BEATMARKER
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    ui.painter().rect_filled(sub_rect, 0.0, sub_bg);

                    let sub_name = &app.project.patterns[pat_idx].name;
                    let sub_fg = if sub_is_current {
                        COLOR_TEXT_ACTIVE
                    } else {
                        COLOR_TEXT_DIM
                    };
                    ui.painter().text(
                        egui::pos2(sub_rect.min.x + 6.0, sub_rect.center().y),
                        egui::Align2::LEFT_CENTER,
                        sub_name,
                        FONT_SMALL,
                        sub_fg,
                    );

                    let sub_resp = ui.interact(
                        sub_rect,
                        ui.id().with(("arr_sub", idx, sub_i)),
                        Sense::click_and_drag(),
                    );
                    if sub_resp.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                    if app.arranger_drag.is_some() {
                        let pointer_over = ui
                            .ctx()
                            .pointer_hover_pos()
                            .is_some_and(|p| sub_rect.contains(p));
                        if pointer_over {
                            actions.push(ArrangerAction::DragHoverSub(idx, sub_i));
                        }
                    }
                    if sub_resp.clicked() {
                        actions.push(ArrangerAction::SelectPatternInGroup(idx, sub_i));
                    }
                    if sub_resp.drag_started() {
                        actions.push(ArrangerAction::DragStartSub(idx, sub_i));
                    }
                    if sub_resp.drag_stopped() {
                        actions.push(ArrangerAction::DragEnd);
                    }
                    sub_resp.context_menu(|ui| {
                        sub_pattern_context_menu(ui, idx, sub_i, pat_idx, app, actions);
                    });
                }
            }
        }
    }
}

fn draw_group_header_text(
    ui: &egui::Ui,
    rect: egui::Rect,
    name: &str,
    repeat: u16,
    is_current: bool,
) {
    let fg = if is_current {
        COLOR_TEXT_ACTIVE
    } else {
        COLOR_TEXT
    };
    let display = if repeat > 1 {
        format!("{} ×{}", name, repeat)
    } else {
        name.to_string()
    };
    ui.painter().text(
        egui::pos2(rect.min.x + 12.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &display,
        FONT,
        fg,
    );
}

fn draw_rename_field(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    text_left: f32,
    buf: &mut String,
) -> bool {
    let field_rect = egui::Rect::from_min_max(
        egui::pos2(text_left, rect.min.y),
        egui::pos2(rect.max.x - 4.0, rect.max.y),
    );

    let mut child = ui.new_child(
        UiBuilder::new()
            .max_rect(field_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let te = egui::TextEdit::singleline(buf)
        .font(FONT)
        .text_color(COLOR_TEXT_ACTIVE)
        .desired_width(24.0 * 7.3)
        .margin(egui::Margin::ZERO)
        .frame(false)
        .char_limit(24);

    let resp = child.add(te);
    resp.request_focus();

    let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
    let escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
    let clicked_outside = ui.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !field_rect.contains(p))
    });

    enter || escape || clicked_outside
}

fn item_bg(is_current: bool, is_selected: bool, is_drag_target: bool) -> egui::Color32 {
    if is_drag_target {
        COLOR_ACCENT
    } else if is_current {
        COLOR_PATTERN_BEATMARKER
    } else if is_selected {
        egui::Color32::from_rgb(30, 27, 45)
    } else {
        egui::Color32::TRANSPARENT
    }
}

fn handle_item_interaction(
    ui: &egui::Ui,
    resp: &egui::Response,
    idx: usize,
    _is_dragging: bool,
    actions: &mut Vec<ArrangerAction>,
) {
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    if resp.double_clicked() {
        actions.push(ArrangerAction::StartRename(idx));
    } else if resp.clicked() {
        if ui.input(|i| i.modifiers.shift) {
            actions.push(ArrangerAction::ShiftSelect(idx));
        } else {
            actions.push(ArrangerAction::Select(idx));
        }
    }

    if resp.drag_started() {
        actions.push(ArrangerAction::DragStartItem(idx));
    }
    if resp.drag_stopped() {
        actions.push(ArrangerAction::DragEnd);
    }
}

fn single_context_menu(
    ui: &mut egui::Ui,
    idx: usize,
    pat_idx: usize,
    app: &App,
    actions: &mut Vec<ArrangerAction>,
) {
    let selection = &app.arranger_selection;
    let multi = selection.len() >= 2;

    if multi {
        if ui
            .button(RichText::new("Group").font(FONT).color(COLOR_TEXT_ACTIVE))
            .clicked()
        {
            actions.push(ArrangerAction::GroupSelected);
            ui.close();
        }
        ui.separator();
    }

    ui.label(
        RichText::new("Color")
            .font(FONT_SMALL)
            .color(COLOR_TEXT_DIM),
    );
    ui.horizontal(|ui| {
        for c in PatternColor::ALL {
            let (_, swatch_rect) =
                ui.allocate_space(egui::vec2(COLOR_SWATCH_SIZE, COLOR_SWATCH_SIZE));
            ui.painter().rect_filled(swatch_rect, 2.0, c.to_color32());
            let swatch_resp = ui.interact(
                swatch_rect,
                ui.id().with(("color_swatch", idx, c.label())),
                Sense::click(),
            );
            if swatch_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if swatch_resp.clicked() {
                if multi {
                    for &sel_idx in selection {
                        actions.push(ArrangerAction::SetItemColor(sel_idx, Some(*c)));
                    }
                } else {
                    actions.push(ArrangerAction::SetItemColor(idx, Some(*c)));
                }
                ui.close();
            }
        }
    });
    if app.project.patterns[pat_idx].color.is_some()
        && ui
            .button(
                RichText::new("Clear color")
                    .font(FONT_SMALL)
                    .color(COLOR_TEXT_DIM),
            )
            .clicked()
    {
        if multi {
            for &sel_idx in selection {
                actions.push(ArrangerAction::SetItemColor(sel_idx, None));
            }
        } else {
            actions.push(ArrangerAction::SetItemColor(idx, None));
        }
        ui.close();
    }

    ui.separator();
    if ui
        .button(RichText::new("Clone").font(FONT).color(COLOR_TEXT_ACTIVE))
        .clicked()
    {
        if multi {
            for &sel_idx in selection.iter().rev() {
                actions.push(ArrangerAction::CloneItem(sel_idx));
            }
        } else {
            actions.push(ArrangerAction::CloneItem(idx));
        }
        ui.close();
    }
    if ui
        .button(
            RichText::new("Duplicate")
                .font(FONT)
                .color(COLOR_TEXT_ACTIVE),
        )
        .clicked()
    {
        if multi {
            for &sel_idx in selection.iter().rev() {
                actions.push(ArrangerAction::DuplicateItem(sel_idx));
            }
        } else {
            actions.push(ArrangerAction::DuplicateItem(idx));
        }
        ui.close();
    }
    ui.separator();
    if app.project.arranger.len() > 1
        && ui
            .button(RichText::new("Delete").font(FONT).color(COLOR_TEXT_ACTIVE))
            .clicked()
    {
        if multi {
            let mut sorted: Vec<usize> = selection.clone();
            sorted.sort_unstable();
            for &sel_idx in sorted.iter().rev() {
                actions.push(ArrangerAction::DeleteItem(sel_idx));
            }
        } else {
            actions.push(ArrangerAction::DeleteItem(idx));
        }
        ui.close();
    }
}

fn sub_pattern_context_menu(
    ui: &mut egui::Ui,
    group_idx: usize,
    sub_idx: usize,
    _pat_idx: usize,
    app: &App,
    actions: &mut Vec<ArrangerAction>,
) {
    if ui
        .button(RichText::new("Clone").font(FONT).color(COLOR_TEXT_ACTIVE))
        .clicked()
    {
        actions.push(ArrangerAction::SubClone(group_idx, sub_idx));
        ui.close();
    }
    if ui
        .button(
            RichText::new("Duplicate")
                .font(FONT)
                .color(COLOR_TEXT_ACTIVE),
        )
        .clicked()
    {
        actions.push(ArrangerAction::SubDuplicate(group_idx, sub_idx));
        ui.close();
    }
    ui.separator();
    if ui
        .button(
            RichText::new("Remove from group")
                .font(FONT)
                .color(COLOR_TEXT_ACTIVE),
        )
        .clicked()
    {
        actions.push(ArrangerAction::RemoveFromGroup(group_idx, sub_idx));
        ui.close();
    }
    if let ArrangerItem::Group {
        pattern_indices, ..
    } = &app.project.arranger[group_idx]
        && pattern_indices.len() > 1
        && ui
            .button(RichText::new("Delete").font(FONT).color(COLOR_TEXT_ACTIVE))
            .clicked()
    {
        actions.push(ArrangerAction::SubDelete(group_idx, sub_idx));
        ui.close();
    }
}

fn group_context_menu(
    ui: &mut egui::Ui,
    idx: usize,
    current_repeat: u16,
    app: &App,
    actions: &mut Vec<ArrangerAction>,
) {
    ui.label(
        RichText::new("Group Repeat")
            .font(FONT_SMALL)
            .color(COLOR_TEXT_DIM),
    );
    let mut rep = current_repeat;
    if ui
        .add(egui::DragValue::new(&mut rep).range(1..=999).speed(0.3))
        .changed()
    {
        actions.push(ArrangerAction::SetGroupRepeat(idx, rep));
    }

    ui.separator();
    ui.label(
        RichText::new("Color")
            .font(FONT_SMALL)
            .color(COLOR_TEXT_DIM),
    );
    ui.horizontal(|ui| {
        for c in PatternColor::ALL {
            let (_, swatch_rect) =
                ui.allocate_space(egui::vec2(COLOR_SWATCH_SIZE, COLOR_SWATCH_SIZE));
            ui.painter().rect_filled(swatch_rect, 2.0, c.to_color32());
            let swatch_resp = ui.interact(
                swatch_rect,
                ui.id().with(("grp_color", idx, c.label())),
                Sense::click(),
            );
            if swatch_resp.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if swatch_resp.clicked() {
                actions.push(ArrangerAction::SetItemColor(idx, Some(*c)));
                ui.close();
            }
        }
    });

    ui.separator();
    if ui
        .button(RichText::new("Clone").font(FONT).color(COLOR_TEXT_ACTIVE))
        .clicked()
    {
        actions.push(ArrangerAction::CloneItem(idx));
        ui.close();
    }
    if ui
        .button(
            RichText::new("Duplicate")
                .font(FONT)
                .color(COLOR_TEXT_ACTIVE),
        )
        .clicked()
    {
        actions.push(ArrangerAction::DuplicateItem(idx));
        ui.close();
    }
    if ui
        .button(RichText::new("Ungroup").font(FONT).color(COLOR_TEXT_ACTIVE))
        .clicked()
    {
        actions.push(ArrangerAction::Ungroup(idx));
        ui.close();
    }
    ui.separator();
    if app.project.arranger.len() > 1
        && ui
            .button(RichText::new("Delete").font(FONT).color(COLOR_TEXT_ACTIVE))
            .clicked()
    {
        actions.push(ArrangerAction::DeleteItem(idx));
        ui.close();
    }
}

fn process_actions(app: &mut App, actions: Vec<ArrangerAction>) {
    for action in actions {
        match action {
            ArrangerAction::Select(idx) => {
                app.project.current_item_idx = idx;
                app.arranger_selection.clear();
                app.arranger_selection.push(idx);
                app.cursor.row = 0;
                if app.playback.playing {
                    app.start_playback(false);
                }
            }
            ArrangerAction::ShiftSelect(idx) => {
                if !app.arranger_selection.contains(&idx) {
                    app.arranger_selection.push(idx);
                } else {
                    app.arranger_selection.retain(|&i| i != idx);
                }
            }
            ArrangerAction::StartRename(idx) => {
                let current_name = match &app.project.arranger[idx] {
                    ArrangerItem::Single { pattern_idx } => {
                        app.project.patterns[*pattern_idx].name.clone()
                    }
                    ArrangerItem::Group { name, .. } => name.clone(),
                };
                app.arranger_renaming = Some((idx, current_name));
            }
            ArrangerAction::CommitRename => {
                if let Some((idx, name)) = app.arranger_renaming.take() {
                    let trimmed = name.trim().to_string();
                    let final_name = if trimmed.is_empty() {
                        match &app.project.arranger[idx] {
                            ArrangerItem::Single { pattern_idx } => {
                                app.project.patterns[*pattern_idx].name.clone()
                            }
                            ArrangerItem::Group { name: old_name, .. } => old_name.clone(),
                        }
                    } else {
                        trimmed
                    };
                    match &app.project.arranger[idx] {
                        ArrangerItem::Single { pattern_idx } => {
                            let pi = *pattern_idx;
                            app.project.patterns[pi].name = final_name;
                        }
                        ArrangerItem::Group { .. } => {
                            if let ArrangerItem::Group { ref mut name, .. } =
                                app.project.arranger[idx]
                            {
                                *name = final_name;
                            }
                        }
                    }
                }
            }
            ArrangerAction::AddPattern => {
                app.save_undo_snapshot();
                let name = app.project.next_pattern_name();
                let current = app.project.current_pattern();
                let channels = current.channels;
                let new_pat = crate::project::Pattern::new_from(current, name, channels);
                let new_idx = app.project.patterns.len();
                app.project.patterns.push(new_pat);
                app.project.arranger.push(ArrangerItem::Single {
                    pattern_idx: new_idx,
                });
                app.project.current_item_idx = app.project.arranger.len() - 1;
                app.cursor.row = 0;
            }
            ArrangerAction::DeleteItem(idx) => {
                app.save_undo_snapshot();
                app.project.delete_item(idx);
                app.cursor.row = 0;
                app.arranger_selection.clear();
            }
            ArrangerAction::DuplicateItem(idx) => {
                app.save_undo_snapshot();
                app.project.duplicate_item(idx);
            }
            ArrangerAction::CloneItem(idx) => {
                app.save_undo_snapshot();
                app.project.clone_item(idx);
            }
            ArrangerAction::GroupSelected => {
                app.save_undo_snapshot();
                let indices = app.arranger_selection.clone();
                app.project.group_items(&indices);
                app.arranger_selection.clear();
                app.cursor.row = 0;
            }
            ArrangerAction::Ungroup(idx) => {
                app.save_undo_snapshot();
                app.project.ungroup(idx);
                app.arranger_selection.clear();
            }
            ArrangerAction::SetItemColor(idx, color) => {
                app.save_undo_snapshot();
                match &app.project.arranger[idx] {
                    ArrangerItem::Single { pattern_idx } => {
                        let pi = *pattern_idx;
                        app.project.patterns[pi].color = color;
                    }
                    ArrangerItem::Group { .. } => {
                        if let ArrangerItem::Group {
                            color: ref mut c, ..
                        } = app.project.arranger[idx]
                        {
                            *c = color;
                        }
                    }
                }
            }

            ArrangerAction::SetGroupRepeat(idx, rep) => {
                if let ArrangerItem::Group {
                    repeat: ref mut r, ..
                } = app.project.arranger[idx]
                {
                    *r = rep;
                }
            }
            ArrangerAction::DragStartItem(idx) => {
                app.arranger_drag = Some(crate::app::ArrangerDrag {
                    from: crate::app::DragTarget::Item(idx),
                    current: crate::app::DragTarget::Item(idx),
                });
            }
            ArrangerAction::DragStartSub(group_idx, sub_idx) => {
                app.arranger_drag = Some(crate::app::ArrangerDrag {
                    from: crate::app::DragTarget::SubPattern(group_idx, sub_idx),
                    current: crate::app::DragTarget::SubPattern(group_idx, sub_idx),
                });
            }
            ArrangerAction::DragHoverItem(idx) => {
                if let Some(ref mut drag) = app.arranger_drag {
                    drag.current = crate::app::DragTarget::Item(idx);
                }
            }
            ArrangerAction::DragHoverSub(group_idx, sub_idx) => {
                if let Some(ref mut drag) = app.arranger_drag {
                    drag.current = crate::app::DragTarget::SubPattern(group_idx, sub_idx);
                }
            }
            ArrangerAction::DragEnd => {
                if let Some(drag) = app.arranger_drag.take() {
                    use crate::app::DragTarget;
                    match (&drag.from, &drag.current) {
                        (DragTarget::Item(from), DragTarget::Item(to)) if from != to => {
                            app.save_undo_snapshot();
                            app.project.reorder_item(*from, *to);
                            app.arranger_selection.clear();
                        }
                        (DragTarget::Item(item), DragTarget::SubPattern(group, sub)) => {
                            app.save_undo_snapshot();
                            app.project.move_item_into_group(*item, *group, *sub);
                            app.arranger_selection.clear();
                        }
                        (DragTarget::SubPattern(group, sub), DragTarget::Item(target)) => {
                            app.save_undo_snapshot();
                            app.project.move_sub_pattern_out(*group, *sub, *target);
                            app.arranger_selection.clear();
                        }
                        (
                            DragTarget::SubPattern(from_g, from_s),
                            DragTarget::SubPattern(to_g, to_s),
                        ) => {
                            if from_g == to_g {
                                if from_s != to_s {
                                    app.save_undo_snapshot();
                                    app.project.reorder_sub_pattern(*from_g, *from_s, *to_s);
                                }
                            } else {
                                app.save_undo_snapshot();
                                app.project
                                    .move_sub_between_groups(*from_g, *from_s, *to_g, *to_s);
                            }
                        }
                        _ => {}
                    }
                }
            }
            ArrangerAction::SelectPatternInGroup(group_idx, sub_idx) => {
                app.project.current_item_idx = group_idx;
                app.project.current_sub_pattern_idx = sub_idx;
                app.arranger_selection.clear();
                app.arranger_selection.push(group_idx);
                app.cursor.row = 0;
                if app.playback.playing {
                    app.start_playback(false);
                }
            }
            ArrangerAction::ToggleCollapse(idx) => {
                if let ArrangerItem::Group {
                    collapsed: ref mut c,
                    ..
                } = app.project.arranger[idx]
                {
                    *c = !*c;
                }
            }

            ArrangerAction::SubDuplicate(group_idx, sub_idx) => {
                if let ArrangerItem::Group {
                    pattern_indices, ..
                } = &app.project.arranger[group_idx]
                {
                    let pi = pattern_indices[sub_idx];
                    let mut new_pat = app.project.patterns[pi].clone();
                    new_pat.name = crate::project::Project::increment_name(&new_pat.name);
                    let new_idx = app.project.patterns.len();
                    app.project.patterns.push(new_pat);
                    if let ArrangerItem::Group {
                        pattern_indices, ..
                    } = &mut app.project.arranger[group_idx]
                    {
                        pattern_indices.insert(sub_idx + 1, new_idx);
                    }
                }
            }
            ArrangerAction::SubClone(group_idx, sub_idx) => {
                if let ArrangerItem::Group {
                    pattern_indices, ..
                } = &mut app.project.arranger[group_idx]
                {
                    let pi = pattern_indices[sub_idx];
                    pattern_indices.insert(sub_idx + 1, pi);
                }
            }
            ArrangerAction::SubDelete(group_idx, sub_idx) => {
                if let ArrangerItem::Group {
                    pattern_indices, ..
                } = &mut app.project.arranger[group_idx]
                    && pattern_indices.len() > 1
                {
                    pattern_indices.remove(sub_idx);
                }
            }
            ArrangerAction::RemoveFromGroup(group_idx, sub_idx) => {
                app.save_undo_snapshot();
                app.project
                    .move_sub_pattern_out(group_idx, sub_idx, group_idx + 1);
                app.arranger_selection.clear();
            }
        }
    }
}
