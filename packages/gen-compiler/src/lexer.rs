use crate::error::GenError;

/// Token types for the Gen language
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Rhythm modifiers
    Slash,          // /
    SmallD,         // d (half note)
    SmallO,         // o (whole note)
    Asterisk,       // * (dotted)

    // Note names
    NoteA,
    NoteB,
    NoteC,
    NoteD,
    NoteE,
    NoteF,
    NoteG,
    Rest,           // $

    // Pitch modifiers
    Sharp,          // #
    Flat,           // b
    Natural,        // %
    Underscore,     // _
    Caret,          // ^

    // Tuplet/grouping
    LeftBracket,    // [
    RightBracket,   // ]
    Number(u8),     // 2, 3, 4, 5, 6, etc.

    // Ties
    Hyphen,         // -

    // Slurs
    LeftParen,      // (
    RightParen,     // )

    // Repeats
    RepeatStart,    // ||:
    RepeatEnd,      // :||

    // Endings
    FirstEnding,    // 1.
    SecondEnding,   // 2.

    // Structure
    Newline,
    Whitespace,
    MetadataStart,  // ---

    // Metadata content (raw string until next ---)
    MetadataContent(String),
}

/// A token with its position in the source
#[derive(Debug, Clone)]
pub struct LocatedToken {
    pub token: Token,
    pub line: usize,
    pub column: usize,
}

