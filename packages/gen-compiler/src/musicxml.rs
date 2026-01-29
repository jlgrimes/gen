//! # MusicXML Generator Module
//!
//! This module generates MusicXML output from the Gen AST.
//!
//! ## Purpose
//! The MusicXML generator is the final stage of the compilation pipeline.
//! It takes the validated AST and produces industry-standard MusicXML 3.1
//! output that can be rendered by notation software.
//!
//! ## Supported Features
//!
//! ### Basic Music Notation
//! - Notes and rests with all durations (whole, half, quarter, eighth, sixteenth, 32nd)
//! - Dotted rhythms
//! - Tuplets (triplets, quintuplets, sextuplets, etc.)
//! - Ties and slurs
//! - Accidentals (sharp, flat, natural)
//! - Octave modifiers
//!
//! ### Score Structure
//! - Metadata (title, composer, tempo)
//! - Key signatures (all major and minor keys)
//! - Time signatures (simple and compound meters)
//! - Repeat markers and endings
//! - Mid-score key changes
//!
//! ### Advanced Features
//! - **Instrument Transposition**: Automatic transposition for Bb, Eb, F instruments
//! - **Clef Support**: Treble and bass clefs with automatic octave adjustment
//! - **Mod Points**: Instrument-specific octave shifts per line
//! - **Chord Symbols**: Lead sheet chord notation
//! - **Automatic Beaming**: Intelligent beam grouping based on time signature
//!
//! ## Entry Points
//!
//! ### Basic Compilation
//! ```rust
//! use gen::{parse, to_musicxml};
//!
//! let score = parse("C D E F")?;
//! let musicxml = to_musicxml(&score);
//! // Write to .musicxml file or render with notation software
//! ```
//!
//! ### With Transposition
//! ```rust
//! use gen::{parse, to_musicxml_with_options, Clef, Transposition};
//!
//! let score = parse("C D E F")?;
//! let transposition = Transposition::for_key("Bb"); // Bb instrument
//! let musicxml = to_musicxml_with_options(&score, transposition, Clef::Treble, 0);
//! ```
//!
//! ### With Mod Points
//! ```rust
//! use gen::{parse, to_musicxml_with_mod_points, Clef};
//!
//! let score = parse("@Eb:^ C D E F")?;  // Eb instrument, up one octave
//! let musicxml = to_musicxml_with_mod_points(&score, None, Clef::Treble, 0, Some("eb"));
//! ```
//!
//! ## MusicXML Compatibility
//! Generates MusicXML 3.1 compatible output tested with:
//! - MuseScore 3/4
//! - Finale
//! - Sibelius (via MusicXML import)
//! - OpenSheetMusicDisplay (OSMD) for web rendering
//!
//! ## Beaming Algorithm
//! Automatically groups eighth and sixteenth notes into beams based on:
//! - Time signature beat structure
//! - Note durations
//! - Measure position
//!
//! ## Transposition System
//! - Concert pitch (C) instruments: No transposition
//! - Bb instruments: Written major 2nd higher (transposition up 2 semitones)
//! - Eb instruments: Written major 6th higher (transposition up 9 semitones)
//! - F instruments: Written perfect 4th higher (transposition up 5 semitones)
//!
//! ## Related Modules
//! - `ast` - Defines Score and all music types
//! - `parser` - Creates AST from Gen source
//! - `semantic` - Validates AST before generation

use crate::ast::*;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

/// Clef type for the score
///
/// Determines which staff lines are used for note placement.
/// Treble clef is standard for most instruments; bass clef is for lower-pitched instruments.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Clef {
    #[default]
    Treble,
    Bass,
}

/// Transposition info for MusicXML
#[derive(Clone, Copy, Default)]
pub struct Transposition {
    pub diatonic: i8,   // Number of diatonic steps (letter names) for note transposition
    pub chromatic: i8,  // Number of chromatic half steps for note transposition
    pub fifths: i8,     // Position change on circle of fifths for key signature transposition
}

impl Transposition {
    /// Create transposition for a given viewed key
    /// Returns None for concert pitch (C), Some for transposing instruments
    pub fn for_key(viewed_key: &str) -> Option<Self> {
        match viewed_key.trim() {
            "C" => None, // Concert pitch, no transposition needed
            "Bb" => Some(Transposition { diatonic: 1, chromatic: 2, fifths: 2 }),   // Up a major 2nd
            "Eb" => Some(Transposition { diatonic: 5, chromatic: 9, fifths: 3 }),   // Up a major 6th
            "F" => Some(Transposition { diatonic: 3, chromatic: 5, fifths: -1 }),    // Up a perfect 4th
            _ => None,
        }
    }
}

/// Convert a Score to MusicXML format
pub fn to_musicxml(score: &Score) -> String {
    to_musicxml_full(score, None, Clef::Treble, 0, None)
}

/// Convert a Score to MusicXML format with optional transposition
pub fn to_musicxml_transposed(score: &Score, transposition: Option<Transposition>) -> String {
    to_musicxml_full(score, transposition, Clef::Treble, 0, None)
}

/// Convert a Score to MusicXML format with clef and octave shift options
pub fn to_musicxml_with_options(score: &Score, transposition: Option<Transposition>, clef: Clef, octave_shift: i8) -> String {
    to_musicxml_full(score, transposition, clef, octave_shift, None)
}

/// Convert a Score to MusicXML format with mod points support
/// When an instrument_group is specified, per-line octave shifts from mod_points are applied
pub fn to_musicxml_with_mod_points(
    score: &Score,
    transposition: Option<Transposition>,
    clef: Clef,
    octave_shift: i8,
    instrument_group: Option<InstrumentGroup>,
) -> String {
    to_musicxml_full(score, transposition, clef, octave_shift, instrument_group)
}

/// Convert a Score to MusicXML format with all options
fn to_musicxml_full(
    score: &Score,
    transposition: Option<Transposition>,
    clef: Clef,
    octave_shift: i8,
    instrument_group: Option<InstrumentGroup>,
) -> String {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    // XML declaration
    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .unwrap();

    // DOCTYPE - write as raw text since quick-xml doesn't have direct doctype support
    writer
        .get_mut()
        .get_mut()
        .extend_from_slice(b"<!DOCTYPE score-partwise PUBLIC \"-//Recordare//DTD MusicXML 4.0 Partwise//EN\" \"http://www.musicxml.org/dtds/partwise.dtd\">");

    // Root element
    let mut root = BytesStart::new("score-partwise");
    root.push_attribute(("version", "4.0"));
    writer.write_event(Event::Start(root)).unwrap();

    // Work info (title)
    if let Some(title) = &score.metadata.title {
        writer
            .write_event(Event::Start(BytesStart::new("work")))
            .unwrap();
        write_text_element(&mut writer, "work-title", title);
        writer
            .write_event(Event::End(BytesEnd::new("work")))
            .unwrap();
    }

    // Identification (composer)
    if let Some(composer) = &score.metadata.composer {
        writer
            .write_event(Event::Start(BytesStart::new("identification")))
            .unwrap();
        let mut creator = BytesStart::new("creator");
        creator.push_attribute(("type", "composer"));
        writer.write_event(Event::Start(creator)).unwrap();
        writer
            .write_event(Event::Text(BytesText::new(composer)))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("creator")))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("identification")))
            .unwrap();
    }

    // Part list
    writer
        .write_event(Event::Start(BytesStart::new("part-list")))
        .unwrap();
    let mut score_part = BytesStart::new("score-part");
    score_part.push_attribute(("id", "P1"));
    writer.write_event(Event::Start(score_part)).unwrap();
    let mut part_name = BytesStart::new("part-name");
    part_name.push_attribute(("print-object", "no"));
    writer.write_event(Event::Start(part_name)).unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("part-name")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("score-part")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("part-list")))
        .unwrap();

    // Part with measures
    let mut part = BytesStart::new("part");
    part.push_attribute(("id", "P1"));
    writer.write_event(Event::Start(part)).unwrap();

    // Track current key signature as it changes through the score
    let mut current_key_signature = score.metadata.key_signature.clone();

    for (i, measure) in score.measures.iter().enumerate() {
        // Update key signature if this measure has a key change
        if let Some(ref new_key) = measure.key_change {
            current_key_signature = new_key.clone();
        }

        // Calculate effective octave shift for this measure
        // If we have an instrument group, check if there's a mod point for this measure's source line
        let effective_octave_shift = if let Some(group) = instrument_group {
            // Find the source line for this measure
            let source_line = score.line_to_measure
                .iter()
                .find(|(_, &measure_idx)| measure_idx == i)
                .map(|(&line, _)| line);

            if let Some(line) = source_line {
                // Check for mod point on this line
                if let Some(mod_shift) = score.mod_points.get_shift(line, group) {
                    octave_shift + mod_shift
                } else {
                    octave_shift
                }
            } else {
                octave_shift
            }
        } else {
            octave_shift
        };

        // Determine if this is the first measure with the current ending
        // (we need to open the ending bracket if the previous measure had a different ending or no ending)
        let is_ending_start = if measure.ending.is_some() {
            if i > 0 {
                // Not the first measure - check if previous measure had different ending
                score.measures[i - 1].ending != measure.ending
            } else {
                // First measure in score - start the ending
                true
            }
        } else {
            false
        };

        // Determine if this is the last measure with the current ending
        // (we need to close the ending bracket if the next measure has a different ending or no ending)
        let is_ending_stop = if let Some(current_ending) = measure.ending {
            if i + 1 < score.measures.len() {
                // Not the last measure - check if next measure has different ending
                score.measures[i + 1].ending != Some(current_ending)
            } else {
                // Last measure in score - close the ending only if this measure has an ending
                true
            }
        } else {
            false
        };

        write_measure(
            &mut writer,
            measure,
            i + 1,
            &score.metadata.time_signature,
            &current_key_signature,
            i == 0,
            transposition,
            clef,
            effective_octave_shift,
            is_ending_start,
            is_ending_stop,
            score.metadata.tempo.as_ref(),
        );
    }

    writer
        .write_event(Event::End(BytesEnd::new("part")))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("score-partwise")))
        .unwrap();

    let result = writer.into_inner().into_inner();
    String::from_utf8(result).unwrap()
}

