use thiserror::Error;

#[derive(Error, Debug)]
pub enum GenError {
    #[error("Parse error at line {line}, column {column}: {message}")]
    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    #[error("Invalid metadata: {0}")]
    MetadataError(String),

    #[error("Semantic error at measure {measure}: {message}")]
    SemanticError { measure: usize, message: String },
}
