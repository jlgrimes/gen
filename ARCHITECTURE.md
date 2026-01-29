# Gen Compiler Architecture

**For AI Agents**: This document provides a complete architectural overview of the Gen music notation compiler. Read this first to understand the compilation pipeline, module structure, and how to locate functionality.

---

## Quick Reference

- **Entry point**: `packages/gen-compiler/src/api.rs::compile()`
- **Pipeline**: lexer → parser → semantic → musicxml
- **Data flow**: `.gen source` → `Vec<Token>` → `Score (AST)` → `MusicXML string`
- **Public API**: All in `api.rs`, re-exported through `lib.rs`

---

## Compilation Pipeline

```
┌─────────────────────────────────────────────────────────────────┐
│                     Gen Compiler Pipeline                        │
└─────────────────────────────────────────────────────────────────┘

  .gen file                                            .musicxml
  (Gen source)                                         (MusicXML)
      │                                                     ▲
      │  "C D E F"                                         │
      ▼                                                     │
┌──────────┐      ┌──────────┐      ┌──────────┐         │
│  Lexer   │─────▶│  Parser  │─────▶│ Semantic │─────────┘
│          │Tokens│          │ AST  │ Validator│ Valid AST
│ lexer.rs │      │ parser.rs│      │ semantic │
└──────────┘      └──────────┘      │   .rs    │
                                     └────┬─────┘
                                          │
                                          ▼
                                    ┌──────────┐
                                    │ MusicXML │
                                    │Generator │
                                    │musicxml  │
                                    │   .rs    │
                                    └──────────┘

Optional Playback Path:
  AST ──────▶ [ Playback Engine ] ───▶ PlaybackData (MIDI)
              lib.rs::generate_playback_data()
```

**Pipeline Steps**:

1. **Lexer** (`lexer.rs`) - Tokenizes Gen source into tokens
   - Input: `&str` (Gen source code)
   - Output: `Vec<LocatedToken>` (tokens with line/column info)
   - Key function: `Lexer::tokenize()`

2. **Parser** (`parser.rs`) - Parses tokens into Abstract Syntax Tree
   - Input: `Vec<LocatedToken>`
   - Output: `Score` (AST with metadata + measures)
   - Key function: `parse()`
   - Two-pass algorithm:
     - First pass: Extract metadata, mod points, key changes, chord annotations
     - Second pass: Parse music with context from first pass

3. **Semantic Validator** (`semantic.rs`) - Validates AST correctness
   - Input: `&Score`
   - Output: `Result<(), GenError>`
   - Validates: Measure durations match time signature, repeat matching, ending structure

4. **MusicXML Generator** (`musicxml.rs`) - Generates MusicXML output
   - Input: `&Score`
   - Output: `String` (MusicXML 3.1 format)
   - Key functions: `to_musicxml()`, `to_musicxml_with_options()`
   - Features: Transposition, beaming, chord symbols

5. **Playback Engine** (optional, `lib.rs`) - Generates MIDI playback data
   - Input: `&str` (Gen source)
   - Output: `PlaybackData` (MIDI notes + timing + OSMD matching)
   - Key function: `generate_playback_data()`

---

## Module Map

### Core Compiler (`packages/gen-compiler/src/`)

```
gen-compiler/src/
├── lib.rs              # Public API exports + playback generation
├── ast.rs              # Type definitions (Score, Measure, Note, etc.)
├── error.rs            # Error types (GenError variants)
├── lexer.rs            # Tokenization (String → Vec<Token>)
├── parser.rs           # Parsing (Vec<Token> → Score AST)
├── semantic.rs         # Validation (measure durations, repeats)
└── musicxml.rs         # Generation (Score → MusicXML)
```

#### **lib.rs** (920 lines)
- **Purpose**: Main library entry point
- **Contents**:
  - Public API: `compile()`, `compile_unchecked()`, `compile_with_options()`
  - Playback types: `PlaybackNote`, `PlaybackChord`, `PlaybackData`, `TieType`
  - Playback generation: `generate_playback_data()`, `parse_chord_symbol()`
  - Module re-exports
  - Integration tests (600+ lines)
- **Will be refactored to**: Architectural "map" (~80 lines) + separate playback/ module