/// Helper to write a simple text element
fn write_text_element<W: std::io::Write>(writer: &mut Writer<W>, name: &str, text: &str) {
    writer
        .write_event(Event::Start(BytesStart::new(name)))
        .unwrap();
    writer
        .write_event(Event::Text(BytesText::new(text)))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new(name)))
        .unwrap();
}

/// Beam state for a note
#[derive(Clone, Copy, PartialEq)]
enum BeamState {
    None,
    Begin,
    Continue,
    End,
}

/// Check if a duration is beamable (eighth note or shorter)
fn is_beamable(duration: Duration) -> bool {
    matches!(
        duration,
        Duration::Eighth | Duration::Sixteenth | Duration::ThirtySecond
    )
}

/// Get the duration of an element in divisions (12 per quarter note for internal beam calculations)
/// Using 12 divisions allows clean representation of triplets (divisible by 3)
fn element_divisions_for_beaming(element: &Element) -> u32 {
    match element {
        Element::Note(note) => {
            duration_to_divisions_high_res(note.duration, note.dotted, note.tuplet)
        }
        Element::Rest {
            duration,
            dotted,
            tuplet,
            ..
        } => duration_to_divisions_high_res(*duration, *dotted, *tuplet),
    }
}

/// Calculate beam states for all elements in a measure, respecting beat boundaries
fn calculate_beam_states(elements: &[Element], time_signature: &TimeSignature) -> Vec<BeamState> {
    let mut states = vec![BeamState::None; elements.len()];

    // Calculate beat size in high-res divisions (12 divisions = 1 quarter note)
    // This allows clean representation of triplets (12 is divisible by 3)
    // For 4/4: beat = 12 divisions (quarter note)
    // For 6/8, 9/8, 12/8: beat = 18 divisions (dotted quarter note, 3 eighth notes)
    // For 2/4: beat = 12 divisions (quarter note)
    let beat_divisions = if time_signature.beat_type == 8 && time_signature.beats % 3 == 0 {
        // Compound meter (6/8, 9/8, 12/8): beam in groups of 3 eighth notes (dotted quarter)
        // 3 eighth notes = 18 divisions (each eighth = 6 divisions)
        18
    } else {
        // Simple meter: use beat_type to determine beat size
        48 / time_signature.beat_type as u32
    };

    // Track position in measure and group notes by beat
    let mut position: u32 = 0;
    let mut i = 0;

    while i < elements.len() {
        // Find which beat we're currently in
        let current_beat = position / beat_divisions;
        let beat_start_pos = current_beat * beat_divisions;
        let beat_end_pos = beat_start_pos + beat_divisions;

        // Check if current element is a beamable note
        let is_current_beamable = match &elements[i] {
            Element::Note(note) => is_beamable(note.duration),
            Element::Rest { .. } => false,
        };

        if is_current_beamable {
            // Find consecutive beamable notes within the same beat
            let start = i;
            let mut group_position = position;

            while i < elements.len() && group_position < beat_end_pos {
                match &elements[i] {
                    Element::Note(note) if is_beamable(note.duration) => {
                        let note_divs = element_divisions_for_beaming(&elements[i]);
                        // Check if adding this note would cross the beat boundary
                        if group_position + note_divs > beat_end_pos && i > start {
                            // Don't include this note, it would cross the beat
                            break;
                        }
                        group_position += note_divs;
                        i += 1;
                    }
                    _ => break,
                }
            }
            let end = i;

            // Only beam if we have 2 or more consecutive beamable notes
            if end - start >= 2 {
                states[start] = BeamState::Begin;
                for j in (start + 1)..(end - 1) {
                    states[j] = BeamState::Continue;
                }
                states[end - 1] = BeamState::End;
            }

            // Update position to where we ended up
            position = group_position;
        } else {
            position += element_divisions_for_beaming(&elements[i]);
            i += 1;
        }
    }

    states
}

fn write_tempo_direction<W: std::io::Write>(
    writer: &mut Writer<W>,
    tempo: &crate::ast::Tempo,
    time_signature: &TimeSignature,
) {
    writer
        .write_event(Event::Start(BytesStart::new("direction")))
        .unwrap();

    writer
        .write_event(Event::Start(BytesStart::new("direction-type")))
        .unwrap();

    // Metronome marking
    writer
        .write_event(Event::Start(BytesStart::new("metronome")))
        .unwrap();

    // Determine the beat unit for the metronome
    let beat_unit = tempo.duration.musicxml_type();
    write_text_element(writer, "beat-unit", beat_unit);

    // Add dot if tempo is dotted
    if tempo.dotted {
        writer
            .write_event(Event::Empty(BytesStart::new("beat-unit-dot")))
            .unwrap();
    }

    write_text_element(writer, "per-minute", &tempo.bpm.to_string());

    writer
        .write_event(Event::End(BytesEnd::new("metronome")))
        .unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("direction-type")))
        .unwrap();

    // Sound element with quarter-note tempo for MIDI playback
    let quarter_bpm = tempo.to_quarter_note_bpm();
    let mut sound_elem = BytesStart::new("sound");
    sound_elem.push_attribute(("tempo", quarter_bpm.to_string().as_str()));
    writer.write_event(Event::Empty(sound_elem)).unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("direction")))
        .unwrap();
}

fn write_measure<W: std::io::Write>(
    writer: &mut Writer<W>,
    measure: &Measure,
    number: usize,
    time_signature: &TimeSignature,
    key_signature: &KeySignature,
    include_attributes: bool,
    transposition: Option<Transposition>,
    clef: Clef,
    octave_shift: i8,
    is_ending_start: bool,
    is_ending_stop: bool,
    tempo: Option<&crate::ast::Tempo>,
) {
    let mut measure_elem = BytesStart::new("measure");
    measure_elem.push_attribute(("number", number.to_string().as_str()));
    writer.write_event(Event::Start(measure_elem)).unwrap();

    // Write left barline (repeat start and/or ending start)
    // Only write ending start if this is the first measure with this ending
    if measure.repeat_start || is_ending_start {
        write_left_barline(writer, measure.repeat_start, if is_ending_start { measure.ending } else { None });
    }

    // Write attributes for key changes in mid-score
    if measure.key_change.is_some() && !include_attributes {
        writer
            .write_event(Event::Start(BytesStart::new("attributes")))
            .unwrap();

        // Transpose key signature if transposition is specified
        let transposed_fifths = if let Some(trans) = transposition {
            let new_fifths = key_signature.fifths + trans.fifths;
            // Wrap around: keep in range -7 to +7 (valid key signatures)
            if new_fifths > 7 {
                new_fifths - 12
            } else if new_fifths < -7 {
                new_fifths + 12
            } else {
                new_fifths
            }
        } else {
            key_signature.fifths
        };

        writer
            .write_event(Event::Start(BytesStart::new("key")))
            .unwrap();
        write_text_element(writer, "fifths", &transposed_fifths.to_string());
        let mode_str = match key_signature.mode {
            crate::ast::Mode::Major => "major",
            crate::ast::Mode::Minor => "minor",
        };
        write_text_element(writer, "mode", mode_str);
        writer
            .write_event(Event::End(BytesEnd::new("key")))
            .unwrap();

        writer
            .write_event(Event::End(BytesEnd::new("attributes")))
            .unwrap();
    }

    // Include time signature, key signature, and clef on first measure
    if include_attributes {
        writer
            .write_event(Event::Start(BytesStart::new("attributes")))
            .unwrap();

        write_text_element(writer, "divisions", "4");

        // Transpose key signature if transposition is specified
        let transposed_fifths = if let Some(trans) = transposition {
            let new_fifths = key_signature.fifths + trans.fifths;
            // Wrap around: keep in range -7 to +7 (valid key signatures)
            if new_fifths > 7 {
                new_fifths - 12
            } else if new_fifths < -7 {
                new_fifths + 12
            } else {
                new_fifths
            }
        } else {
            key_signature.fifths
        };

        writer
            .write_event(Event::Start(BytesStart::new("key")))
            .unwrap();
        write_text_element(writer, "fifths", &transposed_fifths.to_string());
        let mode_str = match key_signature.mode {
            crate::ast::Mode::Major => "major",
            crate::ast::Mode::Minor => "minor",
        };
        write_text_element(writer, "mode", mode_str);
        writer
            .write_event(Event::End(BytesEnd::new("key")))
            .unwrap();

        writer
            .write_event(Event::Start(BytesStart::new("time")))
            .unwrap();
        write_text_element(writer, "beats", &time_signature.beats.to_string());
        write_text_element(writer, "beat-type", &time_signature.beat_type.to_string());
        writer
            .write_event(Event::End(BytesEnd::new("time")))
            .unwrap();

        writer
            .write_event(Event::Start(BytesStart::new("clef")))
            .unwrap();
        match clef {
            Clef::Treble => {
                write_text_element(writer, "sign", "G");
                write_text_element(writer, "line", "2");
            }
            Clef::Bass => {
                write_text_element(writer, "sign", "F");
                write_text_element(writer, "line", "4");
            }
        }
        writer
            .write_event(Event::End(BytesEnd::new("clef")))
            .unwrap();

        // Transposition for transposing instruments
        if let Some(trans) = transposition {
            writer
                .write_event(Event::Start(BytesStart::new("transpose")))
                .unwrap();
            write_text_element(writer, "diatonic", &trans.diatonic.to_string());
            write_text_element(writer, "chromatic", &trans.chromatic.to_string());
            writer
                .write_event(Event::End(BytesEnd::new("transpose")))
                .unwrap();
        }

        writer
            .write_event(Event::End(BytesEnd::new("attributes")))
            .unwrap();

        // Add tempo marking on first measure
        if let Some(tempo_info) = tempo {
            write_tempo_direction(writer, tempo_info, time_signature);
        }
    }

    // Calculate beam states for all elements
    let beam_states = calculate_beam_states(&measure.elements, time_signature);

    for (element, beam_state) in measure.elements.iter().zip(beam_states.iter()) {
        write_element(writer, element, *beam_state, octave_shift, key_signature, transposition.as_ref());
    }

    // Write right barline (repeat end and/or ending stop)
    // Only write ending stop if this is the last measure with this ending
    if measure.repeat_end || is_ending_stop {
        write_right_barline(writer, measure.repeat_end, if is_ending_stop { measure.ending } else { None });
    }

    writer
        .write_event(Event::End(BytesEnd::new("measure")))
        .unwrap();
}

