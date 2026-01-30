//! Playback data generation engine
//!
//! Converts a Gen score into MIDI playback data with precise timing for both
//! audio playback and visual note highlighting.

use crate::ast::*;
use crate::error::GenError;
use crate::parser::parse;
use super::chord_parser::parse_chord_symbol;
use super::types::{PlaybackData, PlaybackNote, PlaybackChord, SwingType};

/// Build an expanded sequence of measure indices that respects repeats and volta endings.
///
/// This function handles:
/// - Simple repeats: ||: ... :|| plays the section twice
/// - Volta endings: 1. and 2. endings for first/second time through
/// - Nested structure: repeat sections with different endings
///
/// Returns a Vec of (original_measure_index, osmd_measure_index) pairs.
/// - original_measure_index: index into score.measures for getting the notes
/// - osmd_measure_index: index for OSMD visual matching (always linear 0, 1, 2...)
fn build_playback_sequence(measures: &[Measure]) -> Vec<(usize, usize)> {
    let mut sequence = Vec::new();
    let mut i = 0;
    let mut osmd_idx = 0;

    while i < measures.len() {
        // Check if this measure starts a repeat section
        if measures[i].repeat_start {
            // Find the matching repeat end and endings
            let repeat_start_idx = i;
            let mut repeat_end_idx = i;
            let mut first_ending_start: Option<usize> = None;
            let mut second_ending_start: Option<usize> = None;

            // Scan forward to find repeat end and endings (within or after the repeat)
            let mut j = i;
            while j < measures.len() {
                if let Some(Ending::First) = measures[j].ending {
                    if first_ending_start.is_none() {
                        first_ending_start = Some(j);
                    }
                }
                if let Some(Ending::Second) = measures[j].ending {
                    if second_ending_start.is_none() {
                        second_ending_start = Some(j);
                    }
                }
                if measures[j].repeat_end {
                    repeat_end_idx = j;
                    // If we have a second ending, keep scanning for it past the repeat_end
                    if second_ending_start.is_none() {
                        // Check if there's a second ending right after
                        if j + 1 < measures.len() {
                            if let Some(Ending::Second) = measures[j + 1].ending {
                                second_ending_start = Some(j + 1);
                            }
                        }
                    }
                    break;
                }
                j += 1;
            }

            // If we didn't find a repeat end, treat as no repeat
            if repeat_end_idx == i && !measures[i].repeat_end {
                sequence.push((i, osmd_idx));
                osmd_idx += 1;
                i += 1;
                continue;
            }

            // Determine the end of main section (before any volta endings)
            let main_section_end = first_ending_start.unwrap_or(repeat_end_idx + 1);

            // First pass through the repeated section
            if first_ending_start.is_some() {
                // Play main section (up to first ending)
                for k in repeat_start_idx..main_section_end {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
                // Play first ending (up to repeat_end inclusive)
                for k in main_section_end..=repeat_end_idx {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
            } else {
                // No volta endings - just play through
                for k in repeat_start_idx..=repeat_end_idx {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
            }

            // Second pass through the repeated section
            if let Some(second_start) = second_ending_start {
                // Play main section again
                for k in repeat_start_idx..main_section_end {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
                // Play second ending (one measure typically)
                // Find where second ending ends (until next repeat_start, repeat_end, or end of score)
                let mut second_ending_end = second_start;
                for k in (second_start + 1)..measures.len() {
                    if measures[k].repeat_start || measures[k].ending.is_some() {
                        break;
                    }
                    second_ending_end = k;
                }
                for k in second_start..=second_ending_end {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
                i = second_ending_end + 1;
            } else if first_ending_start.is_some() {
                // Has first ending but no second - still repeat main section and skip first ending
                for k in repeat_start_idx..main_section_end {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
                i = repeat_end_idx + 1;
            } else {
                // No volta endings - just repeat the section
                for k in repeat_start_idx..=repeat_end_idx {
                    sequence.push((k, osmd_idx));
                    osmd_idx += 1;
                }
                i = repeat_end_idx + 1;
            }
        } else if measures[i].repeat_end && !measures[i].repeat_start {
            // Repeat end without a start - find implicit start (beginning or after previous repeat)
            // For now, treat as start of score for simple cases
            sequence.push((i, osmd_idx));
            osmd_idx += 1;

            // Replay from beginning to here
            for k in 0..=i {
                sequence.push((k, osmd_idx));
                osmd_idx += 1;
            }
            i += 1;
        } else {
            // Regular measure - just add it
            sequence.push((i, osmd_idx));
            osmd_idx += 1;
            i += 1;
        }
    }

    sequence
}

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
    let mut notes = Vec::new();
    let mut chords = Vec::new();
    let mut current_key = score.metadata.key_signature.clone();
    let mut pending_tie: Option<(usize, f64)> = None; // (note index, accumulated duration)
    let mut note_index = 0usize;

    // Calculate conversion factor from time-signature beats to quarter-note beats for OSMD matching
    // For 12/8 (beat_type=8): eighth note = 1 TS beat = 0.5 quarter notes, so multiply by 0.5
    // For 4/4 (beat_type=4): quarter note = 1 TS beat = 1.0 quarter note, so multiply by 1.0
    let osmd_to_quarter_multiplier = 4.0 / score.metadata.time_signature.beat_type as f64;

    // Pre-calculate OSMD timestamps for each measure (linear, ignoring repeats)
    // OSMD renders the sheet music linearly, so we need to use the original timestamps
    // when we repeat back to an earlier measure for highlighting to match.
    let mut measure_osmd_times: Vec<f64> = Vec::with_capacity(score.measures.len());
    let mut osmd_time = 0.0;
    for measure in &score.measures {
        measure_osmd_times.push(osmd_time);
        for element in &measure.elements {
            let osmd_duration = match element {
                Element::Note(note) => {
                    let base = note.duration.as_beats(&score.metadata.time_signature);
                    let with_dot = if note.dotted { base * 1.5 } else { base };
                    if let Some(tuplet) = &note.tuplet {
                        let divisions = 4.0;
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
            osmd_time += osmd_duration;
        }
    }

    // Build expanded sequence that respects repeats
    let playback_sequence = build_playback_sequence(&score.measures);

    for (measure_idx, _osmd_measure_idx) in &playback_sequence {
        let measure = &score.measures[*measure_idx];
        let measure_number = measure_idx + 1; // 1-indexed (original measure number)
        let measure_start_time = current_time;

        // Get OSMD time for this measure from pre-calculated values
        // This ensures repeated measures use their original OSMD timestamps for highlighting
        let measure_osmd_start = measure_osmd_times[*measure_idx];
        let mut element_osmd_offset = 0.0;

        // Check for key changes
        if let Some(new_key) = &measure.key_change {
            current_key = new_key.clone();
        }

        for element in &measure.elements {
            let duration = element.total_beats(&score.metadata.time_signature);

            // Calculate OSMD duration for this element (for tracking offset within measure)
            let osmd_duration = match element {
                Element::Note(note) => {
                    let base = note.duration.as_beats(&score.metadata.time_signature);
                    let with_dot = if note.dotted { base * 1.5 } else { base };
                    if let Some(tuplet) = &note.tuplet {
                        let divisions = 4.0;
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

            // Calculate the OSMD timestamp for this element using pre-calculated measure start
            let element_osmd_time = measure_osmd_start + element_osmd_offset;

            match element {
                Element::Note(note) => {
                    // Handle chord symbol if present - uses its own duration (independent from melody)
                    if let Some(chord_ann) = &note.chord {
                        let chord_notes = parse_chord_symbol(&chord_ann.symbol);
                        if !chord_notes.is_empty() {
                            // Use chord's own duration (defaults to whole note)
                            let chord_duration = chord_ann.duration_beats(&score.metadata.time_signature);
                            let osmd_quarter_time = element_osmd_time * osmd_to_quarter_multiplier;
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
                        let osmd_quarter_time = element_osmd_time * osmd_to_quarter_multiplier;
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
                        let osmd_quarter_time = element_osmd_time * osmd_to_quarter_multiplier;
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
                            let osmd_quarter_time = element_osmd_time * osmd_to_quarter_multiplier;
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

            current_time += duration;            // Playback time (triplet-adjusted)
            element_osmd_offset += osmd_duration; // Track position within measure for OSMD
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
    // Note: Swing timing is applied by the frontend during playback, not here.
    // This allows the frontend to apply swing in real-time without affecting
    // the note scheduling or visual synchronization.
    let swing = score.metadata.swing.map(|s| match s {
        crate::ast::Swing::Eighth => SwingType::Eighth,
        crate::ast::Swing::Sixteenth => SwingType::Sixteenth,
    });

    Ok(PlaybackData {
        tempo: quarter_note_bpm,
        notes,
        chords,
        swing,
    })
}
