use eframe::egui::{self, Key};

use crate::app::keybindings::Action;
use crate::app::scale::{Scale, map_key_index_to_note};
use crate::project::Cell;
use crate::project::Note;

use super::{App, ClipboardData, Mode, MovePreview};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        self.clamp_cursor();

        let wants_kb = ctx.wants_keyboard_input();

        let clipboard_handled = ctx.input(|input| {
            if self.text_editing || wants_kb {
                return false;
            }
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
                return true;
            }
            if has_copy {
                self.handle_copy();
                return true;
            }
            if has_paste {
                self.save_undo_snapshot();
                self.handle_paste();
                return true;
            }
            false
        });

        if clipboard_handled {
            return false;
        }

        if self.text_editing || wants_kb {
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

            self.tick_chord_buffer();

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


        if actions.contains(&Action::NoteOff) {
            self.save_undo_snapshot();
            self.handle_note_off();
            return false;
        }

        if self.handle_transpose(actions) {
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

        if actions.contains(&Action::InputTransposeUp) {
            if self.project.transpose < 12 {
                self.project.transpose += 1;
            }
            return false;
        }

        if actions.contains(&Action::InputTransposeDown) {
            if self.project.transpose > -12 {
                self.project.transpose -= 1;
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

        let total_cols = self.total_columns();
        let total_rows = self.project.current_pattern().rows;

        if self.move_preview.is_some() {
            let (_, _, _, _, min_row, max_row) = self.selection_bounds().unwrap();
            let cur_flat = self.flat_col(self.cursor.channel, self.cursor.voice) as isize;
            let anc_flat = self.cursor.selection_anchor
                .map(|(ach, av, _)| self.flat_col(ach, av) as isize)
                .unwrap_or(cur_flat);
            let min_flat = cur_flat.min(anc_flat);
            let max_flat = cur_flat.max(anc_flat);

            let new_min_row = min_row as isize + dr;
            let new_max_row = max_row as isize + dr;
            let new_min_flat = min_flat + dc;
            let new_max_flat = max_flat + dc;

            if new_min_row >= 0 && new_max_row < total_rows as isize
                && new_min_flat >= 0 && new_max_flat < total_cols as isize
            {
                let new_cur_flat = (cur_flat + dc) as usize;
                let new_anc_flat = (anc_flat + dc) as usize;
                let (nch, nv) = self.resolve_flat_col(new_cur_flat).unwrap();
                let (nach, nav) = self.resolve_flat_col(new_anc_flat).unwrap();
                let new_cur_row = (self.cursor.row as isize + dr) as usize;
                self.cursor.channel = nch;
                self.cursor.voice = nv;
                self.cursor.row = new_cur_row;
                if let Some((ach, av, arow)) = self.cursor.selection_anchor.as_mut() {
                    *ach = nach;
                    *av = nav;
                    *arow = (*arow as isize + dr) as usize;
                }
            }
            return true;
        }

        if let Some((min_ch, max_ch, min_v, max_v, min_row, max_row)) = self.selection_bounds() {
            let min_flat = self.flat_col(min_ch, min_v) as isize;
            let max_flat = self.flat_col(max_ch, max_v) as isize;

            let new_min_row = min_row as isize + dr;
            let new_max_row = max_row as isize + dr;
            let new_min_flat = min_flat + dc;
            let new_max_flat = max_flat + dc;

            if new_min_row >= 0 && new_max_row < total_rows as isize
                && new_min_flat >= 0 && new_max_flat < total_cols as isize
            {
                self.save_undo_snapshot();

                let mut cells = Vec::new();
                for flat in min_flat as usize..=max_flat as usize {
                    let (ch, v) = self.resolve_flat_col(flat).unwrap();
                    let ch_rows = self.project.current_pattern().track_rows(ch);
                    for row in min_row..=max_row {
                        if row >= ch_rows {
                            cells.push((flat - min_flat as usize, row - min_row, Cell::Empty));
                        } else {
                            let cell = self.project.current_pattern().get(ch, v, row);
                            cells.push((flat - min_flat as usize, row - min_row, cell));
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

                let cur_flat = self.flat_col(self.cursor.channel, self.cursor.voice) as isize;
                let anc_flat = self.flat_col(anchor.0, anchor.1) as isize;
                let new_cur_flat = (cur_flat + dc) as usize;
                let new_anc_flat = (anc_flat + dc) as usize;
                let (nch, nv) = self.resolve_flat_col(new_cur_flat).unwrap();
                let (nach, nav) = self.resolve_flat_col(new_anc_flat).unwrap();
                let new_cur_row = (self.cursor.row as isize + dr) as usize;
                self.cursor.channel = nch;
                self.cursor.voice = nv;
                self.cursor.row = new_cur_row;
                if let Some((ach, av, arow)) = self.cursor.selection_anchor.as_mut() {
                    *ach = nach;
                    *av = nav;
                    *arow = (*arow as isize + dr) as usize;
                }
            }
        } else {
            let cur_flat = self.flat_col(self.cursor.channel, self.cursor.voice) as isize;
            let new_flat = cur_flat + dc;
            let new_row = self.cursor.row as isize + dr;
            if new_flat >= 0 && new_flat < total_cols as isize
                && new_row >= 0 && new_row < total_rows as isize
            {
                self.save_undo_snapshot();
                let v = self.cursor.voice;
                let cell = self.project.current_pattern().get(self.cursor.channel, v, self.cursor.row);
                self.project.current_pattern_mut().clear(self.cursor.channel, v, self.cursor.row);
                let (nch, nv) = self.resolve_flat_col(new_flat as usize).unwrap();
                self.project.current_pattern_mut().set(nch, nv, new_row as usize, cell);
                self.cursor.channel = nch;
                self.cursor.voice = nv;
                self.cursor.row = new_row as usize;
            }
        }

        true
    }

    fn confirm_move_preview(&mut self) {
        let Some(preview) = self.move_preview.take() else {
            return;
        };
        let (min_ch, _, min_v, _, min_row, _) = self.selection_bounds().unwrap();
        let base_flat = self.flat_col(min_ch, min_v);
        for (col_off, row_off, cell) in &preview.cells {
            let flat = base_flat + col_off;
            let row = min_row + row_off;
            if let Some((ch, v)) = self.resolve_flat_col(flat)
                && row < self.project.current_pattern().track_rows(ch)
            {
                self.project.current_pattern_mut().set(ch, v, row, *cell);
            }
        }
        self.clear_selection();
    }

    pub fn cancel_move_preview(&mut self) {
        let Some(preview) = self.move_preview.take() else {
            return;
        };
        let (orig_ach, orig_av, orig_arow) = preview.origin_anchor;
        let (orig_ch, orig_voice, orig_row) = preview.origin_cursor;
        let orig_min_flat = self.flat_col(orig_ach.min(orig_ch), if orig_ach <= orig_ch { orig_av } else { orig_voice });
        let base_row = orig_arow.min(orig_row);

        for (col_off, row_off, cell) in &preview.cells {
            let flat = orig_min_flat + col_off;
            let row = base_row + row_off;
            if let Some((ch, v)) = self.resolve_flat_col(flat)
                && row < self.project.current_pattern().track_rows(ch)
            {
                self.project.current_pattern_mut().set(ch, v, row, *cell);
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
        let total_channels = self.project.channels;

        match dir {
            Action::CursorUp | Action::SelectUp => {
                if self.cursor.row > 0 {
                    self.cursor.row -= 1;
                } else {
                    self.cursor.row = self.project.current_pattern().track_rows(self.cursor.channel) - 1;
                }
            }
            Action::CursorDown | Action::SelectDown => {
                if self.cursor.row < self.project.current_pattern().track_rows(self.cursor.channel) - 1 {
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
                let track_rows = self.project.current_pattern().track_rows(self.cursor.channel);
                if self.cursor.row >= track_rows {
                    self.cursor.row = track_rows.saturating_sub(1);
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
                let track_rows = self.project.current_pattern().track_rows(self.cursor.channel);
                if self.cursor.row >= track_rows {
                    self.cursor.row = track_rows.saturating_sub(1);
                }
            }
            _ => {}
        }
    }

    fn handle_delete(&mut self) {
        if let Some((min_ch, max_ch, min_v, max_v, min_row, max_row)) = self.selection_bounds() {
            for ch in min_ch..=max_ch {
                let voices = self.project.current_pattern().voice_count(ch);
                for v in 0..voices {
                    let in_sel = if min_ch == max_ch {
                        v >= min_v && v <= max_v
                    } else if ch == min_ch {
                        v >= min_v
                    } else if ch == max_ch {
                        v <= max_v
                    } else {
                        true
                    };
                    if !in_sel {
                        continue;
                    }
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
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.current_pattern().track_rows(self.cursor.channel);
        }
    }

    fn handle_copy(&mut self) {
        let (min_ch, max_ch, min_v, max_v, min_row, max_row) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.voice,
            self.cursor.voice,
            self.cursor.row,
            self.cursor.row,
        ));
        let pat = self.project.current_pattern();
        let base_flat = self.flat_col(min_ch, min_v);
        let end_flat = self.flat_col(max_ch, max_v);

        let mut cells = Vec::new();
        for flat in base_flat..=end_flat {
            if let Some((ch, v)) = self.resolve_flat_col(flat) {
                let col_off = flat - base_flat;
                for row in min_row..=max_row {
                    let cell = pat.get(ch, v, row);
                    cells.push((col_off, row - min_row, cell));
                }
            }
        }
        self.clipboard = Some(ClipboardData::Notes(cells));
    }

    fn handle_paste(&mut self) {
        let Some(clip) = self.clipboard.clone() else {
            return;
        };
        self.clear_selection();

        let cursor_flat = self.flat_col(self.cursor.channel, self.cursor.voice);
        let row_start = self.cursor.row;

        match clip {
            ClipboardData::Notes(cells) => {
                for &(col_off, row_off, cell) in &cells {
                    let target_flat = cursor_flat + col_off;
                    let target_row = row_start + row_off;
                    if let Some((ch, v)) = self.resolve_flat_col(target_flat) {
                        let pat = self.project.current_pattern_mut();
                        if target_row < pat.rows {
                            pat.data[ch][v][target_row] = cell;
                        }
                    }
                }
            }
        }
    }

    fn handle_note_off(&mut self) {
        self.clear_selection();
        if self.poly_input {
            let voices = self.voices_for_channel(self.cursor.channel);
            for v in 0..voices {
                self.project.current_pattern_mut().set(
                    self.cursor.channel,
                    v,
                    self.cursor.row,
                    Cell::NoteOff,
                );
            }
        } else {
            self.project.current_pattern_mut().set(
                self.cursor.channel,
                self.cursor.voice,
                self.cursor.row,
                Cell::NoteOff,
            );
        }
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

        let (min_ch, max_ch, min_v, max_v, min_row, max_row) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.voice,
            self.cursor.voice,
            self.cursor.row,
            self.cursor.row,
        ));

        let voice_in_sel = |ch: usize, v: usize| -> bool {
            if min_ch == max_ch {
                v >= min_v && v <= max_v
            } else if ch == min_ch {
                v >= min_v
            } else if ch == max_ch {
                v <= max_v
            } else {
                true
            }
        };

        let mut min_pitch: Option<u8> = None;
        let mut max_pitch: Option<u8> = None;
        for ch in min_ch..=max_ch {
            let voices = self.project.current_pattern().voice_count(ch);
            for v in 0..voices {
                if !voice_in_sel(ch, v) {
                    continue;
                }
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
                    if !voice_in_sel(ch, v) {
                        continue;
                    }
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



    fn handle_note_keys(&mut self, input: &egui::InputState) {
        let note_keys = [
            Key::Z, Key::X, Key::C, Key::V, Key::B, Key::N, Key::M,
            Key::A, Key::S, Key::D, Key::F, Key::G, Key::H, Key::J,
            Key::K, Key::L, Key::Q, Key::W, Key::E, Key::R, Key::T,
            Key::Y, Key::U, Key::I, Key::O, Key::P,
        ];
        let scale = self.project.scale_index.scale();
        let mut new_notes: Vec<Note> = Vec::new();
        for &k in &note_keys {
            if input.key_pressed(k)
                && let Some(note) = key_to_note(k, self.cursor.octave, scale, self.project.transpose)
            {
                new_notes.push(note);
            }
        }
        if new_notes.is_empty() {
            return;
        }
        if self.poly_input {
            if self.chord_buffer.is_empty() {
                self.save_undo_snapshot();
            }
            for note in &new_notes {
                if !self.chord_buffer.iter().any(|n| n.pitch == note.pitch) {
                    self.chord_buffer.push(*note);
                }
            }
            self.chord_frames_remaining = 3;
            if !self.playback.playing {
                let freqs: Vec<f32> = self.chord_buffer.iter().map(|n| n.frequency()).collect();
                self.audio.preview_notes(
                    &freqs,
                    self.current_track,
                    &self.project.tracks,
                    self.project.master_volume_linear(),
                );
            }
        } else {
            self.save_undo_snapshot();
            self.clear_selection();
            let note = new_notes[0];
            self.project.current_pattern_mut().set(
                self.cursor.channel,
                self.cursor.voice,
                self.cursor.row,
                Cell::NoteOn(note),
            );
            if !self.playback.playing {
                self.audio.preview_notes(
                    &[note.frequency()],
                    self.current_track,
                    &self.project.tracks,
                    self.project.master_volume_linear(),
                );
            }
            self.advance_cursor();
        }
    }

    fn tick_chord_buffer(&mut self) {
        if !self.poly_input || self.chord_buffer.is_empty() {
            return;
        }
        if self.chord_frames_remaining > 0 {
            self.chord_frames_remaining -= 1;
            if self.chord_frames_remaining == 0 {
                self.commit_chord();
            }
        }
    }

    fn commit_chord(&mut self) {
        let notes: Vec<Note> = self.chord_buffer.drain(..).collect();
        if notes.is_empty() {
            return;
        }
        self.clear_selection();
        let voices = self.voices_for_channel(self.cursor.channel);
        for (i, &note) in notes.iter().enumerate() {
            let v = (self.cursor.voice + i).min(voices - 1);
            self.project.current_pattern_mut().set(
                self.cursor.channel,
                v,
                self.cursor.row,
                Cell::NoteOn(note),
            );
        }
        self.advance_cursor();
    }

    fn advance_cursor(&mut self) {
        let rows = self.project.current_pattern().track_rows(self.cursor.channel);
        self.cursor.row = (self.cursor.row + self.project.step) % rows;
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
