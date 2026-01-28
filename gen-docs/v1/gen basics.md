# Anatomy of a document
A document is where a gen file lives.
Each line is its own measure.
At the top of the document, you may specify metadata. Like, time signature. If time signature is not specified, common time (4/4) is used.

This is what specifying time signature looks like

```
---
title: My song
composer: Me
time-signature: 4/4
---

C C G G
A A dG
F F E E
D D dC
```

all available fields for metadata are

| field          | description                                                           | default |
| -------------- | --------------------------------------------------------------------- | ------- |
| title          | What the title of the song is (purely stylistic)                      |         |
| composer       | composer of the song (purely stylistic)                               |         |
| time-signature | Time signature that the song defaults to if none explicitly specified | `4/4`   |
| key-signature  | Key signature of the piece (see table below)                          | `C`     |
| written-pitch  | What the document is written in. Defaults to concert pitch (C)        | `C`     |

### Key Signatures

The `key-signature` field accepts key names or sharp/flat count notation:

**By key name:**
| Key    | Sharps/Flats |
| ------ | ------------ |
| `C`    | 0 (none)     |
| `G`    | 1 sharp      |
| `D`    | 2 sharps     |
| `A`    | 3 sharps     |
| `E`    | 4 sharps     |
| `B`    | 5 sharps     |
| `F#`   | 6 sharps     |
| `C#`   | 7 sharps     |
| `F`    | 1 flat       |
| `Bb`   | 2 flats      |
| `Eb`   | 3 flats      |
| `Ab`   | 4 flats      |
| `Db`   | 5 flats      |
| `Gb`   | 6 flats      |
| `Cb`   | 7 flats      |

**By sharp/flat count:**
| Notation  | Meaning   |
| --------- | --------- |
| `#`       | 1 sharp   |
| `##`      | 2 sharps  |
| `###`     | 3 sharps  |
| `####`    | 4 sharps  |
| `#####`   | 5 sharps  |
| `######`  | 6 sharps  |
| `#######` | 7 sharps  |
| `bb`      | 2 flats   |
| `bbb`     | 3 flats   |
| `bbbb`    | 4 flats   |
| `bbbbb`   | 5 flats   |
| `bbbbbb`  | 6 flats   |
| `bbbbbbb` | 7 flats   |

Note: For 1 flat, use `F` (key name) since `b` alone is ambiguous with B major.

**How key signatures work:** Notes without explicit accidentals will automatically follow the key signature. For example, in G major (1 sharp), an `F` will sound as F#. To override this, use an explicit accidental like `Fb` for F natural.

Example with key signature:
```
---
title: My Song in G Major
key-signature: G
time-signature: 4/4
---

G A B C^
D^ E^ F^ G^
```

Note: The `F` in the second line will automatically be sharped because we're in G major. You don't need to write `F#`.

Example using sharp count:
```
---
title: My Song in D Major
key-signature: ##
---

D E F G
A B C D^
```
Each line is its own measure. If you do a new line, it will do a new measure.
# Anatomy of a note

`/Ab_`
`[rhythm][note][pitch] ...`

| rhythm modifier | result         |
| --------------- | -------------- |
| `[none]`        | Quarter note   |
| `/`             | Eighth note    |
| `//`            | Sixteenth note |
| `///`           | 32nd note      |
| `d`             | Half note      |
| `o`             | Whole note     |
| `/*`            | dotted eighth  |
| `d*`            | dotted half    |

| note | result          |
| ---- | --------------- |
| `A`  | A               |
| `B`  | B               |
| `C`  | C               |
| `D`  | D               |
| `E`  | E               |
| `F`  | F               |
| `G`  | G               |
| `$`  | Rest (no sound) |
For rest specifically, rhythm modifiers can be applied to modify the duration of the rest. however, pitch modifiers do not apply to rests for obvious reasons.

there are two different modifiers to pitch.

| modifier type         | what it's for                    |
| --------------------- | -------------------------------- |
| first level modifier  | specifying quality (flat, sharp) |
| second level modifier | specifying octave (8va, 8vb)     |


| first level pitch modifier | result  |
| -------------------------- | ------- |
| `b`                        | Flat    |
| `#`                        | Sharp   |
| `[none]`                   | Natural |