/// Write a left barline element with optional repeat and ending
fn write_left_barline<W: std::io::Write>(
    writer: &mut Writer<W>,
    repeat_start: bool,
    ending: Option<Ending>,
) {
    let mut barline = BytesStart::new("barline");
    barline.push_attribute(("location", "left"));
    writer.write_event(Event::Start(barline)).unwrap();

    // Bar style: heavy-light for repeat start, otherwise regular
    if repeat_start {
        write_text_element(writer, "bar-style", "heavy-light");
    }

    // Ending element for volta brackets
    if let Some(ending_type) = ending {
        let mut ending_elem = BytesStart::new("ending");
        let number = match ending_type {
            Ending::First => "1",
            Ending::Second => "2",
        };
        ending_elem.push_attribute(("number", number));
        ending_elem.push_attribute(("type", "start"));
        writer.write_event(Event::Start(ending_elem)).unwrap();
        // Text content for the ending bracket
        let text = match ending_type {
            Ending::First => "1.",
            Ending::Second => "2.",
        };
        writer.write_event(Event::Text(BytesText::new(text))).unwrap();
        writer.write_event(Event::End(BytesEnd::new("ending"))).unwrap();
    }

    // Repeat direction
    if repeat_start {
        let mut repeat = BytesStart::new("repeat");
        repeat.push_attribute(("direction", "forward"));
        writer.write_event(Event::Empty(repeat)).unwrap();
    }

    writer
        .write_event(Event::End(BytesEnd::new("barline")))
        .unwrap();
}

/// Write a right barline element with optional repeat and ending
fn write_right_barline<W: std::io::Write>(
    writer: &mut Writer<W>,
    repeat_end: bool,
    ending: Option<Ending>,
) {
    let mut barline = BytesStart::new("barline");
    barline.push_attribute(("location", "right"));
    writer.write_event(Event::Start(barline)).unwrap();

    // Bar style: light-heavy for repeat end, otherwise regular
    if repeat_end {
        write_text_element(writer, "bar-style", "light-heavy");
    }

    // Ending element - for first ending, type is "stop" at the right barline
    // For second ending, we also use "stop" to close it
    if let Some(ending_type) = ending {
        let mut ending_elem = BytesStart::new("ending");
        let number = match ending_type {
            Ending::First => "1",
            Ending::Second => "2",
        };
        ending_elem.push_attribute(("number", number));
        // First ending with repeat uses "stop", second ending uses "stop" too
        ending_elem.push_attribute(("type", "stop"));
        writer.write_event(Event::Empty(ending_elem)).unwrap();
    }

    // Repeat direction
    if repeat_end {
        let mut repeat = BytesStart::new("repeat");
        repeat.push_attribute(("direction", "backward"));
        writer.write_event(Event::Empty(repeat)).unwrap();
    }

    writer
        .write_event(Event::End(BytesEnd::new("barline")))
        .unwrap();
}

fn write_element<W: std::io::Write>(writer: &mut Writer<W>, element: &Element, beam_state: BeamState, octave_shift: i8, key_signature: &KeySignature, transposition: Option<&Transposition>) {
    match element {
        Element::Note(note) => write_note(writer, note, beam_state, octave_shift, key_signature, transposition),
        Element::Rest {
            duration,
            dotted,
            tuplet,
            chord,
        } => {
            // Write harmony before rest if chord symbol exists
            if let Some(ref chord_ann) = chord {
                write_harmony(writer, &chord_ann.symbol, transposition);
            }
            write_rest(writer, *duration, *dotted, *tuplet);
        }
    }
}

/// Transpose a note's pitch based on diatonic and chromatic intervals
/// Returns (new_step, new_alter, octave_adjustment)
fn transpose_pitch(note_name: NoteName, accidental: Accidental, diatonic: i8, chromatic: i8) -> (NoteName, i8, i8) {
    // Map note names to their position in the scale (C=0, D=1, E=2, F=3, G=4, A=5, B=6)
    let note_to_index = |n: NoteName| match n {
        NoteName::C => 0,
        NoteName::D => 1,
        NoteName::E => 2,
        NoteName::F => 3,
        NoteName::G => 4,
        NoteName::A => 5,
        NoteName::B => 6,
    };

    let index_to_note = |i: i8| match i.rem_euclid(7) {
        0 => NoteName::C,
        1 => NoteName::D,
        2 => NoteName::E,
        3 => NoteName::F,
        4 => NoteName::G,
        5 => NoteName::A,
        6 => NoteName::B,
        _ => unreachable!(),
    };

    // Get the current pitch in semitones (C=0, C#=1, D=2, etc.)
    let note_to_semitone = |n: NoteName| match n {
        NoteName::C => 0,
        NoteName::D => 2,
        NoteName::E => 4,
        NoteName::F => 5,
        NoteName::G => 7,
        NoteName::A => 9,
        NoteName::B => 11,
    };

    let alter_value = match accidental {
        Accidental::Sharp => 1,
        Accidental::Flat => -1,
        Accidental::Natural | Accidental::ForceNatural => 0,
    };

    let current_semitone = note_to_semitone(note_name) + alter_value;

    // Apply transposition
    let new_note_index = note_to_index(note_name) + diatonic;
    let new_semitone = current_semitone + chromatic;

    // Calculate octave change from diatonic steps (wrapping C-D-E-F-G-A-B)
    let octave_adjustment = new_note_index.div_euclid(7);

    // Get the new note name
    let new_note = index_to_note(new_note_index);

    // Calculate what alteration is needed
    let expected_semitone = note_to_semitone(new_note);
    let new_alter = new_semitone.rem_euclid(12) - expected_semitone;

    (new_note, new_alter, octave_adjustment)
}