/// Lexer for tokenizing Gen source code
pub struct Lexer<'a> {
    input: &'a str,
    chars: std::iter::Peekable<std::str::Chars<'a>>,
    line: usize,
    column: usize,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            line: 1,
            column: 1,
            position: 0,
        }
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.position += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    fn check_metadata_marker(&mut self) -> bool {
        // Check if we're at the start of ---
        let remaining = &self.input[self.position..];
        remaining.starts_with("---")
    }

    fn check_first_ending(&self) -> bool {
        let remaining = &self.input[self.position..];
        remaining.starts_with("1.")
    }

    fn check_second_ending(&self) -> bool {
        let remaining = &self.input[self.position..];
        remaining.starts_with("2.")
    }


    pub fn tokenize(&mut self) -> Result<Vec<LocatedToken>, GenError> {
        let mut tokens = Vec::new();
        let mut metadata_started = false;

        while let Some(&c) = self.peek() {
            let line = self.line;
            let column = self.column;

            // Check for metadata markers - skip entire metadata block
            if self.check_metadata_marker() {
                // Consume the three dashes
                self.advance(); // -
                self.advance(); // -
                self.advance(); // -

                // Skip the newline after ---
                if let Some(&'\n') = self.peek() {
                    self.advance();
                }

                // If this is the opening ---, skip until the closing ---
                if !metadata_started {
                    metadata_started = true;

                    // Consume everything until the closing ---
                    while self.peek().is_some() {
                        if self.check_metadata_marker() {
                            // Consume the closing ---
                            self.advance(); // -
                            self.advance(); // -
                            self.advance(); // -
                            // Skip trailing newline
                            if let Some(&'\n') = self.peek() {
                                self.advance();
                            }
                            break;
                        }
                        self.advance();
                    }
                }
                continue;
            }

            // Check for first/second endings (only after metadata is complete)
            if self.check_first_ending() {
                // Consume "1."
                self.advance(); // 1
                self.advance(); // .
                tokens.push(LocatedToken {
                    token: Token::FirstEnding,
                    line,
                    column,
                });
                continue;
            }

            if self.check_second_ending() {
                // Consume "2."
                self.advance(); // 2
                self.advance(); // .
                tokens.push(LocatedToken {
                    token: Token::SecondEnding,
                    line,
                    column,
                });
                continue;
            }

            let token = match c {
                '/' => {
                    self.advance();
                    Token::Slash
                }
                '|' => {
                    self.advance();
                    // Check for ||: (repeat start)
                    if let Some(&'|') = self.peek() {
                        self.advance();
                        if let Some(&':') = self.peek() {
                            self.advance();
                            Token::RepeatStart
                        } else {
                            // Just ||, which is invalid - must be ||: for repeat start
                            return Err(GenError::ParseError {
                                line,
                                column,
                                message: "Unexpected '||'. Did you mean '||:' for repeat start?".to_string(),
                            });
                        }
                    } else {
                        // Standalone | is not valid anymore
                        return Err(GenError::ParseError {
                            line,
                            column,
                            message: "Unexpected '|'. Note: '|' is no longer used for rhythm. For half notes, use 'd'. For repeats, use '||:' or ':||'.".to_string(),
                        });
                    }
                }
                ':' => {
                    self.advance();
                    // Check for :|| (repeat end)
                    if let Some(&'|') = self.peek() {
                        self.advance();
                        if let Some(&'|') = self.peek() {
                            self.advance();
                            Token::RepeatEnd
                        } else {
                            // Just :|, invalid - return error
                            return Err(GenError::ParseError {
                                line,
                                column,
                                message: "Unexpected ':' followed by single '|'. Did you mean ':||'?".to_string(),
                            });
                        }
                    } else {
                        // Standalone : is not valid in Gen syntax
                        return Err(GenError::ParseError {
                            line,
                            column,
                            message: "Unexpected ':'. Did you mean ':||' for repeat end?".to_string(),
                        });
                    }
                }
                'd' => {
                    self.advance();
                    Token::SmallD
                }
                'o' => {
                    self.advance();
                    Token::SmallO
                }
                '*' => {
                    self.advance();
                    Token::Asterisk
                }
                'A' => {
                    self.advance();
                    Token::NoteA
                }
                'B' => {
                    self.advance();
                    Token::NoteB
                }
                'C' => {
                    self.advance();
                    Token::NoteC
                }
                'D' => {
                    self.advance();
                    Token::NoteD
                }
                'E' => {
                    self.advance();
                    Token::NoteE
                }
                'F' => {
                    self.advance();
                    Token::NoteF
                }
                'G' => {
                    self.advance();
                    Token::NoteG
                }
                '$' => {
                    self.advance();
                    Token::Rest
                }
                '#' => {
                    self.advance();
                    Token::Sharp
                }
                'b' => {
                    self.advance();
                    Token::Flat
                }
                '%' => {
                    self.advance();
                    Token::Natural
                }
                '_' => {
                    self.advance();
                    Token::Underscore
                }
                '^' => {
                    self.advance();
                    Token::Caret
                }
                '[' => {
                    self.advance();
                    Token::LeftBracket
                }
                ']' => {
                    self.advance();
                    Token::RightBracket
                }
                '0'..='9' => {
                    self.advance();
                    Token::Number(c.to_digit(10).unwrap() as u8)
                }
                '-' => {
                    self.advance();
                    Token::Hyphen
                }
                '(' => {
                    self.advance();
                    Token::LeftParen
                }
                ')' => {
                    self.advance();
                    Token::RightParen
                }
                '\n' => {
                    self.advance();
                    Token::Newline
                }
                ' ' | '\t' | '\r' => {
                    self.advance();
                    Token::Whitespace
                }
                '@' => {
                    // Annotation/mod point - validate format: @Eb:^, @Bb:_, @ch:Cmaj7, or @:^
                    self.advance();

                    // Collect the annotation content until whitespace, @ or newline
                    let start_pos = self.position;
                    while let Some(&ch) = self.peek() {
                        if ch == '\n' || ch == ' ' || ch == '\t' || ch == '@' {
                            break;
                        }
                        self.advance();
                    }
                    let annotation = &self.input[start_pos..self.position];

                    // Check if this is a chord annotation (@ch:XXX)
                    if annotation.starts_with("ch:") {
                        if annotation.len() <= 3 {
                            return Err(GenError::ParseError {
                                line,
                                column,
                                message: "Chord annotation '@ch:' requires a chord symbol".to_string(),
                            });
                        }
                        // Valid chord annotation - skip it (will be extracted by parser)
                        continue;
                    }

                    // Check if this is a measure octave modifier (@:^, @:_, @:^^, @:__)
                    if annotation.starts_with(':') {
                        let modifier = &annotation[1..];
                        let valid_modifier = matches!(modifier, "^" | "_" | "^^" | "__");
                        if !valid_modifier {
                            return Err(GenError::ParseError {
                                line,
                                column,
                                message: format!("Invalid measure octave modifier '@{}'. Expected: @:^, @:_, @:^^, or @:__", annotation),
                            });
                        }
                        // Valid measure octave modifier - skip it (will be extracted by parser)
                        continue;
                    }

                    // Otherwise, validate mod point format: should be like "Eb:^" or "Bb:_"
                    // Format: Group (Eb or Bb) + colon + modifier (^ or _)
                    if !annotation.is_empty() {
                        let valid = if let Some(colon_pos) = annotation.find(':') {
                            let group = &annotation[..colon_pos];
                            let modifier = &annotation[colon_pos + 1..];
                            let valid_group = group.eq_ignore_ascii_case("Eb") || group.eq_ignore_ascii_case("Bb");
                            let valid_modifier = modifier == "^" || modifier == "_" || modifier == "^^" || modifier == "__";
                            valid_group && valid_modifier
                        } else {
                            false
                        };

                        if !valid {
                            return Err(GenError::ParseError {
                                line,
                                column,
                                message: format!("Invalid annotation '@{}'. Expected: @ch:ChordName, @Eb:^, @Bb:_, or @:^", annotation.trim()),
                            });
                        }
                    } else {
                        return Err(GenError::ParseError {
                            line,
                            column,
                            message: "Empty annotation. Expected: @ch:ChordName, @Eb:^, @Bb:_, or @:^".to_string(),
                        });
                    }

                    // Don't emit a token, continue to process more characters (possibly another @)
                    continue;
                }
                _ => {
                    return Err(GenError::ParseError {
                        line,
                        column,
                        message: format!("Unexpected character: '{}'", c),
                    });
                }
            };

            tokens.push(LocatedToken {
                token,
                line,
                column,
            });
        }

        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_notes() {
        let mut lexer = Lexer::new("C D E");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::NoteE,
            ]
        );
    }

    #[test]
    fn test_rhythm_modifiers() {
        let mut lexer = Lexer::new("/C dD");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::Slash,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::SmallD,
                &Token::NoteD,
            ]
        );
    }

    #[test]
    fn test_repeat_start() {
        let mut lexer = Lexer::new("||: C D");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::RepeatStart,
                &Token::Whitespace,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
            ]
        );
    }

    #[test]
    fn test_repeat_end() {
        let mut lexer = Lexer::new("C D :||");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::RepeatEnd,
            ]
        );
    }

    #[test]
    fn test_repeat_both() {
        let mut lexer = Lexer::new("||: C D :||");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::RepeatStart,
                &Token::Whitespace,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::RepeatEnd,
            ]
        );
    }

    #[test]
    fn test_first_ending() {
        let mut lexer = Lexer::new("1. C D :||");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::FirstEnding,
                &Token::Whitespace,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::RepeatEnd,
            ]
        );
    }

    #[test]
    fn test_second_ending() {
        let mut lexer = Lexer::new("2. C D");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::SecondEnding,
                &Token::Whitespace,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
            ]
        );
    }

    #[test]
    fn test_comment_skipped() {
        let mut lexer = Lexer::new("C D E @Eb:^");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::NoteE,
                &Token::Whitespace,
            ]
        );
    }

    #[test]
    fn test_comment_with_newline() {
        let mut lexer = Lexer::new("C D E @Eb:^\nF G A");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::NoteE,
                &Token::Whitespace,
                &Token::Newline,
                &Token::NoteF,
                &Token::Whitespace,
                &Token::NoteG,
                &Token::Whitespace,
                &Token::NoteA,
            ]
        );
    }

    #[test]
    fn test_multiple_annotations() {
        let mut lexer = Lexer::new("C D @Eb:^ @Bb:_");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        // Annotations are skipped, but whitespace between them is captured
        assert_eq!(
            token_types,
            vec![
                &Token::NoteC,
                &Token::Whitespace,
                &Token::NoteD,
                &Token::Whitespace,
                &Token::Whitespace, // space between the two annotations
            ]
        );
    }

    #[test]
    fn test_invalid_annotation_missing_modifier() {
        let mut lexer = Lexer::new("C D E @Eb:");
        let result = lexer.tokenize();
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            GenError::ParseError { message, .. } => {
                assert!(message.contains("Invalid annotation"));
            }
            _ => panic!("Expected ParseError"),
        }
    }

    #[test]
    fn test_invalid_annotation_wrong_format() {
        let mut lexer = Lexer::new("C D E @foo");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_annotation_missing_colon() {
        let mut lexer = Lexer::new("C D E @Eb^");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }
}
