---
title: Gen Basics
description: Complete guide to the Gen music notation language
---

## Document Anatomy

A Gen document consists of optional metadata at the top, followed by music content where each line is its own measure.

```
---
title: My song
composer: Me
time-signature: 4/4
---

C C G G
A A Gp
F F E E
D D Cp
```

## Metadata Fields

| Field | Description | Default |
|-------|-------------|---------|
| title | Title of the song | |
| composer | Composer of the song | |
| time-signature | Time signature | `4/4` |
| key-signature | Key signature (see table below) | `C` |
| written-pitch | What the document is written in | `C` |
| tempo | Tempo in BPM with optional rhythm modifier | `120` |

### Key Signatures

The `key-signature` field accepts key names (major or minor), or sharp/flat count notation.

**Major Keys:**

| Key | Sharps/Flats |
|-----|--------------|
| `C` | 0 (none) |
| `G` | 1 sharp |
| `D` | 2 sharps |
| `A` | 3 sharps |
| `E` | 4 sharps |
| `B` | 5 sharps |
| `F#` | 6 sharps |
| `C#` | 7 sharps |
| `F` | 1 flat |
| `Bb` | 2 flats |
| `Eb` | 3 flats |
| `Ab` | 4 flats |
| `Db` | 5 flats |
| `Gb` | 6 flats |
| `Cb` | 7 flats |

**Minor Keys:**

Add `m` suffix to specify minor keys (e.g., `Am`, `Em`, `Dm`, `F#m`, `Bbm`).

| Key | Sharps/Flats | Relative Major |
|-----|--------------|----------------|
| `Am` | 0 (none) | C major |
| `Em` | 1 sharp | G major |
| `Bm` | 2 sharps | D major |
| `F#m` | 3 sharps | A major |
| `Dm` | 1 flat | F major |
| `Gm` | 2 flats | Bb major |
| `Cm` | 3 flats | Eb major |

**By Sharp/Flat Count:**

| Notation | Meaning |
|----------|---------|
| `#` | 1 sharp |
| `##` | 2 sharps |
| `###` | 3 sharps |
| `bb` | 2 flats |
| `bbb` | 3 flats |

For 1 flat, use `F` (key name) since `b` alone is ambiguous.

**How key signatures work:** Notes without explicit accidentals automatically follow the key signature. In G major, an `F` sounds as F#. Override with an explicit accidental like `Fb` for F natural.

### Tempo

The `tempo` field specifies BPM with optional rhythm modifiers.

**Syntax:** `BPM[rhythm]`

| Example | Meaning |
|---------|---------|
| `120` | Quarter note at 120 BPM |
| `160p` | Half note at 160 BPM |
| `180/` | Eighth note at 180 BPM |
| `60o` | Whole note at 60 BPM |
| `"120*"` | Dotted quarter at 120 BPM (quote required) |

---

## Note Anatomy

```
_Ab/
[octave][note][accidental][rhythm]
```

### Rhythm Modifiers

| Modifier | Result |
|----------|--------|
| (none) | Quarter note |
| `/` | Eighth note |
| `//` | Sixteenth note |
| `///` | 32nd note |
| `p` | Half note |
| `o` | Whole note |
| `/*` | Dotted eighth |
| `p*` | Dotted half |

### Notes

| Note | Result |
|------|--------|
| `A` - `G` | Notes A through G |
| `$` | Rest (no sound) |

Rhythm modifiers apply to rests, but pitch modifiers do not.

### Octave Modifiers (before the note)

| Modifier | Result |
|----------|--------|
| (none) | Middle octave |
| `_` | One octave down |
| `__` | Two octaves down |
| `^` | One octave up |
| `^^` | Two octaves up |

### Accidentals (after the note name)

| Modifier | Result |
|----------|--------|
| `b` | Flat |
| `#` | Sharp |
| (none) | Natural |

### Understanding Octave Boundaries

The octave system is **absolute** and **always based on C**, regardless of key signature:

- **Base octave:** C, D, E, F, G, A, B
- **High octave (^):** ^C, ^D, ^E, ^F, ^G, ^A, ^B
- **Low octave (_):** _C, _D, _E, _F, _G, _A, _B

**The octave always resets at C:**
- B to ^C is moving up across the octave boundary
- ^B to ^C stays in the same octave

**Examples:**
- Melody going up: `G A B ^C ^D ^E`
- Melody going down: `^E ^D ^C B A G`

---

## Group Modifiers

Groups apply modifiers to multiple notes using brackets `[...]`.

### Rhythm Grouping

Apply the same rhythm to multiple notes:

```
[C D E F]//
```

Equivalent to: `C// D// E// F//`

### Octave Modifiers on Groups

Apply octave shifts to entire groups by putting the modifier **before** the bracket:

```
^[A B C D]      # All notes up one octave
^[A B C D]/    # Eighth notes, all up one octave
_[E F G A]     # All notes down one octave
```

**Combining with individual modifiers:**

```
^[^A B _C]     # A becomes ^^, B becomes ^, _C becomes middle
```

### Measure Octave Modifiers

Apply octave shift to ALL notes in a measure with `@:^` or `@:_`:

```
A B C D @:^    # All notes shifted up one octave
C D E F @:__   # All notes shifted down two octaves
```

---

## Tuplets

Create triplets (and other tuplets) with brackets and a number:

```
[_G C E]3      # Quarter note triplet
[C D E]3/      # Eighth note triplet
```

**Tuplet types:**

| Syntax | Meaning |
|--------|---------|
| `[...]2` | Duplet |
| `[...]3` | Triplet |
| `[...]4` | Quadruplet |
| `[...]5` | Quintuplet |
| `[...]6` | Sextuplet |

**With rhythm:**

| Syntax | Meaning |
|--------|---------|
| `[...]3` | Quarter note triplet |
| `[...]3/` | Eighth note triplet |
| `[...]3//` | Sixteenth note triplet |
| `[...]3p` | Half note triplet |

---

## Ties

Connect notes with a hyphen:

```
C-D
```

Ties only work between individual notes.

---

## Slurs

Wrap notes in parentheses:

```
(_Bb D F) Bb
```

Slurs the first three notes.

---

## Chord Symbols

Add chord symbols with `@ch:`:

```
@ch:C C E G ^C          # C chord on first note
@ch:Dm7 D F A ^C        # Dm7 chord on first note
@ch:C C D @ch:G E F G   # Multiple chords per measure
```

---

## Key Changes

Change key signature mid-piece with `@key:`:

```
---
key-signature: C
---
C D E F                    # C major
@key:G G A B ^C            # Changes to G major
@key:F F G A Bb            # Changes to F major
```

---

## Repeats

Mark repeat sections:

```
||: C D E F
G A B ^C :||
```

- `||:` starts a repeat (beginning of measure)
- `:||` ends a repeat (end of measure)

### First/Second Endings

```
Fo
1. Eb $ Bb* Ab/ :||
2. Eb $ Ebp
```

Requirements:
- `1.` must have a repeat sign at the end
- `2.` must immediately follow `1.`
- `2.` cannot have a repeat sign

---

## Instrument Transposition

The viewer supports transposing instruments:

| Instrument | Written Pitch |
|------------|---------------|
| Flute | ^C |
| Piccolo | ^^C |
| Clarinet | Bb |
| Bass Clarinet | _Bb |
| Alto Sax | Eb |
| Tenor Sax | _Bb |
| Baritone Sax | _Eb |
