use eframe::egui::Color32;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PatternColor {
    Coral,
    Amber,
    Lime,
    Teal,
    Sky,
    Indigo,
    Violet,
    Rose,
    Mint,
    Slate,
}

impl PatternColor {
    pub const ALL: &[Self] = &[
        Self::Coral,
        Self::Amber,
        Self::Lime,
        Self::Teal,
        Self::Sky,
        Self::Indigo,
        Self::Violet,
        Self::Rose,
        Self::Mint,
        Self::Slate,
    ];

    pub fn random() -> Self {
        Self::ALL[fastrand::usize(..Self::ALL.len())]
    }

    pub fn to_color32(self) -> Color32 {
        match self {
            Self::Coral => Color32::from_rgb(235, 110, 95),
            Self::Amber => Color32::from_rgb(230, 180, 70),
            Self::Lime => Color32::from_rgb(140, 200, 80),
            Self::Teal => Color32::from_rgb(70, 190, 175),
            Self::Sky => Color32::from_rgb(90, 170, 230),
            Self::Indigo => Color32::from_rgb(110, 120, 210),
            Self::Violet => Color32::from_rgb(170, 110, 210),
            Self::Rose => Color32::from_rgb(210, 110, 170),
            Self::Mint => Color32::from_rgb(110, 210, 170),
            Self::Slate => Color32::from_rgb(140, 150, 170),
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Coral => "Coral",
            Self::Amber => "Amber",
            Self::Lime => "Lime",
            Self::Teal => "Teal",
            Self::Sky => "Sky",
            Self::Indigo => "Indigo",
            Self::Violet => "Violet",
            Self::Rose => "Rose",
            Self::Mint => "Mint",
            Self::Slate => "Slate",
        }
    }
}

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

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

fn smallest_prime_factor(n: usize) -> usize {
    if n <= 1 {
        return n;
    }
    if n.is_multiple_of(2) {
        return 2;
    }
    let mut i = 3;
    while i * i <= n {
        if n.is_multiple_of(i) {
            return i;
        }
        i += 2;
    }
    n
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub name: String,
    pub color: Option<PatternColor>,
    pub repeat: u16,
    pub rows: usize,
    pub bpm: u16,
    pub time_sig_numerator: u8,
    pub time_sig_denominator: u8,
    pub note_value: u8,
    pub measures: u8,
    pub track_note_values: Vec<u8>,
    pub data: Vec<Vec<Vec<Cell>>>,
}

impl Pattern {
    pub fn new(name: String, channels: usize, rows: usize) -> Self {
        Self {
            name,
            color: None,
            repeat: 1,
            rows,
            bpm: 120,
            time_sig_numerator: 4,
            time_sig_denominator: 4,
            note_value: 16,
            measures: 1,
            track_note_values: vec![16; channels],
            data: vec![vec![vec![Cell::Empty; rows]]; channels],
        }
    }

    pub fn new_from(source: &Pattern, name: String, channels: usize) -> Self {
        let mut track_nvs = source.track_note_values.clone();
        let last_nv = track_nvs.last().copied().unwrap_or(source.note_value);
        track_nvs.resize(channels, last_nv);

        let mut data = Vec::with_capacity(channels);
        for &nv in track_nvs.iter().take(channels) {
            let ch_rows = Self::rows_per_measure_static(
                nv,
                source.time_sig_numerator,
                source.time_sig_denominator,
            ) * source.measures as usize;
            data.push(vec![vec![Cell::Empty; ch_rows]]);
        }
        let max_rows = data.iter().map(|ch| ch[0].len()).max().unwrap_or(1);

        Self {
            name,
            color: source.color,
            repeat: source.repeat,
            rows: max_rows,
            bpm: source.bpm,
            time_sig_numerator: source.time_sig_numerator,
            time_sig_denominator: source.time_sig_denominator,
            note_value: source.note_value,
            measures: source.measures,
            track_note_values: track_nvs,
            data,
        }
    }

    fn rows_per_measure_static(note_value: u8, numerator: u8, denominator: u8) -> usize {
        (note_value as usize * numerator as usize / denominator as usize).max(1)
    }

    pub fn rows_per_measure(&self) -> usize {
        Self::rows_per_measure_static(self.note_value, self.time_sig_numerator, self.time_sig_denominator)
    }

    pub fn rows_per_measure_for_track(&self, ch: usize) -> usize {
        let nv = self.track_note_values.get(ch).copied().unwrap_or(self.note_value);
        Self::rows_per_measure_static(nv, self.time_sig_numerator, self.time_sig_denominator)
    }

    pub fn computed_rows(&self) -> usize {
        if self.track_note_values.is_empty() {
            return self.rows_per_measure() * self.measures as usize;
        }
        (0..self.track_note_values.len())
            .map(|ch| self.computed_rows_for_track(ch))
            .max()
            .unwrap_or(1)
    }

    pub fn computed_rows_for_track(&self, ch: usize) -> usize {
        self.rows_per_measure_for_track(ch) * self.measures as usize
    }

    pub fn track_rows(&self, ch: usize) -> usize {
        self.data.get(ch).and_then(|voices| voices.first()).map(|v| v.len()).unwrap_or(self.rows)
    }

    pub fn primary_row_group_for_track(&self, ch: usize) -> usize {
        let rpm = self.rows_per_measure_for_track(ch);
        let num = self.time_sig_numerator as usize;
        rpm / gcd(rpm, num)
    }

