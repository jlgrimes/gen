use crate::ast::*;
use crate::error::GenError;
use crate::lexer::{Lexer, LocatedToken, Token};

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

        while let Some(t) = self.current() {
            if t.token == Token::Newline {
                self.advance();
                break;
            }

            if t.token == Token::Whitespace {
                self.advance();
                continue;
            }

            let element = self.parse_element()?;
            elements.push(element);
        }

        if elements.is_empty() {
            Ok(None)
        } else {
            Ok(Some(Measure { elements }))
        }
    }

    /// Parse a single element (note or rest with rhythm)
    fn parse_element(&mut self) -> Result<Element, GenError> {
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
                Ok(Element::Rest { duration, dotted })
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
        let mut backslash_count = 0;
        let mut has_pipe = false;
        let mut has_o = false;
        let mut dotted = false;

        // Count rhythm modifiers
        loop {
            let Some(t) = self.current() else { break };

            match &t.token {
                Token::Backslash => {
                    self.advance();
                    backslash_count += 1;
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
        let duration = match (backslash_count, has_pipe, has_o) {
            (0, false, true) => Duration::Whole,        // o
            (0, true, true) => Duration::Half,          // |o
            (0, false, false) | (0, true, false) => Duration::Quarter, // (none) or |
            (1, false, false) => Duration::Eighth,      // \
            (2, false, false) => Duration::Sixteenth,   // \\
            (3, false, false) => Duration::ThirtySecond, // \\\
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
        let score = parse("\\C |oD oE").unwrap();
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
}
