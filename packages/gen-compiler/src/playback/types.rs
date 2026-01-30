//! Playback data type definitions
//!
//! This module defines the types used for MIDI playback and visual note highlighting.

use serde::Serialize;

/// Tie type for notes
///
/// Indicates whether a note is part of a tied group and how it should be handled
/// for audio playback and visual rendering.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TieType {
    /// Regular note, not part of a tie
    None,
    /// Starts a tie (play audio)
    Start,
    /// Middle of a tie (visual only, no audio)
    Continue,
    /// Ends a tie (visual only, no audio)
    End,
}

/// Playback data for a single note
///
/// Contains ALL information needed for both audio playback and visual highlighting.
///
/// # Fields
/// - `midi_note`: Concert pitch MIDI note for audio playback (unaffected by clef)
/// - `display_midi_note`: Display MIDI note (includes clef offset, for matching with sheet music)
/// - `start_time`: Actual playback start time in beats (triplet-adjusted)
/// - `duration`: Actual playback duration in beats (triplet-adjusted)
/// - `note_index`: Sequential index (0, 1, 2, ...) for matching with OSMD note order
/// - `measure_number`: Which measure this note is in (1-indexed)
/// - `beat_in_measure`: Beat position within the measure (for OSMD timestamp matching)
/// - `osmd_timestamp`: OSMD's display timestamp (accumulated note lengths, not triplet-adjusted)
/// - `osmd_match_key`: Pre-computed key for matching with OSMD GraphicalNotes: "{midi}_{timestamp}"
///
/// # MIDI Note vs Display MIDI Note
/// - **Concert Pitch (midi_note)**: Used for audio playback, unaffected by clef
///   - Example: Treble clef C4 = MIDI 60, Bass clef C4 = MIDI 60 (same pitch)
/// - **Display MIDI (display_midi_note)**: Includes clef offset for matching visual notes
///   - Example: Treble clef C4 = MIDI 60, Bass clef C4 display = MIDI 36 (shows 2 octaves lower)
///
/// # Triplet Timing
/// - `start_time` and `duration` use actual triplet math (e.g., 0.667 beats per note in triplet)
/// - `osmd_timestamp` uses MusicXML quantized durations (e.g., 0.5 beats per triplet note)
/// - This dual-timing system enables correct audio playback AND visual note matching
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackNote {
    pub midi_note: u8,
    pub display_midi_note: u8,
    pub start_time: f64,
    pub duration: f64,
    pub note_index: usize,
    pub measure_number: usize,
    pub beat_in_measure: f64,
    pub osmd_timestamp: f64,
    pub osmd_match_key: String,
}

/// Playback data for a chord (multiple notes played simultaneously)
///
/// Used for chord accompaniment in lead sheet style.
///
/// # Fields
/// - `midi_notes`: MIDI note numbers for all notes in the chord
/// - `start_time`: Time in beats from start of the score (for audio playback)
/// - `duration`: Duration in beats
/// - `osmd_timestamp`: OSMD's display timestamp (for visual highlighting)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackChord {
    pub midi_notes: Vec<u8>,
    pub start_time: f64,
    pub duration: f64,
    pub osmd_timestamp: f64,
}

/// Swing feel for playback
///
/// Specifies which note duration should be played with swing feel.
/// Standard jazz swing uses a 2:1 ratio (triplet-based), where the first note
/// of a pair gets 2/3 of the beat and the second gets 1/3.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SwingType {
    /// Swing eighth notes (standard jazz swing)
    Eighth,
    /// Swing sixteenth notes (funk/fusion style)
    Sixteenth,
}

/// Playback data for an entire score
///
/// Contains all information needed to play back a score with audio and visual highlighting.
///
/// # Fields
/// - `tempo`: Tempo in BPM (beats per minute, where beat = quarter note)
/// - `notes`: All melody notes with timing and OSMD matching info
/// - `chords`: Chord accompaniment (always piano, from @ch: annotations)
/// - `swing`: Optional swing feel (eighth or sixteenth notes)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackData {
    pub tempo: u16,
    pub notes: Vec<PlaybackNote>,
    pub chords: Vec<PlaybackChord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swing: Option<SwingType>,
}