| second level pitch modifier | result        |
| --------------------------- | ------------- |
| `[none]`                    | Middle octave |
| `_`                         | 8vb           |
| `__`                        | 8vbb          |
| `^`                         | 8va           |
| `^^`                        | 8vaa          |
*note - these second level modifiers are only to be applied at the note level*

## Group modifiers
Sometimes, you want to modify more than one note at a time. For this you can use group modifiers. Groups are defined by putting notes in brackets `[...]`.

### Rhythm grouping
You can apply the same rhythm to multiple notes by putting the rhythm modifier before the bracket:

`//[C D E F]` is the same as `//C //D //E //F`

### Octave modifiers on groups
You can apply octave modifiers to an entire group by putting the octave modifier **after** the closing bracket. This will shift all notes in the group by the specified amount.

**Syntax:** `[notes...]^` or `[notes...]_`

**Examples:**
- `[A B C D]^` - all four notes shifted up one octave (equivalent to `A^ B^ C^ D^`)
- `/[A B C D]^` - eighth notes, all shifted up one octave (equivalent to `/A^ /B^ /C^ /D^`)
- `[E F G A]_` - all notes shifted down one octave
- `[C D E F]^^` - all notes shifted up two octaves

**Combining with individual octave modifiers:**
Group octave modifiers apply on top of individual note octave modifiers:
- `[A^ B C_]^` - A becomes double high (^^ total), B becomes high (^ from group), C_ becomes middle (_ + ^ = middle)

**Works with tuplets too:**
- `3[C D E]^` - quarter note triplet, all notes shifted up one octave
- `/3[C D E]^` - eighth note triplet, all notes shifted up one octave

### Measure octave modifiers
You can apply an octave shift to ALL notes in a measure by adding `@:^` or `@:_` anywhere in the measure (typically at the end).

**Syntax:** `@:^`, `@:_`, `@:^^`, or `@:__`

**Examples:**
- `A B C D @:^` - all four notes shifted up one octave (equivalent to `A^ B^ C^ D^`)
- `C D E F @:__` - all notes shifted down two octaves
- `//[D C] /*D //C //D /*E @:^` - all notes (including those in brackets) shifted up one octave

**Combining with other octave modifiers:**
Measure octave modifiers apply on top of individual note modifiers and group modifiers:
- `A^ B C @:^` - A becomes ^^ (double high), B becomes ^, C becomes ^
- `[A B]^ @:^` - all notes become ^^ (group ^ + measure ^)

**Note:** This is similar to instrument group modifiers (@Eb:^, @Bb:^) but applies to ALL notes in the measure regardless of instrument.

## Ties

Ties can be indicated with hyphen between the notes. For example:

`C-D`

is a C tied with a D. Ties can only be between individual notes.

## Triplets

The first obvious use case of this is triplets, where you want to play three notes as triplets. To make triplets with Gen, it looks like this

`3[G_ C E] /3[dG E] dG`

Some notes on the triplets
1. It is notated as a triplet because there is a 3 at the START of the bracket set.
2. You can specify how fast the triplet is by putting the rhythm modifier BEFORE the number and bracket. If not specified, it is a quarter note triplet. For example, `/3[C E G]` is an eighth note triplet.
	1. With this, normally, the notes duration are default to quarter note. However, if in a triplet modifier, the default duration of the note would be whatever the triplet duration specifies, instead of quarter note. In the example shown, C E G would all have duration 8th note triplets since defined in the eighth note triplet rhythm bracket. You can also override rhythm like you see from above.

Other tuplets can be specified using the respective numbers:

| syntax    | meaning                 |
| --------- | ----------------------- |
| `2[...]`  | duplet                  |
| `3[...]`  | triplet                 |
| `4[...]`  | four-et? (the four one) |
| `5[...]`  | quintuplet              |
| `6[...]`  | sextuplet               |
| and so on |                         |

You can also specify the tuplet rhythm:

| syntax     | meaning                    |
| ---------- | -------------------------- |
| `3[...]`   | quarter note triplet       |
| `/3[...]`  | eighth note triplet        |
| `//3[...]` | sixteenth note triplet     |
| `d3[...]` | half note triplet          |
| and so on  |                            |

## Other rhythm groupings

You can also group together notes with the same rhythm by using brackets and specifying the rhythm in front. Like this

`//[C D E F]` instead of `//C //D //E //F`

This works with any rhythm modifier. Notes inside the bracket will use the group's rhythm unless they have an explicit rhythm specified.

## Slurs

