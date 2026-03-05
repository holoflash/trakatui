use eframe::egui::{self, Key};

use crate::app::keybindings::Action;
use crate::app::scale::{Scale, map_key_index_to_note};
use crate::project::Note;
use crate::project::{Cell, Effect, SampleData};

use super::{App, ClipboardData, Mode, SettingsField, SubColumn, SynthSettingsField};

impl App {
    pub fn handle_input(&mut self, ctx: &egui::Context) -> bool {
        if self.text_editing {
            return false;
        }
        ctx.input(|input| {
            let actions = self.keybindings.active_actions(input);

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

            match self.mode {
                Mode::Edit => self.handle_edit_input(input, &actions),
                Mode::Settings => {
                    self.handle_settings_input(&actions);
                    false
                }
                Mode::SynthEdit => {
                    self.handle_synth_input(&actions);
                    false
                }
            }
        })
    }

    fn handle_edit_input(&mut self, input: &egui::InputState, actions: &[Action]) -> bool {
        if actions.contains(&Action::SwitchToSynth) {
            self.clear_selection();
            self.mode = Mode::SynthEdit;
            self.synth_field = SynthSettingsField::Instrument;
            return false;
        }

        if actions.contains(&Action::SwitchToSettings) {
            self.clear_selection();
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Scale;
            return false;
        }

        if self.handle_move(actions) {
            return false;
        }

        if self.handle_cursor_and_select(actions) {
            return false;
        }

        if actions.contains(&Action::Delete) {
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
                self.handle_copy();
                self.handle_delete();
                return false;
            }
            if has_copy {
                self.handle_copy();
                return false;
            }
            if has_paste {
                self.handle_paste();
                return false;
            }
        }

        if actions.contains(&Action::NoteOff) && self.cursor.sub_column == SubColumn::Note {
            self.handle_note_off();
            return false;
        }

        if self.handle_transpose(actions) {
            return false;
        }

        if actions.contains(&Action::FillAscending) {
            self.handle_fill(true);
            return false;
        }

