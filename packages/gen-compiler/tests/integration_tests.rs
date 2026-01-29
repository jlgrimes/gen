//! Integration tests for the Gen compiler
//!
//! Tests full compilation pipeline from Gen source to MusicXML output.

use gen::compile;

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
