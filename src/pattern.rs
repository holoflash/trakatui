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

    pub fn name(&self) -> String {
        let names = [
            "C-", "C#", "D-", "D#", "E-", "F-", "F#", "G-", "G#", "A-", "A#", "B-",
        ];
        let octave = (self.pitch / 12).saturating_sub(1);
        let note_idx = (self.pitch % 12) as usize;
        format!("{}{}", names[note_idx], octave)
    }

    pub fn frequency(&self) -> f32 {
        440.0 * 2.0_f32.powf((self.pitch as f32 - 69.0) / 12.0)
    }
}

pub type Cell = Option<Note>;

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
            data: vec![vec![None; rows]; channels],
        }
    }

    pub fn get(&self, channel: usize, row: usize) -> Cell {
        self.data[channel][row]
    }

    pub fn set(&mut self, channel: usize, row: usize, note: Cell) {
        self.data[channel][row] = note;
    }

    pub fn clear(&mut self, channel: usize, row: usize) {
        self.data[channel][row] = None;
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
        let mut pat = Pattern::new(4, 64);
        assert_eq!(pat.get(0, 0), None);
        pat.set(0, 0, Some(Note::new(60)));
        assert_eq!(pat.get(0, 0), Some(Note::new(60)));
        pat.clear(0, 0);
        assert_eq!(pat.get(0, 0), None);
    }
}
