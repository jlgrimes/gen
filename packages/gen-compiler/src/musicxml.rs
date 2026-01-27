use crate::ast::*;
use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;
use std::io::Cursor;

/// Clef type for the score
#[derive(Clone, Copy, Default, PartialEq)]
pub enum Clef {
    #[default]
    Treble,
    Bass,
}

/// Transposition info for MusicXML
#[derive(Clone, Copy, Default)]
pub struct Transposition {
    pub diatonic: i8,   // Number of diatonic steps (letter names)
    pub chromatic: i8,  // Number of chromatic half steps
}

impl Transposition {
    /// Create transposition for a given viewed key
    /// Returns None for concert pitch (C), Some for transposing instruments
    pub fn for_key(viewed_key: &str) -> Option<Self> {
        match viewed_key.trim() {
            "C" => None, // Concert pitch, no transposition needed
            "Bb" => Some(Transposition { diatonic: 1, chromatic: 2 }),   // Up a major 2nd
            "Eb" => Some(Transposition { diatonic: 5, chromatic: 9 }),   // Up a major 6th
            "F" => Some(Transposition { diatonic: 4, chromatic: 5 }),    // Up a perfect 4th (or down a 5th)
            _ => None,
        }
    }
}

/// Convert a Score to MusicXML format
pub fn to_musicxml(score: &Score) -> String {
    to_musicxml_full(score, None, Clef::Treble, 0)
}

/// Convert a Score to MusicXML format with optional transposition
pub fn to_musicxml_transposed(score: &Score, transposition: Option<Transposition>) -> String {
    to_musicxml_full(score, transposition, Clef::Treble, 0)
}

/// Convert a Score to MusicXML format with clef and octave shift options
pub fn to_musicxml_with_options(score: &Score, transposition: Option<Transposition>, clef: Clef, octave_shift: i8) -> String {
    to_musicxml_full(score, transposition, clef, octave_shift)
}

/// Convert a Score to MusicXML format with all options
fn to_musicxml_full(score: &Score, transposition: Option<Transposition>, clef: Clef, octave_shift: i8) -> String {
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

    for (i, measure) in score.measures.iter().enumerate() {
        write_measure(
            &mut writer,
            measure,
            i + 1,
            &score.metadata.time_signature,
            &score.metadata.key_signature,
            i == 0,
            transposition,
            clef,
            octave_shift,
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
        } => duration_to_divisions_high_res(*duration, *dotted, *tuplet),
    }
}

/// Calculate beam states for all elements in a measure, respecting beat boundaries
fn calculate_beam_states(elements: &[Element], time_signature: &TimeSignature) -> Vec<BeamState> {
    let mut states = vec![BeamState::None; elements.len()];

    // Calculate beat size in high-res divisions (12 divisions = 1 quarter note)
    // This allows clean representation of triplets (12 is divisible by 3)
    // For 4/4: beat = 12 divisions (quarter note)
    // For 6/8: beat = 6 divisions (eighth note beat, typical for compound time)
    // For 2/4: beat = 12 divisions (quarter note)
    let beat_divisions = 48 / time_signature.beat_type as u32;

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
) {
    let mut measure_elem = BytesStart::new("measure");
    measure_elem.push_attribute(("number", number.to_string().as_str()));
    writer.write_event(Event::Start(measure_elem)).unwrap();

    // Write left barline (repeat start and/or ending start)
    if measure.repeat_start || measure.ending.is_some() {
        write_left_barline(writer, measure.repeat_start, measure.ending);
    }

    // Include time signature, key signature, and clef on first measure
    if include_attributes {
        writer
            .write_event(Event::Start(BytesStart::new("attributes")))
            .unwrap();

        write_text_element(writer, "divisions", "4");

        writer
            .write_event(Event::Start(BytesStart::new("key")))
            .unwrap();
        write_text_element(writer, "fifths", &key_signature.fifths.to_string());
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
    }

    // Calculate beam states for all elements
    let beam_states = calculate_beam_states(&measure.elements, time_signature);

    for (element, beam_state) in measure.elements.iter().zip(beam_states.iter()) {
        write_element(writer, element, *beam_state, octave_shift, key_signature);
    }

    // Write right barline (repeat end and/or ending stop)
    if measure.repeat_end || measure.ending.is_some() {
        write_right_barline(writer, measure.repeat_end, measure.ending);
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

fn write_element<W: std::io::Write>(writer: &mut Writer<W>, element: &Element, beam_state: BeamState, octave_shift: i8, key_signature: &KeySignature) {
    match element {
        Element::Note(note) => write_note(writer, note, beam_state, octave_shift, key_signature),
        Element::Rest {
            duration,
            dotted,
            tuplet,
        } => write_rest(writer, *duration, *dotted, *tuplet),
    }
}

fn write_note<W: std::io::Write>(writer: &mut Writer<W>, note: &Note, beam_state: BeamState, octave_shift: i8, key_signature: &KeySignature) {
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

    // Pitch
    writer
        .write_event(Event::Start(BytesStart::new("pitch")))
        .unwrap();
    write_text_element(writer, "step", note_name_to_str(note.name));

    // Alter for sharps/flats
    match effective_accidental {
        Accidental::Sharp => write_text_element(writer, "alter", "1"),
        Accidental::Flat => write_text_element(writer, "alter", "-1"),
        Accidental::Natural | Accidental::ForceNatural => {}
    }

    // Octave (middle C = octave 4, adjusted by octave_shift)
    let base_octave: i8 = match note.octave {
        Octave::DoubleLow => 2,
        Octave::Low => 3,
        Octave::Middle => 4,
        Octave::High => 5,
        Octave::DoubleHigh => 6,
    };
    let octave = (base_octave + octave_shift).max(0).min(9);
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
        let score = parse("[C D E]3").unwrap();
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
        let trans = Transposition { diatonic: 1, chromatic: 2 };
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
        let score = parse("1st: C C C C :||").unwrap();
        let xml = to_musicxml(&score);

        // Should contain ending element with number 1
        assert!(xml.contains("<ending number=\"1\" type=\"start\">"));
        assert!(xml.contains("1."));
        assert!(xml.contains("<ending number=\"1\" type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_second_ending() {
        let score = parse("2nd: C C C C").unwrap();
        let xml = to_musicxml(&score);

        // Should contain ending element with number 2
        assert!(xml.contains("<ending number=\"2\" type=\"start\">"));
        assert!(xml.contains("2."));
        assert!(xml.contains("<ending number=\"2\" type=\"stop\"/>"));
    }

    #[test]
    fn test_musicxml_first_and_second_endings() {
        let score = parse("||: C C C C\n1st: D D D D :||\n2nd: E E E E").unwrap();
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
        let source = r#"---
key-signature: ###
---
F C G D"#;
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
}
