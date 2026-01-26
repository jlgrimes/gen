use serde::Deserialize;

/// Time signature (e.g., 4/4, 3/4, 6/8)
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSignature {
    pub beats: u8,
    pub beat_type: u8,
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            beats: 4,
            beat_type: 4,
        }
    }
}

/// Pitch class for written-pitch transposition
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Pitch {
    pub note: NoteName,
    pub octave_offset: i8, // ^ = +1, ^^ = +2, _ = -1, __ = -2
}

/// Key signature (number of sharps/flats)
/// Positive = sharps, Negative = flats, Zero = C major / A minor
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeySignature {
    pub fifths: i8, // -7 to +7 (flats to sharps)
}

impl KeySignature {
    /// Parse a key signature string like "G", "D", "F", "Bb", "Eb", etc.
    pub fn from_str(s: &str) -> Option<Self> {
        let fifths = match s.trim() {
            // Major keys
            "C" => 0,
            "G" => 1,
            "D" => 2,
            "A" => 3,
            "E" => 4,
            "B" => 5,
            "F#" | "Fs" => 6,
            "C#" | "Cs" => 7,
            "F" => -1,
            "Bb" | "Bf" => -2,
            "Eb" | "Ef" => -3,
            "Ab" | "Af" => -4,
            "Db" | "Df" => -5,
            "Gb" | "Gf" => -6,
            "Cb" | "Cf" => -7,
            _ => return None,
        };
        Some(Self { fifths })
    }
}

/// Document metadata from YAML header
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub time_signature: TimeSignature,
    pub key_signature: KeySignature,
    pub written_pitch: Pitch,
}

/// Raw metadata for YAML deserialization
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct RawMetadata {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub time_signature: Option<String>,
    pub key_signature: Option<String>,
    pub written_pitch: Option<String>,
}

/// Note names A through G
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum NoteName {
    #[default]
    C,
    D,
    E,
    F,
    G,
    A,
    B,
}

/// Accidentals: sharp, flat, or natural
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Accidental {
    #[default]
    Natural,
    Sharp,
    Flat,
}

/// Octave relative to middle octave
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Octave {
    DoubleLow,  // __
    Low,        // _
    #[default]
    Middle,     // (none)
    High,       // ^
    DoubleHigh, // ^^
}

/// Note duration
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Duration {
    Whole,       // o
    Half,        // |o
    #[default]
    Quarter,     // (none) or |
    Eighth,      // \
    Sixteenth,   // \\
    ThirtySecond, // \\\
}

impl Duration {
    /// Returns the duration as a fraction of a whole note
    pub fn as_fraction(&self) -> f64 {
        match self {
            Duration::Whole => 1.0,
            Duration::Half => 0.5,
            Duration::Quarter => 0.25,
            Duration::Eighth => 0.125,
            Duration::Sixteenth => 0.0625,
            Duration::ThirtySecond => 0.03125,
        }
    }

    /// MusicXML type name
    pub fn musicxml_type(&self) -> &'static str {
        match self {
            Duration::Whole => "whole",
            Duration::Half => "half",
            Duration::Quarter => "quarter",
            Duration::Eighth => "eighth",
            Duration::Sixteenth => "16th",
            Duration::ThirtySecond => "32nd",
        }
    }
}

/// Tuplet information for a note (e.g., triplet = 3 notes in the time of 2)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TupletInfo {
    pub actual_notes: u8,   // Number of notes played (e.g., 3 for triplet)
    pub normal_notes: u8,   // Number of notes in normal time (e.g., 2 for triplet)
    pub is_start: bool,     // First note of the tuplet group
    pub is_stop: bool,      // Last note of the tuplet group
}

impl TupletInfo {
    /// Create tuplet info for a standard tuplet (N notes in the time of the next lower power of 2)
    pub fn new(actual_notes: u8) -> Self {
        // Standard tuplet: N notes in the time of (N-1) for odd, or N in N-1 for even
        // But the common convention is:
        // 3 (triplet) = 3 in the time of 2
        // 5 (quintuplet) = 5 in the time of 4
        // 6 (sextuplet) = 6 in the time of 4
        // 7 (septuplet) = 7 in the time of 4
        let normal_notes = if actual_notes <= 4 {
            actual_notes - 1
        } else {
            4 // For 5, 6, 7, etc., they're typically in the time of 4
        };

        Self {
            actual_notes,
            normal_notes,
            is_start: false,
            is_stop: false,
        }
    }
}

/// A musical note
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub name: NoteName,
    pub accidental: Accidental,
    pub octave: Octave,
    pub duration: Duration,
    pub dotted: bool,
    pub tuplet: Option<TupletInfo>,
}

/// An element in a measure: either a note or a rest
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Note(Note),
    Rest { duration: Duration, dotted: bool, tuplet: Option<TupletInfo> },
}

/// A single measure containing musical elements
#[derive(Debug, Clone)]
pub struct Measure {
    pub elements: Vec<Element>,
}

/// A complete musical score
#[derive(Debug, Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<Measure>,
}
