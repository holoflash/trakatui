use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use crate::app::{App, Mode, SettingsField};
use crate::pattern::Cell;
use crate::scale::root_name;
use crate::synth::CHANNEL_INSTRUMENTS;

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(1),
        Constraint::Length(3),
    ])
    .split(area);

    draw_header(frame, app, chunks[0]);

    let content = Layout::horizontal([Constraint::Min(1), Constraint::Length(40)]).split(chunks[1]);
    draw_pattern(frame, app, content[0]);
    draw_settings(frame, app, content[1]);

    draw_footer(frame, app, chunks[2]);
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let mode_str = if app.playing {
        "▶ PLAYING"
    } else {
        match app.mode {
            Mode::Edit => "EDIT",
            Mode::Settings => "SETTINGS",
        }
    };
    let mode_color = if app.playing {
        Color::Green
    } else {
        match app.mode {
            Mode::Edit => Color::Cyan,
            Mode::Settings => Color::Yellow,
        }
    };

    let root = root_name(app.transpose);
    let scale_name = app.scale_index.scale().name;
    let key_label = format!("{} {}", root, scale_name);
    let header = Paragraph::new(Line::from(vec![
        Span::styled(
            " TRAKATUI ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("[{}]", mode_str),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("Oct:{}", app.octave),
            Style::default().fg(Color::Yellow),
        ),
        Span::raw("  "),
        Span::styled(
            format!("BPM:{}", app.bpm),
            Style::default().fg(Color::Magenta),
        ),
        Span::raw("  "),
        Span::styled(key_label, Style::default().fg(Color::Green)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(header, area);
}

fn draw_pattern(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.mode == Mode::Edit {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 2 || inner.width < 20 {
        return;
    }

    let visible_rows = (inner.height as usize).saturating_sub(1);
    let scroll_offset = calculate_scroll(app, visible_rows);

    let mut header_spans = vec![Span::styled(
        "     ",
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM),
    )];
    let inst_colors = [Color::Cyan, Color::Yellow, Color::Red, Color::Magenta];
    for ch in 0..app.pattern.channels {
        let waveform = CHANNEL_INSTRUMENTS[ch % CHANNEL_INSTRUMENTS.len()];
        let color = inst_colors[ch % inst_colors.len()];
        header_spans.push(Span::styled(
            format!("│ {} ", waveform.name()),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    let header_line = Line::from(header_spans);

    let mut lines = vec![header_line];

    for vis_row in 0..visible_rows {
        let row = scroll_offset + vis_row;
        if row >= app.pattern.rows {
            break;
        }

        let mut spans = Vec::new();

        let row_style = if app.playing && row == app.playback_row {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(format!("  {:02} ", row), row_style));

        for ch in 0..app.pattern.channels {
            let is_cursor =
                app.mode == Mode::Edit && ch == app.cursor_channel && row == app.cursor_row;
            let is_playback = app.playing && row == app.playback_row;
            let cell = app.pattern.get(ch, row);
            let cell_text = match cell {
                Cell::NoteOn(note) => note.name(),
                Cell::NoteOff => "OFF".to_string(),
                Cell::Empty => "···".to_string(),
            };

            let style = if is_cursor {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if is_playback {
                Style::default().fg(Color::Black).bg(Color::Green)
            } else {
                match cell {
                    Cell::NoteOn(_) => Style::default().fg(Color::White),
                    Cell::NoteOff => Style::default().fg(Color::Red),
                    Cell::Empty => Style::default().fg(Color::DarkGray),
                }
            };

            spans.push(Span::styled("│", Style::default().fg(Color::DarkGray)));
            spans.push(Span::styled(format!(" {} ", cell_text), style));
        }

        lines.push(Line::from(spans));
    }

    let grid = Paragraph::new(lines);
    frame.render_widget(grid, inner);
}

fn calculate_scroll(app: &App, visible_rows: usize) -> usize {
    let focus_row = if app.playing {
        app.playback_row
    } else {
        app.cursor_row
    };

    if focus_row < visible_rows / 2 {
        0
    } else if focus_row + visible_rows / 2 >= app.pattern.rows {
        app.pattern.rows.saturating_sub(visible_rows)
    } else {
        focus_row - visible_rows / 2
    }
}

fn draw_settings(frame: &mut Frame, app: &App, area: Rect) {
    let border_color = if app.mode == Mode::Settings {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let dim = Style::default().fg(Color::DarkGray);
    let label_style = Style::default().fg(Color::Gray);
    let value_style = Style::default()
        .fg(Color::White)
        .add_modifier(Modifier::BOLD);
    let selected_value = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let cursor_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);

    let is_bpm = app.settings_field == SettingsField::Bpm;
    let is_len = app.settings_field == SettingsField::PatternLength;
    let is_scale = app.settings_field == SettingsField::Scale;
    let is_trans = app.settings_field == SettingsField::Transpose;
    let is_export = app.settings_field == SettingsField::ExportWav;

    let cursor = |active: bool| -> Span {
        if active {
            Span::styled(" ▸ ", cursor_style)
        } else {
            Span::raw("   ")
        }
    };

    let arrows = |active: bool, val: &str| -> Vec<Span> {
        if active {
            vec![
                Span::styled("◄ ", Style::default().fg(Color::DarkGray)),
                Span::styled(val.to_string(), selected_value),
                Span::styled(" ►", Style::default().fg(Color::DarkGray)),
            ]
        } else {
            vec![
                Span::raw("  "),
                Span::styled(val.to_string(), value_style),
                Span::raw("  "),
            ]
        }
    };

    let mut bpm_spans = vec![cursor(is_bpm), Span::styled("BPM   ", label_style)];
    bpm_spans.extend(arrows(is_bpm, &format!("{:>3}", app.bpm)));

    let mut len_spans = vec![cursor(is_len), Span::styled("Length   ", label_style)];
    len_spans.extend(arrows(is_len, &format!("{:>3}", app.pattern.rows)));

    let scale_name = app.scale_index.scale().name;
    let mut scale_spans = vec![cursor(is_scale), Span::styled("Scale ", label_style)];
    scale_spans.extend(arrows(is_scale, &format!("{:>9}", scale_name)));

    let mut trans_spans = vec![cursor(is_trans), Span::styled("Transpose ", label_style)];
    trans_spans.extend(arrows(is_trans, &format!("{:>3}", app.transpose)));

    let export_style = if is_export {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let mut lines = vec![
        Line::from(Span::styled(
            " Settings",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(" ─────────────────────────────", dim)),
        Line::from(""),
        Line::from(bpm_spans),
        Line::from(""),
        Line::from(len_spans),
        Line::from(""),
        Line::from(scale_spans),
        Line::from(""),
        Line::from(trans_spans),
        Line::from(""),
        Line::from(Span::styled(" ─────────────────────────────", dim)),
        Line::from(""),
        Line::from(vec![
            cursor(is_export),
            Span::styled(" Export WAV ", export_style),
        ]),
    ];

    if let Some(ref msg) = app.status_message {
        let color = if msg.starts_with("Exported") {
            Color::Green
        } else {
            Color::Red
        };
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            format!("   {}", msg),
            Style::default().fg(color),
        )));
    }

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let horizontal = Layout::horizontal([Constraint::Percentage(100)])
        .flex(Flex::Center)
        .split(area);

    let help_text = match app.mode {
        Mode::Edit => {
            "Z..M/Q..U:note  TAB:off  DEL:clear  ,/.:oct  ENTER:play  2:settings  ESC:quit"
        }
        _ if app.playing => "ENTER:stop  ESC:stop",
        Mode::Settings => {
            "\u{2191}\u{2193}:select  \u{2190}\u{2192}:adjust  ENTER:confirm  1:pattern  ESC:back"
        }
    };

    let footer = Paragraph::new(Line::from(Span::styled(
        help_text,
        Style::default().fg(Color::DarkGray),
    )))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Center);

    frame.render_widget(footer, horizontal[0]);
}
