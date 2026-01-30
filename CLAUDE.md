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
tempo: 120
---

C D E F
G A B ^C
```

**Tempo field:**
The `tempo` field accepts BPM with optional rhythm modifiers at the END:
- `tempo: 120` - Quarter note at 120 BPM (default)
- `tempo: 160p` - Half note at 160 BPM (quarter = 320 BPM)
- `tempo: 180/` - Eighth note at 180 BPM (quarter = 90 BPM)
- `tempo: 60o` - Whole note at 60 BPM (quarter = 240 BPM)
- `tempo: "120*"` - Dotted quarter at 120 BPM (quarter = 180 BPM, must quote)
- `tempo: "80p*"` - Dotted half at 80 BPM (quarter = 240 BPM, must quote)

**Key signatures support both major and minor keys:**
- Major keys: `C`, `G`, `D`, `A`, `E`, `B`, `F#`, `C#`, `F`, `Bb`, `Eb`, `Ab`, `Db`, `Gb`, `Cb`
- Minor keys: Add 'm' suffix (e.g., `Am`, `Em`, `Dm`, `Cm`, `F#m`, `Bbm`, `Ebm`)
- Sharp/flat count: `#`, `##`, `###` (sharps) or `bb`, `bbb`, `bbbb` (flats)

### Note Format
`[octave][note][accidental][rhythm]`

**Octave modifiers (at BEGINNING):**
- `^` = octave up, `^^` = two octaves up
- `_` = octave down, `__` = two octaves down

**Notes:** `A B C D E F G` or `$` for rest

**Accidentals (after note name):**
- `#` = sharp, `b` = flat

**Rhythm modifiers (at END):**
- (none) = quarter note
- `/` = eighth note
- `//` = sixteenth note
- `///` = 32nd note
- `p` = half note
- `o` = whole note
- `*` = dotted (always at end, after rhythm)

**CRITICAL - Octave System (ALWAYS ABSOLUTE):**
- **The octave range is ALWAYS C through B, regardless of key signature**
- **Base octave (no modifier): C D E F G A B** - this is the "middle" octave
- **High octave (^ modifier): ^C ^D ^E ^F ^G ^A ^B** - this is one octave up
- **Low octave (_ modifier): _C _D _E _F _G _A _B** - this is one octave down
- **The octave ALWAYS "resets" at C** - so B to ^C is going up, but ^B to ^C is staying in the same octave
- **Key signature does NOT affect octave boundaries** - even in F major or Eb minor, the octave break is still at C
- Example: A melody that goes G A B ^C ^D is going up through the octave break at C
- Example: In "Happy Birthday", the sustained "you" notes are ^C, ^D because they're above B
- Example: If jumping from high notes to low, you might go ^E ^D ^C B A G (going down through octaves)

### Examples
```
C           # C quarter note
E/          # E eighth note
Gp          # G half note
_Ab/        # Ab eighth note, one octave down
^F#//       # F# sixteenth note, one octave up
$           # quarter rest
$/          # eighth rest
Gp*         # G dotted half note
```

### Key Changes
Change the key signature in the middle of a piece:
```
@key:G      # Change to G major (1 sharp)
@key:Bb     # Change to Bb major (2 flats)
@key:##     # Change to 2 sharps (D major)
```

**Placement:** At the beginning of a measure
**Effect:** Changes key signature from this point forward
**Works with:** All key notation (key names or sharp/flat count)
**Transposition:** Automatically transposes for instrument groups

### Bracket Groups
Groups apply modifiers to multiple notes at once. Octave goes BEFORE the bracket, rhythm and tuplet go AFTER.

**Rhythm grouping:**
- `[C D E F]//` = `C// D// E// F//` (all sixteenth notes)

**Tuplets:**
- `[C D E]3` = quarter note triplet (3 notes in time of 2)
- `[C D E]3/` = eighth note triplet

**Octave modifiers on groups:**
- `^[A B C D]` = `^A ^B ^C ^D` (all notes up one octave)
- `^[A B C D]/` = `^A/ ^B/ ^C/ ^D/` (eighth notes, all up one octave)
- `^[C D E]3` = quarter note triplet, all up one octave
- `^[C D E]3/` = eighth note triplet, all up one octave

Group octave modifiers are applied **after** individual note modifiers:
- `^[^A B _C]` results in ^^A, ^B, C (middle)

### Measure Octave Modifiers
Apply octave shift to ALL notes in a measure:
- `A B C D @:^` = `^A ^B ^C ^D` (all notes up one octave)
- `@:_`, `@:^^`, `@:__` also supported
- Stacks with individual and group modifiers: `^[A B] @:^` results in all notes ^^
- Similar to instrument group modifiers (@Eb:^) but affects all notes

### Chord Symbols
Add chord symbols (lead sheet notation) above notes using curly braces:

**Syntax:**
```
{Cmaj7}:G B D     # Attached - chord inherits G's duration
{Cmaj7} G B D     # Standalone - chord has whole note duration (default)
{Am7}p G B D      # Standalone - chord has half note duration
{F}/ G B D        # Standalone - chord has eighth note duration
```

**Rule:**
- `:` after `}` = attached (chord inherits the note's duration for playback)
- No `:` = standalone with its own duration (default whole note, or specify with rhythm modifier)

**Chord notation:**
- Root note: `C`, `D`, `E`, `F`, `G`, `A`, `B`
- Accidentals: `#` (sharp), `b` (flat) - e.g., `{F#}`, `{Bb7}`
- Quality: `m` (minor), `maj7`, `7`, `m7`, `dim`, `aug`, etc.
- Slash chords: `{C/E}` (C major with E in bass)

**Examples:**
```
{C} C E G         # C major chord, whole note duration
{Am7}:Gp E D      # Am7 attached to Gp, inherits half note duration
{Dm7} [C E A]/    # Dm7 whole note, eighth note arpeggio
{G7}p {C} G       # G7 half note, then C whole note on G
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
