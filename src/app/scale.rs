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
    name: "HARMONIC MIN",
    intervals: &[0, 2, 3, 5, 7, 8, 11],
};

pub const MELODIC_MINOR: Scale = Scale {
    name: "MELODIC MIN",
    intervals: &[0, 2, 3, 5, 7, 9, 11],
};

pub const HARMONIC_MAJOR: Scale = Scale {
    name: "HARMONIC MAJ",
    intervals: &[0, 2, 4, 5, 7, 8, 11],
};

pub const DOUBLE_HARMONIC: Scale = Scale {
    name: "DOUBLE HARMONIC",
    intervals: &[0, 1, 4, 5, 7, 8, 11],
};

pub const PHRYGIAN_DOMINANT: Scale = Scale {
    name: "PHRYGIAN DOMINANT",
    intervals: &[0, 1, 4, 5, 7, 8, 10],
};

pub const LYDIAN_DOMINANT: Scale = Scale {
    name: "LYDIAN DOMINANT",
    intervals: &[0, 2, 4, 6, 7, 9, 10],
};

pub const WHOLE_TONE: Scale = Scale {
    name: "WHOLE TONE",
    intervals: &[0, 2, 4, 6, 8, 10],
};

pub const DIMINISHED_HW: Scale = Scale {
    name: "DIMINISHED H-W",
    intervals: &[0, 1, 3, 4, 6, 7, 9, 10],
};

pub const BLUES: Scale = Scale {
    name: "BLUES",
    intervals: &[0, 3, 5, 6, 7, 10],
};

pub const MINOR_PENTATONIC: Scale = Scale {
    name: "MINOR PENTATONIC",
    intervals: &[0, 3, 5, 7, 10],
};

pub const MAJOR_PENTATONIC: Scale = Scale {
    name: "MAJOR PENTATONIC",
    intervals: &[0, 2, 4, 7, 9],
};

pub const HIRAJOSHI: Scale = Scale {
    name: "HIRAJOSHI",
    intervals: &[0, 2, 3, 7, 8],
};

pub const IN_SEN: Scale = Scale {
    name: "IN SEN",
    intervals: &[0, 1, 5, 7, 10],
};

pub const HUNGARIAN_MINOR: Scale = Scale {
    name: "HUNGARIAN MINOR",
    intervals: &[0, 2, 3, 6, 7, 8, 11],
};

pub const ENIGMATIC: Scale = Scale {
    name: "ENIGMATIC",
    intervals: &[0, 1, 4, 6, 8, 10, 11],
};

pub const PROMETHEUS: Scale = Scale {
    name: "PROMETHEUS",
    intervals: &[0, 2, 4, 6, 9, 10],
};

pub const SCALES: &[Scale] = &[
    CHROMATIC,
    MAJOR,
    MINOR,
    HARMONIC_MINOR,
    MELODIC_MINOR,
    HARMONIC_MAJOR,
    DOUBLE_HARMONIC,
    PHRYGIAN_DOMINANT,
    LYDIAN_DOMINANT,
    WHOLE_TONE,
    DIMINISHED_HW,
    BLUES,
    MINOR_PENTATONIC,
    MAJOR_PENTATONIC,
    HIRAJOSHI,
    IN_SEN,
    HUNGARIAN_MINOR,
    ENIGMATIC,
    PROMETHEUS,
];

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaleIndex(pub usize);

impl ScaleIndex {
    pub fn scale(self) -> &'static Scale {
        &SCALES[self.0]
    }
}

pub fn map_key_index_to_note(key_index: u8, octave: u8, scale: &Scale, transpose: i8) -> u8 {
    let len = u8::try_from(scale.intervals.len()).expect("scale too large");
    let scale_octave = key_index / len;
    let scale_degree = key_index % len;
    let semitone = scale.intervals[scale_degree as usize];

    let note =
        i16::from(octave + scale_octave) * 12 + i16::from(semitone) + i16::from(transpose) + 1;
    u8::try_from(note.clamp(1, 96)).expect("clamped to 1..=96")
}
