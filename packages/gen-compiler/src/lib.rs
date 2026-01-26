pub mod ast;
pub mod error;
pub mod lexer;
pub mod musicxml;
pub mod parser;
pub mod semantic;

pub use ast::*;
pub use error::*;
pub use musicxml::to_musicxml;
pub use parser::parse;
pub use semantic::validate;

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
