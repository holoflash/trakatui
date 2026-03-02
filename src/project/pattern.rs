#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Effect {
    pub kind: u8,
    pub param: u8,
}

pub type EffectCommand = Option<Effect>;

pub fn effect_display(cmd: EffectCommand) -> String {
    match cmd {
        Some(fx) => format!("{:X}{:02X}", fx.kind, fx.param),
        None => "···".to_string(),
    }
}

pub fn volume_display(vol: Option<u8>) -> String {
    match vol {
        Some(v) => format!("{:02X}", v),
        None => "··".to_string(),
    }
}

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
    pub volumes: Vec<Vec<Option<u8>>>,
    pub effects: Vec<Vec<EffectCommand>>,
}

impl Pattern {
    pub fn new(channels: usize, rows: usize) -> Self {
        Self {
            channels,
            rows,
            data: vec![vec![Cell::Empty; rows]; channels],
            volumes: vec![vec![None; rows]; channels],
            effects: vec![vec![None; rows]; channels],
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

    pub fn get_effect(&self, channel: usize, row: usize) -> EffectCommand {
        self.effects[channel][row]
    }

    pub fn set_effect(&mut self, channel: usize, row: usize, cmd: EffectCommand) {
        self.effects[channel][row] = cmd;
    }

    pub fn clear_effect(&mut self, channel: usize, row: usize) {
        self.effects[channel][row] = None;
    }

    pub fn get_volume(&self, channel: usize, row: usize) -> Option<u8> {
        self.volumes[channel][row]
    }

    pub fn set_volume(&mut self, channel: usize, row: usize, vol: Option<u8>) {
        self.volumes[channel][row] = vol;
    }

    pub fn clear_volume(&mut self, channel: usize, row: usize) {
        self.volumes[channel][row] = None;
    }

    pub fn resize(&mut self, new_rows: usize) {
        if new_rows > self.data[0].len() {
            for ch in &mut self.data {
                ch.resize(new_rows, Cell::Empty);
            }
            for ch in &mut self.volumes {
                ch.resize(new_rows, None);
            }
            for ch in &mut self.effects {
                ch.resize(new_rows, None);
            }
        }
        self.rows = new_rows;
    }

    #[allow(dead_code)]
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

    #[test]
    fn effect_command_basics() {
        let mut pat = Pattern::new(2, 8);
        assert_eq!(pat.get_effect(0, 0), None);

        let cmd = Some(Effect {
            kind: 1,
            param: 0x20,
        });
        pat.set_effect(0, 0, cmd);
        assert_eq!(pat.get_effect(0, 0), cmd);

        pat.clear_effect(0, 0);
        assert_eq!(pat.get_effect(0, 0), None);
    }

    #[test]
    fn effect_display_formatting() {
        assert_eq!(effect_display(None), "···");
        assert_eq!(
            effect_display(Some(Effect {
                kind: 1,
                param: 0xFF
            })),
            "1FF"
        );
        assert_eq!(
            effect_display(Some(Effect {
                kind: 0xA,
                param: 0x04
            })),
            "A04"
        );
        assert_eq!(
            effect_display(Some(Effect {
                kind: 2,
                param: 0x30
            })),
            "230"
        );
    }
}
