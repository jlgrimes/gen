use wasm_bindgen::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct CompileError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Serialize)]
struct ScoreInfo {
    name: String,
    content: String,
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

/// Get all embedded scores
#[wasm_bindgen]
pub fn list_scores() -> JsValue {
    let scores: Vec<ScoreInfo> = gen_scores::get_all_scores()
        .into_iter()
        .map(|s| ScoreInfo {
            name: s.name,
            content: s.content,
        })
        .collect();
    serde_wasm_bindgen::to_value(&scores).unwrap()
}

/// Get a specific score by name
#[wasm_bindgen]
pub fn get_score(name: &str) -> JsValue {
    match gen_scores::get_score(name) {
        Some(s) => serde_wasm_bindgen::to_value(&ScoreInfo {
            name: s.name,
            content: s.content,
        })
        .unwrap(),
        None => JsValue::NULL,
    }
}
