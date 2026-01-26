use crate::ast::*;
use crate::error::GenError;

/// Validate a score for semantic correctness
pub fn validate(score: &Score) -> Result<(), GenError> {
    for (i, measure) in score.measures.iter().enumerate() {
        validate_measure(measure, &score.metadata.time_signature, i + 1)?;
    }
    Ok(())
}

/// Validate a single measure
fn validate_measure(
    measure: &Measure,
    time_signature: &TimeSignature,
    measure_number: usize,
) -> Result<(), GenError> {
    let total_duration = calculate_measure_duration(measure);
    let expected_duration = time_signature_duration(time_signature);

    // Allow some floating point tolerance
    let tolerance = 0.001;
    if (total_duration - expected_duration).abs() > tolerance {
        return Err(GenError::SemanticError {
            measure: measure_number,
            message: format!(
                "Measure duration mismatch: expected {} beats, got {} beats",
                expected_duration * (time_signature.beat_type as f64),
                total_duration * (time_signature.beat_type as f64)
            ),
        });
    }

    Ok(())
}

/// Calculate the total duration of a measure as a fraction of a whole note
fn calculate_measure_duration(measure: &Measure) -> f64 {
    measure
        .elements
        .iter()
        .map(|e| element_duration(e))
        .sum()
}

/// Get the duration of an element as a fraction of a whole note
fn element_duration(element: &Element) -> f64 {
    let (duration, dotted) = match element {
        Element::Note(note) => (note.duration, note.dotted),
        Element::Rest { duration, dotted } => (*duration, *dotted),
    };

    let base = duration.as_fraction();
    if dotted {
        base * 1.5
    } else {
        base
    }
}

/// Get the expected duration of a measure based on time signature
/// Returns duration as a fraction of a whole note
fn time_signature_duration(ts: &TimeSignature) -> f64 {
    // beats / beat_type gives us the fraction of a whole note
    // e.g., 4/4 = 4/4 = 1.0 whole note
    // e.g., 3/4 = 3/4 = 0.75 whole note
    // e.g., 6/8 = 6/8 = 0.75 whole note
    (ts.beats as f64) / (ts.beat_type as f64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_valid_4_4_measure() {
        let score = parse("C C C C").unwrap(); // 4 quarter notes
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_invalid_measure_too_long() {
        let score = parse("C C C C C").unwrap(); // 5 quarter notes
        assert!(validate(&score).is_err());
    }

    #[test]
    fn test_valid_with_different_durations() {
        let score = parse("|oC |oC").unwrap(); // 2 half notes
        assert!(validate(&score).is_ok());
    }
}
