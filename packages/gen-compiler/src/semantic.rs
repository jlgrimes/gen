//! # Semantic Validation Module
//!
//! This module validates the semantic correctness of a parsed Gen score.
//!
//! ## Purpose
//! After parsing, the AST may be syntactically valid but semantically incorrect.
//! This module checks for logical errors that would make the score invalid:
//! - Measure durations that don't match the time signature
//! - Unmatched repeat markers
//! - Incorrectly structured first/second endings
//!
//! ## Validation Rules
//!
//! ### Measure Duration
//! - Each measure's total duration (sum of all note/rest durations) must match the time signature
//! - Example: In 4/4 time, each measure must have exactly 4 beats
//! - Dotted notes and tuplets are correctly calculated
//!
//! ### Repeat Markers
//! - `||:` (repeat start) must be paired with `:||` (repeat end)
//! - No nested repeats without closing the previous one
//! - No repeat end without a matching repeat start
//!
//! ### Endings
//! - First ending (`|1`) must come before second ending (`|2`)
//! - Second ending must exist if first ending exists
//! - Endings must be within a repeat structure
//!
//! ## Entry Point
//! `validate(score: &Score) -> Result<(), GenError>`
//!
//! ## Example
//! ```rust
//! use gen::{parse, validate};
//!
//! let source = "C D E F";  // 4 quarter notes = 4 beats (valid in 4/4)
//! let score = parse(source)?;
//! validate(&score)?;  // Passes validation
//! ```
//!
//! ## Related Modules
//! - `ast` - Defines Score and Measure types
//! - `error` - Returns GenError::SemanticError with measure numbers

use crate::ast::*;
use crate::error::GenError;

/// Validate a score for semantic correctness
///
/// Checks three main validation rules:
/// 1. Measure durations match time signature
/// 2. Repeat markers are properly matched
/// 3. Endings are correctly structured
pub fn validate(score: &Score) -> Result<(), GenError> {
    for (i, measure) in score.measures.iter().enumerate() {
        validate_measure(measure, &score.metadata.time_signature, i + 1)?;
    }
    validate_repeats(score)?;
    validate_endings(score)?;
    Ok(())
}

/// Validate that repeat markers are properly matched
fn validate_repeats(score: &Score) -> Result<(), GenError> {
    let mut repeat_start_measure: Option<usize> = None;

    for (i, measure) in score.measures.iter().enumerate() {
        let measure_number = i + 1;

        if measure.repeat_start {
            if repeat_start_measure.is_some() {
                // Nested repeat start without closing the previous one
                return Err(GenError::SemanticError {
                    measure: measure_number,
                    message: "Repeat start (||:) found without closing the previous repeat. Close the previous repeat with :|| first.".to_string(),
                });
            }
            repeat_start_measure = Some(measure_number);
        }

        if measure.repeat_end {
            if repeat_start_measure.is_none() {
                return Err(GenError::SemanticError {
                    measure: measure_number,
                    message: "Repeat end (:||) found without a matching repeat start (||:)".to_string(),
                });
            }
            repeat_start_measure = None;
        }
    }

    // Check if there's an unclosed repeat
    if let Some(start_measure) = repeat_start_measure {
        return Err(GenError::SemanticError {
            measure: start_measure,
            message: "Repeat start (||:) at this measure has no matching repeat end (:||)".to_string(),
        });
    }

    Ok(())
}

