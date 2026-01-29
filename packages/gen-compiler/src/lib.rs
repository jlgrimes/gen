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
/// Contains ALL information needed for both audio playback and visual highlighting
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackNote {
    pub midi_note: u8,          // Concert pitch MIDI note (for audio playback)
    pub display_midi_note: u8,  // Display MIDI note (transposed, for matching with sheet music)
    pub start_time: f64,        // Actual playback start time in beats (with triplet calculations)
    pub duration: f64,          // Actual playback duration in beats (with triplet calculations)
    pub note_index: usize,      // Sequential index (0, 1, 2, ...) for matching with OSMD note order
    pub measure_number: usize,  // Which measure this note is in (1-indexed)
    pub beat_in_measure: f64,   // Beat position within the measure (for OSMD timestamp matching)
    pub osmd_timestamp: f64,    // OSMD's display timestamp (accumulated note lengths, not triplet-adjusted)
    pub osmd_match_key: String, // Pre-computed key for matching with OSMD GraphicalNotes: "{midi}_{timestamp}"
}

/// Playback data for a chord (multiple notes played simultaneously)
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackChord {
    pub midi_notes: Vec<u8>, // MIDI note numbers for chord
    pub start_time: f64,     // Time in beats from start
    pub duration: f64,       // Duration in beats
}

/// Playback data for an entire score
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackData {
    pub tempo: u16,           // BPM
    pub notes: Vec<PlaybackNote>,
    pub chords: Vec<PlaybackChord>, // Chord accompaniment (always piano)
}

