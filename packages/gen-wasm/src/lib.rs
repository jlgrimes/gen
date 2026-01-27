use wasm_bindgen::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct CompileError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

fn error_to_compile_error(e: gen::GenError) -> CompileError {
    match e {
        gen::GenError::ParseError { line, column, message } => CompileError {
            message,
            line: Some(line),
            column: Some(column),
        },
        gen::GenError::MetadataError(msg) => CompileError {
            message: msg,
            line: None,
            column: None,
        },
        gen::GenError::SemanticError { measure, message } => CompileError {
            message: format!("Measure {}: {}", measure, message),
            line: None,
            column: None,
        },
    }
}

/// Compile Gen source to MusicXML with validation
#[wasm_bindgen]
pub fn compile(source: &str) -> Result<String, JsValue> {
    gen::compile(source)
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

/// Compile Gen source to MusicXML without validation
#[wasm_bindgen]
pub fn compile_unchecked(source: &str) -> Result<String, JsValue> {
    gen::compile_unchecked(source)
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

/// Compile Gen source to MusicXML with custom clef and octave shift
#[wasm_bindgen]
pub fn compile_with_options(source: &str, clef: &str, octave_shift: i8) -> Result<String, JsValue> {
    gen::compile_with_options(source, clef, octave_shift)
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

/// Compile Gen source to MusicXML with mod points support for instrument-specific octave shifts
#[wasm_bindgen]
pub fn compile_with_mod_points(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<String>,
) -> Result<String, JsValue> {
    gen::compile_with_mod_points(source, clef, octave_shift, instrument_group.as_deref())
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}
