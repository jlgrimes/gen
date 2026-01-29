//! # Abstract Syntax Tree (AST) Types
//!
//! This module defines all type structures for the Gen music notation language AST.
//!
//! ## Type Hierarchy
//! ```text
//! Score
//!   ├── Metadata (title, composer, key sig, time sig, tempo)
//!   ├── ModPoints (per-line octave shifts for instruments)
//!   ├── line_to_measure: HashMap<line, measure_idx>
//!   └── Vec<Measure>
//!         ├── Vec<Element> (Note | Rest)
//!         ├── repeat_start/end: bool
//!         ├── ending: Option<Ending>
//!         └── key_change: Option<KeySignature>
//!
//! Element (enum)
//!   ├── Note
//!   │     ├── name: NoteName (A-G)
//!   │     ├── accidental: Accidental (#, b, natural)
//!   │     ├── octave: Octave (^, ^^, _, __)
//!   │     ├── duration: Duration (whole, half, quarter, eighth, sixteenth, 32nd)
//!   │     ├── dotted: bool
//!   │     ├── tuplet: Option<TupletInfo>
//!   │     ├── tie_start/stop: bool
//!   │     ├── slur_start/stop: bool
//!   │     └── chord: Option<String>
//!   └── Rest
//!         ├── duration: Duration
//!         ├── dotted: bool
//!         ├── tuplet: Option<TupletInfo>
//!         └── chord: Option<String>
//! ```
//!
//! ## Key Concepts
//!
//! ### Element
//! Either a `Note` or `Rest` (discriminated union). Each element has a duration
//! which can be modified by dotted rhythms and tuplets.
//!
//! ### Duration Calculation
//! - **Base rhythm** + **dotted modifier** + **tuplet** = actual duration
//! - Example: Dotted quarter note = `1.0 * 1.5 = 1.5 beats`
//! - Example: Quarter note triplet = `1.0 * (2/3) = 0.667 beats`
//!
//! ### Octave System (CRITICAL)
//! - Octaves are **ALWAYS** relative to the C-B range, regardless of key signature
//! - Base octave (no modifier): `C D E F G A B` (middle octave)
//! - High octave (`^` modifier): `C^ D^ E^ F^ G^ A^ B^` (one octave up)
//! - Low octave (`_` modifier): `C_ D_ E_ F_ G_ A_ B_` (one octave down)
//! - The octave **ALWAYS resets at C**, not at the tonic of the key
//! - Example: In F major, `E F G A B C^` goes up through the octave break at C
//!
//! ### Tuplets
//! - Defined by `actual_notes` notes in the time of `normal_notes`
//! - Common: triplet = 3 notes in time of 2 (3:2 ratio)
//! - Quintuplet = 5 notes in time of 4 (5:4 ratio)
//! - Duration per note = `(normal_notes / actual_notes) * base_duration`
//!
//! ### Ties
//! - `tie_start = true` and `tie_stop = false`: First note of a tied group
//! - `tie_start = true` and `tie_stop = true`: Middle note (continues tie)
//! - `tie_start = false` and `tie_stop = true`: Last note of a tied group
//! - Only the first note plays audio; others are visual only
//!
//! ## Related Modules
//! - `parser` - Creates these types from Gen source
//! - `semantic` - Validates these types (measure durations, repeats)
//! - `musicxml` - Generates MusicXML from these types
//! - `lib` - Uses these types for playback data generation

use serde::Deserialize;
use std::collections::HashMap;

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

/// Mode for key signature
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Mode {
    #[default]
    Major,
    Minor,
}

/// Key signature (number of sharps/flats)
/// Positive = sharps, Negative = flats, Zero = C major / A minor
#[derive(Debug, Clone, PartialEq, Default)]
pub struct KeySignature {
    pub fifths: i8, // -7 to +7 (flats to sharps)
    pub mode: Mode,
}