fn write_note<W: std::io::Write>(writer: &mut Writer<W>, note: &Note, beam_state: BeamState, octave_shift: i8, key_signature: &KeySignature, transposition: Option<&Transposition>) {
    // Write harmony BEFORE note element if chord symbol exists
    if let Some(ref chord_ann) = note.chord {
        write_harmony(writer, &chord_ann.symbol, transposition);
    }

    writer
        .write_event(Event::Start(BytesStart::new("note")))
        .unwrap();

    // Determine the effective accidental: if no explicit accidental, apply key signature
    // ForceNatural (%) explicitly cancels key signature accidentals
    let effective_accidental = match note.accidental {
        Accidental::Natural => key_signature.accidental_for_note(note.name),
        Accidental::ForceNatural => Accidental::Natural, // Force natural pitch (no alter)
        other => other,
    };

    // Apply transposition if specified
    let (final_note_name, final_alter, transpose_octave_adj) = if let Some(trans) = transposition {
        transpose_pitch(note.name, effective_accidental, trans.diatonic, trans.chromatic)
    } else {
        let alter_value = match effective_accidental {
            Accidental::Sharp => 1,
            Accidental::Flat => -1,
            Accidental::Natural | Accidental::ForceNatural => 0,
        };
        (note.name, alter_value, 0)
    };

    // Pitch
    writer
        .write_event(Event::Start(BytesStart::new("pitch")))
        .unwrap();
    write_text_element(writer, "step", note_name_to_str(final_note_name));

    // Alter for sharps/flats
    if final_alter != 0 {
        write_text_element(writer, "alter", &final_alter.to_string());
    }

    // Octave (middle C = octave 4, adjusted by octave_shift and transposition)
    let base_octave: i8 = match note.octave {
        Octave::DoubleLow => 2,
        Octave::Low => 3,
        Octave::Middle => 4,
        Octave::High => 5,
        Octave::DoubleHigh => 6,
    };
    let octave = (base_octave + octave_shift + transpose_octave_adj).max(0).min(9);
    write_text_element(writer, "octave", &octave.to_string());
    writer
        .write_event(Event::End(BytesEnd::new("pitch")))
        .unwrap();

    // Duration (in divisions - 4 per quarter note)
    let divisions = duration_to_divisions_with_tuplet(note.duration, note.dotted, note.tuplet);
    write_text_element(writer, "duration", &divisions.to_string());

    // Ties (for playback - must come before <type>)
    if note.tie_start {
        let mut tie = BytesStart::new("tie");
        tie.push_attribute(("type", "start"));
        writer.write_event(Event::Empty(tie)).unwrap();
    }
    if note.tie_stop {
        let mut tie = BytesStart::new("tie");
        tie.push_attribute(("type", "stop"));
        writer.write_event(Event::Empty(tie)).unwrap();
    }

    // Type
    write_text_element(writer, "type", note.duration.musicxml_type());

    // Dot if dotted
    if note.dotted {
        writer
            .write_event(Event::Empty(BytesStart::new("dot")))
            .unwrap();
    }

    // Time modification for tuplets
    if let Some(tuplet_info) = note.tuplet {
        writer
            .write_event(Event::Start(BytesStart::new("time-modification")))
            .unwrap();
        write_text_element(writer, "actual-notes", &tuplet_info.actual_notes.to_string());
        write_text_element(writer, "normal-notes", &tuplet_info.normal_notes.to_string());
        writer
            .write_event(Event::End(BytesEnd::new("time-modification")))
            .unwrap();
    }

    // Beam (for eighth notes and shorter)
    match beam_state {
        BeamState::Begin => write_beam(writer, "begin"),
        BeamState::Continue => write_beam(writer, "continue"),
        BeamState::End => write_beam(writer, "end"),
        BeamState::None => {}
    }

    // Notations (tuplet markers, ties, slurs, and accidentals display)
    let has_tuplet_notation = note
        .tuplet
        .map(|t| t.is_start || t.is_stop)
        .unwrap_or(false);
    let has_tie_notation = note.tie_start || note.tie_stop;
    let has_slur_notation = note.slur_start || note.slur_stop;
    if has_tuplet_notation || has_tie_notation || has_slur_notation {
        writer
            .write_event(Event::Start(BytesStart::new("notations")))
            .unwrap();

        // Tied notations (for visual display)
        if note.tie_start {
            let mut tied = BytesStart::new("tied");
            tied.push_attribute(("type", "start"));
            writer.write_event(Event::Empty(tied)).unwrap();
        }
        if note.tie_stop {
            let mut tied = BytesStart::new("tied");
            tied.push_attribute(("type", "stop"));
            writer.write_event(Event::Empty(tied)).unwrap();
        }

        // Slur notations
        if note.slur_start {
            let mut slur = BytesStart::new("slur");
            slur.push_attribute(("type", "start"));
            slur.push_attribute(("number", "1"));
            writer.write_event(Event::Empty(slur)).unwrap();
        }
        if note.slur_stop {
            let mut slur = BytesStart::new("slur");
            slur.push_attribute(("type", "stop"));
            slur.push_attribute(("number", "1"));
            writer.write_event(Event::Empty(slur)).unwrap();
        }

        // Tuplet notations
        if let Some(tuplet_info) = note.tuplet {
            if tuplet_info.is_start {
                let mut tuplet = BytesStart::new("tuplet");
                tuplet.push_attribute(("type", "start"));
                tuplet.push_attribute(("bracket", "yes"));
                writer.write_event(Event::Empty(tuplet)).unwrap();
            }
            if tuplet_info.is_stop {
                let mut tuplet = BytesStart::new("tuplet");
                tuplet.push_attribute(("type", "stop"));
                writer.write_event(Event::Empty(tuplet)).unwrap();
            }
        }

        writer
            .write_event(Event::End(BytesEnd::new("notations")))
            .unwrap();
    }

    // Accidental display
    match note.accidental {
        Accidental::Sharp => write_text_element(writer, "accidental", "sharp"),
        Accidental::Flat => write_text_element(writer, "accidental", "flat"),
        Accidental::ForceNatural => write_text_element(writer, "accidental", "natural"),
        Accidental::Natural => {}
    }

    writer
        .write_event(Event::End(BytesEnd::new("note")))
        .unwrap();
}

fn write_beam<W: std::io::Write>(writer: &mut Writer<W>, beam_type: &str) {
    let mut beam = BytesStart::new("beam");
    beam.push_attribute(("number", "1"));
    writer.write_event(Event::Start(beam)).unwrap();
    writer
        .write_event(Event::Text(BytesText::new(beam_type)))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("beam")))
        .unwrap();
}

fn write_rest<W: std::io::Write>(
    writer: &mut Writer<W>,
    duration: Duration,
    dotted: bool,
    tuplet: Option<TupletInfo>,
) {
    writer
        .write_event(Event::Start(BytesStart::new("note")))
        .unwrap();

    writer
        .write_event(Event::Empty(BytesStart::new("rest")))
        .unwrap();

    let divisions = duration_to_divisions_with_tuplet(duration, dotted, tuplet);
    write_text_element(writer, "duration", &divisions.to_string());
    write_text_element(writer, "type", duration.musicxml_type());

    if dotted {
        writer
            .write_event(Event::Empty(BytesStart::new("dot")))
            .unwrap();
    }

    // Time modification for tuplets
    if let Some(tuplet_info) = tuplet {
        writer
            .write_event(Event::Start(BytesStart::new("time-modification")))
            .unwrap();
        write_text_element(writer, "actual-notes", &tuplet_info.actual_notes.to_string());
        write_text_element(writer, "normal-notes", &tuplet_info.normal_notes.to_string());
        writer
            .write_event(Event::End(BytesEnd::new("time-modification")))
            .unwrap();
    }

    // Notations (tuplet markers)
    let has_tuplet_notation = tuplet.map(|t| t.is_start || t.is_stop).unwrap_or(false);
    if has_tuplet_notation {
        writer
            .write_event(Event::Start(BytesStart::new("notations")))
            .unwrap();
        if let Some(tuplet_info) = tuplet {
            if tuplet_info.is_start {
                let mut tuplet_elem = BytesStart::new("tuplet");
                tuplet_elem.push_attribute(("type", "start"));
                tuplet_elem.push_attribute(("bracket", "yes"));
                writer.write_event(Event::Empty(tuplet_elem)).unwrap();
            }
            if tuplet_info.is_stop {
                let mut tuplet_elem = BytesStart::new("tuplet");
                tuplet_elem.push_attribute(("type", "stop"));
                writer.write_event(Event::Empty(tuplet_elem)).unwrap();
            }
        }
        writer
            .write_event(Event::End(BytesEnd::new("notations")))
            .unwrap();
    }

    writer
        .write_event(Event::End(BytesEnd::new("note")))
        .unwrap();
}

/// Transpose a chord root note by the given transposition interval
/// Only transposes the root note letter, preserves quality (maj7, m7, etc.)
fn transpose_chord_root(chord_symbol: &str, transposition: &Transposition) -> String {
    if chord_symbol.is_empty() {
        return chord_symbol.to_string();
    }

    // Extract root note (first character)
    let root_char = chord_symbol.chars().next().unwrap();

    // Check if second character is an accidental
    let mut quality_start = 1;
    let has_accidental = if chord_symbol.len() > 1 {
        match chord_symbol.chars().nth(1) {
            Some('#') | Some('b') => {
                quality_start = 2;
                true
            }
            _ => false
        }
    } else {
        false
    };

    // Get the quality/extension part (everything after root + accidental)
    let quality = if quality_start < chord_symbol.len() {
        &chord_symbol[quality_start..]
    } else {
        ""
    };

    // Map note letter to number (C=0, D=1, E=2, F=3, G=4, A=5, B=6)
    let note_to_num = |c: char| -> i8 {
        match c {
            'C' => 0, 'D' => 1, 'E' => 2, 'F' => 3, 'G' => 4, 'A' => 5, 'B' => 6,
            _ => 0,
        }
    };

    let num_to_note = |n: i8| -> char {
        match n {
            0 => 'C', 1 => 'D', 2 => 'E', 3 => 'F', 4 => 'G', 5 => 'A', 6 => 'B',
            _ => 'C',
        }
    };

    // Chromatic values for each note (C=0, C#=1, D=2, etc.)
    let note_chromatic = |c: char| -> i8 {
        match c {
            'C' => 0, 'D' => 2, 'E' => 4, 'F' => 5, 'G' => 7, 'A' => 9, 'B' => 11,
            _ => 0,
        }
    };

    // Get starting chromatic value with accidental
    let mut chromatic = note_chromatic(root_char);
    if has_accidental {
        match chord_symbol.chars().nth(1) {
            Some('#') => chromatic += 1,
            Some('b') => chromatic -= 1,
            _ => {}
        }
    }

    // Apply transposition
    let new_note_num = (note_to_num(root_char) + transposition.diatonic).rem_euclid(7);
    let new_chromatic = (chromatic + transposition.chromatic).rem_euclid(12);

    let new_note = num_to_note(new_note_num);
    let new_note_base_chromatic = note_chromatic(new_note);

    // Calculate the accidental needed
    let accidental_diff = (new_chromatic - new_note_base_chromatic).rem_euclid(12);

    let accidental_str = match accidental_diff {
        0 => "",        // Natural
        1 => "#",       // Sharp
        11 => "b",      // Flat (same as -1 mod 12)
        2 => "##",      // Double sharp (rare)
        10 => "bb",     // Double flat (rare)
        _ => "",        // Should not happen with standard transpositions
    };

    format!("{}{}{}", new_note, accidental_str, quality)
}