#### **ast.rs** (483 lines)
- **Purpose**: Define all Abstract Syntax Tree types
- **Key types**:
  - `Score` - Complete parsed score (metadata + measures + mod points)
  - `Metadata` - YAML frontmatter (title, composer, key, time signature, tempo)
  - `Measure` - Single measure (elements, repeat markers, endings)
  - `Element` - Note or Rest (enum)
  - `Note` - Musical note (pitch, duration, accidentals, ties, slurs, tuplets)
  - `Duration` - Rhythm values (whole, half, quarter, eighth, sixteenth, 32nd)
  - `KeySignature`, `TimeSignature`, `Accidental`, `Octave`, etc.
- **Related modules**: Used by parser, semantic, musicxml, playback

#### **error.rs** (17 lines)
- **Purpose**: Error type definitions
- **Key type**: `GenError` enum with variants:
  - `ParseError` - Parsing errors with line/column info
  - `MetadataError` - Invalid YAML metadata
  - `SemanticError` - Validation errors with measure info

#### **lexer.rs** (658 lines)
- **Purpose**: Tokenize Gen source into tokens
- **Key types**:
  - `Token` - Token types (Note, Rest, Duration modifiers, etc.)
  - `LocatedToken` - Token + line/column location
  - `Lexer` - Tokenizer state machine
- **Entry point**: `Lexer::tokenize(source: &str) -> Vec<LocatedToken>`
- **Tests**: 200 lines inline

#### **parser.rs** (2,574 lines) ⚠️ LARGE
- **Purpose**: Parse tokens into AST
- **Key types**:
  - `Parser` - Parser state (tokens, position, annotations)
  - `ChordAnnotations` - Chord symbol tracking
  - `TupletContext` - Tuplet parsing context
- **Entry point**: `parse(source: &str) -> Result<Score, GenError>`
- **Sub-concerns** (to be split):
  - Metadata parsing (YAML frontmatter)
  - Music parsing (notes, measures, elements)
  - Tuplet/bracket group parsing
  - Tie/slur handling
  - Repeat and ending markers
  - Mod point tracking
- **Tests**: 1,500+ lines inline
- **Will be refactored to**: `parser/` directory with sub-modules

#### **semantic.rs** (290 lines)
- **Purpose**: Validate AST correctness
- **Entry point**: `validate(score: &Score) -> Result<(), GenError>`
- **Validates**:
  - Measure durations match time signature
  - Repeat start/end matching
  - Ending structure (first/second endings)
- **Tests**: 125 lines inline

#### **musicxml.rs** (2,011 lines) ⚠️ LARGE
- **Purpose**: Generate MusicXML from AST
- **Key types**:
  - `Clef` - Treble or Bass
  - `Transposition` - Instrument transposition (Bb, Eb, F)
  - `BeamState` - Beam calculation for grouping
- **Entry points**:
  - `to_musicxml(score: &Score) -> String`
  - `to_musicxml_with_options()` - Custom clef/octave/transposition
  - `to_musicxml_with_mod_points()` - Instrument groups
- **Sub-concerns** (to be split):
  - Core XML generation
  - Transposition logic
  - Beaming calculations
  - Measure and note XML writing
  - Attribute generation (key, time, clef)
- **Tests**: 800+ lines inline
- **Will be refactored to**: `musicxml/` directory with sub-modules

---

## Module Dependency Graph

```
                   ┌─────────┐
                   │  lib.rs │ (Public API + Playback)
                   └────┬────┘
                        │
           ┌────────────┼────────────┐
           │            │            │
        ┌──▼──┐      ┌──▼──┐     ┌──▼──────┐
        │lexer│      │ ast │     │ error   │
        └──┬──┘      └──┬──┘     └────▲────┘
           │            │              │
           └────────┬───┴──────────────┤
                 ┌──▼──────┐           │
                 │ parser  │───────────┤
                 └──┬──────┘           │
                    │                  │
                 ┌──▼──────┐           │
                 │semantic │───────────┤
                 └──┬──────┘           │
                    │                  │
                 ┌──▼──────┐           │
                 │musicxml │───────────┘
                 └─────────┘

Legend:
  ─▶ depends on / imports from
  │  data flow
```

