use crate::ast::{Accidental, Element, KeySignature, Measure, Note, NoteName, Score};

/// Interval in semitones for transposition
/// Calculated from viewed_key: how many semitones UP to transpose
fn get_transposition_interval(viewed_key: &str) -> Option<i8> {
    // viewed_key is like "Bb", "Eb", "F", "C"
    // For Bb instruments: concert C sounds like Bb, so we transpose UP a whole step (2 semitones)
    // For Eb instruments: concert C sounds like Eb, so we transpose UP a minor 3rd (3 semitones)
    // Wait - actually it's the opposite direction for reading:
    // Bb clarinet: to make their written D sound like concert C, we write D (up a whole step)
    // So viewed_key "Bb" means transpose UP 2 semitones
    match viewed_key.trim() {
        "C" => Some(0),
        "F" => Some(5),   // Up a perfect 4th (or down a 5th)
        "Bb" => Some(2),  // Up a whole step
        "Eb" => Some(9),  // Up a major 6th (or minor 3rd = 3, but we go UP)
        _ => None,
    }
}

/// Get the number of fifths to move on the circle of fifths
fn get_fifths_offset(viewed_key: &str) -> Option<i8> {
    match viewed_key.trim() {
        "C" => Some(0),
        "F" => Some(-1),  // 1 flat direction
        "Bb" => Some(-2), // 2 flats direction
        "Eb" => Some(-3), // 3 flats direction
        _ => None,
    }
}

/// Note name to semitone offset from C
fn note_to_semitone(name: NoteName, accidental: Accidental) -> i8 {
    let base: i8 = match name {
        NoteName::C => 0,
        NoteName::D => 2,
        NoteName::E => 4,
        NoteName::F => 5,
        NoteName::G => 7,
        NoteName::A => 9,
        NoteName::B => 11,
    };
    let acc: i8 = match accidental {
        Accidental::Sharp => 1,
        Accidental::Flat => -1,
        Accidental::Natural => 0,
    };
    (base + acc).rem_euclid(12)
}

/// Convert semitone to note name and accidental
/// Returns (NoteName, Accidental, octave_adjustment)
fn semitone_to_note(semitone: i8, prefer_flat: bool) -> (NoteName, Accidental, i8) {
    let normalized = semitone.rem_euclid(12);
    let octave_adj = if semitone < 0 {
        -1
    } else if semitone >= 12 {
        1
    } else {
        0
    };

    // Map semitones to note names
    // 0=C, 1=C#/Db, 2=D, 3=D#/Eb, 4=E, 5=F, 6=F#/Gb, 7=G, 8=G#/Ab, 9=A, 10=A#/Bb, 11=B
    let (name, acc) = match normalized {
        0 => (NoteName::C, Accidental::Natural),
        1 => if prefer_flat { (NoteName::D, Accidental::Flat) } else { (NoteName::C, Accidental::Sharp) },
        2 => (NoteName::D, Accidental::Natural),
        3 => if prefer_flat { (NoteName::E, Accidental::Flat) } else { (NoteName::D, Accidental::Sharp) },
        4 => (NoteName::E, Accidental::Natural),
        5 => (NoteName::F, Accidental::Natural),
        6 => if prefer_flat { (NoteName::G, Accidental::Flat) } else { (NoteName::F, Accidental::Sharp) },
        7 => (NoteName::G, Accidental::Natural),
        8 => if prefer_flat { (NoteName::A, Accidental::Flat) } else { (NoteName::G, Accidental::Sharp) },
        9 => (NoteName::A, Accidental::Natural),
        10 => if prefer_flat { (NoteName::B, Accidental::Flat) } else { (NoteName::A, Accidental::Sharp) },
        11 => (NoteName::B, Accidental::Natural),
        _ => unreachable!(),
    };

    (name, acc, octave_adj)
}

/// Transpose a single note by the given number of semitones
fn transpose_note(note: &Note, semitones: i8, prefer_flat: bool) -> Note {
    let current = note_to_semitone(note.name, note.accidental);
    let new_semitone = current + semitones;
    let (new_name, new_acc, octave_adj) = semitone_to_note(new_semitone, prefer_flat);

    // Adjust octave if we wrapped
    let new_octave = match (note.octave, octave_adj) {
        (crate::ast::Octave::DoubleLow, -1) => crate::ast::Octave::DoubleLow, // Can't go lower
        (crate::ast::Octave::DoubleLow, 1) => crate::ast::Octave::Low,
        (crate::ast::Octave::Low, -1) => crate::ast::Octave::DoubleLow,
        (crate::ast::Octave::Low, 1) => crate::ast::Octave::Middle,
        (crate::ast::Octave::Middle, -1) => crate::ast::Octave::Low,
        (crate::ast::Octave::Middle, 1) => crate::ast::Octave::High,
        (crate::ast::Octave::High, -1) => crate::ast::Octave::Middle,
        (crate::ast::Octave::High, 1) => crate::ast::Octave::DoubleHigh,
        (crate::ast::Octave::DoubleHigh, -1) => crate::ast::Octave::High,
        (crate::ast::Octave::DoubleHigh, 1) => crate::ast::Octave::DoubleHigh, // Can't go higher
        (oct, 0) => oct,
        _ => note.octave,
    };

    Note {
        name: new_name,
        accidental: new_acc,
        octave: new_octave,
        duration: note.duration,
        dotted: note.dotted,
        tuplet: note.tuplet,
        tie_start: note.tie_start,
        tie_stop: note.tie_stop,
    }
}

/// Transpose the key signature by moving on the circle of fifths
fn transpose_key_signature(key: &KeySignature, fifths_offset: i8) -> KeySignature {
    // Moving on circle of fifths: positive = more sharps, negative = more flats
    // But we want to ADD the offset (if going to Bb, we add -2 fifths)
    let new_fifths = key.fifths - fifths_offset;
    // Clamp to valid range
    let clamped = new_fifths.clamp(-7, 7);
    KeySignature { fifths: clamped }
}

/// Transpose an entire score for a given viewed key (e.g., "Bb", "Eb", "F")
pub fn transpose_score(score: &Score, viewed_key: &str) -> Option<Score> {
    let semitones = get_transposition_interval(viewed_key)?;
    let fifths_offset = get_fifths_offset(viewed_key)?;

    if semitones == 0 {
        return Some(score.clone());
    }

    // Determine if we should prefer flats based on the target key
    let prefer_flat = fifths_offset < 0;

    // Transpose key signature
    let new_key = transpose_key_signature(&score.metadata.key_signature, fifths_offset);

    // Transpose all notes
    let new_measures: Vec<Measure> = score.measures.iter().map(|measure| {
        let new_elements: Vec<Element> = measure.elements.iter().map(|elem| {
            match elem {
                Element::Note(note) => Element::Note(transpose_note(note, semitones, prefer_flat)),
                Element::Rest { duration, dotted, tuplet } => Element::Rest {
                    duration: *duration,
                    dotted: *dotted,
                    tuplet: *tuplet,
                },
            }
        }).collect();

        Measure {
            elements: new_elements,
            repeat_start: measure.repeat_start,
            repeat_end: measure.repeat_end,
        }
    }).collect();

    Some(Score {
        metadata: crate::ast::Metadata {
            title: score.metadata.title.clone(),
            composer: score.metadata.composer.clone(),
            time_signature: score.metadata.time_signature.clone(),
            key_signature: new_key,
            written_pitch: score.metadata.written_pitch.clone(),
        },
        measures: new_measures,
    })
}
