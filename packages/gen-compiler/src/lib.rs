//! # Gen Music Notation Compiler
//!
//! A text-based music notation language compiler that generates industry-standard MusicXML.
//!
//! ## Compilation Pipeline
//!
//! ```text
//! .gen source → Lexer → Parser → Semantic → MusicXML Generator → .musicxml
//! ```
//!
//! 1. **Lexer** ([`lexer`]) - Tokenizes Gen source into tokens with location info
//! 2. **Parser** ([`parser`]) - Parses tokens into Abstract Syntax Tree
//!    - First pass: Extract metadata, mod points, key changes, chord annotations
//!    - Second pass: Parse music with context from first pass
//! 3. **Semantic** ([`semantic`]) - Validates AST (measure durations, repeats, endings)
//! 4. **MusicXML Generator** ([`musicxml`]) - Generates MusicXML output
//! 5. **Playback** (this module) - Optional MIDI playback data generation
//!
//! ## Quick Start
//!
//! ```rust
//! use gen::compile;
//!
//! let source = r#"---
//! title: My Song
//! composer: Me
//! time-signature: 4/4
//! key-signature: C
//! tempo: 120
//! ---
//! C D E F
//! G A B C^
//! "#;
//!
//! let musicxml = compile(source)?;
//! // Write musicxml to file or render with notation software
//! # Ok::<(), gen::GenError>(())
//! ```
//!
//! ## Public API Entry Points
//!
//! ### Compilation Functions
//! - [`compile()`] - Full compilation with validation (recommended)
//! - [`compile_unchecked()`] - Skip validation (for partial/incomplete scores)
//! - [`compile_with_options()`] - Custom clef, octave shift, transposition
//! - [`compile_with_mod_points()`] - Instrument-specific rendering with mod points
//!
//! ### Playback Functions
//! - [`generate_playback_data()`] - Generate MIDI playback data with timing info
//!
//! ### Low-Level API
//! - [`parse()`] - Parse Gen source into AST
//! - [`validate()`] - Validate AST semantic correctness
//! - [`to_musicxml()`] - Generate MusicXML from AST
//!
//! ## Gen Language Syntax Overview
//!
//! ### Note Format
//! `[rhythm][note][pitch]`
//!
//! - **Rhythm modifiers**: `/` (eighth), `//` (sixteenth), `d` (half), `o` (whole), `*` (dotted)
//! - **Notes**: A-G or `$` (rest)
//! - **Pitch modifiers**: `#` (sharp), `b` (flat), `^` (octave up), `_` (octave down)
//!
//! ### Examples
//! - `C` - C quarter note
//! - `/E` - E eighth note
//! - `dG*` - G dotted half note
//! - `//F#^` - F# sixteenth note, one octave up
//! - `$` - quarter rest
//!
//! ### Tuplets
//! - `3[C D E]` - Quarter note triplet (3 notes in time of 2)
//! - `/3[A B C]` - Eighth note triplet
//! - `5[C D E F G]` - Quintuplet (5 in time of 4)
//!
//! ### Ties and Slurs
//! - `C-C` - Two tied quarter notes (play as half note)
//! - `(C D E F)` - Slurred phrase
//!
//! ### Repeats and Endings
//! - `||:` - Repeat start
//! - `:||` - Repeat end
//! - `|1` - First ending
//! - `|2` - Second ending
//!
//! ## Module Structure
//!
//! - [`ast`] - Abstract Syntax Tree type definitions (Score, Measure, Note, etc.)
//! - [`error`] - Error types (GenError variants)
//! - [`lexer`] - Tokenization (String → Vec<Token>)
//! - [`parser`] - Parsing (Vec<Token> → Score AST)
//! - [`semantic`] - Validation (measure durations, repeats)
//! - [`musicxml`] - MusicXML generation (Score → MusicXML string)
//!
//! ## Additional Resources
//!
//! - **ARCHITECTURE.md** - Detailed architectural documentation for agents
//! - **CLAUDE.md** - High-level project overview and language syntax
//! - **gen-docs** - Complete language documentation and examples
//!
//! ## Features
//!
//! - ✅ All standard music notation (notes, rests, ties, slurs, tuplets)
//! - ✅ Full key signature support (major and minor keys)
//! - ✅ Any time signature (4/4, 3/4, 6/8, 5/4, 7/8, etc.)
//! - ✅ Instrument transposition (Bb, Eb, F)
//! - ✅ Chord symbols for lead sheets
//! - ✅ Repeats and endings
//! - ✅ Mid-score key changes
//! - ✅ Automatic beaming
//! - ✅ MIDI playback data generation
//! - ✅ Integration with OpenSheetMusicDisplay (OSMD)

// Core modules
pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod musicxml;
pub mod playback;

// Public API
pub mod api;

// Re-export core types
pub use ast::*;
pub use error::*;

// Re-export pipeline functions
pub use parser::parse;
pub use semantic::validate;
pub use musicxml::{to_musicxml, to_musicxml_with_options, to_musicxml_with_mod_points, Clef, Transposition};

// Re-export playback functions
pub use playback::{generate_playback_data, PlaybackData, PlaybackNote, PlaybackChord, TieType};

// Re-export API functions for convenience
pub use api::{compile, compile_unchecked, compile_with_options, compile_with_mod_points};

