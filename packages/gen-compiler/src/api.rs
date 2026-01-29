//! # Public API
//!
//! This module contains the main entry points for the Gen compiler library.
//!
//! ## Compilation Functions
//!
//! - [`compile()`] - Full compilation with validation (recommended for complete scores)
//! - [`compile_unchecked()`] - Skip validation (useful for partial/incomplete scores)
//! - [`compile_with_options()`] - Custom clef, octave shift, and transposition
//! - [`compile_with_mod_points()`] - Instrument-specific rendering with mod points
//!
//! ## Typical Usage
//!
//! ```rust
//! use gen::compile;
//!
//! let source = r#"---
//! title: My Song
//! composer: Me
//! ---
//! C D E F
//! G A B C^
//! "#;
//!
//! let musicxml = compile(source)?;
//! // Write to .musicxml file or render with notation software
//! # Ok::<(), gen::GenError>(())
//! ```
//!
//! ## Advanced Usage
//!
//! For transposing instruments or custom clefs:
//!
//! ```rust
//! use gen::{compile_with_options, Transposition};
//!
//! let source = "C D E F";
//!
//! // Compile for Bb clarinet (transposes down a major 2nd)
//! let musicxml = compile_with_options(
//!     source,
//!     "treble",
//!     0,
//!     Transposition::for_key("Bb")
//! )?;
//! # Ok::<(), gen::GenError>(())
//! ```

use crate::{
    parse, to_musicxml, to_musicxml_with_mod_points, to_musicxml_with_options, validate, Clef,
    GenError, InstrumentGroup, Transposition,
};

/// Compile a Gen source string to MusicXML.
///
/// This is the main entry point for the library. It performs full compilation with validation.
///
/// # Pipeline
/// 1. Tokenize source with lexer
/// 2. Parse tokens into AST
/// 3. Validate AST (measure durations, repeats, endings)
/// 4. Generate MusicXML output
///
/// # Example
/// ```rust
/// use gen::compile;
///
/// let source = "C D E F";
/// let musicxml = compile(source)?;
/// // Write to .musicxml file or render
/// # Ok::<(), gen::GenError>(())
/// ```
///
/// # Errors
/// Returns [`GenError`] if parsing, validation, or generation fails.
pub fn compile(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    validate(&score)?;
    Ok(to_musicxml(&score))
}

/// Compile without validation (useful for partial/incomplete scores).
///
/// Skips semantic validation, allowing compilation of scores with:
/// - Incomplete measures (duration doesn't match time signature)
/// - Unmatched repeat markers
/// - Invalid ending structures
///
/// # Example
/// ```rust
/// use gen::compile_unchecked;
///
/// // Incomplete measure (only 3 beats in 4/4 time)
/// let source = "C D E";
/// let musicxml = compile_unchecked(source)?;
/// # Ok::<(), gen::GenError>(())
/// ```
pub fn compile_unchecked(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    Ok(to_musicxml(&score))
}

/// Compile with custom clef, octave shift, and transposition options.
///
/// # Parameters
/// - `source` - Gen source code
/// - `clef` - "treble" or "bass"
/// - `octave_shift` - Octave adjustment (-2 to +2)
/// - `transposition` - Optional transposition for instruments (Bb, Eb, F)
///
/// # Example
/// ```rust
/// use gen::{compile_with_options, Transposition};
///
/// let source = "C D E F";
///
/// // Bass clef, down one octave, for Bb instrument
/// let musicxml = compile_with_options(
///     source,
///     "bass",
///     -1,
///     Transposition::for_key("Bb")
/// )?;
/// # Ok::<(), gen::GenError>(())
/// ```
pub fn compile_with_options(
    source: &str,
    clef: &str,
    octave_shift: i8,
    transposition: Option<Transposition>,
) -> Result<String, GenError> {
    let score = parse(source)?;
    let clef = match clef {
        "bass" => Clef::Bass,
        _ => Clef::Treble,
    };
    Ok(to_musicxml_with_options(
        &score,
        transposition,
        clef,
        octave_shift,
    ))
}

/// Compile with mod points support for instrument-specific octave shifts.
///
/// Mod points (`@Eb:^`, `@Bb:_`) allow different instruments to render the same
/// score with different octave shifts based on their instrument group.
///
/// # Parameters
/// - `source` - Gen source code with optional mod points
/// - `clef` - "treble" or "bass"
/// - `octave_shift` - Base octave adjustment
/// - `instrument_group` - "eb", "bb", or None
/// - `transpose_key` - "C" (concert pitch), "Bb", "Eb", or "F"
///
/// # Example
/// ```rust
/// use gen::compile_with_mod_points;
///
/// let source = r#"
/// C D @Eb:^ E F
/// "#;
///
/// // Compile for Eb alto sax (applies @Eb: mod points)
/// let musicxml = compile_with_mod_points(
///     source,
///     "treble",
///     0,
///     Some("eb"),
///     Some("Eb")
/// )?;
/// # Ok::<(), gen::GenError>(())
/// ```
pub fn compile_with_mod_points(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
    transpose_key: Option<&str>,
) -> Result<String, GenError> {
    let score = parse(source)?;
    let clef = match clef {
        "bass" => Clef::Bass,
        _ => Clef::Treble,
    };
    let group = instrument_group.and_then(InstrumentGroup::from_str);
    let transposition = transpose_key.and_then(Transposition::for_key);
    Ok(to_musicxml_with_mod_points(
        &score,
        transposition,
        clef,
        octave_shift,
        group,
    ))
}
