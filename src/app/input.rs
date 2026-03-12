use eframe::egui::{self, Key};

use crate::app::keybindings::Action;
use crate::app::scale::{Scale, map_key_index_to_note};
use crate::project::Cell;
use crate::project::Note;

use super::{App, ClipboardData, Mode, MovePreview};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        if self.text_editing || ctx.wants_keyboard_input() {
            return false;
        }
        ctx.input(|input| {
            let cmd = input.modifiers.command;
            let shift = input.modifiers.shift;

            if cmd && !shift && input.key_pressed(Key::Z) {
                self.undo();
                return false;
            }
            if cmd && shift && input.key_pressed(Key::Z) {
                self.redo();
                return false;
            }
            if cmd && shift && input.key_pressed(Key::S) {
                self.do_save_as();
                return false;
            }
            if cmd && !shift && input.key_pressed(Key::S) {
                self.do_quick_save();
                return false;
            }

            let actions = self.keybindings.active_actions(input);

            if self.move_preview.is_some() {
                let is_move_action = [
                    Action::MoveUp,
                    Action::MoveDown,
                    Action::MoveLeft,
                    Action::MoveRight,
                ]
                .iter()
                .any(|a| actions.contains(a));

                let any_key_pressed = input
                    .events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }));

                if actions.contains(&Action::PlayStop) {
                    self.confirm_move_preview();
                    return false;
                } else if !is_move_action && (any_key_pressed || !actions.is_empty()) {
                    self.cancel_move_preview();
                    return false;
                }
            }

            if actions.contains(&Action::PlayStop) {
                if self.playback.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(false);
                }
                return false;
            }

            if actions.contains(&Action::PlayFromCursor) {
                if self.playback.playing {
                    self.stop_playback();
                } else {
                    self.clear_selection();
                    self.start_playback(true);
                }
                return false;
            }

            let result = match self.mode {
                Mode::Edit => self.handle_edit_input(input, &actions),
            };

            self.current_track = self.cursor.channel;

            result
        })
    }

    fn handle_edit_input(&mut self, input: &egui::InputState, actions: &[Action]) -> bool {
        if self.handle_move(actions) {
            return false;
        }

        if self.handle_cursor_and_select(actions) {
            return false;
        }

        if actions.contains(&Action::Delete) {
            self.save_undo_snapshot();
            self.handle_delete();
            return false;
        }

        {
            let has_copy = input.events.iter().any(|e| matches!(e, egui::Event::Copy));
            let has_cut = input.events.iter().any(|e| matches!(e, egui::Event::Cut));
            let has_paste = input
                .events
                .iter()
                .any(|e| matches!(e, egui::Event::Paste(_)));

            if has_cut {
                self.save_undo_snapshot();
                self.handle_copy();
                self.handle_delete();
                return false;
            }
            if has_copy {
                self.handle_copy();
                return false;
            }
            if has_paste {
                self.save_undo_snapshot();
                self.handle_paste();
                return false;
            }
        }

        if actions.contains(&Action::NoteOff) {
            self.save_undo_snapshot();
            self.handle_note_off();
            return false;
        }

        if self.handle_transpose(actions) {
            return false;
        }

        if actions.contains(&Action::FillAscending) {
            self.save_undo_snapshot();
            self.handle_fill(true);
            return false;
        }

        if actions.contains(&Action::FillDescending) {
            self.save_undo_snapshot();
            self.handle_fill(false);
            return false;
        }

        if actions.contains(&Action::OctaveUp) {
            if self.cursor.octave < 8 {
                self.cursor.octave += 1;
            }
            return false;
        }

        if actions.contains(&Action::OctaveDown) {
            if self.cursor.octave > 0 {
                self.cursor.octave -= 1;
            }
            return false;
        }

        if actions.contains(&Action::Escape) {
            if self.cursor.selection_anchor.is_some() {
                self.clear_selection();
            } else if self.playback.playing {
                self.stop_playback();
            } else {
                return true;
            }
            return false;
        }

        self.handle_note_keys(input);

        false
    }

    fn handle_move(&mut self, actions: &[Action]) -> bool {
        let move_action = [
            Action::MoveUp,
            Action::MoveDown,
            Action::MoveLeft,
            Action::MoveRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        let Some(dir) = move_action else {
            return false;
        };

        let (dr, dc): (isize, isize) = match dir {
            Action::MoveUp => (-1, 0),
            Action::MoveDown => (1, 0),
            Action::MoveLeft => (0, -1),
            Action::MoveRight => (0, 1),
            _ => unreachable!(),
        };

        if self.move_preview.is_some() {
            let (min_ch, max_ch, _, _, min_row, max_row) = self.selection_bounds().unwrap();
            let in_bounds = min_row.checked_add_signed(dr).is_some()
                && max_row
                    .checked_add_signed(dr)
                    .is_some_and(|r| r < self.project.current_pattern().rows)
                && min_ch.checked_add_signed(dc).is_some()
                && max_ch
                    .checked_add_signed(dc)
                    .is_some_and(|c| c < self.project.current_pattern().channels);
            if in_bounds {
                self.cursor.channel = self.cursor.channel.checked_add_signed(dc).unwrap();
                self.cursor.row = self.cursor.row.checked_add_signed(dr).unwrap();
                if let Some((ach, _, arow)) = self.cursor.selection_anchor.as_mut() {
                    *ach = ach.checked_add_signed(dc).unwrap();
                    *arow = arow.checked_add_signed(dr).unwrap();
                }
            }
            return true;
        }

        if let Some((min_ch, max_ch, _, _, min_row, max_row)) = self.selection_bounds() {
            let in_bounds = min_row.checked_add_signed(dr).is_some()
                && max_row
                    .checked_add_signed(dr)
                    .is_some_and(|r| r < self.project.current_pattern().rows)
                && min_ch.checked_add_signed(dc).is_some()
                && max_ch
                    .checked_add_signed(dc)
                    .is_some_and(|c| c < self.project.current_pattern().channels);

            if in_bounds {
                self.save_undo_snapshot();

                let mut cells = Vec::new();
                for ch in min_ch..=max_ch {
                    let voices = self.project.current_pattern().voice_count(ch);
                    for v in 0..voices {
                        for row in min_row..=max_row {
                            let cell = self.project.current_pattern().get(ch, v, row);
                            cells.push((ch - min_ch, v, row - min_row, cell));
                            self.project.current_pattern_mut().clear(ch, v, row);
                        }
                    }
                }

                let anchor = self.cursor.selection_anchor.unwrap();
                self.move_preview = Some(MovePreview {
                    cells,
                    origin_anchor: anchor,
                    origin_cursor: (self.cursor.channel, self.cursor.voice, self.cursor.row),
                });

                self.cursor.channel = self.cursor.channel.checked_add_signed(dc).unwrap();
                self.cursor.row = self.cursor.row.checked_add_signed(dr).unwrap();
                if let Some((ach, _, arow)) = self.cursor.selection_anchor.as_mut() {
                    *ach = ach.checked_add_signed(dc).unwrap();
                    *arow = arow.checked_add_signed(dr).unwrap();
                }
            }
        } else if let (Some(new_row), Some(new_ch)) = (
            self.cursor.row.checked_add_signed(dr),
            self.cursor.channel.checked_add_signed(dc),
        ) && new_row < self.project.current_pattern().rows
            && new_ch < self.project.current_pattern().channels
        {
            self.save_undo_snapshot();
            let v = self.cursor.voice;
            let cell = self
                .project
                .current_pattern()
                .get(self.cursor.channel, v, self.cursor.row);
            self.project
                .current_pattern_mut()
                .clear(self.cursor.channel, v, self.cursor.row);

            let target_v = v.min(
                self.project
                    .current_pattern()
                    .voice_count(new_ch)
                    .saturating_sub(1),
            );
            self.project
                .current_pattern_mut()
                .set(new_ch, target_v, new_row, cell);
            self.cursor.channel = new_ch;
            self.cursor.voice = target_v;
            self.cursor.row = new_row;
        }

        true
    }

    fn confirm_move_preview(&mut self) {
        let Some(preview) = self.move_preview.take() else {
            return;
        };
        let (min_ch, _, _, _, min_row, _) = self.selection_bounds().unwrap();
        for (ch_off, v, row_off, cell) in &preview.cells {
            let ch = min_ch + ch_off;
            let row = min_row + row_off;
            if ch < self.project.current_pattern().channels
                && *v < self.project.current_pattern().voice_count(ch)
                && row < self.project.current_pattern().rows
            {
                self.project.current_pattern_mut().set(ch, *v, row, *cell);
            }
        }
        self.clear_selection();
    }

    pub fn cancel_move_preview(&mut self) {
        let Some(preview) = self.move_preview.take() else {
            return;
        };
        let (orig_ach, _orig_avoice, orig_arow) = preview.origin_anchor;
        let (orig_ch, orig_voice, orig_row) = preview.origin_cursor;
        let base_ch = orig_ach.min(orig_ch);
        let base_row = orig_arow.min(orig_row);

        for (ch_off, v, row_off, cell) in &preview.cells {
            let ch = base_ch + ch_off;
            let row = base_row + row_off;
            if ch < self.project.current_pattern().channels
                && *v < self.project.current_pattern().voice_count(ch)
                && row < self.project.current_pattern().rows
            {
                self.project.current_pattern_mut().set(ch, *v, row, *cell);
            }
        }
        self.cursor.selection_anchor = Some(preview.origin_anchor);
        self.cursor.channel = orig_ch;
        self.cursor.voice = orig_voice;
        self.cursor.row = orig_row;
    }

    fn handle_cursor_and_select(&mut self, actions: &[Action]) -> bool {
        let select_action = [
            Action::SelectUp,
            Action::SelectDown,
            Action::SelectLeft,
            Action::SelectRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if select_action.is_some() && self.cursor.selection_anchor.is_none() {
            self.cursor.selection_anchor =
                Some((self.cursor.channel, self.cursor.voice, self.cursor.row));
        }

        let cursor_action = [
            Action::CursorUp,
            Action::CursorDown,
            Action::CursorLeft,
            Action::CursorRight,
        ]
        .iter()
        .find(|a| actions.contains(a))
        .copied();

        if let Some(dir) = cursor_action {
            if select_action.is_none() {
                self.clear_selection();
            }
            self.move_cursor(dir);
            return true;
        }

        if let Some(dir) = select_action {
            self.move_cursor(dir);
            return true;
        }

        false
    }

    fn move_cursor(&mut self, dir: Action) {
        let total_channels = self.project.current_pattern().channels;

        match dir {
            Action::CursorUp | Action::SelectUp => {
                if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                } else {
                    self.cursor.row = self.project.current_pattern().rows - 1;
                }
            }
            Action::CursorDown | Action::SelectDown => {
                if self.cursor.row < self.project.current_pattern().rows - 1 {
                    self.cursor.row += 1;
                } else {
                    self.cursor.row = 0;
                }
            }
            Action::CursorLeft | Action::SelectLeft => {
                if self.cursor.voice > 0 {
                    self.cursor.voice -= 1;
                } else if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                    self.cursor.voice = self
                        .voices_for_channel(self.cursor.channel)
                        .saturating_sub(1);
                } else {
                    self.cursor.channel = total_channels - 1;
                    self.cursor.voice = self
                        .voices_for_channel(self.cursor.channel)
                        .saturating_sub(1);
                }
            }
            Action::CursorRight | Action::SelectRight => {
                let voices = self.voices_for_channel(self.cursor.channel);
                if self.cursor.voice < voices - 1 {
                    self.cursor.voice += 1;
                } else if self.cursor.channel < total_channels - 1 {
                    self.cursor.channel += 1;
                    self.cursor.voice = 0;
                } else {
                    self.cursor.channel = 0;
                    self.cursor.voice = 0;
                }
            }
            _ => {}
        }
    }

    fn handle_delete(&mut self) {
        if let Some((min_ch, max_ch, _, _, min_row, max_row)) = self.selection_bounds() {
            for ch in min_ch..=max_ch {
                let voices = self.project.current_pattern().voice_count(ch);
                for v in 0..voices {
                    for row in min_row..=max_row {
                        self.project.current_pattern_mut().clear(ch, v, row);
                    }
                }
            }
            self.clear_selection();
        } else {
            self.project.current_pattern_mut().clear(
                self.cursor.channel,
                self.cursor.voice,
                self.cursor.row,
            );
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.current_pattern().rows;
        }
    }

    fn handle_copy(&mut self) {
        let (min_ch, max_ch, _, _, min_row, max_row) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.voice,
            self.cursor.voice,
            self.cursor.row,
            self.cursor.row,
        ));
        let pat = self.project.current_pattern();

        let data: Vec<Vec<Vec<Cell>>> = (min_ch..=max_ch)
            .map(|ch| {
                let voices = pat.voice_count(ch);
                (0..voices)
                    .map(|v| (min_row..=max_row).map(|r| pat.data[ch][v][r]).collect())
                    .collect()
            })
            .collect();
        self.clipboard = Some(ClipboardData::Notes(data));
    }

    fn handle_paste(&mut self) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        self.clear_selection();

        let pat = self.project.current_pattern_mut();
        let ch_start = self.cursor.channel;
        let row_start = self.cursor.row;

        match clip {
            ClipboardData::Notes(data) => {
                for (ci, ch_data) in data.iter().enumerate() {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for (vi, voice_data) in ch_data.iter().enumerate() {
                        if vi >= pat.data[ch].len() {
                            break;
                        }
                        for (ri, &cell) in voice_data.iter().enumerate() {
                            let row = row_start + ri;
                            if row >= pat.rows {
                                break;
                            }
                            pat.data[ch][vi][row] = cell;
                        }
                    }
                }
            }
        }
    }

    fn handle_note_off(&mut self) {
        self.clear_selection();
        self.project.current_pattern_mut().set(
            self.cursor.channel,
            self.cursor.voice,
            self.cursor.row,
            Cell::NoteOff,
        );
        self.advance_cursor();
    }

    fn handle_transpose(&mut self, actions: &[Action]) -> bool {
        let delta: i16 = if actions.contains(&Action::TransposeOctaveUp) {
            12
        } else if actions.contains(&Action::TransposeOctaveDown) {
            -12
        } else if actions.contains(&Action::TransposeUp) {
            1
        } else if actions.contains(&Action::TransposeDown) {
            -1
        } else {
            return false;
        };

        let (min_ch, max_ch, _, _, min_row, max_row) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.voice,
            self.cursor.voice,
            self.cursor.row,
            self.cursor.row,
        ));

        let mut min_pitch: Option<u8> = None;
        let mut max_pitch: Option<u8> = None;
        for ch in min_ch..=max_ch {
            let voices = self.project.current_pattern().voice_count(ch);
            for v in 0..voices {
                for row in min_row..=max_row {
                    if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, v, row) {
                        min_pitch = Some(min_pitch.map_or(note.pitch, |p: u8| p.min(note.pitch)));
                        max_pitch = Some(max_pitch.map_or(note.pitch, |p: u8| p.max(note.pitch)));
                    }
                }
            }
        }

        let can_transpose = if delta > 0 {
            max_pitch.is_some_and(|p| (i16::from(p) + delta) <= 127)
        } else {
            min_pitch.is_some_and(|p| (i16::from(p) + delta) >= 0)
        };

        if can_transpose {
            self.save_undo_snapshot();
            for ch in min_ch..=max_ch {
                let voices = self.project.current_pattern().voice_count(ch);
                for v in 0..voices {
                    for row in min_row..=max_row {
                        if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, v, row) {
                            let new_pitch = u8::try_from(i16::from(note.pitch) + delta).unwrap();
                            self.project.current_pattern_mut().set(
                                ch,
                                v,
                                row,
                                Cell::NoteOn(crate::project::Note::new(new_pitch)),
                            );
                        }
                    }
                }
            }
        }

        true
    }

    fn handle_fill(&mut self, ascending: bool) {
        let ch = self.cursor.channel;
        let v = self.cursor.voice;
        let start_row = self.cursor.row;
        let total_rows = self.project.current_pattern().rows;

        let cell = self.project.current_pattern().get(ch, v, start_row);
        if let Cell::NoteOn(note) = cell {
            let mut pitch = i16::from(note.pitch);
            for row in (start_row + 1)..total_rows {
                if self.project.current_pattern().get(ch, v, row) != Cell::Empty {
                    break;
                }
                pitch += if ascending { 1 } else { -1 };
                let clamped = pitch.clamp(0, 127) as u8;
                self.project.current_pattern_mut().set(
                    ch,
                    v,
                    row,
                    Cell::NoteOn(Note::new(clamped)),
                );
            }
        }
    }

    fn handle_note_keys(&mut self, input: &egui::InputState) {
        let note_keys = [
            Key::Z,
            Key::X,
            Key::C,
            Key::V,
            Key::B,
            Key::N,
            Key::M,
            Key::A,
            Key::S,
            Key::D,
            Key::F,
            Key::G,
            Key::H,
            Key::J,
            Key::K,
            Key::L,
            Key::Q,
            Key::W,
            Key::E,
            Key::R,
            Key::T,
            Key::Y,
            Key::U,
            Key::I,
            Key::O,
            Key::P,
        ];
        for &k in &note_keys {
            if input.key_pressed(k) {
                self.save_undo_snapshot();
                let scale = self.project.scale_index.scale();
                if let Some(note) =
                    key_to_note(k, self.cursor.octave, scale, self.project.transpose)
                {
                    self.project.current_pattern_mut().set(
                        self.cursor.channel,
                        self.cursor.voice,
                        self.cursor.row,
                        Cell::NoteOn(note),
                    );
                    if !self.playback.playing {
                        self.audio.preview_note(
                            note.frequency(),
                            self.current_track,
                            &self.project.tracks,
                            self.project.master_volume_linear(),
                        );
                    }
                    self.clear_selection();
                    if self.poly_input {
                        let voices = self.voices_for_channel(self.cursor.channel);
                        if self.cursor.voice + 1 < voices {
                            self.cursor.voice += 1;
                        } else {
                            self.cursor.voice = 0;
                            self.advance_cursor();
                        }
                    } else {
                        self.advance_cursor();
                    }
                }
                break;
            }
        }
    }

    fn advance_cursor(&mut self) {
        if self.cursor.row < self.project.current_pattern().rows - 1
            && self.cursor.row + self.project.step < self.project.current_pattern().rows
        {
            self.cursor.row += self.project.step;
        } else {
            self.cursor.row = self.project.current_pattern().rows - 1;
        }
    }
}

pub fn key_to_note(key: Key, octave: u8, scale: &Scale, transpose: i8) -> Option<Note> {
    let idx: Option<u8> = match key {
        Key::Z => Some(0),
        Key::X => Some(1),
        Key::C => Some(2),
        Key::V => Some(3),
        Key::B => Some(4),
        Key::N => Some(5),
        Key::M => Some(6),
        Key::A => Some(7),
        Key::S => Some(8),
        Key::D => Some(9),
        Key::F => Some(10),
        Key::G => Some(11),

        Key::H => Some(12),
        Key::J => Some(13),
        Key::K => Some(14),
        Key::L => Some(15),
        Key::Q => Some(16),
        Key::W => Some(17),
        Key::E => Some(18),
        Key::R => Some(19),
        Key::T => Some(20),
        Key::Y => Some(21),
        Key::U => Some(22),
        Key::I => Some(23),
        Key::O => Some(24),
        Key::P => Some(25),

        _ => None,
    };

    idx.map(|i| {
        let note = map_key_index_to_note(i, octave, scale, transpose);
        Note::new(note)
    })
}
