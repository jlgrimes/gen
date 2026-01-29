//! # Gen Music Notation Compiler
//!
//! A text-based music notation language compiler that generates industry-standard MusicXML.
//!
//! ## Compilation Pipeline
//!
//! ```text
//! .gen source → Lexer → Parser → Semantic → MusicXML Generator → .musicxml
//! ```
//!
//! 1. **Lexer** ([`lexer`]) - Tokenizes Gen source into tokens with location info
//! 2. **Parser** ([`parser`]) - Parses tokens into Abstract Syntax Tree
//!    - First pass: Extract metadata, mod points, key changes, chord annotations
//!    - Second pass: Parse music with context from first pass
//! 3. **Semantic** ([`semantic`]) - Validates AST (measure durations, repeats, endings)
//! 4. **MusicXML Generator** ([`musicxml`]) - Generates MusicXML output
//! 5. **Playback** (this module) - Optional MIDI playback data generation
//!
//! ## Quick Start
//!
//! ```rust
//! use gen::compile;
//!
//! let source = r#"---
//! title: My Song
//! composer: Me
//! time-signature: 4/4
//! key-signature: C
//! tempo: 120
//! ---
//! C D E F
//! G A B C^
//! "#;
//!
//! let musicxml = compile(source)?;
//! // Write musicxml to file or render with notation software
//! # Ok::<(), gen::GenError>(())
//! ```
//!
//! ## Public API Entry Points
//!
//! ### Compilation Functions
//! - [`compile()`] - Full compilation with validation (recommended)
//! - [`compile_unchecked()`] - Skip validation (for partial/incomplete scores)
//! - [`compile_with_options()`] - Custom clef, octave shift, transposition
//! - [`compile_with_mod_points()`] - Instrument-specific rendering with mod points
//!
//! ### Playback Functions
//! - [`generate_playback_data()`] - Generate MIDI playback data with timing info
//!
//! ### Low-Level API
//! - [`parse()`] - Parse Gen source into AST
//! - [`validate()`] - Validate AST semantic correctness
//! - [`to_musicxml()`] - Generate MusicXML from AST
//!
//! ## Gen Language Syntax Overview
//!
//! ### Note Format
//! `[rhythm][note][pitch]`
//!
//! - **Rhythm modifiers**: `/` (eighth), `//` (sixteenth), `d` (half), `o` (whole), `*` (dotted)
//! - **Notes**: A-G or `$` (rest)
//! - **Pitch modifiers**: `#` (sharp), `b` (flat), `^` (octave up), `_` (octave down)
//!
//! ### Examples
//! - `C` - C quarter note
//! - `/E` - E eighth note
//! - `dG*` - G dotted half note
//! - `//F#^` - F# sixteenth note, one octave up
//! - `$` - quarter rest
//!
//! ### Tuplets
//! - `3[C D E]` - Quarter note triplet (3 notes in time of 2)
//! - `/3[A B C]` - Eighth note triplet
//! - `5[C D E F G]` - Quintuplet (5 in time of 4)
//!
//! ### Ties and Slurs
//! - `C-C` - Two tied quarter notes (play as half note)
//! - `(C D E F)` - Slurred phrase
//!
//! ### Repeats and Endings
//! - `||:` - Repeat start
//! - `:||` - Repeat end
//! - `|1` - First ending
//! - `|2` - Second ending
//!
//! ## Module Structure
//!
//! - [`ast`] - Abstract Syntax Tree type definitions (Score, Measure, Note, etc.)
//! - [`error`] - Error types (GenError variants)
//! - [`lexer`] - Tokenization (String → Vec<Token>)
//! - [`parser`] - Parsing (Vec<Token> → Score AST)
//! - [`semantic`] - Validation (measure durations, repeats)
//! - [`musicxml`] - MusicXML generation (Score → MusicXML string)
//!
//! ## Additional Resources
//!
//! - **ARCHITECTURE.md** - Detailed architectural documentation for agents
//! - **CLAUDE.md** - High-level project overview and language syntax
//! - **gen-docs** - Complete language documentation and examples
//!
//! ## Features
//!
//! - ✅ All standard music notation (notes, rests, ties, slurs, tuplets)
//! - ✅ Full key signature support (major and minor keys)
//! - ✅ Any time signature (4/4, 3/4, 6/8, 5/4, 7/8, etc.)
//! - ✅ Instrument transposition (Bb, Eb, F)
//! - ✅ Chord symbols for lead sheets
//! - ✅ Repeats and endings
//! - ✅ Mid-score key changes
//! - ✅ Automatic beaming
//! - ✅ MIDI playback data generation
//! - ✅ Integration with OpenSheetMusicDisplay (OSMD)

