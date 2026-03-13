use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    pub pitch: u8,
}

impl Note {
    pub fn new(pitch: u8) -> Self {
        Self {
            pitch: pitch.min(127),
        }
    }

    pub fn name(self) -> String {
        let names = [
            "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
        ];
        let octave = (self.pitch / 12) as i8 - 1;
        let note_idx = (self.pitch % 12) as usize;
        format!("{}{}", names[note_idx], octave)
    }

    pub fn frequency(self) -> f32 {
        440.0 * ((f32::from(self.pitch) - 69.0) / 12.0).exp2()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cell {
    Empty,
    NoteOn(Note),
    NoteOff,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub channels: usize,
    pub rows: usize,
    pub bpm: u16,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub note_value: u8,
    pub measures: u8,
    pub data: Vec<Vec<Vec<Cell>>>,
}

impl Pattern {
    pub fn new(channels: usize, rows: usize) -> Self {
        Self {
            channels,
            rows,
            bpm: 120,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
            note_value: 4,
            measures: 1,
            data: vec![vec![vec![Cell::Empty; rows]]; channels],
        }
    }

    pub fn new_from(source: &Pattern, channels: usize) -> Self {
        let rows = source.computed_rows();
        Self {
            channels,
            rows,
            bpm: source.bpm,
            time_sig_numerator: source.time_sig_numerator,
            time_sig_denominator: source.time_sig_denominator,
            note_value: source.note_value,
            measures: source.measures,
            data: vec![vec![vec![Cell::Empty; rows]]; channels],
        }
    }

    pub fn computed_rows(&self) -> usize {
        self.time_sig_numerator as usize
            * self.note_value as usize
            * self.measures as usize
    }

    pub fn rows_per_beat(&self) -> usize {
        self.note_value as usize
    }

    pub fn get(&self, channel: usize, voice: usize, row: usize) -> Cell {
        self.data[channel][voice][row]
    }

    pub fn set(&mut self, channel: usize, voice: usize, row: usize, cell: Cell) {
        self.data[channel][voice][row] = cell;
    }

    pub fn clear(&mut self, channel: usize, voice: usize, row: usize) {
        let voices = &mut self.data[channel];
        if voice < voices.len() {
            voices[voice][row] = Cell::Empty;
        }
    }

    pub fn voice_count(&self, channel: usize) -> usize {
        self.data[channel].len()
    }

    pub fn set_voice_count(&mut self, channel: usize, count: usize) {
        let current = self.data[channel].len();
        if count > current {
            for _ in current..count {
                self.data[channel].push(vec![Cell::Empty; self.rows]);
            }
        } else if count < current && count >= 1 {
            self.data[channel].truncate(count);
        }
    }

    pub fn resize(&mut self, new_rows: usize) {
        for ch in &mut self.data {
            for voice in ch.iter_mut() {
                voice.resize(new_rows, Cell::Empty);
            }
        }
        self.rows = new_rows;
    }

    pub fn add_channel(&mut self) {
        self.data.push(vec![vec![Cell::Empty; self.rows]]);
        self.channels += 1;
    }

    pub fn remove_channel(&mut self, idx: usize) {
        if idx < self.channels && self.channels > 1 {
            self.data.remove(idx);
            self.channels -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_names() {
        assert_eq!(Note::new(60).name(), "C-4");
        assert_eq!(Note::new(61).name(), "C#4");
        assert_eq!(Note::new(69).name(), "A-4");
        assert_eq!(Note::new(72).name(), "C-5");
    }

    #[test]
    fn note_frequency() {
        let a4 = Note::new(69);
        assert!((a4.frequency() - 440.0).abs() < 0.01);
    }

    #[test]
    fn pattern_basics() {
        let mut pat = Pattern::new(4, 16);
        assert_eq!(pat.get(0, 0, 0), Cell::Empty);
        pat.set(0, 0, 0, Cell::NoteOn(Note::new(49)));
        assert_eq!(pat.get(0, 0, 0), Cell::NoteOn(Note::new(49)));
        pat.set(0, 0, 1, Cell::NoteOff);
        assert_eq!(pat.get(0, 0, 1), Cell::NoteOff);
        pat.clear(0, 0, 0);
        assert_eq!(pat.get(0, 0, 0), Cell::Empty);
    }

    #[test]
    fn polyphony() {
        let mut pat = Pattern::new(2, 8);
        assert_eq!(pat.voice_count(0), 1);
        pat.set_voice_count(0, 3);
        assert_eq!(pat.voice_count(0), 3);
        pat.set(0, 2, 4, Cell::NoteOn(Note::new(60)));
        assert_eq!(pat.get(0, 2, 4), Cell::NoteOn(Note::new(60)));
        pat.set_voice_count(0, 1);
        assert_eq!(pat.voice_count(0), 1);
    }

    #[test]
    fn default_pattern_settings() {
        let pat = Pattern::new(1, 16);
        assert_eq!(pat.bpm, 120);
        assert_eq!(pat.time_sig_numerator, 4);
        assert_eq!(pat.time_sig_denominator, 4);
        assert_eq!(pat.note_value, 4);
        assert_eq!(pat.measures, 1);
        assert_eq!(pat.computed_rows(), 16);
        assert_eq!(pat.rows_per_beat(), 4);
    }

    #[test]
    fn computed_rows_various() {
        let mut pat = Pattern::new(1, 16);
        pat.time_sig_numerator = 4;
        pat.note_value = 16;
        pat.measures = 2;
        assert_eq!(pat.computed_rows(), 128);

        pat.time_sig_numerator = 7;
        pat.note_value = 8;
        pat.measures = 1;
        assert_eq!(pat.computed_rows(), 56);

        pat.time_sig_numerator = 3;
        pat.note_value = 4;
        pat.measures = 3;
        assert_eq!(pat.computed_rows(), 36);
    }

    #[test]
    fn new_from_inherits_settings() {
        let mut source = Pattern::new(2, 16);
        source.bpm = 140;
        source.time_sig_numerator = 7;
        source.time_sig_denominator = 8;
        source.note_value = 8;
        source.measures = 2;

        let child = Pattern::new_from(&source, 3);
        assert_eq!(child.bpm, 140);
        assert_eq!(child.time_sig_numerator, 7);
        assert_eq!(child.time_sig_denominator, 8);
        assert_eq!(child.note_value, 8);
        assert_eq!(child.measures, 2);
        assert_eq!(child.channels, 3);
        assert_eq!(child.rows, source.computed_rows());
    }
}