impl KeySignature {
    /// Parse a key signature string like "G", "D", "F", "Bb", "Eb", etc.
    /// Also supports minor keys: "Am", "Dm", "Ebm", etc.
    /// Also supports sharp/flat count notation: "#", "##", "###", etc. or "b", "bb", "bbb", etc.
    pub fn from_str(s: &str) -> Option<Self> {
        let trimmed = s.trim();

        // Check for sharp count notation (e.g., "#", "##", "###")
        if !trimmed.is_empty() && trimmed.chars().all(|c| c == '#') {
            let count = trimmed.len() as i8;
            if count >= 1 && count <= 7 {
                return Some(Self { fifths: count, mode: Mode::Major });
            }
            return None;
        }

        // Check for flat count notation (e.g., "b", "bb", "bbb", etc.)
        // Note: "b" alone is ambiguous with B major, so we only accept 2+ for flat count
        // Use "F" for 1 flat instead
        if trimmed.len() >= 2 && trimmed.chars().all(|c| c == 'b') {
            let count = trimmed.len() as i8;
            if count >= 2 && count <= 7 {
                return Some(Self { fifths: -count, mode: Mode::Major });
            }
            return None;
        }

        // Check if it's a minor key (ends with 'm')
        if trimmed.ends_with('m') && trimmed.len() > 1 {
            let key_name = &trimmed[..trimmed.len()-1];
            let fifths = match key_name {
                // Minor keys (using their natural key signature)
                "A" => 0,        // A minor (no sharps/flats, same as C major)
                "E" => 1,        // E minor (1 sharp, same as G major)
                "B" => 2,        // B minor (2 sharps, same as D major)
                "F#" | "Fs" => 3, // F# minor (3 sharps, same as A major)
                "C#" | "Cs" => 4, // C# minor (4 sharps, same as E major)
                "G#" | "Gs" => 5, // G# minor (5 sharps, same as B major)
                "D#" | "Ds" => 6, // D# minor (6 sharps, same as F# major)
                "A#" | "As" => 7, // A# minor (7 sharps, same as C# major)
                "D" => -1,       // D minor (1 flat, same as F major)
                "G" => -2,       // G minor (2 flats, same as Bb major)
                "C" => -3,       // C minor (3 flats, same as Eb major)
                "F" => -4,       // F minor (4 flats, same as Ab major)
                "Bb" | "Bf" => -5, // Bb minor (5 flats, same as Db major)
                "Eb" | "Ef" => -6, // Eb minor (6 flats, same as Gb major)
                "Ab" | "Af" => -7, // Ab minor (7 flats, same as Cb major)
                _ => return None,
            };
            return Some(Self { fifths, mode: Mode::Minor });
        }

        // Major keys
        let fifths = match trimmed {
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
        Some(Self { fifths, mode: Mode::Major })
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

/// Tempo specification with optional rhythm modifier
#[derive(Debug, Clone, PartialEq)]
pub struct Tempo {
    pub bpm: u16,              // Beats per minute at the specified duration
    pub duration: Duration,    // Which note duration gets the beat (default: Quarter)
    pub dotted: bool,          // Whether the duration is dotted (1.5x the duration)
}

impl Default for Tempo {
    fn default() -> Self {
        Self {
            bpm: 120,
            duration: Duration::Quarter,
            dotted: false,
        }
    }
}

impl Tempo {
    /// Convert to quarter note BPM (standard MIDI tempo)
    /// For example, if tempo is "d160" (half note = 160), quarter note = 320
    /// Or if tempo is "*120" (dotted quarter = 120), quarter note = 120 * (1.0 / 1.5) = 80
    pub fn to_quarter_note_bpm(&self) -> f64 {
        // Calculate the multiplier based on the rhythm duration
        let mut multiplier = match self.duration {
            Duration::Whole => 4.0,       // Whole note = 4 quarters
            Duration::Half => 2.0,         // Half note = 2 quarters
            Duration::Quarter => 1.0,      // Quarter note = 1 quarter
            Duration::Eighth => 0.5,       // Eighth note = 0.5 quarters
            Duration::Sixteenth => 0.25,   // Sixteenth = 0.25 quarters
            Duration::ThirtySecond => 0.125, // 32nd = 0.125 quarters
        };

        // If dotted, the duration is 1.5x longer, so we need to adjust the multiplier
        // For example: dotted quarter at 120 BPM means the dotted quarter (1.5 quarters) = 120 BPM
        // So quarter note BPM = 120 * (1.0 / 1.5) = 80
        if self.dotted {
            multiplier = multiplier * 1.5;
        }

        self.bpm as f64 * multiplier
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
    pub tempo: Option<Tempo>, // Tempo with optional rhythm modifier (default 120 quarter notes if not specified)
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
    pub tempo: Option<String>, // Can be just "120" or with rhythm "d160" or "*120"
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
    Half,        // d
    #[default]
    Quarter,     // (none)
    Eighth,      // /
    Sixteenth,   // //
    ThirtySecond, // ///
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

    /// Returns duration in beats based on time signature
    /// In 4/4 time: quarter = 1 beat, eighth = 0.5 beats, etc.
    /// In 6/8 time: eighth = 1 beat, quarter = 2 beats, etc.
    pub fn as_beats(&self, time_sig: &TimeSignature) -> f64 {
        // Calculate based on what note type gets the beat
        // beat_type of 4 means quarter note gets the beat (1/4 of whole note)
        // beat_type of 8 means eighth note gets the beat (1/8 of whole note)

        // Fraction of a whole note that gets one beat
        let beat_value = 1.0 / time_sig.beat_type as f64;

        // This note's fraction of a whole note divided by the beat value
        // Example in 4/4: quarter note (0.25) / beat_value (0.25) = 1.0 beat
        // Example in 4/4: eighth note (0.125) / beat_value (0.25) = 0.5 beats
        self.as_fraction() / beat_value
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

/// Chord annotation with its own duration (independent from the melody)
#[derive(Debug, Clone, PartialEq)]
pub struct ChordAnnotation {
    pub symbol: String,       // Chord symbol (e.g., "Cmaj7", "Dm", "G7")
    pub duration: Duration,   // Duration for playback (default: Whole)
    pub dotted: bool,         // Whether the duration is dotted
}

impl ChordAnnotation {
    /// Create a new chord annotation with default whole-note duration
    pub fn new(symbol: String) -> Self {
        Self {
            symbol,
            duration: Duration::Whole,
            dotted: false,
        }
    }

    /// Create a chord annotation with a specific duration
    pub fn with_duration(symbol: String, duration: Duration, dotted: bool) -> Self {
        Self {
            symbol,
            duration,
            dotted,
        }
    }

    /// Returns duration in beats based on time signature
    pub fn duration_beats(&self, time_sig: &TimeSignature) -> f64 {
        let base = self.duration.as_beats(time_sig);
        if self.dotted { base * 1.5 } else { base }
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
    pub chord: Option<ChordAnnotation>,  // Optional chord symbol with independent duration
}

impl Note {
    /// Returns total duration in beats including dotted and tuplet modifiers
    pub fn total_beats(&self, time_sig: &TimeSignature) -> f64 {
        let base = self.duration.as_beats(time_sig);
        let with_dot = if self.dotted { base * 1.5 } else { base };

        match &self.tuplet {
            Some(tuplet) => with_dot * (tuplet.normal_notes as f64 / tuplet.actual_notes as f64),
            None => with_dot,
        }
    }

    /// Returns MIDI note number (C4 = 60, middle C)
    /// Takes into account: note name, explicit accidental, octave offset, key signature
    pub fn to_midi_note(&self, key_sig: &KeySignature, clef_offset: i8) -> u8 {
        // Base MIDI numbers for each note (C4=60)
        let base_midi = match self.name {
            NoteName::C => 60,
            NoteName::D => 62,
            NoteName::E => 64,
            NoteName::F => 65,
            NoteName::G => 67,
            NoteName::A => 69,
            NoteName::B => 71,
        };

        // Apply key signature accidentals if note doesn't have explicit accidental
        let accidental_offset = match self.accidental {
            Accidental::Sharp => 1,
            Accidental::Flat => -1,
            Accidental::ForceNatural => 0,
            Accidental::Natural => {
                // Follow key signature
                match key_sig.accidental_for_note(self.name) {
                    Accidental::Sharp => 1,
                    Accidental::Flat => -1,
                    _ => 0,
                }
            }
        };

        // Apply octave offset (^ = +12, _ = -12, etc.)
        let octave_offset = match self.octave {
            Octave::DoubleLow => -24,
            Octave::Low => -12,
            Octave::Middle => 0,
            Octave::High => 12,
            Octave::DoubleHigh => 24,
        };

        // Apply clef offset (treble vs bass, etc.)
        let total = base_midi + accidental_offset + octave_offset + (clef_offset * 12);

        // Clamp to valid MIDI range (0-127)
        total.clamp(0, 127) as u8
    }
}

/// An element in a measure: either a note or a rest
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Note(Note),
    Rest { duration: Duration, dotted: bool, tuplet: Option<TupletInfo>, chord: Option<ChordAnnotation> },
}

impl Element {
    /// Returns total duration in beats
    pub fn total_beats(&self, time_sig: &TimeSignature) -> f64 {
        match self {
            Element::Note(note) => note.total_beats(time_sig),
            Element::Rest { duration, dotted, tuplet, .. } => {
                let base = duration.as_beats(time_sig);
                let with_dot = if *dotted { base * 1.5 } else { base };

                match tuplet {
                    Some(t) => with_dot * (t.normal_notes as f64 / t.actual_notes as f64),
                    None => with_dot,
                }
            }
        }
    }
}

/// Ending type for volta brackets (1st/2nd endings)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ending {
    First,   // 1.
    Second,  // 2.
}

/// A single measure containing musical elements
#[derive(Debug, Clone)]
pub struct Measure {
    pub elements: Vec<Element>,
    pub repeat_start: bool,   // ||: at the beginning of the measure
    pub repeat_end: bool,     // :|| at the end of the measure
    pub ending: Option<Ending>, // 1. or 2. volta bracket
    pub key_change: Option<KeySignature>, // @key: annotation - changes key signature from this point forward
}

/// Instrument groups for mod points
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InstrumentGroup {
    Eb,  // Alto sax, Baritone sax
    Bb,  // Trumpet, Tenor sax, Clarinet
}

impl InstrumentGroup {
    /// Parse from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "eb" => Some(InstrumentGroup::Eb),
            "bb" => Some(InstrumentGroup::Bb),
            _ => None,
        }
    }
}

/// Mod points - per-line octave shifts for instrument groups
#[derive(Debug, Clone, Default)]
pub struct ModPoints {
    /// Maps line number -> (instrument group -> octave shift)
    /// Line numbers are 1-indexed (matching editor display)
    pub points: HashMap<usize, HashMap<InstrumentGroup, i8>>,
}

impl ModPoints {
    /// Get the octave shift for a specific line and instrument group
    pub fn get_shift(&self, line: usize, group: InstrumentGroup) -> Option<i8> {
        self.points.get(&line).and_then(|groups| groups.get(&group).copied())
    }

    /// Set the octave shift for a specific line and instrument group
    pub fn set_shift(&mut self, line: usize, group: InstrumentGroup, shift: i8) {
        self.points
            .entry(line)
            .or_insert_with(HashMap::new)
            .insert(group, shift);
    }

    /// Remove the octave shift for a specific line and instrument group
    pub fn remove_shift(&mut self, line: usize, group: InstrumentGroup) {
        if let Some(groups) = self.points.get_mut(&line) {
            groups.remove(&group);
            if groups.is_empty() {
                self.points.remove(&line);
            }
        }
    }
}

/// A complete musical score
#[derive(Debug, Clone)]
pub struct Score {
    pub metadata: Metadata,
    pub measures: Vec<Measure>,
    pub mod_points: ModPoints,
    /// Maps source line number (1-indexed) to measure index
    pub line_to_measure: HashMap<usize, usize>,
}