/// Write a harmony (chord symbol) element
fn write_harmony<W: std::io::Write>(writer: &mut Writer<W>, chord_symbol: &str, transposition: Option<&Transposition>) {
    // Transpose the chord symbol if transposition is specified
    let transposed_symbol = if let Some(trans) = transposition {
        transpose_chord_root(chord_symbol, trans)
    } else {
        chord_symbol.to_string()
    };

    let chord_symbol = &transposed_symbol;

    writer
        .write_event(Event::Start(BytesStart::new("harmony")))
        .unwrap();

    // Parse root note (first character)
    let root_step = chord_symbol.chars().next().unwrap_or('C');

    writer
        .write_event(Event::Start(BytesStart::new("root")))
        .unwrap();
    write_text_element(writer, "root-step", &root_step.to_string());

    // Check for sharp/flat in second character
    if chord_symbol.len() > 1 {
        match chord_symbol.chars().nth(1) {
            Some('#') => write_text_element(writer, "root-alter", "1"),
            Some('b') => write_text_element(writer, "root-alter", "-1"),
            _ => {}
        }
    }
    writer
        .write_event(Event::End(BytesEnd::new("root")))
        .unwrap();

    // Parse chord quality from the chord symbol
    // Extract quality after root note and accidental
    let quality_start = if chord_symbol.len() > 1 {
        match chord_symbol.chars().nth(1) {
            Some('#') | Some('b') => 2,
            _ => 1,
        }
    } else {
        1
    };

    let quality = if quality_start < chord_symbol.len() {
        &chord_symbol[quality_start..]
    } else {
        ""
    };

    // Map quality string to MusicXML kind
    let kind_value = match quality {
        "" => "major",
        "m" | "min" | "-" => "minor",
        "maj7" | "M7" | "Δ7" => "major-seventh",
        "m7" | "min7" | "-7" => "minor-seventh",
        "7" => "dominant",
        "dim" | "o" | "°" => "diminished",
        "dim7" | "o7" | "°7" => "diminished-seventh",
        "m7b5" | "ø" | "half-dim" => "half-diminished",
        "aug" | "+" => "augmented",
        "sus4" | "sus" => "suspended-fourth",
        "sus2" => "suspended-second",
        "6" => "major-sixth",
        "m6" => "minor-sixth",
        "9" => "dominant-ninth",
        "maj9" | "M9" => "major-ninth",
        "m9" => "minor-ninth",
        "11" => "dominant-11th",
        "maj11" => "major-11th",
        "m11" => "minor-11th",
        "13" => "dominant-13th",
        "maj13" => "major-13th",
        "m13" => "minor-13th",
        _ => "other", // For complex/altered chords
    };

    // Kind element with text attribute for display and proper kind for parsing
    let mut kind = BytesStart::new("kind");
    kind.push_attribute(("text", chord_symbol.as_str()));
    writer.write_event(Event::Start(kind)).unwrap();
    writer
        .write_event(Event::Text(BytesText::new(kind_value)))
        .unwrap();
    writer
        .write_event(Event::End(BytesEnd::new("kind")))
        .unwrap();

    writer
        .write_event(Event::End(BytesEnd::new("harmony")))
        .unwrap();
}

fn note_name_to_str(name: NoteName) -> &'static str {
    match name {
        NoteName::C => "C",
        NoteName::D => "D",
        NoteName::E => "E",
        NoteName::F => "F",
        NoteName::G => "G",
        NoteName::A => "A",
        NoteName::B => "B",
    }
}

/// Convert duration to MusicXML divisions (4 per quarter note)
fn duration_to_divisions(duration: Duration, dotted: bool) -> u32 {
    let base = match duration {
        Duration::Whole => 16,
        Duration::Half => 8,
        Duration::Quarter => 4,
        Duration::Eighth => 2,
        Duration::Sixteenth => 1,
        Duration::ThirtySecond => 1, // MusicXML may need finer divisions for 32nd notes
    };

    if dotted {
        base + (base / 2)
    } else {
        base
    }
}

/// Convert duration to high-resolution divisions (12 per quarter note) for beam calculations
/// Using 12 divisions allows clean representation of triplets (divisible by 2, 3, 4, 6)
fn duration_to_divisions_high_res(
    duration: Duration,
    dotted: bool,
    tuplet: Option<TupletInfo>,
) -> u32 {
    let base = match duration {
        Duration::Whole => 48,
        Duration::Half => 24,
        Duration::Quarter => 12,
        Duration::Eighth => 6,
        Duration::Sixteenth => 3,
        Duration::ThirtySecond => 2, // Approximate, but fine for beaming
    };

    let with_dot = if dotted { base + (base / 2) } else { base };

    if let Some(tuplet_info) = tuplet {
        // For tuplets, actual duration = base * (normal_notes / actual_notes)
        // e.g., triplet = base * 2/3
        (with_dot * tuplet_info.normal_notes as u32) / tuplet_info.actual_notes as u32
    } else {
        with_dot
    }
}

