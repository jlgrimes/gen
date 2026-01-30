# Gen Syntax Highlighting Rules

This document is the **single source of truth** for Gen syntax highlighting.
When updating highlighting, update this file first, then update both:
1. `syntaxes/gen.tmLanguage.json` (for VS Code)
2. `monarch.ts` (for Monaco/web editor)

## Token Types and Colors

| Token | Description | Example | VS Code Scope | Monaco Token | Color |
|-------|-------------|---------|---------------|--------------|-------|
| **Notes** | Note names with optional accidentals | `C`, `D#`, `Eb`, `F%` | `variable.other.note` | `variable` | Blue |
| **Rests** | Rest symbol | `$` | `variable.other.rest` | `variable` | Blue |
| **Octave** | Octave modifiers | `^`, `^^`, `_`, `__` | `keyword.control.octave` | `keyword` | Purple |
| **Rhythm** | Duration modifiers | `/`, `//`, `p`, `o`, `*` | `constant.numeric.rhythm` | `number` | Orange |
| **Tuplet** | Tuplet numbers after brackets | `]3`, `]5/` | `constant.numeric.tuplet` | `number` | Orange |
| **Brackets** | Grouping brackets | `[`, `]` | `punctuation.section.brackets` | `delimiter.bracket` | Default |
| **Repeats** | Repeat markers | `\|\|:`, `:\|\|` | `keyword.control.repeat` | `keyword` | Purple |
| **Endings** | First/second endings | `1.`, `2.` | `keyword.control.ending` | `keyword` | Purple |
| **Annotations** | All @ annotations | `@ch:C`, `@key:G`, `@pickup` | `entity.name.function.annotation` | `annotation` | Yellow |
| **Comments** | Line comments | `// comment` | `comment.line` | `comment` | Green |
| **Metadata Key** | YAML frontmatter keys | `title:`, `composer:` | `entity.name.tag.yaml` | `type` | Red |
| **Metadata Value** | YAML frontmatter values | `My Song` | `string.unquoted.yaml` | `string` | Green |
| **Metadata Delim** | YAML delimiters | `---` | `punctuation.definition.frontmatter` | `delimiter` | Default |

## Pattern Definitions

### Notes
```
Pattern: (octave)?(note)(accidental)?(rhythm)?(dot)?
- octave: ^+ | _+
- note: A | B | C | D | E | F | G
- accidental: # | b | %
- rhythm: / | // | /// | p | o
- dot: *

Examples: C, D/, E#p, ^Fb//, _G*, ^^A#o*
```

### Rests
```
Pattern: $(rhythm)?(dot)?
Examples: $, $/, $p, $o*
```

### Brackets (Grouping/Tuplets)
```
Pattern: (octave)?[(content)](tuplet)?(rhythm)?(dot)?
- tuplet: 3 | 5 | 6 | 7 (etc)

Examples: [C D E]/, ^[A B]3, [$ C D E F]5/
```

### Annotations
```
@ch:<chord>(rhythm)?(dot)?     - Chord annotation
@key:<key>                      - Key change (G, Bb, F#m, ##, bbb)
@(Eb|Bb|F|C|G):(octave)        - Instrument group octave shift
@:(octave)                      - Measure octave modifier
@pickup                         - Pickup measure marker
```

### Metadata (YAML Frontmatter)
```
---
key: value
another-key: another value
---
```

### Comments
```
// This is a comment
```

### Repeats & Endings
```
||:  - Repeat start
:||  - Repeat end
1.   - First ending
2.   - Second ending
```

## Updating Instructions

### For VS Code (TextMate Grammar)
Edit `syntaxes/gen.tmLanguage.json`:
- Use regex patterns in `match` or `begin`/`end`
- Assign scopes from the "VS Code Scope" column above
- Test with "Developer: Inspect Editor Tokens and Scopes" command

### For Monaco (Monarch Tokenizer)
Edit `monarch.ts`:
- Use regex patterns in tokenizer rules
- Assign tokens from the "Monaco Token" column above
- Token names map to theme colors automatically

## Testing Changes
1. VS Code: Reload window, open a .gen file, check highlighting
2. Monaco: Run `pnpm tauri dev`, check editor highlighting matches
3. Both should have the same visual appearance
