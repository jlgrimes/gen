# Gen Compiler Architecture

The Gen compiler is written in Rust and transforms `.gen` source files into MusicXML.

## Pipeline

```
Source Code → Lexer → Tokens → Parser → AST → Semantic Analysis → MusicXML
```

## Modules

### lexer.rs

Tokenizes the input source into a stream of tokens.

**Token types:**
- `RhythmModifier` - Duration indicators (`/`, `//`, `d`, `o`, etc.)
- `NoteName` - Note letters (A-G)
- `Rest` - Rest indicator (`$`)
- `Accidental` - Sharp (`#`) or flat (`b`)
- `OctaveModifier` - Octave shifts (`^`, `^^`, `_`, `__`)
- `Dot` - Dotted note indicator (`*`)
- `MetadataStart` / `MetadataEnd` - YAML header delimiters (`---`)
- `Whitespace` / `Newline` - Structural tokens

### parser.rs

Converts tokens into an Abstract Syntax Tree (AST).

**Parsing stages:**
1. Extract YAML metadata between `---` markers
2. Parse each line as a measure
3. Parse each token group as a note or rest

**Metadata parsing:**
Uses `serde_yaml` to deserialize the YAML header into `RawMetadata`, then converts to typed `Metadata`.

### ast.rs

Defines all type structures:

```rust
Score {
    metadata: Metadata,
    measures: Vec<Measure>,
}

Metadata {
    title: Option<String>,
    composer: Option<String>,
    time_signature: TimeSignature,
    key_signature: KeySignature,
    written_pitch: Pitch,
}

Measure {
    elements: Vec<Element>,
}

Element {
    Note { name, accidental, octave, duration, dotted },
    Rest { duration, dotted },
}
```

### semantic.rs

Validates the parsed score:

- **Measure duration validation**: Ensures each measure's total duration matches the time signature
- Returns detailed errors with line numbers for invalid measures

### musicxml.rs

Generates MusicXML output from the validated AST.

**MusicXML structure produced:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <work><work-title>...</work-title></work>
  <identification>
    <creator type="composer">...</creator>
  </identification>
  <part-list>
    <score-part id="P1">
      <part-name print-object="no"></part-name>
    </score-part>
  </part-list>
  <part id="P1">
    <measure number="1">
      <attributes>
        <divisions>8</divisions>
        <key><fifths>0</fifths></key>
        <time><beats>4</beats><beat-type>4</beat-type></time>
        <clef><sign>G</sign><line>2</line></clef>
      </attributes>
      <note>...</note>
    </measure>
  </part>
</score-partwise>
```

### error.rs

Defines error types with source location information:

```rust
pub enum GenError {
    LexerError { message: String, line: usize, column: usize },
    ParseError { message: String, line: usize, column: usize },
    SemanticError { message: String, line: usize },
    MetadataError(String),
}
```

All errors implement `Display` for user-friendly messages.

## Public API

```rust
// Main entry point - parse, validate, and generate MusicXML
gen::compile(source: &str) -> Result<String, GenError>

// Skip validation (for real-time editing with incomplete measures)
gen::compile_unchecked(source: &str) -> Result<String, GenError>

// Individual stages
gen::parse(source: &str) -> Result<Score, GenError>
gen::validate(score: &Score) -> Result<(), GenError>
gen::to_musicxml(score: &Score) -> String
```

## Duration Calculations

The compiler uses divisions of 8 per quarter note for MusicXML timing:

| Duration     | Divisions |
| ------------ | --------- |
| Whole        | 32        |
| Half         | 16        |
| Quarter      | 8         |
| Eighth       | 4         |
| Sixteenth    | 2         |
| 32nd         | 1         |

Dotted notes add 50% to the base duration.

## Octave Mapping

Gen uses a middle-octave-centered system. MusicXML octave 4 = middle C.

| Gen Modifier | Octave | Example Note |
| ------------ | ------ | ------------ |
| `__`         | 2      | C two octaves below middle C |
| `_`          | 3      | C one octave below middle C |
| (none)       | 4      | Middle C |
| `^`          | 5      | C one octave above middle C |
| `^^`         | 6      | C two octaves above middle C |

## Error Handling

The compiler is strict by design:

- Invalid tokens produce lexer errors with exact position
- Malformed notes produce parser errors
- Measures with incorrect duration produce semantic errors
- All errors include line numbers for debugging

Use `compile_unchecked()` during editing to skip duration validation.
