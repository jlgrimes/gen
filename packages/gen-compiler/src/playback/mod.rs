//! # Playback Module
//!
//! Generate MIDI playback data from Gen scores for audio playback and visual highlighting.
//!
//! ## Purpose
//! This module converts a parsed Gen score into MIDI playback data that can be used for:
//! 1. **Audio playback** - MIDI note numbers, timing, and duration for synthesizers
//! 2. **Visual highlighting** - Matching notes on the rendered sheet music during playback
//! 3. **Chord accompaniment** - Piano chords from `@ch:` annotations
//!
//! ## Sub-modules
//! - `types` - PlaybackData, PlaybackNote, PlaybackChord type definitions
//! - `engine` - Main playback data generation logic
//! - `chord_parser` - Chord symbol parsing (C, Am, G7, etc.)
//!
//! ## Key Types
//! - [`PlaybackData`] - Complete playback info (notes + chords + tempo)
//! - [`PlaybackNote`] - Single note with MIDI pitch, timing, and OSMD matching info
//! - [`PlaybackChord`] - Chord accompaniment (multiple notes simultaneously)
//!
//! ## Entry Point
//! [`generate_playback_data()`] - Convert Gen source to playback data
//!
//! ## Example
//! ```rust
//! use gen::playback::generate_playback_data;
//!
//! let source = r#"---
//! tempo: 120
//! ---
//! C D E F
//! "#;
//!
//! let data = generate_playback_data(source, "treble", 0, None, None).unwrap();
//!
//! assert_eq!(data.tempo, 120);
//! assert_eq!(data.notes.len(), 4);
//! assert_eq!(data.notes[0].midi_note, 60); // C4
//! ```
//!
//! ## Dual-Timing System
//!
//! The playback engine maintains two separate timing tracks:
//!
//! ### Playback Time
//! - Used for actual audio playback
//! - Correctly calculates triplet durations
//! - Example: Quarter note triplet = 0.667 beats per note
//!
//! ### OSMD Time
//! - Used for visual note matching with OpenSheetMusicDisplay
//! - Uses MusicXML quantized durations
//! - Example: Quarter note triplet = 0.5 beats per note
//!
//! This enables correct audio playback AND visual note highlighting.
//!
//! ## MIDI Note System
//!
//! Each note has two MIDI values:
//!
//! ### Concert Pitch (midi_note)
//! - For audio playback
//! - Unaffected by clef
//! - Example: C4 = MIDI 60 (treble or bass)
//!
//! ### Display MIDI (display_midi_note)
//! - For matching visual notes
//! - Includes clef offset
//! - Example: Treble C4 = 60, Bass C4 = 36 (displays 2 octaves lower)
//!
//! ## Related Modules
//! - `ast` - Uses Score, Note, Measure types
//! - `parser` - Parses Gen source into Score
//! - `musicxml` - Parallel output format (MusicXML for rendering, playback for audio)

mod types;
mod engine;
mod chord_parser;

#[cfg(test)]
mod tests;

pub use types::{PlaybackData, PlaybackNote, PlaybackChord, TieType};
pub use engine::generate_playback_data;
pub use chord_parser::parse_chord_symbol;
