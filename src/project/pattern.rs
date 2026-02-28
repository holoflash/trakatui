pub type EffectCommand = Option<[u8; 4]>;

pub fn effect_display(cmd: EffectCommand) -> String {
    cmd.map_or_else(
        || "····".to_string(),
        |bytes| {
            let mut s = String::with_capacity(4);
            for &b in &bytes {
                s.push(if b.is_ascii_graphic() {
                    b as char
                } else {
                    '·'
                });
            }
            s
        },
    )
}

/// Parse a pitch-bend effect command.
/// Returns `Some((semitones, steps))` for valid PU/PD commands, `None` otherwise.
/// - `PUxy` = pitch up, `x` semitones in `y` steps (hex digits).
/// - `PDxy` = pitch down, `x` semitones in `y` steps (hex digits).
/// - Semitones are signed: positive for PU, negative for PD.
/// - `PU00` / `PD00` = stop ongoing bend (returns `Some((0, 0))`).
pub fn parse_pitch_bend(cmd: [u8; 4]) -> Option<(i8, u8)> {
    if cmd[0] != b'P' {
        return None;
    }
    let direction: i8 = match cmd[1] {
        b'U' => 1,
        b'D' => -1,
        _ => return None,
    };
    let semitones = hex_digit(cmd[2])?;
    let steps = hex_digit(cmd[3])?;
    Some((direction * semitones as i8, steps))
}

const fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'A'..=b'F' => Some(b - b'A' + 10),
        b'a'..=b'f' => Some(b - b'a' + 10),
        _ => None,
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
    pub effects: Vec<Vec<EffectCommand>>,
}

impl Pattern {
    pub fn new(channels: usize, rows: usize) -> Self {
        Self {
            channels,
            rows,
            data: vec![vec![Cell::Empty; rows]; channels],
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

    pub fn resize(&mut self, new_rows: usize) {
        if new_rows > self.data[0].len() {
            for ch in &mut self.data {
                ch.resize(new_rows, Cell::Empty);
            }
            for ch in &mut self.effects {
                ch.resize(new_rows, None);
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

    #[test]
    fn effect_command_basics() {
        let mut pat = Pattern::new(2, 8);
        assert_eq!(pat.get_effect(0, 0), None);

        let cmd = Some([b'P', b'U', b'2', b'1']);
        pat.set_effect(0, 0, cmd);
        assert_eq!(pat.get_effect(0, 0), cmd);

        pat.clear_effect(0, 0);
        assert_eq!(pat.get_effect(0, 0), None);
    }

    #[test]
    fn effect_display_formatting() {
        assert_eq!(effect_display(None), "····");
        assert_eq!(effect_display(Some([b'P', b'U', b'2', b'1'])), "PU21");
        assert_eq!(effect_display(Some([b'P', b'D', b'A', b'F'])), "PDAF");
    }

    #[test]
    fn pitch_bend_parsing() {
        assert_eq!(parse_pitch_bend([b'P', b'U', b'2', b'1']), Some((2, 1)));
        assert_eq!(parse_pitch_bend([b'P', b'D', b'3', b'4']), Some((-3, 4)));
        assert_eq!(parse_pitch_bend([b'P', b'U', b'0', b'0']), Some((0, 0)));
        assert_eq!(parse_pitch_bend([b'P', b'U', b'A', b'F']), Some((10, 15)));
        assert_eq!(parse_pitch_bend([b'X', b'Y', b'0', b'0']), None);
        assert_eq!(parse_pitch_bend([b'P', b'X', b'0', b'0']), None);
        assert_eq!(parse_pitch_bend([b'P', b'U', b'G', b'0']), None);
    }
}