/// Parse a chord symbol into MIDI notes
/// Returns a Vec of MIDI note numbers relative to C4 (60)
fn parse_chord_symbol(chord_symbol: &str) -> Vec<u8> {
    // Extract root note and chord quality
    let chars: Vec<char> = chord_symbol.chars().collect();
    if chars.is_empty() {
        return vec![];
    }

    // Parse root note
    let root_name = chars[0];
    let mut idx = 1;

    // Check for accidental
    let accidental = if idx < chars.len() && (chars[idx] == '#' || chars[idx] == 'b') {
        idx += 1;
        if chars[idx - 1] == '#' { 1 } else { -1 }
    } else {
        0
    };

    // Base MIDI note for root (C4 = 60, but we'll use C3 = 48 for chords)
    let base_midi = match root_name {
        'C' => 48,
        'D' => 50,
        'E' => 52,
        'F' => 53,
        'G' => 55,
        'A' => 57,
        'B' => 59,
        _ => return vec![],
    };
    let root = (base_midi + accidental) as u8;

    // Parse chord quality from remaining string
    let quality = &chord_symbol[idx..];

    // Return intervals relative to root
    // Using common jazz/pop chord voicings
    match quality {
        // Major triads
        "" | "maj" | "M" => vec![root, root + 4, root + 7],

        // Minor triads
        "m" | "min" | "-" => vec![root, root + 3, root + 7],

        // Dominant 7th
        "7" => vec![root, root + 4, root + 7, root + 10],

        // Major 7th
        "maj7" | "M7" => vec![root, root + 4, root + 7, root + 11],

        // Minor 7th
        "m7" | "min7" | "-7" => vec![root, root + 3, root + 7, root + 10],

        // Diminished
        "dim" | "°" => vec![root, root + 3, root + 6],

        // Augmented
        "aug" | "+" => vec![root, root + 4, root + 8],

        // Sus chords
        "sus4" => vec![root, root + 5, root + 7],
        "sus2" => vec![root, root + 2, root + 7],

        // Extended chords
        "9" => vec![root, root + 4, root + 7, root + 10, root + 14],
        "maj9" | "M9" => vec![root, root + 4, root + 7, root + 11, root + 14],
        "m9" | "min9" => vec![root, root + 3, root + 7, root + 10, root + 14],

        // Default to major if unknown
        _ => vec![root, root + 4, root + 7],
    }
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

    let mut current_time = 0.0;      // Playback time (triplet-adjusted)
    let mut osmd_time = 0.0;         // OSMD display time (not triplet-adjusted)
    let mut notes = Vec::new();
    let mut chords = Vec::new();
    let mut current_key = score.metadata.key_signature.clone();
    let mut pending_tie: Option<(usize, f64)> = None; // (note index, accumulated duration)
    let mut note_index = 0usize;

    for (measure_idx, measure) in score.measures.iter().enumerate() {
        let measure_number = measure_idx + 1; // 1-indexed
        let measure_start_time = current_time;

        // Check for key changes
        if let Some(new_key) = &measure.key_change {
            current_key = new_key.clone();
        }

        for element in &measure.elements {
            let duration = element.total_beats(&score.metadata.time_signature);

            // OSMD accumulates using MusicXML duration values (quantized to divisions)
            // For triplets, MusicXML uses floor((normal * base * divisions) / actual) / divisions
            // With divisions=4: quarter triplet (3:2) = floor((2*1*4)/3)/4 = floor(2.67)/4 = 2/4 = 0.5
            let osmd_duration = match element {
                Element::Note(note) => {
                    let base = note.duration.as_beats(&score.metadata.time_signature);
                    let with_dot = if note.dotted { base * 1.5 } else { base };
                    if let Some(tuplet) = &note.tuplet {
                        // Calculate MusicXML quantized duration
                        let divisions = 4.0; // Standard: quarter note = 4 divisions
                        let musicxml_dur = ((tuplet.normal_notes as f64 * with_dot * divisions) / tuplet.actual_notes as f64).floor();
                        musicxml_dur / divisions
                    } else {
                        with_dot
                    }
                },
                Element::Rest { duration: rest_dur, dotted, tuplet, .. } => {
                    let base = rest_dur.as_beats(&score.metadata.time_signature);
                    let with_dot = if *dotted { base * 1.5 } else { base };
                    if let Some(t) = tuplet {
                        let divisions = 4.0;
                        let musicxml_dur = ((t.normal_notes as f64 * with_dot * divisions) / t.actual_notes as f64).floor();
                        musicxml_dur / divisions
                    } else {
                        with_dot
                    }
                },
            };

            match element {
                Element::Note(note) => {
                    // Handle chord symbol if present
                    if let Some(chord_symbol) = &note.chord {
                        let chord_notes = parse_chord_symbol(chord_symbol);
                        if !chord_notes.is_empty() {
                            chords.push(PlaybackChord {
                                midi_notes: chord_notes,
                                start_time: current_time,
                                duration,
                            });
                        }
                    }

                    if note.tie_start && !note.tie_stop {
                        // Start of a tied group - create note and track it
                        let note_idx = notes.len();
                        let beat_in_measure = current_time - measure_start_time;
                        let display_midi = note.to_midi_note(&current_key, total_offset);
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, 0), // Concert pitch (no offset)
                            display_midi_note: display_midi, // Display pitch (with offset)
                            start_time: current_time,
                            duration,
                            note_index,
                            measure_number,
                            beat_in_measure,
                            osmd_timestamp: osmd_time,
                            osmd_match_key: format!("{}_{:.3}", display_midi, osmd_time),
                        });
                        note_index += 1;
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
                        let beat_in_measure = current_time - measure_start_time;
                        let display_midi = note.to_midi_note(&current_key, total_offset);
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, 0), // Concert pitch (no offset)
                            display_midi_note: display_midi, // Display pitch (with offset)
                            start_time: current_time,
                            duration,
                            note_index,
                            measure_number,
                            beat_in_measure,
                            osmd_timestamp: osmd_time,
                            osmd_match_key: format!("{}_{:.3}", display_midi, osmd_time),
                        });
                        note_index += 1;
                        pending_tie = None;
                    }
                }
                Element::Rest { chord, .. } => {
                    // Handle chord symbol on rest if present
                    if let Some(chord_symbol) = chord {
                        let chord_notes = parse_chord_symbol(chord_symbol);
                        if !chord_notes.is_empty() {
                            chords.push(PlaybackChord {
                                midi_notes: chord_notes,
                                start_time: current_time,
                                duration,
                            });
                        }
                    }
                    // Rests just advance time
                    pending_tie = None;
                }
            }

            current_time += duration;        // Playback time (triplet-adjusted)
            osmd_time += osmd_duration;      // OSMD time (not triplet-adjusted)
        }
    }

    Ok(PlaybackData {
        tempo: score.metadata.tempo.unwrap_or(120),
        notes,
        chords,
    })
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // Playback tests
    #[test]
    fn test_playback_basic_timing() {
        let source = r#"---
tempo: 120
---
C D E F
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        assert_eq!(data.tempo, 120);
        assert_eq!(data.notes.len(), 4);

        // Each quarter note should be 1 beat
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[0].duration, 1.0);
        assert_eq!(data.notes[1].start_time, 1.0);
        assert_eq!(data.notes[1].duration, 1.0);
        assert_eq!(data.notes[2].start_time, 2.0);
        assert_eq!(data.notes[3].start_time, 3.0);
    }

    #[test]
    fn test_playback_midi_notes() {
        let source = r#"---
key-signature: C
---
C D E F G A B C^
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // C4=60, D4=62, E4=64, F4=65, G4=67, A4=69, B4=71, C5=72
        assert_eq!(data.notes[0].midi_note, 60); // C4
        assert_eq!(data.notes[1].midi_note, 62); // D4
        assert_eq!(data.notes[2].midi_note, 64); // E4
        assert_eq!(data.notes[3].midi_note, 65); // F4
        assert_eq!(data.notes[4].midi_note, 67); // G4
        assert_eq!(data.notes[5].midi_note, 69); // A4
        assert_eq!(data.notes[6].midi_note, 71); // B4
        assert_eq!(data.notes[7].midi_note, 72); // C5
    }

    #[test]
    fn test_playback_with_ties() {
        let source = r#"---
tempo: 120
---
C- C d$
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should have 2 notes: C (tied, duration 2 beats) and half rest
        assert_eq!(data.notes.len(), 1);
        assert_eq!(data.notes[0].midi_note, 60); // C4
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[0].duration, 2.0); // Two quarter notes tied
    }

    #[test]
    fn test_playback_different_rhythms() {
        let source = r#"---
tempo: 120
---
dC /C /C C
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        assert_eq!(data.notes.len(), 4);
        assert_eq!(data.notes[0].duration, 2.0); // Half note
        assert_eq!(data.notes[1].duration, 0.5); // Eighth note
        assert_eq!(data.notes[2].duration, 0.5); // Eighth note
        assert_eq!(data.notes[3].duration, 1.0); // Quarter note

        // Check timing
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[1].start_time, 2.0);
        assert_eq!(data.notes[2].start_time, 2.5);
        assert_eq!(data.notes[3].start_time, 3.0);
    }

    #[test]
    fn test_playback_ode_to_joy() {
        let source = r#"---
title: Ode to Joy
tempo: 160
time-signature: 4/4
---
E E F G
G F E D
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // First 8 notes: E E F G G F E D
        assert_eq!(data.notes.len(), 8);

        // E4=64, F4=65, G4=67, D4=62
        assert_eq!(data.notes[0].midi_note, 64); // E4
        assert_eq!(data.notes[0].start_time, 0.0);

        assert_eq!(data.notes[1].midi_note, 64); // E4
        assert_eq!(data.notes[1].start_time, 1.0);

        assert_eq!(data.notes[2].midi_note, 65); // F4
        assert_eq!(data.notes[2].start_time, 2.0);

        assert_eq!(data.notes[3].midi_note, 67); // G4
        assert_eq!(data.notes[3].start_time, 3.0);

        assert_eq!(data.notes[4].midi_note, 67); // G4
        assert_eq!(data.notes[4].start_time, 4.0);

        assert_eq!(data.notes[5].midi_note, 65); // F4
        assert_eq!(data.notes[5].start_time, 5.0);

        assert_eq!(data.notes[6].midi_note, 64); // E4
        assert_eq!(data.notes[6].start_time, 6.0);

        assert_eq!(data.notes[7].midi_note, 62); // D4
        assert_eq!(data.notes[7].start_time, 7.0);
    }

    #[test]
    fn test_playback_with_rests() {
        let source = r#"---
tempo: 120
---
C $ C $
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should have 2 notes (rests don't produce notes)
        assert_eq!(data.notes.len(), 2);
        assert_eq!(data.notes[0].start_time, 0.0);
        assert_eq!(data.notes[1].start_time, 2.0); // After quarter rest
    }

    #[test]
    fn test_playback_default_tempo() {
        let source = "C D E F";
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should default to 120 BPM
        assert_eq!(data.tempo, 120);
    }

    #[test]
    fn test_playback_bass_clef() {
        let source = "C D E";
        let result = generate_playback_data(source, "bass", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Bass clef is 2 octaves lower: C2=36, D2=38, E2=40
        assert_eq!(data.notes[0].midi_note, 36);
        assert_eq!(data.notes[1].midi_note, 38);
        assert_eq!(data.notes[2].midi_note, 40);
    }

    #[test]
    fn test_playback_octave_shift() {
        let source = "C D E";
        let result = generate_playback_data(source, "treble", 1, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // C5=72, D5=74, E5=76 (one octave up)
        assert_eq!(data.notes[0].midi_note, 72);
        assert_eq!(data.notes[1].midi_note, 74);
        assert_eq!(data.notes[2].midi_note, 76);
    }

    #[test]
    fn test_playback_key_signature() {
        let source = r#"---
key-signature: G
---
F G A
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // In G major, F is F# (66), G is natural (67), A is natural (69)
        assert_eq!(data.notes[0].midi_note, 66); // F#4
        assert_eq!(data.notes[1].midi_note, 67); // G4
        assert_eq!(data.notes[2].midi_note, 69); // A4
    }

    #[test]
    fn test_playback_with_chords() {
        let source = r#"---
tempo: 120
---
C D E F
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should have melody notes but no chords (no chord symbols in source)
        assert_eq!(data.notes.len(), 4);
        assert_eq!(data.chords.len(), 0);
    }

    #[test]
    fn test_playback_chord_extraction() {
        let source = r#"---
tempo: 120
---
@ch:C C @ch:G D @ch:Am E @ch:F F
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should have 4 melody notes and 4 chords
        assert_eq!(data.notes.len(), 4);
        assert_eq!(data.chords.len(), 4);

        // Verify first chord (C major: C3, E3, G3)
        assert_eq!(data.chords[0].midi_notes, vec![48, 52, 55]);
        assert_eq!(data.chords[0].start_time, 0.0);
        assert_eq!(data.chords[0].duration, 1.0); // Quarter note

        // Verify second chord (G major: G3, B3, D4)
        assert_eq!(data.chords[1].midi_notes, vec![55, 59, 62]);
        assert_eq!(data.chords[1].start_time, 1.0);

        // Verify third chord (A minor: A3, C4, E4)
        assert_eq!(data.chords[2].midi_notes, vec![57, 60, 64]);
        assert_eq!(data.chords[2].start_time, 2.0);

        // Verify fourth chord (F major: F3, A3, C4)
        assert_eq!(data.chords[3].midi_notes, vec![53, 57, 60]);
        assert_eq!(data.chords[3].start_time, 3.0);
    }

    #[test]
    fn test_playback_chord_on_rest() {
        let source = r#"---
tempo: 120
---
@ch:C $ C C C
"#;
        let result = generate_playback_data(source, "treble", 0, None);
        assert!(result.is_ok());
        let data = result.unwrap();

        // Should have 3 melody notes and 1 chord (on the rest)
        assert_eq!(data.notes.len(), 3);
        assert_eq!(data.chords.len(), 1);

        // Chord should be at start (during the rest)
        assert_eq!(data.chords[0].start_time, 0.0);
        assert_eq!(data.chords[0].midi_notes, vec![48, 52, 55]); // C major
    }

    #[test]
    fn test_chord_parsing() {
        // Test major chord
        let c_major = parse_chord_symbol("C");
        assert_eq!(c_major, vec![48, 52, 55]); // C3, E3, G3

        // Test minor chord
        let d_minor = parse_chord_symbol("Dm");
        assert_eq!(d_minor, vec![50, 53, 57]); // D3, F3, A3

        // Test dominant 7th
        let g7 = parse_chord_symbol("G7");
        assert_eq!(g7, vec![55, 59, 62, 65]); // G3, B3, D4, F4

        // Test major 7th
        let cmaj7 = parse_chord_symbol("Cmaj7");
        assert_eq!(cmaj7, vec![48, 52, 55, 59]); // C3, E3, G3, B3

        // Test with accidentals
        let f_sharp_major = parse_chord_symbol("F#");
        assert_eq!(f_sharp_major, vec![54, 58, 61]); // F#3, A#3, C#4

        let b_flat_minor = parse_chord_symbol("Bbm");
        assert_eq!(b_flat_minor, vec![58, 61, 65]); // Bb3, Db4, F4
    }

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
        assert!((data.notes[1].duration - 0.6666666666666666).abs() < 0.0001);

        assert_eq!(data.notes[2].midi_note, 64); // E
        assert!((data.notes[2].start_time - 1.6666666666666667).abs() < 0.0001);

        assert_eq!(data.notes[3].midi_note, 65); // F
        assert!((data.notes[3].start_time - 2.3333333333333335).abs() < 0.0001);

        // G at beat 3
        assert_eq!(data.notes[4].midi_note, 67);
        assert_eq!(data.notes[4].start_time, 3.0);
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

        // C4 shifted up 1 octave = C5 = MIDI 72 (display MIDI used directly)
        assert_eq!(data.notes[0].midi_note, 60);  // Concert pitch unchanged
        assert_eq!(data.notes[0].display_midi_note, 72);  // Display shifted up
        assert_eq!(data.notes[0].osmd_match_key, "72_0.000");

        // D4 shifted up 1 octave = D5 = MIDI 74
        assert_eq!(data.notes[1].display_midi_note, 74);
        assert_eq!(data.notes[1].osmd_match_key, "74_1.000");
    }