/// Convert duration to MusicXML divisions, accounting for tuplets
fn duration_to_divisions_with_tuplet(
    duration: Duration,
    dotted: bool,
    tuplet: Option<TupletInfo>,
) -> u32 {
    let base = duration_to_divisions(duration, dotted);

    if let Some(tuplet_info) = tuplet {
        // For tuplets, actual duration = base * (normal_notes / actual_notes)
        // e.g., triplet = base * 2/3
        // We need to handle this carefully to avoid integer division issues
        (base * tuplet_info.normal_notes as u32) / tuplet_info.actual_notes as u32
    } else {
        base
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_basic_musicxml_output() {
        let score = parse("C D E F").unwrap();
        let xml = to_musicxml(&score);
        assert!(xml.contains("<score-partwise"));
        assert!(xml.contains("<step>C</step>"));
        assert!(xml.contains("<step>D</step>"));
    }

    #[test]
    fn test_musicxml_with_metadata() {
        let source = r#"---
title: Test
composer: Me
---
C"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);
        assert!(xml.contains("<work-title>Test</work-title>"));
        assert!(xml.contains("composer"));
    }

    #[test]
    fn test_musicxml_triplet_output() {
        let score = parse("3[C D E]").unwrap();
        let xml = to_musicxml(&score);

        // Should contain time-modification for triplets
        assert!(xml.contains("<time-modification>"));
        assert!(xml.contains("<actual-notes>3</actual-notes>"));
        assert!(xml.contains("<normal-notes>2</normal-notes>"));

        // Should contain tuplet notation markers
        assert!(xml.contains("<tuplet type=\"start\""));
        assert!(xml.contains("<tuplet type=\"stop\""));
    }

    #[test]
    fn test_musicxml_tie_output() {
        let score = parse("C-D").unwrap();
        let xml = to_musicxml(&score);

        // Should contain tie elements (for playback)
        assert!(xml.contains("<tie type=\"start\"/>"));
        assert!(xml.contains("<tie type=\"stop\"/>"));

        // Should contain tied elements (for display)
        assert!(xml.contains("<tied type=\"start\"/>"));
        assert!(xml.contains("<tied type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_chained_ties() {
        let score = parse("C-D-E").unwrap();
        let xml = to_musicxml(&score);

        // Count tie start/stop occurrences
        let tie_starts = xml.matches("<tie type=\"start\"/>").count();
        let tie_stops = xml.matches("<tie type=\"stop\"/>").count();

        // First note: start only, Second note: both, Third note: stop only
        assert_eq!(tie_starts, 2, "Should have 2 tie starts (C and D)");
        assert_eq!(tie_stops, 2, "Should have 2 tie stops (D and E)");
    }

    #[test]
    fn test_transposition_xml() {
        let score = parse("C D E F").unwrap();
        let trans = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_transposed(&score, Some(trans));
        assert!(xml.contains("<transpose>"));
        assert!(xml.contains("<diatonic>1</diatonic>"));
        assert!(xml.contains("<chromatic>2</chromatic>"));
    }

    #[test]
    fn test_musicxml_repeat_start() {
        let score = parse("||: C D E F").unwrap();
        let xml = to_musicxml(&score);

        // Should contain barline with repeat forward
        assert!(xml.contains("<barline location=\"left\">"));
        assert!(xml.contains("<bar-style>heavy-light</bar-style>"));
        assert!(xml.contains("<repeat direction=\"forward\"/>"));
    }

    #[test]
    fn test_musicxml_repeat_end() {
        let score = parse("C D E F :||").unwrap();
        let xml = to_musicxml(&score);

        // Should contain barline with repeat backward
        assert!(xml.contains("<barline location=\"right\">"));
        assert!(xml.contains("<bar-style>light-heavy</bar-style>"));
        assert!(xml.contains("<repeat direction=\"backward\"/>"));
    }

    #[test]
    fn test_musicxml_repeat_both() {
        let score = parse("||: C D E F :||").unwrap();
        let xml = to_musicxml(&score);

        // Should contain both repeat barlines
        assert!(xml.contains("<barline location=\"left\">"));
        assert!(xml.contains("<repeat direction=\"forward\"/>"));
        assert!(xml.contains("<barline location=\"right\">"));
        assert!(xml.contains("<repeat direction=\"backward\"/>"));
    }

    #[test]
    fn test_musicxml_slur_output() {
        let score = parse("(C D E)").unwrap();
        let xml = to_musicxml(&score);

        // Should contain slur start and stop elements
        assert!(xml.contains("<slur type=\"start\" number=\"1\"/>"));
        assert!(xml.contains("<slur type=\"stop\" number=\"1\"/>"));
    }

    #[test]
    fn test_musicxml_slur_two_notes() {
        let score = parse("(C D)").unwrap();
        let xml = to_musicxml(&score);

        // Count slur start/stop occurrences
        let slur_starts = xml.matches("<slur type=\"start\"").count();
        let slur_stops = xml.matches("<slur type=\"stop\"").count();

        assert_eq!(slur_starts, 1, "Should have 1 slur start");
        assert_eq!(slur_stops, 1, "Should have 1 slur stop");
    }

    #[test]
    fn test_musicxml_slur_with_ties() {
        // Slur containing a tie: (C-D E)
        let score = parse("(C-D E)").unwrap();
        let xml = to_musicxml(&score);

        // Should contain both slur and tie elements
        assert!(xml.contains("<slur type=\"start\""));
        assert!(xml.contains("<slur type=\"stop\""));
        assert!(xml.contains("<tie type=\"start\"/>"));
        assert!(xml.contains("<tie type=\"stop\"/>"));
        assert!(xml.contains("<tied type=\"start\"/>"));
        assert!(xml.contains("<tied type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_slur_across_measures() {
        // Slur spanning two measures
        let score = parse("(C D E F\nG A B C^)").unwrap();
        let xml = to_musicxml(&score);

        // Should contain slur start and stop
        let slur_starts = xml.matches("<slur type=\"start\"").count();
        let slur_stops = xml.matches("<slur type=\"stop\"").count();

        assert_eq!(slur_starts, 1, "Should have exactly 1 slur start");
        assert_eq!(slur_stops, 1, "Should have exactly 1 slur stop");
    }

    #[test]
    fn test_musicxml_first_ending() {
        let score = parse("1. C C C C :||").unwrap();
        let xml = to_musicxml(&score);

        // Should contain ending element with number 1
        assert!(xml.contains("<ending number=\"1\" type=\"start\">"));
        assert!(xml.contains("1."));
        assert!(xml.contains("<ending number=\"1\" type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_second_ending() {
        let score = parse("2. C C C C").unwrap();
        let xml = to_musicxml(&score);

        // Should contain ending element with number 2
        assert!(xml.contains("<ending number=\"2\" type=\"start\">"));
        assert!(xml.contains("2."));
        assert!(xml.contains("<ending number=\"2\" type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_first_and_second_endings() {
        let score = parse("||: C C C C\n1. D D D D :||\n2. E E E E").unwrap();
        let xml = to_musicxml(&score);

        // Should contain both ending types
        assert!(xml.contains("<ending number=\"1\" type=\"start\">"));
        assert!(xml.contains("<ending number=\"2\" type=\"start\">"));
    }

    #[test]
    fn test_key_signature_sharps_f() {
        // G major has F#, so an F without explicit accidental should be sharped
        let source = r#"---
key-signature: G
---
F"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // F should be altered to F# (alter = 1)
        assert!(xml.contains("<step>F</step>"));
        assert!(xml.contains("<alter>1</alter>"));
        // No accidental display element (it's part of the key signature)
        assert!(!xml.contains("<accidental>sharp</accidental>"));
    }

    #[test]
    fn test_key_signature_d_major() {
        // D major has F# and C#
        let source = r#"---
key-signature: D
---
F C G"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Count alter elements - should have 2 (for F# and C#, but not G)
        let alter_count = xml.matches("<alter>1</alter>").count();
        assert_eq!(alter_count, 2, "D major should sharp F and C");
    }

    #[test]
    fn test_key_signature_flats() {
        // F major has Bb
        let source = r#"---
key-signature: F
---
B"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // B should be altered to Bb (alter = -1)
        assert!(xml.contains("<step>B</step>"));
        assert!(xml.contains("<alter>-1</alter>"));
    }

    #[test]
    fn test_key_signature_explicit_override() {
        // G major has F#, but if user writes Fb explicitly, use that
        let source = r#"---
key-signature: G
---
Fb"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Should have Fb (alter = -1), not F#
        assert!(xml.contains("<step>F</step>"));
        assert!(xml.contains("<alter>-1</alter>"));
        // Should show the accidental since it differs from key signature
        assert!(xml.contains("<accidental>flat</accidental>"));
    }

    #[test]
    fn test_key_signature_c_major_no_alterations() {
        // C major has no sharps or flats
        let source = r#"---
key-signature: C
---
C D E F G A B"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // No alter elements should be present
        assert!(!xml.contains("<alter>"));
    }

    #[test]
    fn test_key_signature_sharp_count_notation() {
        // ### means 3 sharps (A major: F#, C#, G#)
        let source = r####"---
key-signature: "###"
---
F C G D"####;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // F, C, and G should be sharped, D should not
        let alter_count = xml.matches("<alter>1</alter>").count();
        assert_eq!(alter_count, 3, "### should sharp F, C, and G");
    }

    #[test]
    fn test_key_signature_single_sharp() {
        // # means 1 sharp (G major: F#)
        let source = "---\nkey-signature: \"#\"\n---\nF G";
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Only F should be sharped
        let alter_count = xml.matches("<alter>1</alter>").count();
        assert_eq!(alter_count, 1, "# should only sharp F");
    }

    #[test]
    fn test_key_signature_flat_count_notation() {
        // bbb means 3 flats (Eb major: B, E, A)
        let source = r#"---
key-signature: bbb
---
B E A D"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // B, E, and A should be flatted, D should not
        let alter_count = xml.matches("<alter>-1</alter>").count();
        assert_eq!(alter_count, 3, "bbb should flat B, E, and A");
    }

    #[test]
    fn test_force_natural_accidental() {
        // C% should show a natural sign explicitly
        let score = parse("C%").unwrap();
        let xml = to_musicxml(&score);

        // Should have <accidental>natural</accidental> for explicit natural
        assert!(xml.contains("<accidental>natural</accidental>"),
            "Force natural (%) should display natural accidental sign");

        // Should NOT have any <alter> element (natural = no alteration)
        assert!(!xml.contains("<alter>"),
            "Force natural should not have alter element");
    }

    #[test]
    fn test_force_natural_in_sharp_key() {
        // In G major (1 sharp), F is normally sharped
        // F% should force F to be natural
        let source = r#"---
key-signature: G
---
F F%"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // First F should be sharped (from key signature), second F should be natural
        let alter_count = xml.matches("<alter>1</alter>").count();
        assert_eq!(alter_count, 1, "Only first F should be sharped");

        // Second note should have explicit natural sign
        assert!(xml.contains("<accidental>natural</accidental>"),
            "F% should display natural accidental sign");
    }

    #[test]
    fn test_mod_points_octave_shift() {
        // Test that mod points apply per-line octave shifts
        // Line 1 has C at octave 4 (middle), with Eb:^ mod point -> octave 5
        // Line 2 has C at octave 4 (middle), no mod point -> octave 4
        let source = "C D E F @Eb:^\nC D E F";
        let score = parse(source).unwrap();

        // Without instrument group, both should be at octave 4
        let xml_no_group = to_musicxml(&score);
        let octave_4_count = xml_no_group.matches("<octave>4</octave>").count();
        assert_eq!(octave_4_count, 8, "All 8 notes should be at octave 4 without instrument group");

        // With Eb instrument group, line 1 should be at octave 5
        let xml_with_eb = to_musicxml_with_mod_points(&score, None, Clef::Treble, 0, Some(InstrumentGroup::Eb));
        let octave_5_count = xml_with_eb.matches("<octave>5</octave>").count();
        let octave_4_count = xml_with_eb.matches("<octave>4</octave>").count();
        assert_eq!(octave_5_count, 4, "First 4 notes (line 1) should be at octave 5 with Eb group");
        assert_eq!(octave_4_count, 4, "Last 4 notes (line 2) should be at octave 4 with Eb group");

        // With Bb instrument group, no mod points, all should be at octave 4
        let xml_with_bb = to_musicxml_with_mod_points(&score, None, Clef::Treble, 0, Some(InstrumentGroup::Bb));
        let octave_4_count_bb = xml_with_bb.matches("<octave>4</octave>").count();
        assert_eq!(octave_4_count_bb, 8, "All 8 notes should be at octave 4 with Bb group (no mod points)");
    }

    #[test]
    fn test_mod_points_down_octave() {
        // Test octave down modifier
        let source = "C D E F @Eb:_";
        let score = parse(source).unwrap();

        let xml = to_musicxml_with_mod_points(&score, None, Clef::Treble, 0, Some(InstrumentGroup::Eb));
        let octave_3_count = xml.matches("<octave>3</octave>").count();
        assert_eq!(octave_3_count, 4, "All 4 notes should be at octave 3 with Eb:_ mod point");
    }

    #[test]
    fn test_mod_points_combined_with_base_shift() {
        // Test that mod points combine with base octave shift
        // Base shift: +1, Mod point: +1, Result: +2 (octave 6)
        let source = "C D E F @Eb:^";
        let score = parse(source).unwrap();

        let xml = to_musicxml_with_mod_points(&score, None, Clef::Treble, 1, Some(InstrumentGroup::Eb));
        let octave_6_count = xml.matches("<octave>6</octave>").count();
        assert_eq!(octave_6_count, 4, "All 4 notes should be at octave 6 with base +1 and mod +1");
    }

    #[test]
    fn test_mod_points_with_metadata_at_bottom() {
        // Like spain.gen - metadata at bottom of file
        let source = "C D E F @Eb:^\nG A B C\n---\ntitle: Test\n---";
        let score = parse(source).unwrap();

        // With Eb instrument group, line 1 should be at octave 5, line 2 at octave 4
        let xml = to_musicxml_with_mod_points(&score, None, Clef::Treble, 0, Some(InstrumentGroup::Eb));
        let octave_5_count = xml.matches("<octave>5</octave>").count();
        let octave_4_count = xml.matches("<octave>4</octave>").count();
        assert_eq!(octave_5_count, 4, "First 4 notes (line 1) should be at octave 5 with Eb:^ mod point");
        assert_eq!(octave_4_count, 4, "Last 4 notes (line 2) should be at octave 4 (no mod point)");
    }

    #[test]
    fn test_musicxml_harmony() {
        let score = parse("@ch:Cmaj7 C").unwrap();
        let xml = to_musicxml(&score);
        assert!(xml.contains("<harmony"), "MusicXML should contain harmony element");
        assert!(xml.contains("Cmaj7"), "Chord symbol should appear in MusicXML");
        assert!(xml.contains("<root-step>C</root-step>"), "Root note should be C");
    }

    #[test]
    fn test_musicxml_multiple_harmonies() {
        let score = parse("@ch:C C @ch:G E").unwrap();
        let xml = to_musicxml(&score);
        let harmony_count = xml.matches("<harmony").count();
        assert_eq!(harmony_count, 2, "Should have 2 harmony elements");
        assert!(xml.contains("text=\"C\""), "First chord should be C");
        assert!(xml.contains("text=\"G\""), "Second chord should be G");
    }

    #[test]
    fn test_musicxml_harmony_with_sharp() {
        let score = parse("@ch:F# F").unwrap();
        let xml = to_musicxml(&score);
        assert!(xml.contains("<root-step>F</root-step>"), "Root should be F");
        assert!(xml.contains("<root-alter>1</root-alter>"), "Should have sharp alteration");
    }

    #[test]
    fn test_musicxml_harmony_with_flat() {
        let score = parse("@ch:Bb7 B").unwrap();
        let xml = to_musicxml(&score);
        assert!(xml.contains("<root-step>B</root-step>"), "Root should be B");
        assert!(xml.contains("<root-alter>-1</root-alter>"), "Should have flat alteration");
    }

    #[test]
    fn test_musicxml_no_harmony() {
        let score = parse("C D E F").unwrap();
        let xml = to_musicxml(&score);
        assert!(!xml.contains("<harmony>"), "Should not have harmony elements without chord annotations");
    }

    #[test]
    fn test_chord_transposition_bb_instrument() {
        // Bb instrument (Clarinet): transposes up a major 2nd
        // Concert C -> D, Concert G7 -> A7
        let score = parse("@ch:C C @ch:G7 G").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        assert!(xml.contains("text=\"D\""), "Concert C should transpose to D for Bb instrument");
        assert!(xml.contains("text=\"A7\""), "Concert G7 should transpose to A7 for Bb instrument");
    }

    #[test]
    fn test_chord_transposition_eb_instrument() {
        // Eb instrument (Alto Sax): transposes up a major 6th
        // Concert C -> A, Concert F -> D, Concert G7 -> E7
        let score = parse("@ch:C C @ch:F F @ch:G7 G").unwrap();
        let transposition = Transposition { diatonic: 5, chromatic: 9, fifths: 3 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        assert!(xml.contains("text=\"A\""), "Concert C should transpose to A for Eb instrument");
        assert!(xml.contains("text=\"D\""), "Concert F should transpose to D for Eb instrument");
        assert!(xml.contains("text=\"E7\""), "Concert G7 should transpose to E7 for Eb instrument");
    }

    #[test]
    fn test_chord_transposition_with_accidentals() {
        // Bb instrument: Concert Bb7 -> C7, Concert Eb -> F
        let score = parse("@ch:Bb7 B @ch:Eb E").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        assert!(xml.contains("text=\"C7\""), "Concert Bb7 should transpose to C7 for Bb instrument");
        assert!(xml.contains("text=\"F\""), "Concert Eb should transpose to F for Bb instrument");
    }

    #[test]
    fn test_chord_transposition_preserves_quality() {
        // Ensure quality (maj7, m7, dim, etc.) is preserved
        let score = parse("@ch:Cmaj7 C @ch:Dm7 D @ch:Bdim B").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 }; // Bb instrument
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        assert!(xml.contains("text=\"Dmaj7\""), "Cmaj7 should become Dmaj7");
        assert!(xml.contains("text=\"Em7\""), "Dm7 should become Em7");
        assert!(xml.contains("text=\"C#dim\""), "Bdim should become C#dim");
    }

    #[test]
    fn test_chord_no_transposition_concert_pitch() {
        // Concert pitch (no transposition)
        let score = parse("@ch:Cmaj7 C @ch:G7 G").unwrap();
        let xml = to_musicxml(&score);

        assert!(xml.contains("text=\"Cmaj7\""), "Concert pitch should not transpose");
        assert!(xml.contains("text=\"G7\""), "Concert pitch should not transpose");
    }

    #[test]
    fn test_key_signature_transposition_bb_instrument() {
        // Gb major (6 flats, fifths=-6) for Bb instrument (fifths=+2)
        // -6 + 2 = -4 (Ab major, 4 flats)
        let source = r#"---
key-signature: Gb
---
C"#;
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Check that key signature is transposed to -4 (Ab major)
        assert!(xml.contains("<fifths>-4</fifths>"), "Gb major (-6) + Bb transposition (+2) = Ab major (-4)");
    }

    #[test]
    fn test_key_signature_transposition_eb_instrument() {
        // Gb major (6 flats, fifths=-6) for Eb instrument (fifths=+3)
        // -6 + 3 = -3 (Eb major, 3 flats)
        let source = r#"---
key-signature: Gb
---
C"#;
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 5, chromatic: 9, fifths: 3 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // -6 + 3 = -3 (Eb major, 3 flats)
        assert!(xml.contains("<fifths>-3</fifths>"), "Gb major (-6) + Eb transposition (+3) = Eb major (-3)");
    }

    #[test]
    fn test_key_signature_transposition_wraparound() {
        // Test wraparound: B major (5 sharps, fifths=+5) for Eb instrument (fifths=+3)
        // +5 + +3 = +8, which should wrap to -4 (Ab major, 4 flats)
        let source = r#"---
key-signature: B
---
C"#;
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 5, chromatic: 9, fifths: 3 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // +5 + +3 = +8, wrap to -4 (8 - 12 = -4)
        assert!(xml.contains("<fifths>-4</fifths>"), "B major (+5) + Eb transposition (+3) = Ab major (-4, wrapped from +8)");
    }

    #[test]
    fn test_note_transposition_bb_instrument() {
        // Bb instrument: transposes up a major 2nd (C -> D, E -> F#, G -> A)
        let score = parse("C D E F G A B C^").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Check that notes are transposed correctly
        // Concert C -> D, Concert D -> E, Concert E -> F#, Concert F -> G, Concert G -> A, Concert A -> B, Concert B -> C#, Concert C^ -> D^
        assert!(xml.contains("<step>D</step>"), "Concert C should transpose to D");
        assert!(xml.contains("<step>E</step>"), "Concert D should transpose to E");
        assert!(xml.contains("<step>F</step>"), "Concert E should transpose to F");
        assert!(xml.contains("<alter>1</alter>"), "Concert E should have sharp (F#)");
        assert!(xml.contains("<step>G</step>"), "Concert F should transpose to G");
        assert!(xml.contains("<step>A</step>"), "Concert G should transpose to A");
        assert!(xml.contains("<step>B</step>"), "Concert A should transpose to B");
        assert!(xml.contains("<step>C</step>"), "Concert B should transpose to C");
    }

    #[test]
    fn test_note_transposition_eb_instrument() {
        // Eb instrument: transposes up a major 6th (C -> A, D -> B, E -> C#)
        let score = parse("C D E F").unwrap();
        let transposition = Transposition { diatonic: 5, chromatic: 9, fifths: 3 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Concert C -> A, Concert D -> B, Concert E -> C#, Concert F -> D
        assert!(xml.contains("<step>A</step>"), "Concert C should transpose to A");
        assert!(xml.contains("<step>B</step>"), "Concert D should transpose to B");
        assert!(xml.contains("<step>C</step>"), "Concert E should transpose to C");
        assert!(xml.contains("<alter>1</alter>"), "Concert E should have sharp (C#)");
        assert!(xml.contains("<step>D</step>"), "Concert F should transpose to D");
    }

    #[test]
    fn test_note_transposition_with_accidentals_bb() {
        // Bb instrument with accidentals: Bb -> C, C# -> D#, Eb -> F
        let score = parse("Bb C# Eb").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Concert Bb -> C (natural), Concert C# -> D#, Concert Eb -> F (natural)
        // Check for C natural (step C, no alter or alter 0)
        assert!(xml.contains("<step>C</step>"), "Concert Bb should transpose to C");
        // D# should appear
        assert!(xml.contains("<step>D</step>"), "Concert C# should transpose to D#");
        assert!(xml.contains("<alter>1</alter>"), "Concert C# should have sharp");
        // F natural
        assert!(xml.contains("<step>F</step>"), "Concert Eb should transpose to F");
    }

    #[test]
    fn test_note_transposition_octave_crossing() {
        // Test that transposition handles octave crossing correctly
        // High B for Bb instrument should become C# in next octave
        let score = parse("B C^").unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Concert B (octave 4) -> C# (octave 5)
        // Concert C^ (octave 5) -> D (octave 5)
        assert!(xml.contains("<step>C</step>"), "Concert B should transpose to C#");
        assert!(xml.contains("<step>D</step>"), "Concert C^ should transpose to D");

        // Count octaves - should have octave 5 appear
        assert!(xml.contains("<octave>5</octave>"), "Should have notes in octave 5");
    }

    #[test]
    fn test_note_transposition_f_instrument() {
        // F instrument: transposes up a perfect 4th (C -> F, D -> G, E -> A)
        let score = parse("C D E F").unwrap();
        let transposition = Transposition { diatonic: 3, chromatic: 5, fifths: -1 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Concert C -> F, Concert D -> G, Concert E -> A, Concert F -> Bb
        assert!(xml.contains("<step>F</step>"), "Concert C should transpose to F");
        assert!(xml.contains("<step>G</step>"), "Concert D should transpose to G");
        assert!(xml.contains("<step>A</step>"), "Concert E should transpose to A");
        assert!(xml.contains("<step>B</step>"), "Concert F should transpose to B");
        assert!(xml.contains("<alter>-1</alter>"), "Concert F should have flat (Bb)");
    }

    #[test]
    fn test_note_transposition_with_key_signature_gb_major_bb() {
        // Test the original issue: Gb major (6 flats) for Bb instrument
        // Key signature should be Ab major (4 flats)
        // Concert C in Gb major (which is actually Cb due to key sig) -> D in Ab major (Db)
        let source = r#"---
key-signature: Gb
---
C D E F G A B C^"#;
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Key signature should be -4 (Ab major, 4 flats)
        assert!(xml.contains("<fifths>-4</fifths>"), "Gb major should transpose to Ab major (4 flats)");

        // Notes should transpose: C->D, D->E, E->F#, F->G, G->A, A->B, B->C#, C^->D^
        assert!(xml.contains("<step>D</step>"), "Should contain D notes");
        assert!(xml.contains("<step>E</step>"), "Should contain E notes");
        assert!(xml.contains("<step>F</step>"), "Should contain F notes");
        assert!(xml.contains("<step>G</step>"), "Should contain G notes");
        assert!(xml.contains("<step>A</step>"), "Should contain A notes");
        assert!(xml.contains("<step>B</step>"), "Should contain B notes");
        assert!(xml.contains("<step>C</step>"), "Should contain C notes");
    }

    #[test]
    fn test_note_transposition_with_key_signature_gb_major_eb() {
        // Test the original issue: Gb major (6 flats) for Eb instrument
        // Key signature should be Eb major (3 flats)
        let source = r#"---
key-signature: Gb
---
C D E F"#;
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 5, chromatic: 9, fifths: 3 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // Key signature should be -3 (Eb major, 3 flats)
        assert!(xml.contains("<fifths>-3</fifths>"), "Gb major should transpose to Eb major (3 flats)");

        // Notes should transpose: C->A, D->B, E->C#, F->D
        assert!(xml.contains("<step>A</step>"), "Concert C should transpose to A");
        assert!(xml.contains("<step>B</step>"), "Concert D should transpose to B");
        assert!(xml.contains("<step>C</step>"), "Concert E should transpose to C");
        assert!(xml.contains("<step>D</step>"), "Concert F should transpose to D");
    }

    #[test]
    fn test_concert_pitch_no_transposition() {
        // Concert pitch (C key) should not transpose anything
        let score = parse("C D E F G A B C^").unwrap();
        let xml = to_musicxml(&score); // No transposition

        // Notes should remain as-is
        assert!(xml.contains("<step>C</step>"), "C should remain C");
        assert!(xml.contains("<step>D</step>"), "D should remain D");
        assert!(xml.contains("<step>E</step>"), "E should remain E");
        assert!(xml.contains("<step>F</step>"), "F should remain F");
        assert!(xml.contains("<step>G</step>"), "G should remain G");
        assert!(xml.contains("<step>A</step>"), "A should remain A");
        assert!(xml.contains("<step>B</step>"), "B should remain B");
    }

    #[test]
    fn test_beaming_12_8_time() {
        // 12/8 time: should beam in groups of 3 eighth notes (dotted quarter beats)
        // Measure has 12 eighth notes, should create 4 beam groups
        let source = r#"---
time-signature: 12/8
---
/C /D /E /F /G /A /B /C^ /D^ /E^ /F^ /G^"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Count beam begin/end markers - should have 4 beam groups (12 notes / 3 per group)
        let beam_begin_count = xml.matches("<beam number=\"1\">begin</beam>").count();
        let beam_end_count = xml.matches("<beam number=\"1\">end</beam>").count();

        assert_eq!(beam_begin_count, 4, "Should have 4 beam groups (one per dotted quarter beat)");
        assert_eq!(beam_end_count, 4, "Should have 4 beam groups (one per dotted quarter beat)");
    }

    #[test]
    fn test_beaming_6_8_time() {
        // 6/8 time: should beam in groups of 3 eighth notes (dotted quarter beats)
        let source = r#"---
time-signature: 6/8
---
/C /D /E /F /G /A"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Should have 2 beam groups (6 notes / 3 per group)
        let beam_begin_count = xml.matches("<beam number=\"1\">begin</beam>").count();
        let beam_end_count = xml.matches("<beam number=\"1\">end</beam>").count();

        assert_eq!(beam_begin_count, 2, "Should have 2 beam groups in 6/8");
        assert_eq!(beam_end_count, 2, "Should have 2 beam groups in 6/8");
    }

    #[test]
    fn test_beaming_9_8_time() {
        // 9/8 time: should beam in groups of 3 eighth notes
        let source = r#"---
time-signature: 9/8
---
/C /D /E /F /G /A /B /C^ /D^"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Should have 3 beam groups (9 notes / 3 per group)
        let beam_begin_count = xml.matches("<beam number=\"1\">begin</beam>").count();
        let beam_end_count = xml.matches("<beam number=\"1\">end</beam>").count();

        assert_eq!(beam_begin_count, 3, "Should have 3 beam groups in 9/8");
        assert_eq!(beam_end_count, 3, "Should have 3 beam groups in 9/8");
    }

    #[test]
    fn test_key_change_single_measure() {
        let score = parse("@key:G C D E F").unwrap();
        let xml = to_musicxml(&score);

        // First measure should have attributes with key signature (G major = 1 sharp)
        assert!(xml.contains("<key>"));
        assert!(xml.contains("<fifths>1</fifths>"));
    }

    #[test]
    fn test_key_change_multiple_measures() {
        let source = r#"---
key-signature: C
---
C D E F
@key:D G A B C^
@key:F D E F G"#;
        let score = parse(source).unwrap();
        let xml = to_musicxml(&score);

        // Should have 3 key elements total: C (initial), D, and F
        let key_count = xml.matches("<key>").count();
        assert_eq!(key_count, 3, "Should have 3 key signature elements");

        // First measure: C major (0)
        assert!(xml.contains("<fifths>0</fifths>"));

        // Second measure: D major (2 sharps)
        assert!(xml.contains("<fifths>2</fifths>"));

        // Third measure: F major (1 flat)
        assert!(xml.contains("<fifths>-1</fifths>"));
    }

    #[test]
    fn test_key_change_with_transposition() {
        // Concert pitch: change to D major (2 sharps)
        // For Bb instrument: should transpose to E major (4 sharps)
        let source = "@key:D C D E F";
        let score = parse(source).unwrap();
        let transposition = Transposition { diatonic: 1, chromatic: 2, fifths: 2 };
        let xml = to_musicxml_with_options(&score, Some(transposition), Clef::Treble, 0);

        // D major (2) + Bb transposition (2) = E major (4)
        assert!(xml.contains("<fifths>4</fifths>"));
    }

    #[test]
    fn test_key_change_preserves_current_key() {
        // After key change, notes should use the new key signature
        let source = r#"---
key-signature: C
---
C D E F
@key:G F G A B"#;
        let score = parse(source).unwrap();

        // Second measure should have key change to G (1 sharp: F#)
        assert_eq!(score.measures.len(), 2);
        assert!(score.measures[1].key_change.is_some());
        assert_eq!(score.measures[1].key_change.as_ref().unwrap().fifths, 1);
    }
}
