# Gen - Music Notation Language

Gen is a text-based music notation language that compiles to MusicXML for rendering as sheet music.

## Project Structure

```
gen/
├── packages/
│   ├── gen-compiler/     # Rust compiler (parses .gen → MusicXML)
│   ├── gen-scores/       # Embedded score library
│   └── gen-ui/           # Tauri + React desktop application
├── gen-docs/             # Language documentation
└── target/               # Rust build artifacts
```

## Packages

### gen-compiler
The core Rust compiler that parses Gen syntax and outputs MusicXML.

**Key modules:**
- `ast.rs` - Type definitions (Note, Measure, Score, Metadata, etc.)
- `lexer.rs` - Tokenizes Gen source code
- `parser.rs` - Parses tokens into AST, handles YAML metadata
- `semantic.rs` - Validates measure durations against time signature
- `musicxml.rs` - Generates MusicXML output
- `error.rs` - Error types with line/column info

**Public API:**
```rust
gen::compile(source: &str) -> Result<String, GenError>      // With validation
gen::compile_unchecked(source: &str) -> Result<String, GenError>  // Without validation
gen::parse(source: &str) -> Result<Score, GenError>
gen::validate(score: &Score) -> Result<(), GenError>
gen::to_musicxml(score: &Score) -> String
```

### gen-scores
Rust library that embeds `.gen` score files at compile time.

**Usage:**
- Place `.gen` files in `packages/scores/examples/`
- Scores are embedded via `build.rs` at compile time
- Access via `gen_scores::get_all_scores()` or `gen_scores::get_score(name)`

**To add a new score:**
1. Create a `.gen` file in `packages/scores/examples/`
2. Rebuild the project (`cargo build` in root or `pnpm tauri build` in gen-ui)

### gen-ui
Tauri v2 + React desktop application for editing and viewing Gen scores.

**Tech stack:**
- Tauri v2 (Rust backend)
- React + TypeScript (frontend)
- Tailwind CSS v4
- OpenSheetMusicDisplay (OSMD) for MusicXML rendering

**Key files:**
- `src/App.tsx` - Main application with editor and sheet music view
- `src/components/ui/sidebar.tsx` - Score browser sidebar
- `src-tauri/src/lib.rs` - Tauri commands (compile_gen, list_scores, etc.)

**Development:**
```bash
cd packages/gen-ui
pnpm install
pnpm tauri dev
```

## Gen Language Syntax

### Document Structure
```
---
title: Song Title
composer: Composer Name
time-signature: 4/4
key-signature: G
---

C D E F
G A B C^
```

### Note Format
`[rhythm][note][pitch]`

**Rhythm modifiers:**
- (none) or `|` = quarter note
- `/` = eighth note
- `//` = sixteenth note
- `///` = 32nd note
- `|o` = half note
- `o` = whole note
- `*` suffix = dotted

**Notes:** `A B C D E F G` or `$` for rest

**Pitch modifiers:**
- `#` = sharp, `b` = flat
- `^` = octave up, `^^` = two octaves up
- `_` = octave down, `__` = two octaves down

### Examples
```
C           # C quarter note
/E          # E eighth note
|oG         # G half note
/Ab_        # Ab eighth note, one octave down
//F#^       # F# sixteenth note, one octave up
$           # quarter rest
/$          # eighth rest
```

## Building

### Full project
```bash
cargo build --release
```

### Just the compiler
```bash
cd packages/gen-compiler
cargo build --release
```

### Desktop app
```bash
cd packages/gen-ui
pnpm install
pnpm tauri build
```

## Testing the Compiler

```bash
cd packages/gen-compiler
cargo run -- path/to/score.gen > output.musicxml
```

Or use the library:
```rust
let musicxml = gen::compile(source)?;
```

## Common Tasks

### Add a new metadata field
1. Add field to `RawMetadata` in `ast.rs`
2. Add parsed field to `Metadata` struct
3. Update parser in `parser.rs` to handle the field
4. Update `musicxml.rs` if it affects XML output
5. Update documentation in `gen-docs/v1/gen basics.md`

### Add a new rhythm modifier
1. Update `lexer.rs` to recognize the token
2. Update `Duration` enum in `ast.rs`
3. Update parser in `parser.rs`
4. Update `musicxml_type()` in `ast.rs`
5. Update documentation
