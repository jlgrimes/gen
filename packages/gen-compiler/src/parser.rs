use crate::ast::*;
use crate::error::GenError;
use crate::lexer::{Lexer, LocatedToken, Token};

/// Context for parsing tuplets
struct TupletContext {
    default_duration: Duration,
}

/// Parser for Gen source code
pub struct Parser {
    tokens: Vec<LocatedToken>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<LocatedToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> Option<&LocatedToken> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) -> Option<&LocatedToken> {
        let token = self.tokens.get(self.position);
        self.position += 1;
        token
    }

    fn skip_whitespace_and_newlines(&mut self) {
        while let Some(t) = self.current() {
            if t.token == Token::Whitespace || t.token == Token::Newline {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Parse the entire source into a Score
    pub fn parse(&mut self) -> Result<Score, GenError> {
        let metadata = self.parse_metadata()?;
        self.skip_whitespace_and_newlines();

        let mut measures = Vec::new();
        while self.current().is_some() {
            if let Some(measure) = self.parse_measure()? {
                measures.push(measure);
            }
            self.skip_whitespace_and_newlines();
        }

        Ok(Score { metadata, measures })
    }

    /// Parse YAML metadata block if present
    fn parse_metadata(&mut self) -> Result<Metadata, GenError> {
        // Check for metadata start
        if let Some(t) = self.current() {
            if t.token == Token::MetadataStart {
                self.advance(); // consume first ---

                // Get metadata content
                if let Some(t) = self.current() {
                    if let Token::MetadataContent(content) = &t.token {
                        let content = content.clone();
                        self.advance();

                        // Consume closing ---
                        if let Some(t) = self.current() {
                            if t.token == Token::MetadataStart {
                                self.advance();
                            }
                        }

                        return self.parse_yaml_metadata(&content);
                    }
                }

                // Consume closing --- if no content
                if let Some(t) = self.current() {
                    if t.token == Token::MetadataStart {
                        self.advance();
                    }
                }
            }
        }

        Ok(Metadata::default())
    }

    fn parse_yaml_metadata(&self, content: &str) -> Result<Metadata, GenError> {
        let raw: RawMetadata = serde_yaml::from_str(content)
            .map_err(|e| GenError::MetadataError(e.to_string()))?;

        let time_signature = if let Some(ts) = &raw.time_signature {
            self.parse_time_signature(ts)?
        } else {
            TimeSignature::default()
        };

        let key_signature = if let Some(ks) = &raw.key_signature {
            KeySignature::from_str(ks).ok_or_else(|| {
                GenError::MetadataError(format!("Invalid key signature: {}", ks))
            })?
        } else {
            KeySignature::default()
        };

        let written_pitch = if let Some(wp) = &raw.written_pitch {
            self.parse_pitch(wp)?
        } else {
            Pitch::default()
        };

        Ok(Metadata {
            title: raw.title,
            composer: raw.composer,
            time_signature,
            key_signature,
            written_pitch,
        })
    }

    fn parse_time_signature(&self, s: &str) -> Result<TimeSignature, GenError> {
        let parts: Vec<&str> = s.split('/').collect();
        if parts.len() != 2 {
            return Err(GenError::MetadataError(format!(
                "Invalid time signature: {}",
                s
            )));
        }

        let beats = parts[0]
            .parse()
            .map_err(|_| GenError::MetadataError(format!("Invalid time signature beats: {}", s)))?;
        let beat_type = parts[1]
            .parse()
            .map_err(|_| GenError::MetadataError(format!("Invalid time signature beat type: {}", s)))?;

        Ok(TimeSignature { beats, beat_type })
    }

    fn parse_pitch(&self, s: &str) -> Result<Pitch, GenError> {
        let mut chars = s.chars().peekable();

        // Parse note name
        let note = match chars.next() {
            Some('C') => NoteName::C,
            Some('D') => NoteName::D,
            Some('E') => NoteName::E,
            Some('F') => NoteName::F,
            Some('G') => NoteName::G,
            Some('A') => NoteName::A,
            Some('B') => NoteName::B,
            _ => return Err(GenError::MetadataError(format!("Invalid pitch: {}", s))),
        };

        // Parse octave offset
        let mut octave_offset = 0i8;
        for c in chars {
            match c {
                '^' => octave_offset += 1,
                '_' => octave_offset -= 1,
                _ => {}
            }
        }

        Ok(Pitch { note, octave_offset })
    }

    /// Parse a single measure (one line)
    fn parse_measure(&mut self) -> Result<Option<Measure>, GenError> {
        let mut elements = Vec::new();
        let mut next_note_has_tie_stop = false;
        let mut repeat_start = false;
        let mut repeat_end = false;

        // Check for repeat start at beginning of measure (||:)
        if let Some(t) = self.current() {
            if t.token == Token::RepeatStart {
                repeat_start = true;
                self.advance();
            }
        }

        while let Some(t) = self.current() {
            if t.token == Token::Newline {
                self.advance();
                break;
            }

            if t.token == Token::Whitespace {
                self.advance();
                continue;
            }

            // Check for repeat end (:||)
            if t.token == Token::RepeatEnd {
                repeat_end = true;
                self.advance();
                // After repeat end, we should only see whitespace or newline
                while let Some(t) = self.current() {
                    if t.token == Token::Whitespace {
                        self.advance();
                        continue;
                    }
                    if t.token == Token::Newline {
                        self.advance();
                        break;
                    }
                    // Anything else after :|| is an error
                    return Err(GenError::ParseError {
                        line: t.line,
                        column: t.column,
                        message: "Repeat end (:||) must be at the end of a measure".to_string(),
                    });
                }
                break;
            }

            // Check for tuplet group starting with [
            if t.token == Token::LeftBracket {
                let mut tuplet_elements = self.parse_tuplet_group()?;

                // If there's a pending tie_stop, apply it to the first note in the tuplet
                if next_note_has_tie_stop {
                    if let Some(Element::Note(note)) = tuplet_elements.first_mut() {
                        note.tie_stop = true;
                    }
                    next_note_has_tie_stop = false;
                }

                // Check if there's a hyphen after the tuplet (tie from last note)
                if let Some(t) = self.current() {
                    if t.token == Token::Hyphen {
                        self.advance();
                        if let Some(Element::Note(note)) = tuplet_elements.last_mut() {
                            note.tie_start = true;
                        }
                        next_note_has_tie_stop = true;
                    }
                }

                elements.extend(tuplet_elements);
            } else {
                let mut element = self.parse_element(None)?;

                // Apply tie_stop if pending from previous hyphen
                if next_note_has_tie_stop {
                    if let Element::Note(note) = &mut element {
                        note.tie_stop = true;
                    }
                    next_note_has_tie_stop = false;
                }

                // Check if there's a hyphen after this note (tie to next note)
                if let Some(t) = self.current() {
                    if t.token == Token::Hyphen {
                        self.advance();
                        if let Element::Note(note) = &mut element {
                            note.tie_start = true;
                        }
                        next_note_has_tie_stop = true;
                    }
                }

                elements.push(element);
            }
        }

        if elements.is_empty() && !repeat_start && !repeat_end {
            Ok(None)
        } else {
            Ok(Some(Measure { elements, repeat_start, repeat_end }))
        }
    }

    /// Parse a tuplet group like [C D E]3 or [C E G]/3
    fn parse_tuplet_group(&mut self) -> Result<Vec<Element>, GenError> {
        let (line, column) = self
            .current()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0));

        // Consume the opening bracket
        self.advance(); // [

        // Parse the notes inside the bracket (without tuplet info yet)
        let mut raw_elements = Vec::new();
        while let Some(t) = self.current() {
            if t.token == Token::RightBracket {
                break;
            }
            if t.token == Token::Whitespace {
                self.advance();
                continue;
            }
            if t.token == Token::Newline {
                return Err(GenError::ParseError {
                    line,
                    column,
                    message: "Unexpected newline inside tuplet group".to_string(),
                });
            }

            // Parse element without tuplet context for now
            let element = self.parse_element(None)?;
            raw_elements.push(element);
        }

        // Consume the closing bracket
        if let Some(t) = self.current() {
            if t.token == Token::RightBracket {
                self.advance();
            } else {
                return Err(GenError::ParseError {
                    line,
                    column,
                    message: "Expected closing bracket ]".to_string(),
                });
            }
        }

        // Parse optional rhythm modifier after bracket (e.g., / for eighth note triplet)
        let (tuplet_duration, _) = self.parse_rhythm()?;

        // Parse the tuplet number (e.g., 3 for triplet)
        let actual_notes = if let Some(t) = self.current() {
            if let Token::Number(n) = t.token {
                self.advance();
                n
            } else {
                return Err(GenError::ParseError {
                    line: t.line,
                    column: t.column,
                    message: "Expected tuplet number after ]".to_string(),
                });
            }
        } else {
            return Err(GenError::ParseError {
                line,
                column,
                message: "Expected tuplet number after ]".to_string(),
            });
        };

        if raw_elements.is_empty() {
            return Err(GenError::ParseError {
                line,
                column,
                message: "Tuplet group cannot be empty".to_string(),
            });
        }

        // Create tuplet info and apply to all elements
        let tuplet_context = TupletContext {
            default_duration: tuplet_duration,
        };

        let mut elements = Vec::with_capacity(raw_elements.len());
        let last_idx = raw_elements.len() - 1;

        for (i, element) in raw_elements.into_iter().enumerate() {
            let mut tuplet_info = TupletInfo::new(actual_notes);
            tuplet_info.is_start = i == 0;
            tuplet_info.is_stop = i == last_idx;

            let element_with_tuplet = match element {
                Element::Note(mut note) => {
                    // If note doesn't have an explicit duration, use the tuplet's default
                    if note.duration == Duration::Quarter {
                        note.duration = tuplet_context.default_duration;
                    }
                    note.tuplet = Some(tuplet_info);
                    Element::Note(note)
                }
                Element::Rest { duration, dotted, .. } => {
                    // If rest doesn't have explicit duration, use tuplet's default
                    let final_duration = if duration == Duration::Quarter {
                        tuplet_context.default_duration
                    } else {
                        duration
                    };
                    Element::Rest {
                        duration: final_duration,
                        dotted,
                        tuplet: Some(tuplet_info),
                    }
                }
            };
            elements.push(element_with_tuplet);
        }

        Ok(elements)
    }

    /// Parse a single element (note or rest with rhythm)
    fn parse_element(&mut self, tuplet_info: Option<TupletInfo>) -> Result<Element, GenError> {
        let (line, column) = self
            .current()
            .map(|t| (t.line, t.column))
            .unwrap_or((0, 0));

        // Parse rhythm prefix
        let (duration, dotted) = self.parse_rhythm()?;

        // Parse note or rest
        let current = self.current().ok_or(GenError::ParseError {
            line,
            column,
            message: "Expected note or rest after rhythm".to_string(),
        })?;

        match &current.token {
            Token::Rest => {
                self.advance();
                Ok(Element::Rest { duration, dotted, tuplet: tuplet_info })
            }
            Token::NoteA | Token::NoteB | Token::NoteC | Token::NoteD | Token::NoteE
            | Token::NoteF | Token::NoteG => {
                let name = self.parse_note_name()?;
                let (accidental, octave) = self.parse_pitch_modifiers();

                Ok(Element::Note(Note {
                    name,
                    accidental,
                    octave,
                    duration,
                    dotted,
                    tuplet: tuplet_info,
                    tie_start: false,
                    tie_stop: false,
                }))
            }
            _ => Err(GenError::ParseError {
                line: current.line,
                column: current.column,
                message: format!("Expected note or rest, found {:?}", current.token),
            }),
        }
    }

    /// Parse rhythm modifiers and return (Duration, dotted)
    fn parse_rhythm(&mut self) -> Result<(Duration, bool), GenError> {
        let mut slash_count = 0;
        let mut has_pipe = false;
        let mut has_o = false;
        let mut dotted = false;

        // Count rhythm modifiers
        loop {
            let Some(t) = self.current() else { break };

            match &t.token {
                Token::Slash => {
                    self.advance();
                    slash_count += 1;
                }
                Token::Pipe => {
                    self.advance();
                    has_pipe = true;
                }
                Token::SmallO => {
                    self.advance();
                    has_o = true;
                }
                Token::Asterisk => {
                    self.advance();
                    dotted = true;
                }
                _ => break,
            }
        }

        // Determine duration based on modifiers
        let duration = match (slash_count, has_pipe, has_o) {
            (0, false, true) => Duration::Whole,        // o
            (0, true, true) => Duration::Half,          // |o
            (0, false, false) | (0, true, false) => Duration::Quarter, // (none) or |
            (1, false, false) => Duration::Eighth,      // /
            (2, false, false) => Duration::Sixteenth,   // //
            (3, false, false) => Duration::ThirtySecond, // ///
            _ => Duration::Quarter, // fallback
        };

        Ok((duration, dotted))
    }

    fn parse_note_name(&mut self) -> Result<NoteName, GenError> {
        let current = self.current().ok_or(GenError::ParseError {
            line: 0,
            column: 0,
            message: "Expected note name".to_string(),
        })?;

        let name = match &current.token {
            Token::NoteA => NoteName::A,
            Token::NoteB => NoteName::B,
            Token::NoteC => NoteName::C,
            Token::NoteD => NoteName::D,
            Token::NoteE => NoteName::E,
            Token::NoteF => NoteName::F,
            Token::NoteG => NoteName::G,
            _ => {
                return Err(GenError::ParseError {
                    line: current.line,
                    column: current.column,
                    message: format!("Expected note name, found {:?}", current.token),
                })
            }
        };

        self.advance();
        Ok(name)
    }

    fn parse_pitch_modifiers(&mut self) -> (Accidental, Octave) {
        let mut accidental = Accidental::Natural;

        // Parse accidental (# or b)
        if let Some(t) = self.current() {
            match &t.token {
                Token::Sharp => {
                    accidental = Accidental::Sharp;
                    self.advance();
                }
                Token::Flat => {
                    accidental = Accidental::Flat;
                    self.advance();
                }
                _ => {}
            }
        }

        // Parse octave modifiers (_ or ^)
        let mut octave_offset = 0i8;
        while let Some(t) = self.current() {
            match &t.token {
                Token::Underscore => {
                    octave_offset -= 1;
                    self.advance();
                }
                Token::Caret => {
                    octave_offset += 1;
                    self.advance();
                }
                _ => break,
            }
        }

        let octave = match octave_offset {
            i if i <= -2 => Octave::DoubleLow,
            -1 => Octave::Low,
            0 => Octave::Middle,
            1 => Octave::High,
            _ => Octave::DoubleHigh,
        };

        (accidental, octave)
    }
}

