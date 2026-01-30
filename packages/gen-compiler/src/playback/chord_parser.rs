//! Chord symbol parsing for MIDI playback
//!
//! Parses chord symbols (C, Am, G7, Dm7b5, A7b9#11, etc.) into MIDI note arrays for accompaniment.
//! Uses a compositional parser that understands chord grammar rather than hardcoding combinations.

/// Interval constants in semitones from root
mod intervals {
    pub const MINOR_2ND: u8 = 1;   // b9
    pub const MAJOR_2ND: u8 = 2;   // 9, sus2
    pub const MINOR_3RD: u8 = 3;
    pub const MAJOR_3RD: u8 = 4;
    pub const PERFECT_4TH: u8 = 5; // 11, sus4
    pub const TRITONE: u8 = 6;     // b5, #11
    pub const PERFECT_5TH: u8 = 7;
    pub const MINOR_6TH: u8 = 8;   // #5, b13
    pub const MAJOR_6TH: u8 = 9;   // 6, 13
    pub const MINOR_7TH: u8 = 10;
    pub const MAJOR_7TH: u8 = 11;
}

use intervals::*;

/// Parse a chord symbol into MIDI notes
///
/// Returns a Vec of MIDI note numbers using common jazz/pop chord voicings.
/// Chords are voiced in the C3 octave (MIDI 48-59) for bass/piano accompaniment.
///
/// # Chord Grammar
/// The parser understands chord symbols compositionally:
/// - Root note: A-G with optional # or b
/// - Quality: major, minor (m, min, -), diminished (dim, °), augmented (aug, +)
/// - 7th type: 7 (dominant), maj7/M7 (major 7th), mM7 (minor-major 7th)
/// - Extensions: 9, 11, 13 (implies lower extensions)
/// - Alterations: b5, #5, b9, #9, #11, b13 (can stack multiple)
/// - Suspensions: sus2, sus4
/// - Added tones: add9, add11, add13, 6
/// - Slash chords: /E, /G# (bass note)
///
/// # Examples
/// ```
/// use gen::playback::parse_chord_symbol;
///
/// // Simple chords
/// assert_eq!(parse_chord_symbol("C"), vec![48, 52, 55]);
/// assert_eq!(parse_chord_symbol("Am"), vec![57, 60, 64]);
///
/// // Extended/altered chords
/// let a7b9 = parse_chord_symbol("A7b9");
/// assert!(a7b9.contains(&57));  // A root
/// assert!(a7b9.contains(&61));  // C# (major 3rd)
/// assert!(a7b9.contains(&64));  // E (5th)
/// assert!(a7b9.contains(&67));  // G (b7)
/// assert!(a7b9.contains(&70));  // Bb (b9)
/// ```
pub fn parse_chord_symbol(chord_symbol: &str) -> Vec<u8> {
    // Check for slash chord (e.g., "C/E", "Am/G")
    let (main_chord, bass_note) = if let Some(slash_pos) = chord_symbol.find('/') {
        (&chord_symbol[..slash_pos], Some(&chord_symbol[slash_pos + 1..]))
    } else {
        (chord_symbol, None)
    };

    // Extract root note and chord quality
    let chars: Vec<char> = main_chord.chars().collect();
    if chars.is_empty() {
        return vec![];
    }

    // Parse root note
    let root_name = chars[0];
    let mut idx = 1;

    // Check for accidental on root
    let accidental = if idx < chars.len() && (chars[idx] == '#' || chars[idx] == 'b') {
        idx += 1;
        if chars[idx - 1] == '#' { 1i8 } else { -1i8 }
    } else {
        0i8
    };

    // Base MIDI note for root (C3 = 48 for chord voicings)
    let base_midi = match root_name {
        'C' => 48i8,
        'D' => 50i8,
        'E' => 52i8,
        'F' => 53i8,
        'G' => 55i8,
        'A' => 57i8,
        'B' => 59i8,
        _ => return vec![],
    };
    let root = (base_midi + accidental) as u8;

    // Parse chord quality from remaining string
    let quality = &main_chord[idx..];

    // Build chord using compositional parsing
    let mut chord_tones = parse_quality(root, quality);

    // Handle slash chord - add bass note
    if let Some(bass_str) = bass_note {
        let bass_chars: Vec<char> = bass_str.chars().collect();
        if !bass_chars.is_empty() {
            let bass_name = bass_chars[0];
            let bass_accidental = if bass_chars.len() > 1 && (bass_chars[1] == '#' || bass_chars[1] == 'b') {
                if bass_chars[1] == '#' { 1i8 } else { -1i8 }
            } else {
                0i8
            };

            let bass_base = match bass_name {
                'C' => 48i8,
                'D' => 50i8,
                'E' => 52i8,
                'F' => 53i8,
                'G' => 55i8,
                'A' => 57i8,
                'B' => 59i8,
                _ => 48i8,
            };
            let bass_midi = (bass_base + bass_accidental) as u8;

            // Remove the bass note from chord tones if it's already there
            chord_tones.retain(|&note| note != bass_midi);

            // Insert bass at the beginning
            chord_tones.insert(0, bass_midi);
        }
    }

    chord_tones
}