/// Validate that first/second endings are properly used
fn validate_endings(score: &Score) -> Result<(), GenError> {
    for (i, measure) in score.measures.iter().enumerate() {
        let measure_number = i + 1;

        match measure.ending {
            Some(Ending::First) => {
                // 1st ending must have a repeat end
                if !measure.repeat_end {
                    return Err(GenError::SemanticError {
                        measure: measure_number,
                        message: "First ending (1.) must end with a repeat sign (:||)".to_string(),
                    });
                }
            }
            Some(Ending::Second) => {
                // 2nd ending cannot have a repeat end
                if measure.repeat_end {
                    return Err(GenError::SemanticError {
                        measure: measure_number,
                        message: "Second ending (2.) cannot have a repeat sign (:||)".to_string(),
                    });
                }

                // 2nd ending must immediately follow a 1st ending
                if i == 0 {
                    return Err(GenError::SemanticError {
                        measure: measure_number,
                        message: "Second ending (2.) must immediately follow a first ending (1.)".to_string(),
                    });
                }

                let prev_measure = &score.measures[i - 1];
                if prev_measure.ending != Some(Ending::First) {
                    return Err(GenError::SemanticError {
                        measure: measure_number,
                        message: "Second ending (2.) must immediately follow a first ending (1.)".to_string(),
                    });
                }
            }
            None => {}
        }
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
    let (duration, dotted, tuplet) = match element {
        Element::Note(note) => (note.duration, note.dotted, note.tuplet),
        Element::Rest { duration, dotted, tuplet, .. } => (*duration, *dotted, *tuplet),
    };

    let mut base = duration.as_fraction();
    if dotted {
        base *= 1.5;
    }

    // Apply tuplet ratio: actual duration = base * (normal_notes / actual_notes)
    // e.g., triplet eighth = eighth * (2/3)
    if let Some(tuplet_info) = tuplet {
        base *= tuplet_info.normal_notes as f64 / tuplet_info.actual_notes as f64;
    }

    base
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
        let score = parse("dC dC").unwrap(); // 2 half notes
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_triplet_duration() {
        // Quarter note triplet (3 quarters in the time of 2) + 2 regular quarters = 4 beats
        let score = parse("3[C D E] C C").unwrap();
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_eighth_note_triplet_duration() {
        // Eighth note triplet (3 eighths in time of 2 eighths = 1 quarter) + 3 regular quarters = 4 beats
        let score = parse("/3[C D E] C C C").unwrap();
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_valid_repeat() {
        // Valid repeat: start and end markers
        let score = parse("||: C C C C\nD D D D :||").unwrap();
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_repeat_same_measure() {
        // Valid: repeat start and end in same measure
        let score = parse("||: C C C C :||").unwrap();
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_repeat_missing_end() {
        // Invalid: repeat start without end
        let score = parse("||: C C C C\nD D D D").unwrap();
        let result = validate(&score);
        assert!(result.is_err());
        if let Err(GenError::SemanticError { message, .. }) = result {
            assert!(message.contains("no matching repeat end"));
        }
    }

    #[test]
    fn test_repeat_missing_start() {
        // Invalid: repeat end without start
        let score = parse("C C C C\nD D D D :||").unwrap();
        let result = validate(&score);
        assert!(result.is_err());
        if let Err(GenError::SemanticError { message, .. }) = result {
            assert!(message.contains("without a matching repeat start"));
        }
    }

    #[test]
    fn test_nested_repeat_error() {
        // Invalid: nested repeat (start without closing previous)
        let score = parse("||: C C C C\n||: D D D D :||").unwrap();
        let result = validate(&score);
        assert!(result.is_err());
    }

    #[test]
    fn test_valid_endings() {
        // Valid: 1st ending with repeat, followed by 2nd ending without repeat
        let score = parse("||: C C C C\n1. D D D D :||\n2. E E E E").unwrap();
        assert!(validate(&score).is_ok());
    }

    #[test]
    fn test_first_ending_without_repeat() {
        // Invalid: 1st ending must have repeat end
        let score = parse("1. C C C C").unwrap();
        let result = validate(&score);
        assert!(result.is_err());
        if let Err(GenError::SemanticError { message, .. }) = result {
            assert!(message.contains("must end with a repeat sign"));
        }
    }

    #[test]
    fn test_second_ending_with_repeat() {
        // Invalid: 2nd ending cannot have repeat end
        // Start a new repeat after 2nd ending to make the :|| valid from repeat perspective,
        // but ending validation should still catch it
        let score = parse("||: C C C C\n1. D D D D :||\n2. ||: E E E E :||").unwrap();
        let result = validate(&score);
        assert!(result.is_err(), "Should fail validation but got: {:?}", result);
        if let Err(GenError::SemanticError { message, .. }) = &result {
            assert!(message.contains("cannot have a repeat sign"), "Expected 'cannot have a repeat sign' but got: {}", message);
        } else {
            panic!("Expected SemanticError but got: {:?}", result);
        }
    }

    #[test]
    fn test_second_ending_without_first() {
        // Invalid: 2nd ending must follow 1st ending
        let score = parse("C C C C\n2. D D D D").unwrap();
        let result = validate(&score);
        assert!(result.is_err());
        if let Err(GenError::SemanticError { message, .. }) = result {
            assert!(message.contains("must immediately follow a first ending"));
        }
    }
}
