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
    /// Also supports sharp/flat count notation: "#", "##", "###", etc. or "b", "bb", "bbb", etc.
    pub fn from_str(s: &str) -> Option<Self> {
        let trimmed = s.trim();

        // Check for sharp count notation (e.g., "#", "##", "###")
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '#') {
            let count = trimmed.len() as i8;
            if count >= 1 && count <= 7 {
                return Some(Self { fifths: count });
            }
            return None;
        }

        // Check for flat count notation (e.g., "b", "bb", "bbb", etc.)
        // Note: "b" alone is ambiguous with B major, so we only accept 2+ for flat count
        // Use "F" for 1 flat instead
        if trimmed.len() >= 2 && trimmed.chars().all(|c| c == 'b') {
            let count = trimmed.len() as i8;
            if count >= 2 && count <= 7 {
                return Some(Self { fifths: -count });
            }
            return None;
        }

        let fifths = match trimmed {
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

    /// Returns the accidental for a note based on this key signature.
    /// Notes without explicit accidentals should use this to determine their pitch.
    /// Order of sharps: F C G D A E B
    /// Order of flats: B E A D G C F
    pub fn accidental_for_note(&self, note: NoteName) -> Accidental {
        if self.fifths > 0 {
            // Sharp keys - sharps are added in order: F C G D A E B
            let sharped_notes = match self.fifths {
                1 => [NoteName::F].as_slice(),
                2 => [NoteName::F, NoteName::C].as_slice(),
                3 => [NoteName::F, NoteName::C, NoteName::G].as_slice(),
                4 => [NoteName::F, NoteName::C, NoteName::G, NoteName::D].as_slice(),
                5 => [NoteName::F, NoteName::C, NoteName::G, NoteName::D, NoteName::A].as_slice(),
                6 => [NoteName::F, NoteName::C, NoteName::G, NoteName::D, NoteName::A, NoteName::E].as_slice(),
                7 => [NoteName::F, NoteName::C, NoteName::G, NoteName::D, NoteName::A, NoteName::E, NoteName::B].as_slice(),
                _ => [].as_slice(),
            };
            if sharped_notes.contains(&note) {
                Accidental::Sharp
            } else {
                Accidental::Natural
            }
        } else if self.fifths < 0 {
            // Flat keys - flats are added in order: B E A D G C F
            let flatted_notes = match self.fifths {
                -1 => [NoteName::B].as_slice(),
                -2 => [NoteName::B, NoteName::E].as_slice(),
                -3 => [NoteName::B, NoteName::E, NoteName::A].as_slice(),
                -4 => [NoteName::B, NoteName::E, NoteName::A, NoteName::D].as_slice(),
                -5 => [NoteName::B, NoteName::E, NoteName::A, NoteName::D, NoteName::G].as_slice(),
                -6 => [NoteName::B, NoteName::E, NoteName::A, NoteName::D, NoteName::G, NoteName::C].as_slice(),
                -7 => [NoteName::B, NoteName::E, NoteName::A, NoteName::D, NoteName::G, NoteName::C, NoteName::F].as_slice(),
                _ => [].as_slice(),
            };
            if flatted_notes.contains(&note) {
                Accidental::Flat
            } else {
                Accidental::Natural
            }
        } else {
            // C major - all notes are natural
            Accidental::Natural
        }
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

/// Accidentals: sharp, flat, natural (default/unspecified), or force natural (explicit %)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Accidental {
    #[default]
    Natural,      // No accidental specified - follows key signature
    Sharp,        // #
    Flat,         // b
    ForceNatural, // % - explicitly show natural sign
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
    pub tie_start: bool,   // This note starts a tie (to the next note)
    pub tie_stop: bool,    // This note ends a tie (from the previous note)
    pub slur_start: bool,  // This note starts a slur
    pub slur_stop: bool,   // This note ends a slur
}

/// An element in a measure: either a note or a rest
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Note(Note),
    Rest { duration: Duration, dotted: bool, tuplet: Option<TupletInfo> },
}

/// Ending type for volta brackets (1st/2nd endings)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ending {
    First,   // 1st:
    Second,  // 2nd:
}

/// A single measure containing musical elements
#[derive(Debug, Clone)]
pub struct Measure {
    pub elements: Vec<Element>,
    pub repeat_start: bool,   // ||: at the beginning of the measure
    pub repeat_end: bool,     // :|| at the end of the measure
    pub ending: Option<Ending>, // 1st: or 2nd: volta bracket
}

/// A complete musical score
#[derive(Debug, Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<Measure>,
}