/// Main parsing function
pub fn parse(source: &str) -> Result<Score, GenError> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    parser.parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_measure() {
        let score = parse("C D E F").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_with_metadata() {
        let source = r#"---
title: Test Song
time-signature: 3/4
---
C D E"#;
        let score = parse(source).unwrap();
        assert_eq!(score.metadata.title, Some("Test Song".to_string()));
        assert_eq!(score.metadata.time_signature.beats, 3);
        assert_eq!(score.metadata.time_signature.beat_type, 4);
    }

    #[test]
    fn test_rhythm_modifiers() {
        let score = parse("/C |oD oE").unwrap();
        let elements = &score.measures[0].elements;

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Half);
        }
        if let Element::Note(n) = &elements[2] {
            assert_eq!(n.duration, Duration::Whole);
        }
    }

    #[test]
    fn test_triplet_parsing() {
        // Quarter note triplet: [C D E]3
        let score = parse("[C D E]3").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // Check that all notes have triplet info
        for (i, element) in elements.iter().enumerate() {
            if let Element::Note(n) = element {
                assert!(n.tuplet.is_some(), "Note {} should have tuplet info", i);
                let tuplet = n.tuplet.unwrap();
                assert_eq!(tuplet.actual_notes, 3);
                assert_eq!(tuplet.normal_notes, 2);

                // Check start/stop markers
                if i == 0 {
                    assert!(tuplet.is_start);
                    assert!(!tuplet.is_stop);
                } else if i == 2 {
                    assert!(!tuplet.is_start);
                    assert!(tuplet.is_stop);
                } else {
                    assert!(!tuplet.is_start);
                    assert!(!tuplet.is_stop);
                }
            }
        }
    }

    #[test]
    fn test_eighth_note_triplet() {
        // Eighth note triplet: [C D E]/3
        let score = parse("[C D E]/3").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        for element in elements.iter() {
            if let Element::Note(n) = element {
                assert_eq!(n.duration, Duration::Eighth);
                assert!(n.tuplet.is_some());
            }
        }
    }

    #[test]
    fn test_triplet_with_mixed_notes() {
        // Triplet with explicit rhythm on first note
        let score = parse("[/C D E]3").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        // First note should be eighth (explicit), others should be quarter (default)
        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
        }
        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Quarter);
        }
    }

    #[test]
    fn test_simple_tie() {
        // C tied to D
        let score = parse("C-D").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 2);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.name, NoteName::C);
            assert!(n.tie_start, "First note should have tie_start");
            assert!(!n.tie_stop, "First note should not have tie_stop");
        } else {
            panic!("Expected note");
        }

        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.name, NoteName::D);
            assert!(!n.tie_start, "Second note should not have tie_start");
            assert!(n.tie_stop, "Second note should have tie_stop");
        } else {
            panic!("Expected note");
        }
    }

    #[test]
    fn test_chained_ties() {
        // C tied to D tied to E
        let score = parse("C-D-E").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 3);

        if let Element::Note(n) = &elements[0] {
            assert!(n.tie_start);
            assert!(!n.tie_stop);
        }

        if let Element::Note(n) = &elements[1] {
            assert!(n.tie_start, "Middle note should have tie_start");
            assert!(n.tie_stop, "Middle note should have tie_stop");
        }

        if let Element::Note(n) = &elements[2] {
            assert!(!n.tie_start);
            assert!(n.tie_stop);
        }
    }

    #[test]
    fn test_tie_with_rhythm_modifiers() {
        // Eighth note C tied to quarter note D
        let score = parse("/C-D").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 2);

        if let Element::Note(n) = &elements[0] {
            assert_eq!(n.duration, Duration::Eighth);
            assert!(n.tie_start);
        }

        if let Element::Note(n) = &elements[1] {
            assert_eq!(n.duration, Duration::Quarter);
            assert!(n.tie_stop);
        }
    }

    #[test]
    fn test_tie_mixed_with_regular_notes() {
        // Tie followed by regular notes
        let score = parse("C-D E F").unwrap();
        let elements = &score.measures[0].elements;

        assert_eq!(elements.len(), 4);

        if let Element::Note(n) = &elements[0] {
            assert!(n.tie_start);
        }

        if let Element::Note(n) = &elements[1] {
            assert!(n.tie_stop);
        }

        // E and F should have no ties
        if let Element::Note(n) = &elements[2] {
            assert!(!n.tie_start);
            assert!(!n.tie_stop);
        }

        if let Element::Note(n) = &elements[3] {
            assert!(!n.tie_start);
            assert!(!n.tie_stop);
        }
    }

    #[test]
    fn test_repeat_start() {
        let score = parse("||: C D E F").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(score.measures[0].repeat_start);
        assert!(!score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_end() {
        let score = parse("C D E F :||").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(!score.measures[0].repeat_start);
        assert!(score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_both_same_measure() {
        let score = parse("||: C D E F :||").unwrap();
        assert_eq!(score.measures.len(), 1);
        assert!(score.measures[0].repeat_start);
        assert!(score.measures[0].repeat_end);
        assert_eq!(score.measures[0].elements.len(), 4);
    }

    #[test]
    fn test_repeat_across_measures() {
        let score = parse("||: C D E F\nG A B C^ :||").unwrap();
        assert_eq!(score.measures.len(), 2);
        assert!(score.measures[0].repeat_start);
        assert!(!score.measures[0].repeat_end);
        assert!(!score.measures[1].repeat_start);
        assert!(score.measures[1].repeat_end);
    }

    #[test]
    fn test_repeat_error_end_not_at_end() {
        // This should fail because :|| is not at the end
        let result = parse("C :|| D E F");
        assert!(result.is_err());
    }
}
