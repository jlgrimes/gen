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

/// Document metadata from YAML header
#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub time_signature: TimeSignature,
    pub written_pitch: Pitch,
}

/// Raw metadata for YAML deserialization
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct RawMetadata {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub time_signature: Option<String>,
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

/// A musical note
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub name: NoteName,
    pub accidental: Accidental,
    pub octave: Octave,
    pub duration: Duration,
    pub dotted: bool,
}

/// An element in a measure: either a note or a rest
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Note(Note),
    Rest { duration: Duration, dotted: bool },
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
