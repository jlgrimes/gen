---
title: Examples
description: Example Gen scores to learn from
---

## Simple Melody

A basic melody demonstrating quarter and half notes:

```
---
title: Simple Melody
time-signature: 4/4
key-signature: C
---

C D E F
G A B ^C
^C B A G
F E D C
```

## Twinkle Twinkle Little Star

Classic melody with repeated notes:

```
---
title: Twinkle Twinkle
composer: Traditional
time-signature: 4/4
key-signature: C
---

C C G G
A A Gp
F F E E
D D Cp
G G F F
E E Dp
G G F F
E E Dp
C C G G
A A Gp
F F E E
D D Cp
```

## Bob-omb Battlefield Intro

Video game melody with varied rhythms:

```
---
title: Bob-omb Battlefield
composer: Koji Kondo
time-signature: 4/4
key-signature: C
tempo: 120
---

^C/ A/ ^C/ ^D// ^C/ E/* F/ F#/
G/ $/ G// G// $// G $*
D// D#// E
```

## Jazz with Chord Symbols

Using chord annotations:

```
---
title: Jazz Phrase
time-signature: 4/4
key-signature: Bb
---

@ch:Bbmaj7 Bb D F A
@ch:Cm7 C Eb G Bb
@ch:Dm7 D F A ^C
@ch:Eb7 Eb G Bb ^Db
```

## Key Change Example

Modulating between keys:

```
---
title: Modulation Demo
key-signature: C
time-signature: 4/4
---

C E G ^C
G B D ^G
@key:G D F# A ^D
G B ^D ^G
@key:D A ^C# ^E ^A
```

## Triplets

Using triplet notation:

```
---
title: Triplet Exercise
time-signature: 4/4
---

[C D E]3 [F G A]3
[G F E]3 [D C _B]3
[C E G]3/ [E G ^C]3/ [G ^C ^E]3/ [^C ^E ^G]3/
```

## Repeats with Endings

Using repeat signs and first/second endings:

```
---
title: Repeat Demo
time-signature: 4/4
---

||: C D E F
G A B ^C
1. ^D ^E ^F ^G :||
2. ^C B A Gp
```
