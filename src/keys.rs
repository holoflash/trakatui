use crate::pattern::Note;
use crossterm::event::KeyCode;

pub fn key_to_note(key: KeyCode, octave: u8) -> Option<Note> {
    let semitone = match key {
        KeyCode::Char('z') => Some((0, 0)),  // C
        KeyCode::Char('x') => Some((0, 1)),  // C#
        KeyCode::Char('c') => Some((0, 2)),  // D
        KeyCode::Char('v') => Some((0, 3)),  // D#
        KeyCode::Char('b') => Some((0, 4)),  // E
        KeyCode::Char('n') => Some((0, 5)),  // F
        KeyCode::Char('m') => Some((0, 6)),  // F#
        KeyCode::Char('a') => Some((0, 7)),  // G
        KeyCode::Char('s') => Some((0, 8)),  // G#
        KeyCode::Char('d') => Some((0, 9)),  // A
        KeyCode::Char('f') => Some((0, 10)), // A#
        KeyCode::Char('g') => Some((0, 11)), // B

        KeyCode::Char('h') => Some((1, 0)),  // C
        KeyCode::Char('j') => Some((1, 1)),  // C#
        KeyCode::Char('k') => Some((1, 2)),  // D
        KeyCode::Char('l') => Some((1, 3)),  // D#
        KeyCode::Char('q') => Some((1, 4)),  // E
        KeyCode::Char('w') => Some((1, 5)),  // F
        KeyCode::Char('e') => Some((1, 6)),  // F#
        KeyCode::Char('r') => Some((1, 7)),  // G
        KeyCode::Char('t') => Some((1, 8)),  // G#
        KeyCode::Char('y') => Some((1, 9)),  // A
        KeyCode::Char('u') => Some((1, 10)), // A#
        KeyCode::Char('i') => Some((1, 11)), // B
        KeyCode::Char('o') => Some((1, 12)), // C
        KeyCode::Char('p') => Some((1, 13)), // C#

        _ => None,
    };

    semitone.map(|(oct_offset, semi)| {
        let midi = ((octave + oct_offset) as u8 + 1) * 12 + semi;
        Note::new(midi)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c4() {
        let note = key_to_note(KeyCode::Char('z'), 4).unwrap();
        assert_eq!(note.pitch, 60);
        assert_eq!(note.name(), "C-4");
    }

    #[test]
    fn a4() {
        let note = key_to_note(KeyCode::Char('d'), 4).unwrap();
        assert_eq!(note.pitch, 69);
    }

    #[test]
    fn upper_octave() {
        let note = key_to_note(KeyCode::Char('h'), 4).unwrap();
        assert_eq!(note.pitch, 72); // C-5
    }

    #[test]
    fn unknown_key() {
        assert!(key_to_note(KeyCode::Char('0'), 4).is_none());
    }
}