        if actions.contains(&Action::FillDescending) {
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

        if self.cursor.sub_column == SubColumn::Effect {
            self.handle_effect_keys(input);
        } else if self.cursor.sub_column == SubColumn::Volume {
            self.handle_volume_keys(input);
        } else if self.cursor.sub_column == SubColumn::Instrument {
            self.handle_instrument_keys(input);
        } else {
            self.handle_note_keys(input);
        }

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

        let on_note = self.cursor.sub_column;

        if let Some((min_ch, max_ch, min_row, max_row, _, _)) = self.selection_bounds() {
            let in_bounds = min_row.checked_add_signed(dr).is_some()
                && max_row
                    .checked_add_signed(dr)
                    .is_some_and(|r| r < self.project.current_pattern().rows)
                && min_ch.checked_add_signed(dc).is_some()
                && max_ch
                    .checked_add_signed(dc)
                    .is_some_and(|c| c < self.project.current_pattern().channels);

            if in_bounds {
                let mut cells = Vec::new();
                for ch in min_ch..=max_ch {
                    for row in min_row..=max_row {
                        let cell = self.project.current_pattern().get(ch, row);
                        let inst = self.project.current_pattern().get_instrument(ch, row);
                        let vol = self.project.current_pattern().get_volume(ch, row);
                        let fx = self.project.current_pattern().get_effect(ch, row);
                        cells.push((ch, row, cell, inst, vol, fx));
                        match on_note {
                            SubColumn::Note => self.project.current_pattern_mut().clear(ch, row),
                            SubColumn::Instrument => {
                                self.project.current_pattern_mut().clear_instrument(ch, row)
                            }
                            SubColumn::Volume => {
                                self.project.current_pattern_mut().clear_volume(ch, row)
                            }
                            SubColumn::Effect => {
                                self.project.current_pattern_mut().clear_effect(ch, row)
                            }
                        }
                    }
                }
                for (ch, row, cell, inst, vol, fx) in cells {
                    let new_ch = ch.checked_add_signed(dc).unwrap();
                    let new_row = row.checked_add_signed(dr).unwrap();
                    match on_note {
                        SubColumn::Note => self
                            .project
                            .current_pattern_mut()
                            .set(new_ch, new_row, cell),
                        SubColumn::Instrument => self
                            .project
                            .current_pattern_mut()
                            .set_instrument(new_ch, new_row, inst),
                        SubColumn::Volume => self
                            .project
                            .current_pattern_mut()
                            .set_volume(new_ch, new_row, vol),
                        SubColumn::Effect => self
                            .project
                            .current_pattern_mut()
                            .set_effect(new_ch, new_row, fx),
                    }
                }
                self.cursor.channel = self.cursor.channel.checked_add_signed(dc).unwrap();
                self.cursor.row = self.cursor.row.checked_add_signed(dr).unwrap();
                if let Some((ach, arow, _)) = self.cursor.selection_anchor.as_mut() {
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
            if on_note == SubColumn::Note {
                let cell = self
                    .project
                    .current_pattern()
                    .get(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set(new_ch, new_row, cell);
            } else if on_note == SubColumn::Instrument {
                let inst = self
                    .project
                    .current_pattern()
                    .get_instrument(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_instrument(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_instrument(new_ch, new_row, inst);
            } else if on_note == SubColumn::Volume {
                let vol = self
                    .project
                    .current_pattern()
                    .get_volume(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_volume(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_volume(new_ch, new_row, vol);
            } else {
                let fx = self
                    .project
                    .current_pattern()
                    .get_effect(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .clear_effect(self.cursor.channel, self.cursor.row);
                self.project
                    .current_pattern_mut()
                    .set_effect(new_ch, new_row, fx);
            }
            self.cursor.channel = new_ch;
            self.cursor.row = new_row;
        }

        true
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
                Some((self.cursor.channel, self.cursor.row, self.cursor.sub_column));
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
            Action::CursorLeft => {
                if self.cursor.sub_column == SubColumn::Effect {
                    self.cursor.sub_column = SubColumn::Volume;
                    self.cursor.volume_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Instrument;
                    self.cursor.instrument_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Note;
                } else if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                } else {
                    self.cursor.channel = self.project.current_pattern().channels - 1;
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                }
            }
            Action::CursorRight => {
                if self.cursor.sub_column == SubColumn::Note {
                    self.cursor.sub_column = SubColumn::Instrument;
                    self.cursor.instrument_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Volume;
                    self.cursor.volume_edit_pos = 0;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Effect;
                    self.cursor.effect_edit_pos = 0;
                } else if self.cursor.channel < self.project.current_pattern().channels - 1 {
                    self.cursor.channel += 1;
                    self.cursor.sub_column = SubColumn::Note;
                } else {
                    self.cursor.channel = 0;
                    self.cursor.sub_column = SubColumn::Note;
                }
            }
            Action::SelectLeft => {
                if self.cursor.sub_column == SubColumn::Effect {
                    self.cursor.sub_column = SubColumn::Volume;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Instrument;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Note;
                } else if self.cursor.channel > 0 {
                    self.cursor.channel -= 1;
                    self.cursor.sub_column = SubColumn::Effect;
                } else {
                    self.cursor.channel = self.project.current_pattern().channels - 1;
                    self.cursor.sub_column = SubColumn::Effect;
                }
            }
            Action::SelectRight => {
                if self.cursor.sub_column == SubColumn::Note {
                    self.cursor.sub_column = SubColumn::Instrument;
                } else if self.cursor.sub_column == SubColumn::Instrument {
                    self.cursor.sub_column = SubColumn::Volume;
                } else if self.cursor.sub_column == SubColumn::Volume {
                    self.cursor.sub_column = SubColumn::Effect;
                } else if self.cursor.channel < self.project.current_pattern().channels - 1 {
                    self.cursor.channel += 1;
                    self.cursor.sub_column = SubColumn::Note;
                } else {
                    self.cursor.channel = 0;
                    self.cursor.sub_column = SubColumn::Note;
                }
            }
            _ => {}
        }
    }

    fn handle_delete(&mut self) {
        if let Some((min_ch, max_ch, min_row, max_row, min_sub, max_sub)) = self.selection_bounds()
        {
            for ch in min_ch..=max_ch {
                for row in min_row..=max_row {
                    let flat_note = ch * 4 + SubColumn::Note as usize;
                    let flat_inst = ch * 4 + SubColumn::Instrument as usize;
                    let flat_vol = ch * 4 + SubColumn::Volume as usize;
                    let flat_fx = ch * 4 + SubColumn::Effect as usize;
                    let sel_start = min_ch * 4 + min_sub as usize;
                    let sel_end = max_ch * 4 + max_sub as usize;

                    if flat_note >= sel_start && flat_note <= sel_end {
                        self.project.current_pattern_mut().clear(ch, row);
                    }
                    if flat_inst >= sel_start && flat_inst <= sel_end {
                        self.project.current_pattern_mut().clear_instrument(ch, row);
                    }
                    if flat_vol >= sel_start && flat_vol <= sel_end {
                        self.project.current_pattern_mut().clear_volume(ch, row);
                    }
                    if flat_fx >= sel_start && flat_fx <= sel_end {
                        self.project.current_pattern_mut().clear_effect(ch, row);
                    }
                }
            }
            self.clear_selection();
        } else {
            match self.cursor.sub_column {
                SubColumn::Note => {
                    self.project
                        .current_pattern_mut()
                        .clear(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Instrument => {
                    self.project
                        .current_pattern_mut()
                        .clear_instrument(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Volume => {
                    self.project
                        .current_pattern_mut()
                        .clear_volume(self.cursor.channel, self.cursor.row);
                }
                SubColumn::Effect => {
                    self.project
                        .current_pattern_mut()
                        .clear_effect(self.cursor.channel, self.cursor.row);
                }
            }
            self.cursor.row = self.cursor.row.wrapping_sub(1) % self.project.current_pattern().rows;
        }
    }

    fn handle_copy(&mut self) {
        let (min_ch, max_ch, min_row, max_row, min_sub, max_sub) =
            self.selection_bounds().unwrap_or((
                self.cursor.channel,
                self.cursor.channel,
                self.cursor.row,
                self.cursor.row,
                self.cursor.sub_column,
                self.cursor.sub_column,
            ));
        let pat = self.project.current_pattern();

        if min_sub != max_sub || min_ch != max_ch {
            let sel_start = min_ch * 4 + min_sub as usize;
            let sel_end = max_ch * 4 + max_sub as usize;

            let has_sub = |sub: SubColumn| -> bool {
                (min_ch..=max_ch).any(|ch| {
                    let flat = ch * 4 + sub as usize;
                    flat >= sel_start && flat <= sel_end
                })
            };

            let notes = if has_sub(SubColumn::Note) {
                Some(
                    (min_ch..=max_ch)
                        .map(|ch| (min_row..=max_row).map(|r| pat.data[ch][r]).collect())
                        .collect(),
                )
            } else {
                None
            };
            let instruments = if has_sub(SubColumn::Instrument) {
                Some(
                    (min_ch..=max_ch)
                        .map(|ch| {
                            (min_row..=max_row)
                                .map(|r| pat.instruments[ch][r])
                                .collect()
                        })
                        .collect(),
                )
            } else {
                None
            };
            let volumes = if has_sub(SubColumn::Volume) {
                Some(
                    (min_ch..=max_ch)
                        .map(|ch| (min_row..=max_row).map(|r| pat.volumes[ch][r]).collect())
                        .collect(),
                )
            } else {
                None
            };
            let effects = if has_sub(SubColumn::Effect) {
                Some(
                    (min_ch..=max_ch)
                        .map(|ch| (min_row..=max_row).map(|r| pat.effects[ch][r]).collect())
                        .collect(),
                )
            } else {
                None
            };
            self.clipboard = Some(ClipboardData::Full {
                notes,
                instruments,
                volumes,
                effects,
            });
            return;
        }

        self.clipboard = Some(match min_sub {
            SubColumn::Note => {
                let data: Vec<Vec<Cell>> = (min_ch..=max_ch)
                    .map(|ch| (min_row..=max_row).map(|r| pat.data[ch][r]).collect())
                    .collect();
                ClipboardData::Notes(data)
            }
            SubColumn::Instrument => {
                let data: Vec<Vec<Option<u8>>> = (min_ch..=max_ch)
                    .map(|ch| {
                        (min_row..=max_row)
                            .map(|r| pat.instruments[ch][r])
                            .collect()
                    })
                    .collect();
                ClipboardData::Instruments(data)
            }
            SubColumn::Volume => {
                let data: Vec<Vec<Option<u8>>> = (min_ch..=max_ch)
                    .map(|ch| (min_row..=max_row).map(|r| pat.volumes[ch][r]).collect())
                    .collect();
                ClipboardData::Volumes(data)
            }
            SubColumn::Effect => {
                let data: Vec<Vec<Option<Effect>>> = (min_ch..=max_ch)
                    .map(|ch| (min_row..=max_row).map(|r| pat.effects[ch][r]).collect())
                    .collect();
                ClipboardData::Effects(data)
            }
        });
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
                for (ci, col) in data.iter().enumerate() {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for (ri, &cell) in col.iter().enumerate() {
                        let row = row_start + ri;
                        if row >= pat.rows {
                            break;
                        }
                        pat.data[ch][row] = cell;
                    }
                }
            }
            ClipboardData::Instruments(data) => {
                for (ci, col) in data.iter().enumerate() {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for (ri, &val) in col.iter().enumerate() {
                        let row = row_start + ri;
                        if row >= pat.rows {
                            break;
                        }
                        pat.instruments[ch][row] = val;
                    }
                }
            }
            ClipboardData::Volumes(data) => {
                for (ci, col) in data.iter().enumerate() {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for (ri, &val) in col.iter().enumerate() {
                        let row = row_start + ri;
                        if row >= pat.rows {
                            break;
                        }
                        pat.volumes[ch][row] = val;
                    }
                }
            }
            ClipboardData::Effects(data) => {
                for (ci, col) in data.iter().enumerate() {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for (ri, &val) in col.iter().enumerate() {
                        let row = row_start + ri;
                        if row >= pat.rows {
                            break;
                        }
                        pat.effects[ch][row] = val;
                    }
                }
            }
            ClipboardData::Full {
                notes,
                instruments,
                volumes,
                effects,
            } => {
                let num_ch = notes
                    .as_ref()
                    .map(|v| v.len())
                    .or_else(|| instruments.as_ref().map(|v| v.len()))
                    .or_else(|| volumes.as_ref().map(|v| v.len()))
                    .or_else(|| effects.as_ref().map(|v| v.len()))
                    .unwrap_or(0);
                let num_rows = notes
                    .as_ref()
                    .and_then(|v| v.first())
                    .map(|c| c.len())
                    .or_else(|| {
                        instruments
                            .as_ref()
                            .and_then(|v| v.first())
                            .map(|c| c.len())
                    })
                    .or_else(|| volumes.as_ref().and_then(|v| v.first()).map(|c| c.len()))
                    .or_else(|| effects.as_ref().and_then(|v| v.first()).map(|c| c.len()))
                    .unwrap_or(0);
                for ci in 0..num_ch {
                    let ch = ch_start + ci;
                    if ch >= pat.channels {
                        break;
                    }
                    for ri in 0..num_rows {
                        let row = row_start + ri;
                        if row >= pat.rows {
                            break;
                        }
                        if let Some(ref n) = notes {
                            pat.data[ch][row] = n[ci][ri];
                        }
                        if let Some(ref inst) = instruments {
                            pat.instruments[ch][row] = inst[ci][ri];
                        }
                        if let Some(ref vol) = volumes {
                            pat.volumes[ch][row] = vol[ci][ri];
                        }
                        if let Some(ref fx) = effects {
                            pat.effects[ch][row] = fx[ci][ri];
                        }
                    }
                }
            }
        }
    }

    fn handle_note_off(&mut self) {
        self.clear_selection();
        self.project
            .current_pattern_mut()
            .set(self.cursor.channel, self.cursor.row, Cell::NoteOff);
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

        let (min_ch, max_ch, min_row, max_row, _, _) = self.selection_bounds().unwrap_or((
            self.cursor.channel,
            self.cursor.channel,
            self.cursor.row,
            self.cursor.row,
            self.cursor.sub_column,
            self.cursor.sub_column,
        ));

        let mut min_pitch: Option<u8> = None;
        let mut max_pitch: Option<u8> = None;
        for ch in min_ch..=max_ch {
            for row in min_row..=max_row {
                if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, row) {
                    min_pitch = Some(min_pitch.map_or(note.pitch, |p: u8| p.min(note.pitch)));
                    max_pitch = Some(max_pitch.map_or(note.pitch, |p: u8| p.max(note.pitch)));
                }
            }
        }

        let can_transpose = if delta > 0 {
            max_pitch.is_some_and(|p| (i16::from(p) + delta) <= 96)
        } else {
            min_pitch.is_some_and(|p| (i16::from(p) + delta) >= 1)
        };

        if can_transpose {
            for ch in min_ch..=max_ch {
                for row in min_row..=max_row {
                    if let Cell::NoteOn(note) = self.project.current_pattern().get(ch, row) {
                        let new_pitch = u8::try_from(i16::from(note.pitch) + delta).unwrap();
                        self.project.current_pattern_mut().set(
                            ch,
                            row,
                            Cell::NoteOn(crate::project::Note::new(new_pitch)),
                        );
                    }
                }
            }
        }

        true
    }

    fn handle_fill(&mut self, ascending: bool) {
        let ch = self.cursor.channel;
        let start_row = self.cursor.row;
        let total_rows = self.project.current_pattern().rows;

        match self.cursor.sub_column {
            SubColumn::Note => {
                let cell = self.project.current_pattern().get(ch, start_row);
                if let Cell::NoteOn(note) = cell {
                    let mut pitch = i16::from(note.pitch);
                    for row in (start_row + 1)..total_rows {
                        if self.project.current_pattern().get(ch, row) != Cell::Empty {
                            break;
                        }
                        pitch += if ascending { 1 } else { -1 };
                        let clamped = pitch.clamp(1, 96) as u8;
                        self.project.current_pattern_mut().set(
                            ch,
                            row,
                            Cell::NoteOn(Note::new(clamped)),
                        );
                    }
                }
            }
            SubColumn::Instrument => {
                if let Some(inst) = self.project.current_pattern().get_instrument(ch, start_row) {
                    let mut val = i16::from(inst);
                    for row in (start_row + 1)..total_rows {
                        if self
                            .project
                            .current_pattern()
                            .get_instrument(ch, row)
                            .is_some()
                        {
                            break;
                        }
                        val += if ascending { 1 } else { -1 };
                        let clamped = val.clamp(0, 0xFF) as u8;
                        self.project
                            .current_pattern_mut()
                            .set_instrument(ch, row, Some(clamped));
                    }
                }
            }
            SubColumn::Volume => {
                if let Some(vol) = self.project.current_pattern().get_volume(ch, start_row) {
                    let mut val = i16::from(vol);
                    for row in (start_row + 1)..total_rows {
                        if self.project.current_pattern().get_volume(ch, row).is_some() {
                            break;
                        }
                        val += if ascending { 1 } else { -1 };
                        let clamped = val.clamp(0, 0xFF) as u8;
                        self.project
                            .current_pattern_mut()
                            .set_volume(ch, row, Some(clamped));
                    }
                }
            }
            SubColumn::Effect => {
                if let Some(fx) = self.project.current_pattern().get_effect(ch, start_row) {
                    let mut param = i16::from(fx.param);
                    for row in (start_row + 1)..total_rows {
                        if self.project.current_pattern().get_effect(ch, row).is_some() {
                            break;
                        }
                        param += if ascending { 1 } else { -1 };
                        let clamped = param.clamp(0, 0xFF) as u8;
                        self.project.current_pattern_mut().set_effect(
                            ch,
                            row,
                            Some(Effect {
                                kind: fx.kind,
                                param: clamped,
                            }),
                        );
                    }
                }
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
                let scale = self.project.scale_index.scale();
                if let Some(note) =
                    key_to_note(k, self.cursor.octave, scale, self.project.transpose)
                {
                    self.project.current_pattern_mut().set(
                        self.cursor.channel,
                        self.cursor.row,
                        Cell::NoteOn(note),
                    );
                    if !self.playback.playing {
                        self.audio.preview_note(
                            note.frequency(),
                            self.current_instrument,
                            &self.project.instruments,
                            self.project.master_volume_linear(),
                        );
                    }
                    self.clear_selection();
                    self.advance_cursor();
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

    fn handle_effect_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.effect_edit_pos;

                let mut fx = self
                    .project
                    .current_pattern()
                    .get_effect(ch, row)
                    .unwrap_or(crate::project::Effect { kind: 0, param: 0 });

                match pos {
                    0 => fx.kind = value,
                    1 => fx.param = (fx.param & 0x0F) | (value << 4),
                    _ => fx.param = (fx.param & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_effect(ch, row, Some(fx));

                if pos < 2 {
                    self.cursor.effect_edit_pos = pos + 1;
                } else {
                    self.cursor.effect_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_volume_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.volume_edit_pos;

                let mut vol = self
                    .project
                    .current_pattern()
                    .get_volume(ch, row)
                    .unwrap_or(0);

                match pos {
                    0 => vol = (value << 4) | (vol & 0x0F),
                    _ => vol = (vol & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_volume(ch, row, Some(vol));

                if pos < 1 {
                    self.cursor.volume_edit_pos = pos + 1;
                } else {
                    self.cursor.volume_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_instrument_keys(&mut self, input: &egui::InputState) {
        let hex_keys = [
            (Key::Num0, 0x0),
            (Key::Num1, 0x1),
            (Key::Num2, 0x2),
            (Key::Num3, 0x3),
            (Key::Num4, 0x4),
            (Key::Num5, 0x5),
            (Key::Num6, 0x6),
            (Key::Num7, 0x7),
            (Key::Num8, 0x8),
            (Key::Num9, 0x9),
            (Key::A, 0xA),
            (Key::B, 0xB),
            (Key::C, 0xC),
            (Key::D, 0xD),
            (Key::E, 0xE),
            (Key::F, 0xF),
        ];

        for &(key, value) in &hex_keys {
            if input.key_pressed(key) {
                let ch = self.cursor.channel;
                let row = self.cursor.row;
                let pos = self.cursor.instrument_edit_pos;

                let mut inst = self
                    .project
                    .current_pattern()
                    .get_instrument(ch, row)
                    .unwrap_or(0);

                match pos {
                    0 => inst = (value << 4) | (inst & 0x0F),
                    _ => inst = (inst & 0xF0) | value,
                }

                self.project
                    .current_pattern_mut()
                    .set_instrument(ch, row, Some(inst));

                if pos < 1 {
                    self.cursor.instrument_edit_pos = pos + 1;
                } else {
                    self.cursor.instrument_edit_pos = 0;
                    self.clear_selection();
                    self.advance_cursor();
                }
                break;
            }
        }
    }

    fn handle_mode_switch(&mut self, actions: &[Action]) -> bool {
        if actions.contains(&Action::Escape) {
            if self.playback.playing {
                self.stop_playback();
            }
            self.mode = Mode::Edit;
            return true;
        }
        if actions.contains(&Action::SwitchToEdit) {
            self.mode = Mode::Edit;
            return true;
        }
        false
    }

    fn handle_settings_input(&mut self, actions: &[Action]) {
        if self.handle_mode_switch(actions) {
            return;
        }

        if actions.contains(&Action::SwitchToSynth) {
            self.mode = Mode::SynthEdit;
            self.synth_field = SynthSettingsField::Instrument;
        } else if actions.contains(&Action::SwitchToSettings) {
        } else if actions.contains(&Action::SettingsDown) {
            self.settings_field = self.settings_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.settings_field = self.settings_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            self.settings_field
                .adjust(&mut self.project, &mut self.cursor.row);
        } else if actions.contains(&Action::SettingsDecrease) {
            self.settings_field
                .adjust_down(&mut self.project, &mut self.cursor.row);
        }
    }

    fn handle_synth_input(&mut self, actions: &[Action]) {
        if self.handle_mode_switch(actions) {
            return;
        }

        if actions.contains(&Action::SwitchToSynth) {
        } else if actions.contains(&Action::SwitchToSettings) {
            self.mode = Mode::Settings;
            self.settings_field = SettingsField::Scale;
        } else if actions.contains(&Action::SettingsDown) {
            self.synth_field = self.synth_field.next();
        } else if actions.contains(&Action::SettingsUp) {
            self.synth_field = self.synth_field.prev();
        } else if actions.contains(&Action::SettingsIncrease) {
            if self.synth_field == SynthSettingsField::Instrument {
                self.current_instrument =
                    (self.current_instrument + 1) % self.project.instruments.len();
                self.envelope_point_idx = 0;
            } else if matches!(
                self.synth_field,
                SynthSettingsField::EnvPoints
                    | SynthSettingsField::EnvPoint
                    | SynthSettingsField::EnvTick
                    | SynthSettingsField::EnvValue
                    | SynthSettingsField::EnvSustain
                    | SynthSettingsField::EnvLoopStart
                    | SynthSettingsField::EnvLoopEnd
            ) {
                let idx = self.current_instrument;
                self.synth_field.adjust_envelope(
                    &mut self.project.instruments[idx],
                    1,
                    &mut self.envelope_point_idx,
                );
            } else {
                let idx = self.current_instrument;
                self.synth_field
                    .adjust(&mut self.project.instruments[idx], 1);
            }
        } else if actions.contains(&Action::SettingsDecrease) {
            if self.synth_field == SynthSettingsField::Instrument {
                if self.current_instrument == 0 {
                    self.current_instrument = self.project.instruments.len() - 1;
                } else {
                    self.current_instrument -= 1;
                }
                self.envelope_point_idx = 0;
            } else if matches!(
                self.synth_field,
                SynthSettingsField::EnvPoints
                    | SynthSettingsField::EnvPoint
                    | SynthSettingsField::EnvTick
                    | SynthSettingsField::EnvValue
                    | SynthSettingsField::EnvSustain
                    | SynthSettingsField::EnvLoopStart
                    | SynthSettingsField::EnvLoopEnd
            ) {
                let idx = self.current_instrument;
                self.synth_field.adjust_envelope(
                    &mut self.project.instruments[idx],
                    -1,
                    &mut self.envelope_point_idx,
                );
            } else {
                let idx = self.current_instrument;
                self.synth_field
                    .adjust(&mut self.project.instruments[idx], -1);
            }
        } else if actions.contains(&Action::LoadSample) {
            self.load_sample_for_instrument(self.current_instrument);
        }
    }

    pub fn open_module_file(&mut self) {
        self.stop_playback();

        let mut dialog = rfd::FileDialog::new()
            .add_filter("Tracker Modules", &["xm", "mod"])
            .set_title("Open Module");

        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }

        if let Some(path) = dialog.pick_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();

            let result = match ext.as_str() {
                "xm" => crate::project::xm::load_xm(&path),
                "mod" => crate::project::mod_file::load_mod(&path),
                _ => Err(format!("Unsupported file type: .{ext}")),
            };

            match result {
                Ok(project) => {
                    self.project = project;
                    self.cursor.channel = 0;
                    self.cursor.row = 0;
                    self.current_instrument = 0;
                    self.project.current_order_idx = 0;
                    let ch_count = self.project.current_pattern().channels;
                    self.muted_channels = vec![false; ch_count];
                }
                Err(e) => {
                    self.status_message = Some(format!("Failed to load module: {e}"));
                }
            }
        }
    }

    fn load_sample_for_instrument(&mut self, inst_idx: usize) {
        let mut dialog = rfd::FileDialog::new()
            .add_filter("Audio Files", &["wav"])
            .set_title("Load Sample");

        if let Some(home) = dirs::home_dir() {
            dialog = dialog.set_directory(home);
        }

        if let Some(path) = dialog.pick_file()
            && let Ok(data) = SampleData::load_from_path(&path)
        {
            self.project.instruments[inst_idx].sample_data = data;
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