/// Parse chord quality string and return intervals to add to root
fn parse_quality(root: u8, quality: &str) -> Vec<u8> {
    let mut third = Some(MAJOR_3RD);  // Default major third
    let mut fifth = Some(PERFECT_5TH); // Default perfect fifth
    let mut seventh: Option<u8> = None;
    let mut extensions: Vec<u8> = Vec::new();

    let q = quality;
    let mut pos = 0;

    // Track what we've parsed to handle order-independent alterations
    let mut has_parsed_base = false;

    while pos < q.len() {
        let remaining = &q[pos..];

        // Try to match patterns from longest to shortest
        if let Some((matched, advance)) = parse_token(remaining, &mut third, &mut fifth, &mut seventh, &mut extensions, has_parsed_base) {
            pos += advance;
            if matched {
                has_parsed_base = true;
            }
        } else {
            // Unknown token - skip character (handling multi-byte UTF-8)
            if let Some(c) = remaining.chars().next() {
                pos += c.len_utf8();
            } else {
                pos += 1;
            }
        }
    }

    // Build final chord
    let mut tones = vec![root];

    if let Some(t) = third {
        tones.push(root + t);
    }
    if let Some(f) = fifth {
        tones.push(root + f);
    }
    if let Some(s) = seventh {
        tones.push(root + s);
    }
    for ext in extensions {
        tones.push(root + ext);
    }

    tones
}