**Import rules**:
- `lib.rs` re-exports all public APIs
- All modules import from `ast` and `error`
- Pipeline flows: lexer → parser → semantic → musicxml
- No circular dependencies

---

## Common Agent Tasks

### "Add a new metadata field"

**Files to modify**:
1. `ast.rs` - Add field to `RawMetadata` struct
2. `parser.rs` - Parse field in metadata extraction logic
3. `musicxml.rs` - Update XML output if field affects rendering (optional)
4. `gen-docs/v1/gen basics.md` - Document the new field

**Example**: Adding a "subtitle" field
```rust
// ast.rs
pub struct RawMetadata {
    pub title: Option<String>,
    pub subtitle: Option<String>,  // NEW
    pub composer: Option<String>,
    // ...
}

// parser.rs (in metadata parsing)
if key == "subtitle" {
    raw_metadata.subtitle = Some(value.to_string());
}

// musicxml.rs (if rendering subtitle)
if let Some(subtitle) = &score.metadata.subtitle {
    writeln!(xml, "    <credit-words>{}</credit-words>", subtitle)?;
}
```

---

### "Add a new rhythm modifier"

**Files to modify**:
1. `lexer.rs` - Add token to `Token` enum, recognize in tokenizer
2. `ast.rs` - Add duration to `Duration` enum, add `musicxml_type()` mapping
3. `parser.rs` - Parse the token into the new duration
4. `gen-docs/v1/gen basics.md` - Document the syntax

**Example**: Adding a "triplet whole note" (hypothetical)
```rust
// lexer.rs
pub enum Token {
    // ...
    TripletWhole,  // NEW: represented as "t" in source
}

// ast.rs
pub enum Duration {
    // ...
    TripletWhole,  // NEW
}

impl Duration {
    pub fn musicxml_type(&self) -> &'static str {
        match self {
            Duration::TripletWhole => "whole",  // NEW
            // ...
        }
    }
}

// parser.rs (in note parsing)
Token::TripletWhole => duration = Duration::TripletWhole,
```

---

### "Understand triplet handling"

**Locations**:
- **Parsing**: `parser.rs` - Look for `parse_bracket_group()`, `TupletInfo`
- **AST**: `ast.rs` - `TupletInfo` struct (actual_notes, normal_notes)
- **XML generation**: `musicxml.rs` - Search for "time-modification", `<tuplet>` tags
- **Playback timing**: `lib.rs` - `generate_playback_data()` calculates triplet duration adjustments
- **Tests**: `lib.rs` - Search for "triplet" in test names

**Key algorithm**: Triplets have `actual_notes = 3`, `normal_notes = 2`
- Duration calculation: `(normal_notes / actual_notes) * base_duration`
- Example: Quarter note triplet = `(2/3) * 1.0 = 0.667 beats`

---

### "Add instrument transposition support"

**Locations**:
- **Transposition logic**: `musicxml.rs` - `Transposition` struct, `for_key()` constructor
- **Instrument groups**: `ast.rs` - `InstrumentGroup` enum (Eb, Bb, F)
- **API**: `lib.rs` - `compile_with_options()`, `compile_with_mod_points()`
- **Mod points**: `parser.rs` - Look for "@Eb:", "@Bb:" syntax parsing

**How it works**:
1. Parse mod points (`@Eb:^`) which specify instrument-specific octave shifts
2. Create `Transposition` for instrument key (Bb, Eb, F)
3. Apply transposition during MusicXML generation
4. Notes are written in transposed key, but playback uses concert pitch

---

### "Debug playback timing issues"

**Locations**:
- **Playback engine**: `lib.rs` - `generate_playback_data()` function (~130 lines)
- **MIDI conversion**: `ast.rs` - `Note::to_midi_note()` method
- **Timing tests**: `lib.rs` - Tests like `test_playback_triplets`, `test_osmd_match_keys`
- **OSMD matching**: `lib.rs` - `PlaybackNote::osmd_match_key` field calculation

**Key concepts**:
- **Playback time**: Actual duration in beats (triplet-adjusted)
- **OSMD time**: Display duration (MusicXML quantized, not triplet-adjusted)
- **MIDI notes**: `midi_note` = concert pitch, `display_midi_note` = transposed for sheet music
- **Ties**: Only the first note in a tied group plays audio, others are visual only

