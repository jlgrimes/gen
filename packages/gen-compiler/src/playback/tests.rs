use super::*;
use crate::{compile, parse};

#[test]
fn test_playback_basic_timing() {
    let source = r#"---
tempo: 120
---
C D E F
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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
C D E F G A B ^C
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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
C- C $p
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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
Cp C/ C/ C
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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
    let result = generate_playback_data(source, "treble", 0, None, None);
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
    let result = generate_playback_data(source, "treble", 0, None, None);
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
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Should default to 120 BPM
    assert_eq!(data.tempo, 120);
}

#[test]
fn test_tempo_quarter_note() {
    let source = r#"---
tempo: 160
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Quarter note at 160 BPM
    assert_eq!(data.tempo, 160);
}

#[test]
fn test_tempo_half_note() {
    let source = r#"---
tempo: 160p
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Half note at 160 BPM = quarter note at 320 BPM
    assert_eq!(data.tempo, 320);
}

#[test]
fn test_tempo_whole_note() {
    let source = r#"---
tempo: 60o
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Whole note at 60 BPM = quarter note at 240 BPM
    assert_eq!(data.tempo, 240);
}

#[test]
fn test_tempo_eighth_note() {
    let source = r#"---
tempo: 120/
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Eighth note at 120 BPM = quarter note at 60 BPM
    assert_eq!(data.tempo, 60);
}

#[test]
fn test_tempo_sixteenth_note() {
    let source = r#"---
tempo: 240//
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Sixteenth note at 240 BPM = quarter note at 60 BPM
    assert_eq!(data.tempo, 60);
}

#[test]
fn test_tempo_dotted_quarter() {
    let source = r#"---
tempo: "120*"
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Dotted quarter at 120 BPM = quarter note at 180 BPM
    // (dotted quarter = 1.5 quarter notes, so 120 * 1.5 = 180)
    assert_eq!(data.tempo, 180);
}

#[test]
fn test_tempo_dotted_half() {
    let source = r#"---
tempo: "160p*"
---
C D E F"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Dotted half note at 160 BPM = quarter note at 480 BPM
    // (dotted half = 3 quarter notes, so 160 * 3 = 480)
    assert_eq!(data.tempo, 480);
}

