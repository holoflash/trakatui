/// Musical scale definitions.
///
/// To add a new scale, append an entry to the `SCALES` array with a name
/// and the semitone intervals from the root within one octave.
#[derive(Debug, Clone, Copy)]
pub struct Scale {
    pub name: &'static str,
    pub intervals: &'static [u8],
}

pub const CHROMATIC: Scale = Scale {
    name: "CHROMATIC",
    intervals: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
};

pub const MAJOR: Scale = Scale {
    name: "MAJOR",
    intervals: &[0, 2, 4, 5, 7, 9, 11],
};

pub const MINOR: Scale = Scale {
    name: "MINOR",
    intervals: &[0, 2, 3, 5, 7, 8, 10],
};

pub const HARMONIC_MINOR: Scale = Scale {
    name: "HARM MIN",
    intervals: &[0, 2, 3, 5, 7, 8, 11],
};

pub const MELODIC_MINOR: Scale = Scale {
    name: "MELO MIN",
    intervals: &[0, 2, 3, 5, 7, 9, 11],
};

pub const HARMONIC_MAJOR: Scale = Scale {
    name: "HARM MAJ",
    intervals: &[0, 2, 4, 5, 7, 8, 11],
};

pub const DOUBLE_HARMONIC: Scale = Scale {
    name: "DBL HARM",
    intervals: &[0, 1, 4, 5, 7, 8, 11],
};

pub const PHRYGIAN_DOMINANT: Scale = Scale {
    name: "PHRY DOM",
    intervals: &[0, 1, 4, 5, 7, 8, 10],
};

pub const LYDIAN_DOMINANT: Scale = Scale {
    name: "LYD DOM",
    intervals: &[0, 2, 4, 6, 7, 9, 10],
};

pub const SCALES: &[Scale] = &[
    MAJOR,
    MINOR,
    CHROMATIC,
    HARMONIC_MINOR,
    MELODIC_MINOR,
    HARMONIC_MAJOR,
    DOUBLE_HARMONIC,
    PHRYGIAN_DOMINANT,
    LYDIAN_DOMINANT,
];

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleIndex(pub usize);

impl ScaleIndex {
    pub fn scale(&self) -> &'static Scale {
        &SCALES[self.0]
    }

    pub fn next(&self) -> Self {
        ScaleIndex((self.0 + 1) % SCALES.len())
    }

    pub fn prev(&self) -> Self {
        if self.0 == 0 {
            ScaleIndex(SCALES.len() - 1)
        } else {
            ScaleIndex(self.0 - 1)
        }
    }
}

const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

pub fn root_name(transpose: i8) -> &'static str {
    NOTE_NAMES[transpose.rem_euclid(12) as usize]
}

pub fn map_key_index_to_midi(key_index: u8, octave: u8, scale: &Scale, transpose: i8) -> u8 {
    let len = scale.intervals.len() as u8;
    let scale_octave = key_index / len;
    let scale_degree = key_index % len;
    let semitone = scale.intervals[scale_degree as usize];

    let midi = ((octave + scale_octave) as i16 + 1) * 12 + semitone as i16 + transpose as i16;
    midi.clamp(0, 127) as u8
}