---

### "Find where X syntax is parsed"

**Strategy**:
1. Search `lexer.rs` for the symbol/character (tokenization)
2. Search `parser.rs` for the token type (parsing into AST)
3. Check `ast.rs` for the corresponding type
4. Look in tests for examples of the syntax

**Example**: Finding "slur" syntax (`(` and `)`)
```bash
# Search for slur tokens
grep -n "Slur" packages/gen-compiler/src/lexer.rs

# Search for slur parsing
grep -n "slur" packages/gen-compiler/src/parser.rs

# Find slur in AST
grep -n "slur" packages/gen-compiler/src/ast.rs

# Find tests
grep -n "slur" packages/gen-compiler/src/parser.rs | grep "#\[test\]"
```

---

## Data Structures

### Score (Top-level AST)
```rust
pub struct Score {
    pub metadata: Metadata,           // Title, composer, key, time sig, tempo
    pub measures: Vec<Measure>,       // All measures in the score
    pub mod_points: ModPoints,        // Per-line octave shifts for instruments
    pub line_to_measure: HashMap<usize, usize>, // Line number → measure index
}
```

### Metadata
```rust
pub struct Metadata {
    pub title: Option<String>,
    pub composer: Option<String>,
    pub key_signature: KeySignature,  // e.g., "C", "G" (1 sharp), "Am"
    pub time_signature: TimeSignature, // e.g., 4/4, 3/4, 6/8
    pub tempo: Option<u16>,           // BPM
}
```

### Measure
```rust
pub struct Measure {
    pub elements: Vec<Element>,       // Notes and rests
    pub repeat_start: bool,           // |: repeat start
    pub repeat_end: bool,             // :| repeat end
    pub ending: Option<Ending>,       // First/second ending
    pub key_change: Option<KeySignature>, // Mid-score key change (@key:G)
}
```

### Element (Note or Rest)
```rust
pub enum Element {
    Note(Note),
    Rest {
        duration: Duration,
        dotted: bool,
        tuplet: Option<TupletInfo>,
        chord: Option<String>,        // Chord symbol on rest
    },
}
```

### Note
```rust
pub struct Note {
    pub name: NoteName,               // A-G
    pub accidental: Accidental,       // Sharp, Flat, Natural
    pub octave: Octave,               // Base, Up1 (^), Up2 (^^), Down1 (_), Down2 (__)
    pub duration: Duration,           // Whole, Half, Quarter, Eighth, Sixteenth, ThirtySecond
    pub dotted: bool,                 // Dotted rhythm (*suffix)
    pub tuplet: Option<TupletInfo>,   // Tuplet info (3:2, 5:4, etc.)
    pub tie_start: bool,              // Start of tie (- suffix)
    pub tie_stop: bool,               // End of tie (- prefix)
    pub slur_start: bool,             // Start of slur (()
    pub slur_stop: bool,              // End of slur ())
    pub chord: Option<String>,        // Chord symbol (@ch:C)
}
```

---

## File Size Guidelines

**Current state**:
- ✅ **Small** (<500 lines): `error.rs` (17), `semantic.rs` (290), `ast.rs` (483)
- ⚠️ **Large** (>1000 lines): `lib.rs` (920), `lexer.rs` (658), `parser.rs` (2,574), `musicxml.rs` (2,011)

**Target state** (after refactoring):
- **Optimal**: 150-400 lines per file
- **Acceptable**: 80-600 lines per file
- Files >600 lines should be split into focused modules

**Planned refactoring**:
1. **lib.rs** → split into `api.rs` + `playback/` module (~80 lines lib.rs remaining)
2. **parser.rs** → split into `parser/` directory (5 sub-modules)
3. **musicxml.rs** → split into `musicxml/` directory (6 sub-modules)

---

## Testing Strategy

### Current Test Organization
- **Inline tests**: Most tests are in `#[cfg(test)]` modules within source files
- **Integration tests**: In `lib.rs` (to be moved to `tests/integration_tests.rs`)

