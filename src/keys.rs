use eframe::egui::Key;

use crate::pattern::Note;
use crate::scale::{Scale, map_key_index_to_midi};

/// Map a keyboard key to a `Note` using the current scale.
///
/// Keys are assigned a linear index (0, 1, 2, …) which is then
/// translated through the scale to the correct MIDI pitch.
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
        let midi = map_key_index_to_midi(i, octave, scale, transpose);
        Note::new(midi)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::CHROMATIC;

    #[test]
    fn chromatic_c4() {
        let note = key_to_note(Key::Z, 4, &CHROMATIC, 0).unwrap();
        assert_eq!(note.pitch, 60);
        assert_eq!(note.name(), "C-4");
    }

    #[test]
    fn chromatic_a4() {
        let note = key_to_note(Key::D, 4, &CHROMATIC, 0).unwrap();
        assert_eq!(note.pitch, 69);
    }

    #[test]
    fn chromatic_upper_octave() {
        let note = key_to_note(Key::H, 4, &CHROMATIC, 0).unwrap();
        assert_eq!(note.pitch, 72);
    }

    #[test]
    fn unknown_key() {
        assert!(key_to_note(Key::Num0, 4, &CHROMATIC, 0).is_none());
    }
}
