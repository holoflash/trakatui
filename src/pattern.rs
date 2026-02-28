#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        let octave = (self.pitch / 12).saturating_sub(1);
        let note_idx = (self.pitch % 12) as usize;
        format!("{}{}", names[note_idx], octave)
    }

    pub fn frequency(self) -> f32 {
        440.0 * ((f32::from(self.pitch) - 69.0) / 12.0).exp2()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Empty,
    NoteOn(Note),
    NoteOff,
}

pub struct Pattern {
    pub channels: usize,
    pub rows: usize,
    pub data: Vec<Vec<Cell>>,
}

impl Pattern {
    pub fn new(channels: usize, rows: usize) -> Self {
        Self {
            channels,
            rows,
            data: vec![vec![Cell::Empty; rows]; channels],
        }
    }

    pub fn get(&self, channel: usize, row: usize) -> Cell {
        self.data[channel][row]
    }

    pub fn set(&mut self, channel: usize, row: usize, cell: Cell) {
        self.data[channel][row] = cell;
    }

    pub fn clear(&mut self, channel: usize, row: usize) {
        self.data[channel][row] = Cell::Empty;
    }

    pub fn resize(&mut self, new_rows: usize) {
        if new_rows > self.data[0].len() {
            for ch in &mut self.data {
                ch.resize(new_rows, Cell::Empty);
            }
        }
        self.rows = new_rows;
    }

    pub fn gate_rows(&self, channel: usize, row: usize) -> usize {
        let mut count = 1;
        for r in (row + 1)..self.rows {
            match self.data[channel][r] {
                Cell::NoteOn(_) | Cell::NoteOff => break,
                Cell::Empty => count += 1,
            }
        }
        count
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
        assert_eq!(pat.get(0, 0), Cell::Empty);
        pat.set(0, 0, Cell::NoteOn(Note::new(60)));
        assert_eq!(pat.get(0, 0), Cell::NoteOn(Note::new(60)));
        pat.set(0, 1, Cell::NoteOff);
        assert_eq!(pat.get(0, 1), Cell::NoteOff);
        pat.clear(0, 0);
        assert_eq!(pat.get(0, 0), Cell::Empty);
    }
}
