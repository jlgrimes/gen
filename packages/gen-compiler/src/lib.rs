pub mod ast;
pub mod error;
pub mod lexer;
pub mod musicxml;
pub mod parser;
pub mod semantic;

use serde::Serialize;

pub use ast::*;
pub use error::*;
pub use musicxml::{to_musicxml, to_musicxml_with_options, to_musicxml_with_mod_points, Clef, Transposition};
pub use parser::parse;
pub use semantic::validate;

/// Compile a Gen source string to MusicXML.
/// This is the main entry point for the library.
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

/// Playback data for a single note
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackNote {
    pub midi_note: u8,      // MIDI note number (0-127, C4 = 60)
    pub start_time: f64,    // Time in beats from start
    pub duration: f64,      // Duration in beats
}

/// Playback data for an entire score
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackData {
    pub tempo: u16,           // BPM
    pub notes: Vec<PlaybackNote>,
}

/// Generate playback data from a Gen source string
/// Returns timing and MIDI note information for audio playback
pub fn generate_playback_data(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
) -> Result<PlaybackData, GenError> {
    let score = parse(source)?;

    // Calculate clef offset for MIDI note calculation
    let clef_offset = match clef {
        "bass" => -2,  // Bass clef is 2 octaves lower
        _ => 0,        // Treble clef is the base
    };

    let total_offset = clef_offset + octave_shift;
    let _group = instrument_group.and_then(InstrumentGroup::from_str); // Reserved for future mod point support

    let mut current_time = 0.0;
    let mut notes = Vec::new();
    let mut current_key = score.metadata.key_signature.clone();
    let mut pending_tie: Option<(usize, f64)> = None; // (note index, accumulated duration)

    for measure in &score.measures {
        // Check for key changes
        if let Some(new_key) = &measure.key_change {
            current_key = new_key.clone();
        }

        for element in &measure.elements {
            let duration = element.total_beats(&score.metadata.time_signature);

            match element {
                Element::Note(note) => {
                    if note.tie_start && !note.tie_stop {
                        // Start of a tied group - create note and track it
                        let note_idx = notes.len();
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, total_offset),
                            start_time: current_time,
                            duration,
                        });
                        pending_tie = Some((note_idx, duration));
                    } else if note.tie_stop && note.tie_start {
                        // Middle of a tied group - extend the first note's duration
                        if let Some((idx, accumulated)) = pending_tie {
                            notes[idx].duration = accumulated + duration;
                            pending_tie = Some((idx, accumulated + duration));
                        }
                    } else if note.tie_stop && !note.tie_start {
                        // End of a tied group - extend the first note's duration
                        if let Some((idx, accumulated)) = pending_tie {
                            notes[idx].duration = accumulated + duration;
                            pending_tie = None;
                        }
                    } else {
                        // Regular note (not tied)
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, total_offset),
                            start_time: current_time,
                            duration,
                        });
                        pending_tie = None;
                    }
                }
                Element::Rest { .. } => {
                    // Rests just advance time
                    pending_tie = None;
                }
            }

            current_time += duration;
        }
    }

    Ok(PlaybackData {
        tempo: score.metadata.tempo.unwrap_or(120),
        notes,
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;

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