/// Parse a single token from the quality string
/// Returns (is_base_quality, chars_consumed) or None if nothing matched
fn parse_token(
    s: &str,
    third: &mut Option<u8>,
    fifth: &mut Option<u8>,
    seventh: &mut Option<u8>,
    extensions: &mut Vec<u8>,
    has_parsed_base: bool,
) -> Option<(bool, usize)> {
    // === Alterations (can appear anywhere after base quality) ===
    // These need to be checked first because they can appear in any order

    // Flat alterations: b5, b9, b11, b13
    if s.starts_with("b5") {
        *fifth = Some(TRITONE);
        return Some((false, 2));
    }
    if s.starts_with("b9") {
        extensions.push(MINOR_2ND + 12); // b9 is minor 2nd up an octave
        return Some((false, 2));
    }
    if s.starts_with("b11") {
        // b11 is enharmonic to major 3rd, rarely used - treat as natural 11
        extensions.push(PERFECT_4TH + 12);
        return Some((false, 3));
    }
    if s.starts_with("b13") {
        extensions.push(MINOR_6TH + 12);
        return Some((false, 3));
    }

    // Sharp alterations: #5, #9, #11, #13
    if s.starts_with("#5") || s.starts_with("+5") {
        *fifth = Some(MINOR_6TH); // augmented 5th
        return Some((false, 2));
    }
    if s.starts_with("#9") {
        extensions.push(MINOR_3RD + 12); // #9 is minor 3rd up an octave
        return Some((false, 2));
    }
    if s.starts_with("#11") {
        extensions.push(TRITONE + 12); // #11 is tritone up an octave
        return Some((false, 3));
    }
    if s.starts_with("#13") {
        // #13 is enharmonic to b7, rarely used
        extensions.push(MINOR_7TH + 12);
        return Some((false, 3));
    }

    // === Base qualities (only parse if we haven't parsed a base yet) ===
    if !has_parsed_base {
        // Minor-major 7th variations (must check before minor)
        if s.starts_with("minMaj7") || s.starts_with("minmaj7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MAJOR_7TH);
            return Some((true, 7));
        }
        if s.starts_with("mMaj7") || s.starts_with("mmaj7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MAJOR_7TH);
            return Some((true, 5));
        }
        if s.starts_with("mM7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MAJOR_7TH);
            return Some((true, 3));
        }
        if s.starts_with("-M7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MAJOR_7TH);
            return Some((true, 3));
        }

        // Major 7th (must check before just "maj" or "7")
        if s.starts_with("maj13") || s.starts_with("Maj13") || s.starts_with("M13") {
            *seventh = Some(MAJOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            extensions.push(MAJOR_6TH + 12); // 13
            let len = if s.starts_with("M13") { 3 } else { 5 };
            return Some((true, len));
        }
        if s.starts_with("maj11") || s.starts_with("Maj11") || s.starts_with("M11") {
            *seventh = Some(MAJOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            let len = if s.starts_with("M11") { 3 } else { 5 };
            return Some((true, len));
        }
        if s.starts_with("maj9") || s.starts_with("Maj9") || s.starts_with("M9") {
            *seventh = Some(MAJOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            let len = if s.starts_with("M9") { 2 } else { 4 };
            return Some((true, len));
        }
        if s.starts_with("maj7") || s.starts_with("Maj7") || s.starts_with("M7") {
            *seventh = Some(MAJOR_7TH);
            let len = if s.starts_with("M7") { 2 } else { 4 };
            return Some((true, len));
        }
        if s.starts_with("maj") || s.starts_with("Maj") || s.starts_with("M") && !s.starts_with("M7") && !s.starts_with("M9") {
            // Explicit major (no change from default)
            let len = if s.starts_with("M") && s.len() == 1 { 1 }
                      else if s.starts_with("M") { 1 }
                      else { 3 };
            return Some((true, len));
        }

        // Diminished (must check before minor since dim starts differently)
        if s.starts_with("dim7") {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            *seventh = Some(MAJOR_6TH); // dim7 = 9 semitones (double-flat 7)
            return Some((true, 4));
        }
        if s.starts_with("°7") {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            *seventh = Some(MAJOR_6TH);
            return Some((true, "°7".len())); // ° is 2 bytes + 7 is 1 byte
        }
        if s.starts_with("dim") {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            return Some((true, 3));
        }
        if s.starts_with('°') {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            return Some((true, '°'.len_utf8()));
        }

        // Half-diminished (minor 7 flat 5)
        if s.starts_with("m7b5") || s.starts_with("min7b5") || s.starts_with("-7b5") {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            *seventh = Some(MINOR_7TH);
            let len = if s.starts_with("min") { 6 } else { 4 };
            return Some((true, len));
        }
        if s.starts_with("ø7") {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            *seventh = Some(MINOR_7TH);
            return Some((true, "ø7".len())); // 3 bytes: ø (2) + 7 (1)
        }
        if s.starts_with('ø') {
            *third = Some(MINOR_3RD);
            *fifth = Some(TRITONE);
            *seventh = Some(MINOR_7TH);
            return Some((true, 'ø'.len_utf8())); // 2 bytes
        }

        // Augmented (must check before just "+")
        if s.starts_with("aug7") || s.starts_with("+7") {
            *fifth = Some(MINOR_6TH);
            *seventh = Some(MINOR_7TH);
            let len = if s.starts_with("aug") { 4 } else { 2 };
            return Some((true, len));
        }
        if s.starts_with("aug") || (s.starts_with("+") && !s.starts_with("+5") && !s.starts_with("+7")) {
            *fifth = Some(MINOR_6TH);
            let len = if s.starts_with("aug") { 3 } else { 1 };
            return Some((true, len));
        }

        // Minor variations (must check after mM7, mMaj7, etc.)
        if s.starts_with("min13") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            extensions.push(MAJOR_6TH + 12); // 13
            return Some((true, 5));
        }
        if s.starts_with("min11") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            return Some((true, 5));
        }
        if s.starts_with("min9") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            return Some((true, 4));
        }
        if s.starts_with("min7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            return Some((true, 4));
        }
        if s.starts_with("min6") {
            *third = Some(MINOR_3RD);
            extensions.push(MAJOR_6TH);
            return Some((true, 4));
        }
        if s.starts_with("min") {
            *third = Some(MINOR_3RD);
            return Some((true, 3));
        }

        // Short minor: m13, m11, m9, m7, m6, m (must check after min*)
        if s.starts_with("m13") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            extensions.push(PERFECT_4TH + 12);
            extensions.push(MAJOR_6TH + 12);
            return Some((true, 3));
        }
        if s.starts_with("m11") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            extensions.push(PERFECT_4TH + 12);
            return Some((true, 3));
        }
        if s.starts_with("m9") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            return Some((true, 2));
        }
        if s.starts_with("m7") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            return Some((true, 2));
        }
        if s.starts_with("m6") {
            *third = Some(MINOR_3RD);
            extensions.push(MAJOR_6TH);
            return Some((true, 2));
        }
        // Check for madd9 before m alone
        if s.starts_with("madd9") {
            *third = Some(MINOR_3RD);
            extensions.push(MAJOR_2ND + 12);
            return Some((true, 5));
        }
        if s.starts_with("m") && !s.starts_with("maj") {
            *third = Some(MINOR_3RD);
            return Some((true, 1));
        }

        // Dash minor: -13, -11, -9, -7, -6, -
        if s.starts_with("-13") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            extensions.push(PERFECT_4TH + 12);
            extensions.push(MAJOR_6TH + 12);
            return Some((true, 3));
        }
        if s.starts_with("-11") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            extensions.push(PERFECT_4TH + 12);
            return Some((true, 3));
        }
        if s.starts_with("-9") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12);
            return Some((true, 2));
        }
        if s.starts_with("-7") && !s.starts_with("-7b5") {
            *third = Some(MINOR_3RD);
            *seventh = Some(MINOR_7TH);
            return Some((true, 2));
        }
        if s.starts_with("-6") {
            *third = Some(MINOR_3RD);
            extensions.push(MAJOR_6TH);
            return Some((true, 2));
        }
        if s == "-" {
            *third = Some(MINOR_3RD);
            return Some((true, 1));
        }

        // Suspended chords
        if s.starts_with("sus4") {
            *third = Some(PERFECT_4TH);
            return Some((true, 4));
        }
        if s.starts_with("sus2") {
            *third = Some(MAJOR_2ND);
            return Some((true, 4));
        }
        if s.starts_with("sus") {
            // sus alone usually means sus4
            *third = Some(PERFECT_4TH);
            return Some((true, 3));
        }

        // Add chords (add tone without implying 7th)
        if s.starts_with("add13") {
            extensions.push(MAJOR_6TH + 12);
            return Some((true, 5));
        }
        if s.starts_with("add11") {
            extensions.push(PERFECT_4TH + 12);
            return Some((true, 5));
        }
        if s.starts_with("add9") {
            extensions.push(MAJOR_2ND + 12);
            return Some((true, 4));
        }

        // Altered dominant
        if s.starts_with("alt") || s.starts_with("7alt") {
            *seventh = Some(MINOR_7TH);
            *fifth = Some(MINOR_6TH); // #5
            extensions.push(MINOR_3RD + 12); // #9
            let len = if s.starts_with("7alt") { 4 } else { 3 };
            return Some((true, len));
        }

        // Dominant extensions: 13, 11, 9, 7 (must check after maj*, min*, m* variants)
        if s.starts_with("13") {
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            extensions.push(MAJOR_6TH + 12); // 13
            return Some((true, 2));
        }
        if s.starts_with("11") {
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            extensions.push(PERFECT_4TH + 12); // 11
            return Some((true, 2));
        }
        if s.starts_with("9") {
            *seventh = Some(MINOR_7TH);
            extensions.push(MAJOR_2ND + 12); // 9
            return Some((true, 1));
        }
        if s.starts_with("7") {
            *seventh = Some(MINOR_7TH);
            return Some((true, 1));
        }

        // 6th chord (major with added 6th, no 7th)
        if s.starts_with("6") {
            extensions.push(MAJOR_6TH);
            return Some((true, 1));
        }
    }

    // === Extensions that can appear after base quality ===
    // Check for numbers that add extensions (e.g., Cmaj7add13)
    if s.starts_with("add13") {
        extensions.push(MAJOR_6TH + 12);
        return Some((false, 5));
    }
    if s.starts_with("add11") {
        extensions.push(PERFECT_4TH + 12);
        return Some((false, 5));
    }
    if s.starts_with("add9") {
        extensions.push(MAJOR_2ND + 12);
        return Some((false, 4));
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_triads() {
        // C major: C3, E3, G3
        assert_eq!(parse_chord_symbol("C"), vec![48, 52, 55]);

        // D minor: D3, F3, A3
        assert_eq!(parse_chord_symbol("Dm"), vec![50, 53, 57]);

        // F# major: F#3, A#3, C#4
        assert_eq!(parse_chord_symbol("F#"), vec![54, 58, 61]);

        // Bb minor: Bb3, Db4, F4
        assert_eq!(parse_chord_symbol("Bbm"), vec![58, 61, 65]);
    }

    #[test]
    fn test_seventh_chords() {
        // G7: G3, B3, D4, F4
        assert_eq!(parse_chord_symbol("G7"), vec![55, 59, 62, 65]);

        // Cmaj7: C3, E3, G3, B3
        assert_eq!(parse_chord_symbol("Cmaj7"), vec![48, 52, 55, 59]);

        // Am7: A3, C4, E4, G4
        assert_eq!(parse_chord_symbol("Am7"), vec![57, 60, 64, 67]);

        // CmM7: C3, Eb3, G3, B3
        assert_eq!(parse_chord_symbol("CmM7"), vec![48, 51, 55, 59]);
    }

    #[test]
    fn test_altered_dominants() {
        // A7b9: A, C#, E, G, Bb(+12)
        let a7b9 = parse_chord_symbol("A7b9");
        assert_eq!(a7b9[0], 57); // A
        assert_eq!(a7b9[1], 61); // C#
        assert_eq!(a7b9[2], 64); // E
        assert_eq!(a7b9[3], 67); // G (b7)
        assert_eq!(a7b9[4], 70); // Bb (b9 = +13 semitones from root)

        // G7#9: G, B, D, F, A#(+12)
        let g7sharp9 = parse_chord_symbol("G7#9");
        assert_eq!(g7sharp9[0], 55); // G
        assert_eq!(g7sharp9[1], 59); // B
        assert_eq!(g7sharp9[2], 62); // D
        assert_eq!(g7sharp9[3], 65); // F
        assert_eq!(g7sharp9[4], 70); // A# (#9 = +15 semitones from root)

        // C7b5: C, E, Gb, Bb
        let c7b5 = parse_chord_symbol("C7b5");
        assert_eq!(c7b5, vec![48, 52, 54, 58]);

        // C7#5: C, E, G#, Bb
        let c7sharp5 = parse_chord_symbol("C7#5");
        assert_eq!(c7sharp5, vec![48, 52, 56, 58]);
    }

    #[test]
    fn test_complex_alterations() {
        // E7b9#11: E, G#, B, D, F(b9), A#(#11)
        let e7b9sharp11 = parse_chord_symbol("E7b9#11");
        assert!(e7b9sharp11.contains(&52)); // E
        assert!(e7b9sharp11.contains(&56)); // G#
        assert!(e7b9sharp11.contains(&59)); // B
        assert!(e7b9sharp11.contains(&62)); // D (b7)
        assert!(e7b9sharp11.contains(&65)); // F (b9)
        assert!(e7b9sharp11.contains(&70)); // A# (#11)

        // D7#9b13
        let d7sharp9b13 = parse_chord_symbol("D7#9b13");
        assert!(d7sharp9b13.contains(&50)); // D
        assert!(d7sharp9b13.contains(&54)); // F#
        assert!(d7sharp9b13.contains(&57)); // A
        assert!(d7sharp9b13.contains(&60)); // C (b7)
        assert!(d7sharp9b13.contains(&65)); // F (#9)
        assert!(d7sharp9b13.contains(&70)); // Bb (b13)
    }

    #[test]
    fn test_diminished_and_half_diminished() {
        // Cdim: C, Eb, Gb
        assert_eq!(parse_chord_symbol("Cdim"), vec![48, 51, 54]);

        // Cdim7: C, Eb, Gb, Bbb(A)
        assert_eq!(parse_chord_symbol("Cdim7"), vec![48, 51, 54, 57]);

        // Cm7b5 (half-dim): C, Eb, Gb, Bb
        assert_eq!(parse_chord_symbol("Cm7b5"), vec![48, 51, 54, 58]);

        // Using ø symbol
        assert_eq!(parse_chord_symbol("Cø"), vec![48, 51, 54, 58]);
    }

    #[test]
    fn test_extended_chords() {
        // C9: C, E, G, Bb, D(+12)
        let c9 = parse_chord_symbol("C9");
        assert_eq!(c9[0], 48); // C
        assert_eq!(c9[1], 52); // E
        assert_eq!(c9[2], 55); // G
        assert_eq!(c9[3], 58); // Bb
        assert_eq!(c9[4], 62); // D (9)

        // Cmaj9
        let cmaj9 = parse_chord_symbol("Cmaj9");
        assert_eq!(cmaj9[3], 59); // B (maj7)
        assert_eq!(cmaj9[4], 62); // D (9)

        // C13
        let c13 = parse_chord_symbol("C13");
        assert!(c13.contains(&48)); // C
        assert!(c13.contains(&52)); // E
        assert!(c13.contains(&55)); // G
        assert!(c13.contains(&58)); // Bb (7)
        assert!(c13.contains(&62)); // D (9)
        assert!(c13.contains(&65)); // F (11)
        assert!(c13.contains(&69)); // A (13)
    }

    #[test]
    fn test_sus_chords() {
        // Csus4: C, F, G
        assert_eq!(parse_chord_symbol("Csus4"), vec![48, 53, 55]);

        // Csus2: C, D, G
        assert_eq!(parse_chord_symbol("Csus2"), vec![48, 50, 55]);
    }

    #[test]
    fn test_add_chords() {
        // Cadd9: C, E, G, D(+12)
        let cadd9 = parse_chord_symbol("Cadd9");
        assert_eq!(cadd9[0], 48);
        assert_eq!(cadd9[1], 52);
        assert_eq!(cadd9[2], 55);
        assert_eq!(cadd9[3], 62); // D (no 7th, just added 9)

        // Cmadd9
        let cmadd9 = parse_chord_symbol("Cmadd9");
        assert_eq!(cmadd9[0], 48);
        assert_eq!(cmadd9[1], 51); // Eb (minor 3rd)
        assert_eq!(cmadd9[2], 55);
        assert_eq!(cmadd9[3], 62);
    }

    #[test]
    fn test_sixth_chords() {
        // C6: C, E, G, A
        assert_eq!(parse_chord_symbol("C6"), vec![48, 52, 55, 57]);

        // Cm6: C, Eb, G, A
        assert_eq!(parse_chord_symbol("Cm6"), vec![48, 51, 55, 57]);
    }

    #[test]
    fn test_slash_chords() {
        // C/E: E in bass, then C, G
        let c_over_e = parse_chord_symbol("C/E");
        assert_eq!(c_over_e[0], 52); // E (bass)
        assert!(c_over_e.contains(&48)); // C
        assert!(c_over_e.contains(&55)); // G

        // Am/G
        let am_over_g = parse_chord_symbol("Am/G");
        assert_eq!(am_over_g[0], 55); // G (bass)
    }

    #[test]
    fn test_sharp_11_chords() {
        // Cmaj7#11
        let cmaj7sharp11 = parse_chord_symbol("Cmaj7#11");
        assert!(cmaj7sharp11.contains(&48)); // C
        assert!(cmaj7sharp11.contains(&52)); // E
        assert!(cmaj7sharp11.contains(&55)); // G
        assert!(cmaj7sharp11.contains(&59)); // B (maj7)
        assert!(cmaj7sharp11.contains(&66)); // F# (#11)
    }
}
