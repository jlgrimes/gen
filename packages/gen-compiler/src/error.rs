//! # Error Types
//!
//! This module defines all error types for the Gen compiler.
//!
//! All errors include location information (line/column or measure number) to help users
//! identify and fix issues in their Gen source code.
//!
//! ## Error Types
//! - `ParseError` - Lexer/parser errors with line and column information
//! - `MetadataError` - Invalid YAML metadata in frontmatter
//! - `SemanticError` - Validation errors with measure number
//!
//! ## Usage
//! ```rust
//! use gen::{compile, GenError};
//!
//! match compile(source) {
//!     Ok(musicxml) => println!("Success!"),
//!     Err(GenError::ParseError { line, column, message }) => {
//!         eprintln!("Parse error at {}:{}: {}", line, column, message);
//!     }
//!     Err(GenError::SemanticError { measure, message }) => {
//!         eprintln!("Semantic error in measure {}: {}", measure, message);
//!     }
//!     Err(e) => eprintln!("Error: {}", e),
//! }
//! ```

use thiserror::Error;

#[derive(Error, Debug)]
pub enum GenError {
    /// Parse error with location information.
    ///
    /// Occurs during lexing or parsing when the Gen source has syntax errors.
    ///
    /// # Example
    /// ```
    /// # use gen::GenError;
    /// let err = GenError::ParseError {
    ///     line: 5,
    ///     column: 10,
    ///     message: "Unexpected token 'X'".to_string(),
    /// };
    /// assert_eq!(err.to_string(), "Parse error at line 5, column 10: Unexpected token 'X'");
    /// ```
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    /// Invalid metadata error.
    ///
    /// Occurs when YAML frontmatter is invalid or contains unsupported values.
    ///
    /// # Example
    /// ```
    /// # use gen::GenError;
    /// let err = GenError::MetadataError("time-signature must be in format N/D".to_string());
    /// assert_eq!(err.to_string(), "Invalid metadata: time-signature must be in format N/D");
    /// ```
    #[error("Invalid metadata: {0}")]
    MetadataError(String),

    /// Semantic validation error with measure information.
    ///
    /// Occurs during validation when measure durations don't match time signature,
    /// repeats are unmatched, or endings are incorrectly structured.
    ///
    /// # Example
    /// ```
    /// # use gen::GenError;
    /// let err = GenError::SemanticError {
    ///     measure: 3,
    ///     message: "Measure duration (3.5 beats) doesn't match time signature (4/4 = 4 beats)".to_string(),
    /// };
    /// assert_eq!(
    ///     err.to_string(),
    ///     "Semantic error at measure 3: Measure duration (3.5 beats) doesn't match time signature (4/4 = 4 beats)"
    /// );
    /// ```
    #[error("Semantic error at measure {measure}: {message}")]
    SemanticError { measure: usize, message: String },
}
