# Gen Documentation

Gen is a text-based music notation language that compiles to MusicXML for rendering as sheet music.

## Quick Start

Write music with simple text syntax:
```
---
title: Twinkle Twinkle
time-signature: 4/4
key-signature: C
---

C C G G
A A dG
F F E E
D D dC
```

## Documentation

- [Gen Basics](v1/gen%20basics.md) - Language syntax, notes, rhythms, metadata
- [Compiler Architecture](v1/compiler.md) - How the Rust compiler works
- [Gen UI Application](v1/gen-ui.md) - Desktop application guide
- [Examples](v1/examples.md) - Sample scores

## Project Structure

```
gen/
├── packages/
│   ├── gen-compiler/    # Rust compiler
│   ├── gen-scores/      # Embedded score library
│   └── gen-ui/          # Tauri + React desktop app
└── gen-docs/            # This documentation
```

## Getting Started

### Run the Desktop App
```bash
cd packages/gen-ui
pnpm install
pnpm tauri dev
```

### Compile from Command Line
```bash
cd packages/gen-compiler
cargo run -- path/to/score.gen
```

## Language Overview

### Note Format
`[rhythm][note][pitch]`

- **Rhythm**: `/` (eighth), `d` (half), `o` (whole), etc.
- **Note**: `A B C D E F G` or `$` for rest
- **Pitch**: `#` (sharp), `b` (flat), `^` (octave up), `_` (octave down)

### Example Notes
```
C       # Quarter note C
/E      # Eighth note E
dG     # Half note G
oC      # Whole note C
Ab_     # Quarter note Ab, one octave down
F#^     # Quarter note F#, one octave up
$       # Quarter rest
/$      # Eighth rest
```

See [Gen Basics](v1/gen%20basics.md) for complete documentation.
