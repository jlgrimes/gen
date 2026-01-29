//! Chord symbol parsing for MIDI playback
//!
//! Parses chord symbols (C, Am, G7, Dm7, etc.) into MIDI note arrays for accompaniment.

/// Parse a chord symbol into MIDI notes
///
/// Returns a Vec of MIDI note numbers using common jazz/pop chord voicings.
/// Chords are voiced in the C3 octave (MIDI 48-59) for bass/piano accompaniment.
///
/// # Supported Chord Types
/// - **Major**: `C`, `maj`, `M` → root, major 3rd, perfect 5th
/// - **Minor**: `m`, `min`, `-` → root, minor 3rd, perfect 5th
/// - **Dominant 7th**: `7` → root, major 3rd, perfect 5th, minor 7th
/// - **Major 7th**: `maj7`, `M7` → root, major 3rd, perfect 5th, major 7th
/// - **Minor 7th**: `m7`, `min7`, `-7` → root, minor 3rd, perfect 5th, minor 7th
/// - **Diminished**: `dim`, `°` → root, minor 3rd, diminished 5th
/// - **Augmented**: `aug`, `+` → root, major 3rd, augmented 5th
/// - **Sus4**: `sus4` → root, perfect 4th, perfect 5th
/// - **Sus2**: `sus2` → root, major 2nd, perfect 5th
/// - **9th chords**: `9`, `maj9`, `m9` → 7th chord + major 9th
/// - **Slash chords**: `C/E`, `Am/G` → chord with specified bass note
///
/// # Examples
/// ```
/// use gen::playback::parse_chord_symbol;
///
/// // C major: C3, E3, G3
/// assert_eq!(parse_chord_symbol("C"), vec![48, 52, 55]);
///
/// // D minor: D3, F3, A3
/// assert_eq!(parse_chord_symbol("Dm"), vec![50, 53, 57]);
///
/// // G7: G3, B3, D4, F4
/// assert_eq!(parse_chord_symbol("G7"), vec![55, 59, 62, 65]);
///
/// // F# major: F#3, A#3, C#4
/// assert_eq!(parse_chord_symbol("F#"), vec![54, 58, 61]);
///
/// // C/E (C major with E in bass): E3, C3, G3
/// assert_eq!(parse_chord_symbol("C/E"), vec![52, 48, 55]);
/// ```
///
/// # MIDI Note Reference
/// - C3 = 48, D3 = 50, E3 = 52, F3 = 53, G3 = 55, A3 = 57, B3 = 59
/// - Intervals: minor 3rd = +3, major 3rd = +4, perfect 5th = +7, minor 7th = +10, major 7th = +11
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

    // Check for accidental
    let accidental = if idx < chars.len() && (chars[idx] == '#' || chars[idx] == 'b') {
        idx += 1;
        if chars[idx - 1] == '#' { 1 } else { -1 }
    } else {
        0
    };

    // Base MIDI note for root (C4 = 60, but we'll use C3 = 48 for chords)
    let base_midi = match root_name {
        'C' => 48,
        'D' => 50,
        'E' => 52,
        'F' => 53,
        'G' => 55,
        'A' => 57,
        'B' => 59,
        _ => return vec![],
    };
    let root = (base_midi + accidental) as u8;

    // Parse chord quality from remaining string
    let quality = &main_chord[idx..];

    // Get chord tones (without bass modification)
    let mut chord_tones = match quality {
        // Major triads
        "" | "maj" | "M" => vec![root, root + 4, root + 7],

        // Minor triads
        "m" | "min" | "-" => vec![root, root + 3, root + 7],

        // Dominant 7th
        "7" => vec![root, root + 4, root + 7, root + 10],

        // Major 7th
        "maj7" | "M7" => vec![root, root + 4, root + 7, root + 11],

        // Minor 7th
        "m7" | "min7" | "-7" => vec![root, root + 3, root + 7, root + 10],

        // Diminished
        "dim" | "°" => vec![root, root + 3, root + 6],

        // Augmented
        "aug" | "+" => vec![root, root + 4, root + 8],

        // Sus chords
        "sus4" => vec![root, root + 5, root + 7],
        "sus2" => vec![root, root + 2, root + 7],

        // Extended chords
        "9" => vec![root, root + 4, root + 7, root + 10, root + 14],
        "maj9" | "M9" => vec![root, root + 4, root + 7, root + 11, root + 14],
        "m9" | "min9" => vec![root, root + 3, root + 7, root + 10, root + 14],

        // Add9 chords (triad + 9th, no 7th)
        "add9" => vec![root, root + 4, root + 7, root + 14],
        "madd9" => vec![root, root + 3, root + 7, root + 14],

        // Augmented 7th (dominant 7th with raised 5th)
        "7#5" | "7+5" | "aug7" => vec![root, root + 4, root + 8, root + 10],

        // Dominant 7th with flat 5
        "7b5" => vec![root, root + 4, root + 6, root + 10],

        // Default to major if unknown
        _ => vec![root, root + 4, root + 7],
    };

    // Handle slash chord - add bass note an octave below
    if let Some(bass_str) = bass_note {
        let bass_chars: Vec<char> = bass_str.chars().collect();
        if !bass_chars.is_empty() {
            let bass_name = bass_chars[0];
            let bass_accidental = if bass_chars.len() > 1 && (bass_chars[1] == '#' || bass_chars[1] == 'b') {
                if bass_chars[1] == '#' { 1 } else { -1 }
            } else {
                0
            };

            // Bass note in C3 octave (same octave as chord voicing, but will be lowest)
            let bass_base = match bass_name {
                'C' => 48,  // C3
                'D' => 50,
                'E' => 52,
                'F' => 53,
                'G' => 55,
                'A' => 57,
                'B' => 59,
                _ => 48,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chord_parsing() {
        // Test major chord
        let c_major = parse_chord_symbol("C");
        assert_eq!(c_major, vec![48, 52, 55]); // C3, E3, G3

        // Test minor chord
        let d_minor = parse_chord_symbol("Dm");
        assert_eq!(d_minor, vec![50, 53, 57]); // D3, F3, A3

        // Test dominant 7th
        let g7 = parse_chord_symbol("G7");
        assert_eq!(g7, vec![55, 59, 62, 65]); // G3, B3, D4, F4

        // Test major 7th
        let cmaj7 = parse_chord_symbol("Cmaj7");
        assert_eq!(cmaj7, vec![48, 52, 55, 59]); // C3, E3, G3, B3

        // Test with accidentals
        let f_sharp_major = parse_chord_symbol("F#");
        assert_eq!(f_sharp_major, vec![54, 58, 61]); // F#3, A#3, C#4

        let b_flat_minor = parse_chord_symbol("Bbm");
        assert_eq!(b_flat_minor, vec![58, 61, 65]); // Bb3, Db4, F4
    }

    #[test]
    fn test_slash_chords() {
        // C/E - C major with E in bass (first inversion)
        // E3 (52) moves to front, removed from original position
        let c_over_e = parse_chord_symbol("C/E");
        assert_eq!(c_over_e, vec![52, 48, 55]); // E3, C3, G3

        // C/G - C major with G in bass (second inversion)
        let c_over_g = parse_chord_symbol("C/G");
        assert_eq!(c_over_g, vec![55, 48, 52]); // G3, C3, E3

        // Am/G - A minor with G in bass
        // G3 (55) added to front, not in original Am chord
        let am_over_g = parse_chord_symbol("Am/G");
        assert_eq!(am_over_g, vec![55, 57, 60, 64]); // G3, A3, C4, E4

        // D/F# - D major with F# in bass
        let d_over_fsharp = parse_chord_symbol("D/F#");
        assert_eq!(d_over_fsharp, vec![54, 50, 57]); // F#3, D3, A3

        // C/Ab - C major with Ab in bass (the original user request!)
        let c_over_ab = parse_chord_symbol("C/Ab");
        assert_eq!(c_over_ab, vec![56, 48, 52, 55]); // Ab3, C3, E3, G3
    }
}