Slurs can be represented with a normal parantheses set around the group you want a slur between ()

for example

`(Bb_ D F) Bb`

slurs the first three notes

## Chord Symbols

Chord symbols can be added to notes using the `@ch:` annotation. Place the annotation before the note you want the chord to appear on.

**Syntax:** `@ch:ChordName`

**Examples:**
```
@ch:C C E G C^          # C major chord on first note
@ch:Dm7 D F A C^        # Dm7 chord on first note
@ch:C C D @ch:G E F G   # C chord on C, G chord on E
```

**Multiple chords per measure:**
You can have as many chord annotations as needed in a single measure:
```
@ch:Cmaj7 C E @ch:Dm7 D F @ch:G7 G B
```

**Chord symbol format:**
Gen accepts any chord symbol text without validation. Common formats include:
- Basic triads: `C`, `Dm`, `Eb`, `F#`
- Seventh chords: `Cmaj7`, `Dm7`, `G7`, `Bm7b5`
- Extended chords: `C9`, `Dm11`, `G13`
- Altered chords: `Csus4`, `Cadd9`, `C7alt`

The chord symbol will appear above the staff in the rendered sheet music.

## Key Changes

Key changes allow you to change the key signature in the middle of a piece. Place the `@key:` annotation at the beginning of the measure where you want the key change to occur.

**Syntax:** `@key:KeyName`

**Examples:**
```
---
key-signature: C
---
C D E F                    # Measure 1: C major
@key:G G A B C^            # Measure 2: changes to G major (1 sharp)
@key:F F G A Bb            # Measure 3: changes to F major (1 flat)
```

**How key changes work:**
- The key change applies from the measure where it appears onwards
- All subsequent measures use the new key signature until another key change occurs
- Notes without explicit accidentals will automatically follow the new key signature
- Works with the same key notation as the metadata `key-signature` field (key names like `G`, `Bb`, or sharp/flat count like `##`, `bb`)

**Key change with transposition:**
Key changes work correctly with transposing instruments. The key signature will be automatically transposed to match the instrument's written pitch.

**Example with modulation:**
```
---
title: Modulating Song
key-signature: C
time-signature: 4/4
---

C E G C^                   # C major section
G B D G^
@key:G G A B C^            # Modulates to G major
D E F# G                   # F is now F# due to key signature
```

**Combining with other annotations:**
You can use key changes alongside other annotations in the same measure:
```
@key:D @ch:Dmaj7 D F# A D^  # Key change to D major with Dmaj7 chord
```

# Other notation

## Repeats

Repeats can be notated by putting `||:` alone where you want to start, and `:||` at the end.
a compiler error will be thrown if:
- Repeat start is not at beginning of measure
- repeat end is not at end of measure
- repeat start does not have a matching repeat end

## First/second endings

since these are measure level modifiers, they go at the beginning of the measure to indicate first or second ending respectively.
this is how you use it
```
oF
1. Eb $ *Bb /Ab :||
2. Eb $ dEb
```

some requirements
1. "1." modifier is required to have a repeat sign at the end of it. if not, throw error
2. "2." modifier cannot have a repeat sign at the end of it. Also, must immediately proceed a line with "1." AND a repeat. else, throw error

to continue a first second ending to multiple measures, simply prepend consecutive lines with 1. or 2.

# gen viewer specifics
the gen viewer will be a tauri app that renders the gen file into real sheet music you can read.
The viewer will allow you to view by instrument, which transposes the music to given instrumet/key. It changes a new property on the client side called `viewed-pitch`, which is used to transpose the gen notation before it is translated to sheet music. The value of viewed-pitch is whichever note middle C should be mapped to.

`viewed-pitch` Can also specify which octave is written. For example, for flute it'll be something like `viewed-pitch: C^`, since the flute reads music an octave above everyone else.

`viewed-clef` - TODO: render different kind of clef. default is always treble.

The sheet music is rendered with MusicXML. or something whichever is best

# instrument specifics

You can specify a viewed pitch or click on an instrument from a dropdown which sets the viewed-pitch for you

| Instrument    | viewed-pitch |
| ------------- | ------------ |
| Flute         | C^           |
| Piccolo       | C^^          |
| Clarinet      | Bb           |
| Bass Clarinet | Bb_          |
| Alto Sax      | Eb           |
| Tenor Sax     | Bb_          |
| Baritone Sax  | Eb_          |