### Test Location Guide
- **Lexer tests**: `lexer.rs` - Tokenization tests (~200 lines)
- **Parser tests**: `parser.rs` - Parsing tests (~1,500 lines)
- **Semantic tests**: `semantic.rs` - Validation tests (~125 lines)
- **MusicXML tests**: `musicxml.rs` - Generation tests (~800 lines)
- **Playback tests**: `lib.rs` - Playback/MIDI tests (~400 lines)
- **Integration tests**: `lib.rs` - Full compilation tests (~350 lines)

### Running Tests
```bash
# All tests
cargo test

# Specific module
cargo test --lib lexer
cargo test --lib parser
cargo test --lib semantic

# Specific test
cargo test test_playback_triplets

# With output
cargo test -- --nocapture
```

---

## Build and Development

### Build Commands
```bash
# Build compiler library
cd packages/gen-compiler
cargo build --release

# Build desktop app (includes compiler)
cd packages/gen-ui
pnpm tauri build

# Development mode (desktop app)
pnpm tauri dev
```

### Project Structure
```
gen/
├── packages/
│   ├── gen-compiler/      # Rust compiler (this document describes this)
│   ├── gen-scores/        # Embedded score library
│   ├── gen-ui/            # Shared React/TypeScript UI components
│   ├── gen-desktop/       # Tauri v2 desktop app
│   ├── gen-wasm/          # WebAssembly bindings
│   ├── gen-web/           # Web application
│   └── gen-docs/          # Documentation package (npm exportable)
├── ARCHITECTURE.md        # This file
├── CLAUDE.md              # High-level project overview
└── Cargo.toml             # Rust workspace configuration
```

---

## Future Refactoring Plan

### Phase 1: Documentation (Complete)
- ✅ Create this ARCHITECTURE.md
- ⏳ Add module-level rustdoc to all .rs files
- ⏳ Add type hierarchy diagram to ast.rs
- ⏳ Add examples to error.rs

### Phase 2: Split lib.rs
- Extract playback code → `playback/` module (types, engine, chord parser, tests)
- Extract public API → `api.rs`
- Rewrite lib.rs as architectural "map" (~80 lines)
- Move integration tests → `tests/integration_tests.rs`

### Phase 3: Split parser.rs
- Create `parser/` directory
  - `parser/mod.rs` - Parser struct, main entry point
  - `parser/metadata.rs` - YAML metadata parsing
  - `parser/music.rs` - Music element parsing
  - `parser/tuplets.rs` - Tuplet/bracket group parsing
  - `parser/annotations.rs` - Chord symbols, mod points
  - `parser/tests/` - Organized test files

### Phase 4: Split musicxml.rs
- Create `musicxml/` directory
  - `musicxml/mod.rs` - Public exports
  - `musicxml/generator.rs` - Core XML generation
  - `musicxml/transposition.rs` - Instrument transposition
  - `musicxml/measure.rs` - Measure and note XML
  - `musicxml/beaming.rs` - Beam calculations
  - `musicxml/attributes.rs` - Key/time/clef attributes
  - `musicxml/tests/` - Organized test files

**Benefits for AI Agents**:
1. **Faster task location**: Grep finds focused 150-400 line files instead of 2,000+ line files
2. **Better context management**: Load 3-4 relevant files vs. 1 giant file
3. **Clearer architecture**: This document + focused modules = instant understanding
4. **Self-service**: Common tasks documented above with exact steps

---

## Conventions and Standards

### Error Handling
- All errors are `GenError` variants with location info
- Parser errors include line/column numbers
- Semantic errors include measure numbers
- Always provide helpful error messages for users

### Code Style
- Follow Rust idioms and naming conventions
- Use descriptive variable names (avoid single letters except in loops)
- Comment complex algorithms (especially beaming, tuplet calculations)
- Write tests for all new features

### Git Workflow
- Atomic commits per feature/fix
- Descriptive commit messages
- Test before committing (cargo test)

---

## Additional Resources

- **CLAUDE.md** - High-level project overview and language syntax
- **gen-docs/v1/gen basics.md** - Gen language syntax reference
- **gen-docs/v1/compiler.md** - Compiler implementation details
- **gen-docs/v1/examples.md** - Example Gen scores

---

**Document Version**: 1.0
**Last Updated**: 2026-01-29
**Maintained for**: AI Agents and Human Developers