    pub fn secondary_row_group_for_track(&self, ch: usize) -> usize {
        let primary = self.primary_row_group_for_track(ch);
        if primary <= 1 {
            return 0;
        }
        let spf = smallest_prime_factor(primary);
        if spf < primary { primary / spf } else { 0 }
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
        let ch_rows = self.track_rows(channel);
        let current = self.data[channel].len();
        if count > current {
            for _ in current..count {
                self.data[channel].push(vec![Cell::Empty; ch_rows]);
            }
        } else if count < current && count >= 1 {
            self.data[channel].truncate(count);
        }
    }

    pub fn resize(&mut self, new_max_rows: usize) {
        for (ch, ch_data) in self.data.iter_mut().enumerate() {
            let ch_rows = self
                .track_note_values
                .get(ch)
                .map(|&nv| {
                    Self::rows_per_measure_static(
                        nv,
                        self.time_sig_numerator,
                        self.time_sig_denominator,
                    ) * self.measures as usize
                })
                .unwrap_or(new_max_rows);
            for voice in ch_data.iter_mut() {
                voice.resize(ch_rows, Cell::Empty);
            }
        }
        self.rows = new_max_rows;
    }

    pub fn resize_track(&mut self, ch: usize) {
        let ch_rows = self.computed_rows_for_track(ch);
        for voice in &mut self.data[ch] {
            voice.resize(ch_rows, Cell::Empty);
        }
        self.rows = self.computed_rows();
    }

    pub fn add_channel(&mut self) {
        let last_nv = self.track_note_values.last().copied().unwrap_or(self.note_value);
        self.track_note_values.push(last_nv);
        let ch_rows = Self::rows_per_measure_static(
            last_nv,
            self.time_sig_numerator,
            self.time_sig_denominator,
        ) * self.measures as usize;
        self.data.push(vec![vec![Cell::Empty; ch_rows]]);
        self.rows = self.computed_rows();
    }

    pub fn remove_channel(&mut self, idx: usize) {
        if idx < self.data.len() && self.data.len() > 1 {
            self.data.remove(idx);
            if idx < self.track_note_values.len() {
                self.track_note_values.remove(idx);
            }
            self.rows = self.computed_rows();
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
        let mut pat = Pattern::new("P01".into(), 4, 16);
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
        let mut pat = Pattern::new("P01".into(), 2, 8);
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
        let pat = Pattern::new("Test".into(), 1, 16);
        assert_eq!(pat.name, "Test");
        assert_eq!(pat.color, None);
        assert_eq!(pat.repeat, 1);
        assert_eq!(pat.bpm, 120);
        assert_eq!(pat.time_sig_numerator, 4);
        assert_eq!(pat.time_sig_denominator, 4);
        assert_eq!(pat.note_value, 16);
        assert_eq!(pat.measures, 1);
        assert_eq!(pat.computed_rows(), 16);
        assert_eq!(pat.primary_row_group_for_track(0), 4);
    }

    #[test]
    fn computed_rows_various() {
        let mut pat = Pattern::new("P01".into(), 1, 16);
        pat.time_sig_numerator = 4;
        pat.note_value = 16;
        pat.track_note_values[0] = 16;
        pat.measures = 2;
        assert_eq!(pat.computed_rows(), 32);

        pat.time_sig_numerator = 7;
        pat.time_sig_denominator = 8;
        pat.note_value = 8;
        pat.track_note_values[0] = 8;
        pat.measures = 1;
        assert_eq!(pat.computed_rows(), 7);

        pat.time_sig_numerator = 3;
        pat.time_sig_denominator = 4;
        pat.note_value = 4;
        pat.track_note_values[0] = 4;
        pat.measures = 3;
        assert_eq!(pat.computed_rows(), 9);

        pat.time_sig_numerator = 4;
        pat.time_sig_denominator = 4;
        pat.note_value = 24;
        pat.track_note_values[0] = 24;
        pat.measures = 1;
        assert_eq!(pat.computed_rows(), 24);
        assert_eq!(pat.primary_row_group_for_track(0), 6);
    }

    #[test]
    fn new_from_inherits_settings() {
        let mut source = Pattern::new("Source".into(), 2, 16);
        source.bpm = 140;
        source.time_sig_numerator = 7;
        source.time_sig_denominator = 8;
        source.note_value = 12;
        source.track_note_values = vec![12; 2];
        source.measures = 2;
        source.repeat = 3;
        source.color = Some(PatternColor::Coral);

        let child = Pattern::new_from(&source, "Child".into(), 3);
        assert_eq!(child.name, "Child");
        assert_eq!(child.bpm, 140);
        assert_eq!(child.time_sig_numerator, 7);
        assert_eq!(child.time_sig_denominator, 8);
        assert_eq!(child.note_value, 12);
        assert_eq!(child.measures, 2);
        assert_eq!(child.repeat, 3);
        assert_eq!(child.color, Some(PatternColor::Coral));
        assert_eq!(child.data.len(), 3);
        assert_eq!(child.rows, source.computed_rows());
        assert_eq!(child.track_note_values, vec![12, 12, 12]);
    }

    #[test]
    fn per_track_subdivisions() {
        let mut pat = Pattern::new("P01".into(), 2, 16);
        pat.time_sig_numerator = 4;
        pat.time_sig_denominator = 4;
        pat.measures = 1;
        pat.track_note_values = vec![4, 16];
        pat.resize_track(0);
        pat.resize_track(1);

        assert_eq!(pat.computed_rows_for_track(0), 4);
        assert_eq!(pat.computed_rows_for_track(1), 16);
        assert_eq!(pat.track_rows(0), 4);
        assert_eq!(pat.track_rows(1), 16);
        assert_eq!(pat.computed_rows(), 16);
        assert_eq!(pat.rows, 16);
    }
}

