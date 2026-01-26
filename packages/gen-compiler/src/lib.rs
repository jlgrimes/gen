pub mod ast;
pub mod error;
pub mod lexer;
pub mod musicxml;
pub mod parser;
pub mod semantic;
pub mod transpose;

pub use ast::*;
pub use error::*;
pub use musicxml::to_musicxml;
pub use parser::parse;
pub use semantic::validate;
pub use transpose::transpose_score;

/// Compile a Gen source string to MusicXML.
/// This is the main entry point for the library.
pub fn compile(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    validate(&score)?;
    Ok(to_musicxml(&score))
}

/// Compile without validation (useful for partial/incomplete scores)
pub fn compile_unchecked(source: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    Ok(to_musicxml(&score))
}

/// Compile with transposition for a viewed key (e.g., "Bb", "Eb", "F")
pub fn compile_transposed(source: &str, viewed_key: &str) -> Result<String, GenError> {
    let score = parse(source)?;
    let transposed = transpose_score(&score, viewed_key)
        .ok_or_else(|| GenError::MetadataError(format!("Unknown transposition key: {}", viewed_key)))?;
    Ok(to_musicxml(&transposed))
}