pub mod ast;
pub mod error;
pub mod lexer;
pub mod musicxml;
pub mod parser;
pub mod semantic;
pub mod playback;

pub use ast::*;
pub use error::*;
pub use musicxml::{to_musicxml, to_musicxml_with_options, to_musicxml_with_mod_points, Clef, Transposition};
pub use parser::parse;
pub use semantic::validate;
pub use playback::{generate_playback_data, PlaybackData, PlaybackNote, PlaybackChord, TieType};

/// Compile a Gen source string to MusicXML.
///
/// This is the main entry point for the library. It performs full compilation with validation.
///
/// # Pipeline
/// 1. Tokenize source with [`lexer`]
/// 2. Parse tokens into AST with [`parse()`]
/// 3. Validate AST with [`validate()`]
/// 4. Generate MusicXML with [`to_musicxml()`]
///
/// # Example
/// ```rust
/// use gen::compile;
///
/// let source = "C D E F";
/// let musicxml = compile(source)?;
/// // Write to .musicxml file or render
/// # Ok::<(), gen::GenError>(())
/// ```
///
/// # Errors
/// Returns [`GenError`] if parsing, validation, or generation fails.
pub fn compile(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    validate(&score)?;
    Ok(to_musicxml(&score))
}

/// Compile without validation (useful for partial/incomplete scores)
pub fn compile_unchecked(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    Ok(to_musicxml(&score))
}

/// Compile with custom clef and octave shift options
pub fn compile_with_options(source: &str, clef: &str, octave_shift: i8, transposition: Option<Transposition>) -> Result<String, GenError> {
    let score = parse(source)?;
    let clef = match clef {
        "bass" => Clef::Bass,
        _ => Clef::Treble,
    };
    Ok(to_musicxml_with_options(&score, transposition, clef, octave_shift))
}

