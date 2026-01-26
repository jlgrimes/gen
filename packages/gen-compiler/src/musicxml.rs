use crate::ast::*;

/// Convert a Score to MusicXML format
pub fn to_musicxml(score: &Score) -> String {
    let mut xml = String::new();

    // XML declaration and doctype
    xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str(r#"<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">"#);
    xml.push('\n');

    // Root element
    xml.push_str(r#"<score-partwise version="4.0">"#);
    xml.push('\n');

    // Work info (title)
    if let Some(title) = &score.metadata.title {
        xml.push_str("  <work>\n");
        xml.push_str(&format!("    <work-title>{}</work-title>\n", escape_xml(title)));
        xml.push_str("  </work>\n");
    }

    // Identification (composer)
    if let Some(composer) = &score.metadata.composer {
        xml.push_str("  <identification>\n");
        xml.push_str(&format!(
            "    <creator type=\"composer\">{}</creator>\n",
            escape_xml(composer)
        ));
        xml.push_str("  </identification>\n");
    }

    // Part list
    xml.push_str("  <part-list>\n");
    xml.push_str("    <score-part id=\"P1\">\n");
    xml.push_str("      <part-name print-object=\"no\"></part-name>\n");
    xml.push_str("    </score-part>\n");
    xml.push_str("  </part-list>\n");

    // Part with measures
    xml.push_str("  <part id=\"P1\">\n");

    for (i, measure) in score.measures.iter().enumerate() {
        xml.push_str(&measure_to_xml(
            measure,
            i + 1,
            &score.metadata.time_signature,
            &score.metadata.key_signature,
            i == 0,
        ));
    }

    xml.push_str("  </part>\n");
    xml.push_str("</score-partwise>\n");

    xml
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
        Element::Note(note) => duration_to_divisions_high_res(note.duration, note.dotted, note.tuplet),
        Element::Rest { duration, dotted, tuplet } => duration_to_divisions_high_res(*duration, *dotted, *tuplet),
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

fn measure_to_xml(
    measure: &Measure,
    number: usize,
    time_signature: &TimeSignature,
    key_signature: &KeySignature,
    include_attributes: bool,
) -> String {
    let mut xml = String::new();

    xml.push_str(&format!("    <measure number=\"{}\">\n", number));

    // Include time signature, key signature, and clef on first measure
    if include_attributes {
        xml.push_str("      <attributes>\n");
        xml.push_str("        <divisions>4</divisions>\n"); // 4 divisions per quarter note
        xml.push_str("        <key>\n");
        xml.push_str(&format!("          <fifths>{}</fifths>\n", key_signature.fifths));
        xml.push_str("        </key>\n");
        xml.push_str("        <time>\n");
        xml.push_str(&format!("          <beats>{}</beats>\n", time_signature.beats));
        xml.push_str(&format!(
            "          <beat-type>{}</beat-type>\n",
            time_signature.beat_type
        ));
        xml.push_str("        </time>\n");
        xml.push_str("        <clef>\n");
        xml.push_str("          <sign>G</sign>\n");
        xml.push_str("          <line>2</line>\n");
        xml.push_str("        </clef>\n");
        xml.push_str("      </attributes>\n");
    }

    // Calculate beam states for all elements
    let beam_states = calculate_beam_states(&measure.elements, time_signature);

    for (element, beam_state) in measure.elements.iter().zip(beam_states.iter()) {
        xml.push_str(&element_to_xml(element, *beam_state));
    }

    xml.push_str("    </measure>\n");
    xml
}

fn element_to_xml(element: &Element, beam_state: BeamState) -> String {
    match element {
        Element::Note(note) => note_to_xml(note, beam_state),
        Element::Rest { duration, dotted, tuplet } => rest_to_xml(*duration, *dotted, *tuplet),
    }
}

fn note_to_xml(note: &Note, beam_state: BeamState) -> String {
    let mut xml = String::new();

    xml.push_str("      <note>\n");

    // Pitch
    xml.push_str("        <pitch>\n");
    xml.push_str(&format!("          <step>{}</step>\n", note_name_to_str(note.name)));

    // Alter for sharps/flats
    match note.accidental {
        Accidental::Sharp => xml.push_str("          <alter>1</alter>\n"),
        Accidental::Flat => xml.push_str("          <alter>-1</alter>\n"),
        Accidental::Natural => {}
    }

    // Octave (middle C = octave 4)
    let octave = match note.octave {
        Octave::DoubleLow => 2,
        Octave::Low => 3,
        Octave::Middle => 4,
        Octave::High => 5,
        Octave::DoubleHigh => 6,
    };
    xml.push_str(&format!("          <octave>{}</octave>\n", octave));
    xml.push_str("        </pitch>\n");

    // Duration (in divisions - 4 per quarter note)
    // For tuplets, we need to calculate the actual played duration
    let divisions = duration_to_divisions_with_tuplet(note.duration, note.dotted, note.tuplet);
    xml.push_str(&format!("        <duration>{}</duration>\n", divisions));

    // Ties (for playback - must come before <type>)
    if note.tie_start {
        xml.push_str("        <tie type=\"start\"/>\n");
    }
    if note.tie_stop {
        xml.push_str("        <tie type=\"stop\"/>\n");
    }

    // Type
    xml.push_str(&format!("        <type>{}</type>\n", note.duration.musicxml_type()));

    // Dot if dotted
    if note.dotted {
        xml.push_str("        <dot/>\n");
    }

    // Time modification for tuplets
    if let Some(tuplet_info) = note.tuplet {
        xml.push_str("        <time-modification>\n");
        xml.push_str(&format!("          <actual-notes>{}</actual-notes>\n", tuplet_info.actual_notes));
        xml.push_str(&format!("          <normal-notes>{}</normal-notes>\n", tuplet_info.normal_notes));
        xml.push_str("        </time-modification>\n");
    }

    // Beam (for eighth notes and shorter)
    match beam_state {
        BeamState::Begin => xml.push_str("        <beam number=\"1\">begin</beam>\n"),
        BeamState::Continue => xml.push_str("        <beam number=\"1\">continue</beam>\n"),
        BeamState::End => xml.push_str("        <beam number=\"1\">end</beam>\n"),
        BeamState::None => {}
    }

    // Notations (tuplet markers, ties, and accidentals display)
    let has_tuplet_notation = note.tuplet.map(|t| t.is_start || t.is_stop).unwrap_or(false);
    let has_tie_notation = note.tie_start || note.tie_stop;
    if has_tuplet_notation || has_tie_notation {
        xml.push_str("        <notations>\n");
        // Tied notations (for visual display)
        if note.tie_start {
            xml.push_str("          <tied type=\"start\"/>\n");
        }
        if note.tie_stop {
            xml.push_str("          <tied type=\"stop\"/>\n");
        }
        // Tuplet notations
        if let Some(tuplet_info) = note.tuplet {
            if tuplet_info.is_start {
                xml.push_str("          <tuplet type=\"start\" bracket=\"yes\"/>\n");
            }
            if tuplet_info.is_stop {
                xml.push_str("          <tuplet type=\"stop\"/>\n");
            }
        }
        xml.push_str("        </notations>\n");
    }

    // Accidental display
    match note.accidental {
        Accidental::Sharp => xml.push_str("        <accidental>sharp</accidental>\n"),
        Accidental::Flat => xml.push_str("        <accidental>flat</accidental>\n"),
        Accidental::Natural => {}
    }

    xml.push_str("      </note>\n");
    xml
}

fn rest_to_xml(duration: Duration, dotted: bool, tuplet: Option<TupletInfo>) -> String {
    let mut xml = String::new();

    xml.push_str("      <note>\n");
    xml.push_str("        <rest/>\n");

    let divisions = duration_to_divisions_with_tuplet(duration, dotted, tuplet);
    xml.push_str(&format!("        <duration>{}</duration>\n", divisions));
    xml.push_str(&format!("        <type>{}</type>\n", duration.musicxml_type()));

    if dotted {
        xml.push_str("        <dot/>\n");
    }

    // Time modification for tuplets
    if let Some(tuplet_info) = tuplet {
        xml.push_str("        <time-modification>\n");
        xml.push_str(&format!("          <actual-notes>{}</actual-notes>\n", tuplet_info.actual_notes));
        xml.push_str(&format!("          <normal-notes>{}</normal-notes>\n", tuplet_info.normal_notes));
        xml.push_str("        </time-modification>\n");
    }

    // Notations (tuplet markers)
    let has_tuplet_notation = tuplet.map(|t| t.is_start || t.is_stop).unwrap_or(false);
    if has_tuplet_notation {
        xml.push_str("        <notations>\n");
        if let Some(tuplet_info) = tuplet {
            if tuplet_info.is_start {
                xml.push_str("          <tuplet type=\"start\" bracket=\"yes\"/>\n");
            }
            if tuplet_info.is_stop {
                xml.push_str("          <tuplet type=\"stop\"/>\n");
            }
        }
        xml.push_str("        </notations>\n");
    }

    xml.push_str("      </note>\n");
    xml
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
fn duration_to_divisions_high_res(duration: Duration, dotted: bool, tuplet: Option<TupletInfo>) -> u32 {
    let base = match duration {
        Duration::Whole => 48,
        Duration::Half => 24,
        Duration::Quarter => 12,
        Duration::Eighth => 6,
        Duration::Sixteenth => 3,
        Duration::ThirtySecond => 2, // Approximate, but fine for beaming
    };

    let with_dot = if dotted {
        base + (base / 2)
    } else {
        base
    };

    if let Some(tuplet_info) = tuplet {
        // For tuplets, actual duration = base * (normal_notes / actual_notes)
        // e.g., triplet = base * 2/3
        (with_dot * tuplet_info.normal_notes as u32) / tuplet_info.actual_notes as u32
    } else {
        with_dot
    }
}

/// Convert duration to MusicXML divisions, accounting for tuplets
fn duration_to_divisions_with_tuplet(duration: Duration, dotted: bool, tuplet: Option<TupletInfo>) -> u32 {
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

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
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
}
