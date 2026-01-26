use crate::error::GenError;

/// Token types for the Gen language
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Rhythm modifiers
    Slash,          // /
    Pipe,           // |
    SmallO,         // o
    Asterisk,       // *

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
    Underscore,     // _
    Caret,          // ^

    // Tuplet/grouping
    LeftBracket,    // [
    RightBracket,   // ]
    Number(u8),     // 2, 3, 4, 5, 6, etc.

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

    fn consume_metadata_content(&mut self) -> String {
        let start = self.position;

        // Consume until we hit another ---
        while self.peek().is_some() {
            if self.check_metadata_marker() {
                break;
            }
            self.advance();
        }

        self.input[start..self.position].to_string()
    }

    pub fn tokenize(&mut self) -> Result<Vec<LocatedToken>, GenError> {
        let mut tokens = Vec::new();
        let mut metadata_started = false;

        while let Some(&c) = self.peek() {
            let line = self.line;
            let column = self.column;

            // Check for metadata markers
            if self.check_metadata_marker() {
                // Consume the three dashes
                self.advance(); // -
                self.advance(); // -
                self.advance(); // -

                tokens.push(LocatedToken {
                    token: Token::MetadataStart,
                    line,
                    column,
                });

                if !metadata_started {
                    metadata_started = true;

                    // Skip the newline after ---
                    if let Some(&'\n') = self.peek() {
                        self.advance();
                    }

                    // Consume metadata content
                    let content = self.consume_metadata_content();
                    if !content.trim().is_empty() {
                        tokens.push(LocatedToken {
                            token: Token::MetadataContent(content),
                            line: line + 1,
                            column: 1,
                        });
                    }
                }
                continue;
            }

            let token = match c {
                '/' => {
                    self.advance();
                    Token::Slash
                }
                '|' => {
                    self.advance();
                    Token::Pipe
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
                '\n' => {
                    self.advance();
                    Token::Newline
                }
                ' ' | '\t' | '\r' => {
                    self.advance();
                    Token::Whitespace
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
        let mut lexer = Lexer::new("/C |oD");
        let tokens = lexer.tokenize().unwrap();
        let token_types: Vec<_> = tokens.iter().map(|t| &t.token).collect();
        assert_eq!(
            token_types,
            vec![
                &Token::Slash,
                &Token::NoteC,
                &Token::Whitespace,
                &Token::Pipe,
                &Token::SmallO,
                &Token::NoteD,
            ]
        );
    }
}