/// Compile with mod points support for instrument-specific octave shifts
/// instrument_group: "eb" for Eb instruments, "bb" for Bb instruments, or None
/// transpose_key: "C" (concert), "Bb", "Eb", or "F" for transposing instruments
pub fn compile_with_mod_points(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
    transpose_key: Option<&str>,
) -> Result<String, GenError> {
    let score = parse(source)?;
    let clef = match clef {
        "bass" => Clef::Bass,
        _ => Clef::Treble,
    };
    let group = instrument_group.and_then(InstrumentGroup::from_str);
    let transposition = transpose_key.and_then(Transposition::for_key);
    Ok(to_musicxml_with_mod_points(&score, transposition, clef, octave_shift, group))
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Compilation tests
    #[test]
    fn test_compile_with_rhythm_groupings() {
        // 8 sixteenth notes = 2 beats, + 2 quarter notes = 4 beats total
        let source = r#"---
title: Rhythm Grouping Test
---
//[C D E F] //[G A B C^] C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile rhythm groupings successfully");
        let xml = result.unwrap();
        assert!(xml.contains("<duration>"));
        assert!(xml.contains("Rhythm Grouping Test"));
    }

    #[test]
    fn test_compile_with_triplets_new_syntax() {
        // Triplet (3 quarters in time of 2 = 2 beats) + 2 quarter notes = 4 beats
        let source = r#"---
title: Triplet Test
---
3[C D E] C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile triplets with new syntax");
        let xml = result.unwrap();
        assert!(xml.contains("<time-modification>"));
        assert!(xml.contains("<actual-notes>3</actual-notes>"));
        assert!(xml.contains("<normal-notes>2</normal-notes>"));
    }

    #[test]
    fn test_compile_with_eighth_note_triplet() {
        // Eighth triplet (3 eighths in time of 2 eighths = 1 beat) + 3 quarters = 4 beats
        let source = r#"---
title: Eighth Note Triplet Test
---
/3[C D E] C C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile eighth note triplets");
        let xml = result.unwrap();
        assert!(xml.contains("<time-modification>"));
        assert!(xml.contains("<type>eighth</type>"));
    }

    #[test]
    fn test_compile_mixed_rhythm_grouping_and_triplets() {
        let source = r#"---
title: Mixed Test
---
//[C D E F] //[G A B C^] C C
/3[C D E] C C C
3[C D E] C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile mixed rhythm groupings and triplets");
    }

    #[test]
    fn test_compile_quintuplet() {
        // 5 quarters in time of 4 = 4 beats, + 1 quarter = 5 beats (5/4 time)
        let source = r#"---
title: Quintuplet Test
time-signature: 5/4
---
5[C D E F G] C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile quintuplets");
        let xml = result.unwrap();
        assert!(xml.contains("<actual-notes>5</actual-notes>"));
        assert!(xml.contains("<normal-notes>4</normal-notes>"));
    }

    #[test]
    fn test_compile_sextuplet() {
        // 6 quarters in time of 4 = 4 beats
        let source = r#"---
title: Sextuplet Test
---
6[C D E F G A]
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile sextuplets");
        let xml = result.unwrap();
        assert!(xml.contains("<actual-notes>6</actual-notes>"));
        assert!(xml.contains("<normal-notes>4</normal-notes>"));
    }

    #[test]
    fn test_rhythm_grouping_with_rests() {
        // 4 sixteenths (1 beat) + 2 quarters (2 beats) = 3 beats, need one more
        let source = r#"---
title: Rhythm Grouping with Rests
---
//[C D $ F] C C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile rhythm groupings with rests");
        let xml = result.unwrap();
        assert!(xml.contains("<rest"));
    }

    #[test]
    fn test_triplet_with_accidentals() {
        // Triplet (2 beats) + 2 quarters = 4 beats
        let source = r#"---
title: Triplet with Accidentals
---
3[C# Eb F#] C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile triplets with accidentals");
        let xml = result.unwrap();
        assert!(xml.contains("<alter>1</alter>")); // Sharp
        assert!(xml.contains("<alter>-1</alter>")); // Flat
    }

    #[test]
    fn test_rhythm_grouping_with_ties() {
        // Test ties within rhythm groupings (e.g., /[C D E-] with tie on last note)
        // 3 eighths (1.5 beats) + tie + 1 eighth (0.5 beat) + 2 quarters (2 beats) = 4 beats total
        let source = r#"---
title: Rhythm Grouping with Ties
---
/[C D E-] /E C C
"#;
        let result = compile(source);
        assert!(result.is_ok(), "Should compile rhythm groupings with ties");
        let xml = result.unwrap();
        assert!(xml.contains("<tied type=\"start\"/>")); // Tie start
        assert!(xml.contains("<tied type=\"stop\"/>")); // Tie stop
    }
}

    #[test]
    fn test_playback_triplets() {
        let source = r#"---
tempo: 120
---
C /3[D E F] G
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // C, D, E, F (triplet), G
        assert_eq!(data.notes.len(), 5);

        println!("\n=== Triplet Playback Data ===");
        for (i, note) in data.notes.iter().enumerate() {
            println!("Note {}: MIDI {} at beat {:.4}, duration {:.4}",
                i, note.midi_note, note.start_time, note.duration);
        }

        // C at beat 0
        assert_eq!(data.notes[0].midi_note, 60);
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[0].duration, 1.0);

        // Triplet notes: 3 eighth notes in the space of 2 eighth notes (1 beat)
        // Duration of each = 1.0 / 3 = 0.333...
        assert_eq!(data.notes[1].midi_note, 62); // D
        assert!((data.notes[1].start_time - 1.0).abs() < 0.0001);
        assert!((data.notes[1].duration - 0.3333333333333333).abs() < 0.0001);

        assert_eq!(data.notes[2].midi_note, 64); // E
        assert!((data.notes[2].start_time - 1.3333333333333333).abs() < 0.0001);

        assert_eq!(data.notes[3].midi_note, 65); // F
        assert!((data.notes[3].start_time - 1.6666666666666667).abs() < 0.0001);

        // G at beat 2 (triplet takes 1 beat total: from 1.0 to 2.0)
        assert_eq!(data.notes[4].midi_note, 67);
        assert!((data.notes[4].start_time - 2.0).abs() < 0.0001);
    }

    #[test]
    fn test_osmd_match_keys() {
        let source = r#"---
tempo: 120
---
C D E
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // C4 = MIDI 60, display 60 (OSMD uses display MIDI directly)
        assert_eq!(data.notes[0].osmd_match_key, "60_0.000");

        // D4 = MIDI 62, display 62
        assert_eq!(data.notes[1].osmd_match_key, "62_1.000");

        // E4 = MIDI 64, display 64
        assert_eq!(data.notes[2].osmd_match_key, "64_2.000");
    }

    #[test]
    fn test_osmd_match_keys_triplets() {
        let source = r#"---
tempo: 120
---
C 3[D E F] G
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        println!("\n=== OSMD Match Keys for Triplets ===");
        for (i, note) in data.notes.iter().enumerate() {
            println!("Note {}: playback={:.3}, osmd={:.3}, key=\"{}\"",
                i, note.start_time, note.osmd_timestamp, note.osmd_match_key);
        }

        // C at beat 0 (both playback and OSMD) - C4 = MIDI 60
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[0].osmd_timestamp, 0.0);
        assert_eq!(data.notes[0].osmd_match_key, "60_0.000");

        // D: playback 1.0, OSMD 1.0 (start of triplet) - D4 = MIDI 62
        assert_eq!(data.notes[1].start_time, 1.0);
        assert_eq!(data.notes[1].osmd_timestamp, 1.0);
        assert_eq!(data.notes[1].osmd_match_key, "62_1.000");

        // E: playback 1.667 (triplet math), OSMD 1.5 (MusicXML quantized) - E4 = MIDI 64
        assert!((data.notes[2].start_time - 1.6666666666666667).abs() < 0.0001);
        assert_eq!(data.notes[2].osmd_timestamp, 1.5);
        assert_eq!(data.notes[2].osmd_match_key, "64_1.500");

        // F: playback 2.333 (triplet math), OSMD 2.0 (MusicXML quantized) - F4 = MIDI 65
        assert!((data.notes[3].start_time - 2.3333333333333335).abs() < 0.0001);
        assert_eq!(data.notes[3].osmd_timestamp, 2.0);
        assert_eq!(data.notes[3].osmd_match_key, "65_2.000");

        // G: playback ~3.0 (floating point), OSMD 2.5 (MusicXML quantized) - G4 = MIDI 67
        assert!((data.notes[4].start_time - 3.0).abs() < 0.0001);
        assert_eq!(data.notes[4].osmd_timestamp, 2.5);
        assert_eq!(data.notes[4].osmd_match_key, "67_2.500");
    }

    #[test]
    fn test_osmd_match_keys_with_octave_shift() {
        let source = r#"---
tempo: 120
---
C D E
"#;

        // Octave shift up (+1 octave = +12 semitones)
        let result = generate_playback_data(source, "treble", 1, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // C4 shifted up 1 octave = C5 = MIDI 72 (both playback and display)
        assert_eq!(data.notes[0].midi_note, 72);  // Playback includes octave shift
        assert_eq!(data.notes[0].display_midi_note, 72);  // Display shifted up
        assert_eq!(data.notes[0].osmd_match_key, "72_0.000");

        // D4 shifted up 1 octave = D5 = MIDI 74
        assert_eq!(data.notes[1].midi_note, 74);
        assert_eq!(data.notes[1].display_midi_note, 74);
        assert_eq!(data.notes[1].osmd_match_key, "74_1.000");
    }
