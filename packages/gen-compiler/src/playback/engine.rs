//! Playback data generation engine
//!
//! Converts a Gen score into MIDI playback data with precise timing for both
//! audio playback and visual note highlighting.

use crate::ast::*;
use crate::error::GenError;
use crate::parser::parse;
use super::chord_parser::parse_chord_symbol;
use super::types::{PlaybackData, PlaybackNote, PlaybackChord, SwingType};

/// Generate playback data from a Gen source string
///
/// Returns timing and MIDI note information for audio playback and visual highlighting.
///
/// # Parameters
/// - `source`: Gen source code string
/// - `clef`: "treble" or "bass" - affects display MIDI notes
/// - `octave_shift`: Shift playback pitch by N octaves (-2 to +2 typical)
/// - `instrument_group`: Reserved for future mod point support (currently unused)
///
/// # Returns
/// `PlaybackData` containing:
/// - MIDI notes with timing (for audio playback)
/// - Display MIDI notes (for visual note matching with OSMD)
/// - Chord accompaniment
/// - Tempo in BPM
///
/// # Timing System
/// This function maintains **two separate timing tracks**:
///
/// ## 1. Playback Time (current_time)
/// - Used for actual audio playback
/// - Correctly calculates triplet durations
/// - Example: Quarter note triplet = 0.667 beats per note
///
/// ## 2. OSMD Time (osmd_time)
/// - Used for visual note matching with OpenSheetMusicDisplay
/// - Uses MusicXML quantized durations
/// - Example: Quarter note triplet = 0.5 beats per note (MusicXML quantization)
///
/// This dual-timing system ensures:
/// - Audio plays back at correct speed (using playback time)
/// - Visual highlighting matches the rendered sheet music (using OSMD time)
///
/// # MIDI Note System
/// Each `PlaybackNote` contains two MIDI values:
///
/// ## Concert Pitch (midi_note)
/// - Used for audio playback
/// - Unaffected by clef setting
/// - Example: Treble C4 = 60, Bass C4 = 60 (same pitch)
///
/// ## Display MIDI (display_midi_note)
/// - Used for matching visual notes on the staff
/// - Includes clef offset
/// - Example: Treble C4 = 60, Bass C4 = 36 (shows 2 octaves lower on staff)
///
/// # Tie Handling
/// Tied notes are handled specially:
/// - Tie start: Creates first note with full duration accumulation
/// - Tie middle: Extends first note's duration (no new note created for audio)
/// - Tie end: Finalizes the total duration
/// - Only the first note in a tied group produces audio
///
/// # Example
/// ```rust
/// use gen::playback::generate_playback_data;
///
/// let source = "C D E F";
/// let data = generate_playback_data(source, "treble", 0, None, None).unwrap();
///
/// assert_eq!(data.notes.len(), 4);
/// assert_eq!(data.tempo, 120); // Default tempo
/// assert_eq!(data.notes[0].midi_note, 60); // C4
/// ```
pub fn generate_playback_data(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
    transpose_key: Option<&str>,
) -> Result<PlaybackData, GenError> {
    let score = parse(source)?;

    // Calculate clef offset for display MIDI note calculation
    // Bass clef displays 2 octaves lower than treble
    let clef_offset = match clef {
        "bass" => -2,  // Bass clef is 2 octaves lower
        _ => 0,        // Treble clef is the base
    };

    // Calculate transposition offset for display MIDI (in semitones)
    // For transposing instruments, the written pitch is transposed UP from concert pitch
    // E.g., Eb instrument: concert C (60) appears as D (62) on the page, so chromatic = +2 semitones
    let transposition_chromatic: i8 = if let Some(key) = transpose_key {
        crate::musicxml::Transposition::for_key(key)
            .map(|t| t.chromatic)
            .unwrap_or(0)
    } else {
        0
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

    // Calculate conversion factor from time-signature beats to quarter-note beats for OSMD matching
    // For 12/8 (beat_type=8): eighth note = 1 TS beat = 0.5 quarter notes, so multiply by 0.5
    // For 4/4 (beat_type=4): quarter note = 1 TS beat = 1.0 quarter note, so multiply by 1.0
    let osmd_to_quarter_multiplier = 4.0 / score.metadata.time_signature.beat_type as f64;

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
                    // Handle chord symbol if present - uses its own duration (independent from melody)
                    if let Some(chord_ann) = &note.chord {
                        let chord_notes = parse_chord_symbol(&chord_ann.symbol);
                        if !chord_notes.is_empty() {
                            // Use chord's own duration (defaults to whole note)
                            let chord_duration = chord_ann.duration_beats(&score.metadata.time_signature);
                            let osmd_quarter_time = osmd_time * osmd_to_quarter_multiplier;
                            chords.push(PlaybackChord {
                                midi_notes: chord_notes,
                                start_time: current_time,
                                duration: chord_duration,
                                osmd_timestamp: osmd_quarter_time,
                            });
                        }
                    }

                    if note.tie_start && !note.tie_stop {
                        // Start of a tied group - create note and track it
                        let note_idx = notes.len();
                        let beat_in_measure = current_time - measure_start_time;
                        let display_midi_base = note.to_midi_note(&current_key, total_offset);
                        let display_midi = (display_midi_base as i16 + transposition_chromatic as i16).clamp(0, 127) as u8;
                        let osmd_quarter_time = osmd_time * osmd_to_quarter_multiplier;
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, octave_shift), // Playback pitch (with octave shift, no clef offset)
                            display_midi_note: display_midi, // Display pitch (with full offset + transposition)
                            start_time: current_time,
                            duration,
                            note_index,
                            measure_number,
                            beat_in_measure,
                            osmd_timestamp: osmd_quarter_time,
                            osmd_match_key: format!("{}_{:.3}", display_midi, osmd_quarter_time),
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
                        let display_midi_base = note.to_midi_note(&current_key, total_offset);
                        let display_midi = (display_midi_base as i16 + transposition_chromatic as i16).clamp(0, 127) as u8;
                        let osmd_quarter_time = osmd_time * osmd_to_quarter_multiplier;
                        notes.push(PlaybackNote {
                            midi_note: note.to_midi_note(&current_key, octave_shift), // Playback pitch (with octave shift, no clef offset)
                            display_midi_note: display_midi, // Display pitch (with full offset + transposition)
                            start_time: current_time,
                            duration,
                            note_index,
                            measure_number,
                            beat_in_measure,
                            osmd_timestamp: osmd_quarter_time,
                            osmd_match_key: format!("{}_{:.3}", display_midi, osmd_quarter_time),
                        });
                        note_index += 1;
                        pending_tie = None;
                    }
                }
                Element::Rest { chord, .. } => {
                    // Handle chord symbol on rest if present - uses its own duration
                    if let Some(chord_ann) = chord {
                        let chord_notes = parse_chord_symbol(&chord_ann.symbol);
                        if !chord_notes.is_empty() {
                            // Use chord's own duration (defaults to whole note)
                            let chord_duration = chord_ann.duration_beats(&score.metadata.time_signature);
                            let osmd_quarter_time = osmd_time * osmd_to_quarter_multiplier;
                            chords.push(PlaybackChord {
                                midi_notes: chord_notes,
                                start_time: current_time,
                                duration: chord_duration,
                                osmd_timestamp: osmd_quarter_time,
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

    // Get tempo and calculate beat conversion
    // If tempo specifies a rhythm (e.g., "*88" = dotted quarter), use that as the beat unit
    // Otherwise default to quarter note
    let (tempo_bpm, tempo_beat_duration) = if let Some(ref tempo) = score.metadata.tempo {
        let beat_duration = tempo.duration.as_beats(&score.metadata.time_signature);
        let with_dot = if tempo.dotted { beat_duration * 1.5 } else { beat_duration };
        (tempo.bpm, with_dot)
    } else {
        // Default: 120 quarter-note BPM
        let quarter_duration = crate::ast::Duration::Quarter.as_beats(&score.metadata.time_signature);
        (120, quarter_duration)
    };

    // Convert all startTime and duration from time-signature beats to tempo's beat unit
    // For example: tempo "*88" in 12/8 has tempo_beat_duration = 3 (dotted quarter = 3 eighths)
    // So we divide all times by 3 to convert from eighth-note beats to dotted-quarter beats
    // Note: beat_in_measure is NOT converted - it stays in time-signature beats for reference
    for note in &mut notes {
        note.start_time /= tempo_beat_duration;
        note.duration /= tempo_beat_duration;
    }

    for chord in &mut chords {
        chord.start_time /= tempo_beat_duration;
        chord.duration /= tempo_beat_duration;
    }

    // Return quarter-note equivalent BPM for a unified playback API
    let quarter_note_bpm = if let Some(ref tempo) = score.metadata.tempo {
        tempo.to_quarter_note_bpm() as u16
    } else {
        120 // Default quarter-note BPM
    };

    // Convert swing metadata to playback swing type
    let swing = score.metadata.swing.map(|s| match s {
        crate::ast::Swing::Eighth => SwingType::Eighth,
        crate::ast::Swing::Sixteenth => SwingType::Sixteenth,
    });

    // Apply swing timing adjustments to notes
    if let Some(ref swing_type) = swing {
        apply_swing(&mut notes, swing_type, &score.metadata.time_signature);
    }

    Ok(PlaybackData {
        tempo: quarter_note_bpm,
        notes,
        chords,
        swing,
    })
}

/// Apply swing timing to notes
///
/// Swing adjusts the timing of pairs of notes at a specific subdivision.
/// Standard jazz swing uses a 2:1 ratio (triplet-based):
/// - First note of pair: gets 2/3 of the combined duration
/// - Second note of pair: gets 1/3 of the combined duration, starts later
///
/// For eighth note swing in 4/4:
/// - Beat positions 0, 1, 2, 3 are "on the beat" (unchanged)
/// - Beat positions 0.5, 1.5, 2.5, 3.5 are "off the beat" (shifted later)
///
/// The off-beat note is delayed from 50% to 67% of the beat.
fn apply_swing(notes: &mut Vec<PlaybackNote>, swing_type: &SwingType, time_sig: &TimeSignature) {
    // Determine the swing unit size in beats
    // For eighth swing: 0.5 beats (in 4/4), for sixteenth swing: 0.25 beats
    let swing_unit = match swing_type {
        SwingType::Eighth => crate::ast::Duration::Eighth.as_beats(time_sig),
        SwingType::Sixteenth => crate::ast::Duration::Sixteenth.as_beats(time_sig),
    };

    // Swing ratio: 2:1 means the off-beat is at 2/3 instead of 1/2 of the beat pair
    // A beat pair = 2 * swing_unit (e.g., 1.0 beats for eighth swing in 4/4)
    // Off-beat moves from 0.5 to 0.667 of the beat pair
    // Shift = (2/3 - 1/2) * beat_pair = 1/6 * 2 * swing_unit = swing_unit / 3
    let swing_shift = swing_unit * 2.0 * (2.0 / 3.0 - 0.5); // ~0.167 for eighths in 4/4

    for note in notes.iter_mut() {
        // Check if this note falls on an "off-beat" position
        // Off-beats are at odd multiples of the swing unit (1, 3, 5... units from start)
        // In 4/4 with eighth swing: 0.5, 1.5, 2.5, 3.5 beats = 1, 3, 5, 7 eighth-note units
        let position_in_units = note.start_time / swing_unit;
        let rounded_units = position_in_units.round();

        // Check if this position aligns with a swing unit boundary (within tolerance)
        if (position_in_units - rounded_units).abs() < 0.01 {
            // Check if it's an odd multiple (off-beat)
            let unit_index = rounded_units as i64;
            if unit_index % 2 == 1 {
                // This is an off-beat note - delay it
                note.start_time += swing_shift;
            }
        }
    }
}