#[test]
fn test_playback_bass_clef() {
    let source = "C D E";
    let result = generate_playback_data(source, "bass", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // With no octave shift, playback should be at base pitch: C4=60, D4=62, E4=64
    assert_eq!(data.notes[0].midi_note, 60);
    assert_eq!(data.notes[1].midi_note, 62);
    assert_eq!(data.notes[2].midi_note, 64);
    // Display MIDI should include bass clef offset (-2 octaves): C2=36, D2=38, E2=40
    assert_eq!(data.notes[0].display_midi_note, 36);
    assert_eq!(data.notes[1].display_midi_note, 38);
    assert_eq!(data.notes[2].display_midi_note, 40);
}

#[test]
fn test_playback_octave_shift() {
    let source = "C D E";
    let result = generate_playback_data(source, "treble", 1, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // C5=72, D5=74, E5=76 (one octave up) - playback should reflect the shift
    assert_eq!(data.notes[0].midi_note, 72);
    assert_eq!(data.notes[1].midi_note, 74);
    assert_eq!(data.notes[2].midi_note, 76);
    // Display MIDI should match playback MIDI when there's no clef offset
    assert_eq!(data.notes[0].display_midi_note, 72);
    assert_eq!(data.notes[1].display_midi_note, 74);
    assert_eq!(data.notes[2].display_midi_note, 76);
}

#[test]
fn test_playback_key_signature() {
    let source = r#"---
key-signature: G
---
F G A
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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
    let result = generate_playback_data(source, "treble", 0, None, None);
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
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Should have 4 melody notes and 4 chords
    assert_eq!(data.notes.len(), 4);
    assert_eq!(data.chords.len(), 4);

    // Verify first chord (C major: C3, E3, G3)
    assert_eq!(data.chords[0].midi_notes, vec![48, 52, 55]);
    assert_eq!(data.chords[0].start_time, 0.0);
    assert_eq!(data.chords[0].duration, 4.0); // Whole note (default)

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
fn test_playback_chord_with_duration() {
    // Test chord with explicit half note duration
    let source = r#"---
tempo: 120
---
@ch:Cp C D E F
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    assert_eq!(data.chords.len(), 1);
    assert_eq!(data.chords[0].midi_notes, vec![48, 52, 55]); // C major
    assert_eq!(data.chords[0].duration, 2.0); // Half note duration
}

#[test]
fn test_playback_chord_on_rest() {
    let source = r#"---
tempo: 120
---
@ch:C $ C C C
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
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

#[test]
fn test_playback_triplets() {
    let source = r#"---
tempo: 120
---
C [D E F]3/ G
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // C, D, E, F (triplet), G
    assert_eq!(data.notes.len(), 5);

    println!("\n=== Triplet Playback Data ===");
    for (i, note) in data.notes.iter().enumerate() {
        println!(
            "Note {}: MIDI {} at beat {:.4}, duration {:.4}",
            i, note.midi_note, note.start_time, note.duration
        );
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
    let result = generate_playback_data(source, "treble", 0, None, None);
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
C [D E F]3 G
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    println!("\n=== OSMD Match Keys for Triplets ===");
    for (i, note) in data.notes.iter().enumerate() {
        println!(
            "Note {}: playback={:.3}, osmd={:.3}, key=\"{}\"",
            i, note.start_time, note.osmd_timestamp, note.osmd_match_key
        );
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
    let result = generate_playback_data(source, "treble", 1, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // C4 shifted up 1 octave = C5 = MIDI 72 (both playback and display)
    assert_eq!(data.notes[0].midi_note, 72); // Playback includes octave shift
    assert_eq!(data.notes[0].display_midi_note, 72); // Display shifted up
    assert_eq!(data.notes[0].osmd_match_key, "72_0.000");

    // D4 shifted up 1 octave = D5 = MIDI 74
    assert_eq!(data.notes[1].midi_note, 74);
    assert_eq!(data.notes[1].display_midi_note, 74);
    assert_eq!(data.notes[1].osmd_match_key, "74_1.000");
}

#[test]
fn test_swing_eighth_notes() {
    // Test swing metadata is correctly parsed and passed to PlaybackData
    // NOTE: Swing timing adjustment is now handled by the frontend during playback
    let source = r#"---
swing: /
---
[C D E F]/
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Swing type should be set
    assert_eq!(data.swing, Some(crate::playback::types::SwingType::Eighth));

    // Note timings should be straight (swing applied by frontend)
    assert!((data.notes[0].start_time - 0.0).abs() < 0.01);
    assert!((data.notes[1].start_time - 0.5).abs() < 0.01);
    assert!((data.notes[2].start_time - 1.0).abs() < 0.01);
    assert!((data.notes[3].start_time - 1.5).abs() < 0.01);
}

#[test]
fn test_swing_sixteenth_notes() {
    // Test sixteenth note swing metadata
    let source = r#"---
swing: //
---
[C D E F]//
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Swing type should be set to sixteenth
    assert_eq!(data.swing, Some(crate::playback::types::SwingType::Sixteenth));

    // Note timings should be straight (swing applied by frontend)
    assert!((data.notes[0].start_time - 0.0).abs() < 0.01);
    assert!((data.notes[1].start_time - 0.25).abs() < 0.01);
    assert!((data.notes[2].start_time - 0.5).abs() < 0.01);
    assert!((data.notes[3].start_time - 0.75).abs() < 0.01);
}

#[test]
fn test_no_swing_by_default() {
    // Verify no swing is set when not specified in metadata
    let source = r#"---
tempo: 120
---
[C D E F]/
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // No swing should be set
    assert_eq!(data.swing, None);

    // Notes should be at straight timing
    assert!((data.notes[0].start_time - 0.0).abs() < 0.01);
    assert!((data.notes[1].start_time - 0.5).abs() < 0.01);
    assert!((data.notes[2].start_time - 1.0).abs() < 0.01);
    assert!((data.notes[3].start_time - 1.5).abs() < 0.01);
}

#[test]
fn test_swing_with_complex_pattern() {
    // Test that swing is correctly set with complex patterns
    // (swing timing applied by frontend, not here)
    let source = r#"---
title: Test Pattern
time-signature: 4/4
swing: /
---
Cp [B F]/
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Verify swing is set
    assert_eq!(data.swing, Some(crate::playback::types::SwingType::Eighth));

    // Verify notes are at straight timing (swing applied by frontend)
    assert_eq!(data.notes.len(), 3);
    assert!((data.notes[0].start_time - 0.0).abs() < 0.01); // C half
    assert!((data.notes[1].start_time - 2.0).abs() < 0.01); // B eighth
    assert!((data.notes[2].start_time - 2.5).abs() < 0.01); // F eighth
}

// ==================== REPEAT TESTS ====================

#[test]
fn test_playback_simple_repeat() {
    // ||: C D :|| should play C D C D
    let source = r#"---
tempo: 120
time-signature: 4/4
---
||: C D E F :||
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // 4 notes * 2 repetitions = 8 notes
    assert_eq!(data.notes.len(), 8, "Expected 8 notes (repeat), got {}", data.notes.len());

    // First pass: C D E F at beats 0, 1, 2, 3
    assert_eq!(data.notes[0].midi_note, 60); // C
    assert_eq!(data.notes[0].start_time, 0.0);
    assert_eq!(data.notes[1].midi_note, 62); // D
    assert_eq!(data.notes[1].start_time, 1.0);
    assert_eq!(data.notes[2].midi_note, 64); // E
    assert_eq!(data.notes[2].start_time, 2.0);
    assert_eq!(data.notes[3].midi_note, 65); // F
    assert_eq!(data.notes[3].start_time, 3.0);

    // Second pass: C D E F at beats 4, 5, 6, 7
    assert_eq!(data.notes[4].midi_note, 60); // C
    assert_eq!(data.notes[4].start_time, 4.0);
    assert_eq!(data.notes[5].midi_note, 62); // D
    assert_eq!(data.notes[5].start_time, 5.0);
    assert_eq!(data.notes[6].midi_note, 64); // E
    assert_eq!(data.notes[6].start_time, 6.0);
    assert_eq!(data.notes[7].midi_note, 65); // F
    assert_eq!(data.notes[7].start_time, 7.0);
}

#[test]
fn test_playback_repeat_multiple_measures() {
    // ||: C D
    //     E F :||
    // Should play both measures twice
    let source = r#"---
tempo: 120
time-signature: 4/4
---
||: C D E F
G A B ^C :||
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // 8 notes * 2 repetitions = 16 notes
    assert_eq!(data.notes.len(), 16, "Expected 16 notes, got {}", data.notes.len());

    // Verify the sequence plays twice
    // First measure first time: C D E F at 0, 1, 2, 3
    assert_eq!(data.notes[0].midi_note, 60); // C
    assert_eq!(data.notes[3].midi_note, 65); // F

    // Second measure first time: G A B ^C at 4, 5, 6, 7
    assert_eq!(data.notes[4].midi_note, 67); // G
    assert_eq!(data.notes[7].midi_note, 72); // ^C

    // First measure second time: C D E F at 8, 9, 10, 11
    assert_eq!(data.notes[8].midi_note, 60); // C
    assert_eq!(data.notes[8].start_time, 8.0);

    // Second measure second time: G A B ^C at 12, 13, 14, 15
    assert_eq!(data.notes[12].midi_note, 67); // G
    assert_eq!(data.notes[15].midi_note, 72); // ^C
}

#[test]
fn test_playback_with_intro_and_repeat() {
    // Intro, then repeated section
    let source = r#"---
tempo: 120
time-signature: 4/4
---
A B C D
||: E F G A :||
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // 4 intro notes + 4 repeated notes * 2 = 12 notes
    assert_eq!(data.notes.len(), 12, "Expected 12 notes, got {}", data.notes.len());

    // Intro: A B C D at beats 0-3
    assert_eq!(data.notes[0].midi_note, 69); // A
    assert_eq!(data.notes[3].midi_note, 62); // D

    // First pass of repeat: E F G A at beats 4-7
    assert_eq!(data.notes[4].midi_note, 64); // E
    assert_eq!(data.notes[4].start_time, 4.0);

    // Second pass of repeat: E F G A at beats 8-11
    assert_eq!(data.notes[8].midi_note, 64); // E
    assert_eq!(data.notes[8].start_time, 8.0);
}

#[test]
fn test_playback_volta_endings() {
    // ||: C D 1. E F :|| 2. G A ||
    // Should play: C D E F, C D G A
    let source = r#"---
tempo: 120
time-signature: 4/4
---
||: C D E F
1. G A B ^C :||
2. ^D ^E ^F ^G
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    println!("\n=== Volta Endings Playback ===");
    for (i, note) in data.notes.iter().enumerate() {
        println!("Note {}: MIDI {} at beat {:.1}", i, note.midi_note, note.start_time);
    }

    // First pass: C D E F (main) + G A B ^C (1st ending) = 8 notes
    // Second pass: C D E F (main) + ^D ^E ^F ^G (2nd ending) = 8 notes
    // Total: 16 notes
    assert_eq!(data.notes.len(), 16, "Expected 16 notes with volta, got {}", data.notes.len());

    // First time through: main section + 1st ending
    // C D E F at 0, 1, 2, 3
    assert_eq!(data.notes[0].midi_note, 60); // C
    assert_eq!(data.notes[3].midi_note, 65); // F

    // 1st ending: G A B ^C at 4, 5, 6, 7
    assert_eq!(data.notes[4].midi_note, 67); // G
    assert_eq!(data.notes[7].midi_note, 72); // ^C

    // Second time through: main section + 2nd ending
    // C D E F at 8, 9, 10, 11
    assert_eq!(data.notes[8].midi_note, 60); // C
    assert_eq!(data.notes[8].start_time, 8.0);

    // 2nd ending: ^D ^E ^F ^G at 12, 13, 14, 15
    assert_eq!(data.notes[12].midi_note, 74); // ^D
    assert_eq!(data.notes[15].midi_note, 79); // ^G
}

#[test]
fn test_playback_no_repeat() {
    // No repeat markers - should play through once
    let source = r#"---
tempo: 120
---
C D E F
"#;
    let result = generate_playback_data(source, "treble", 0, None, None);
    assert!(result.is_ok());
    let data = result.unwrap();

    // Just 4 notes, no repetition
    assert_eq!(data.notes.len(), 4);
    assert_eq!(data.notes[0].midi_note, 60);
    assert_eq!(data.notes[3].midi_note, 65);
}
