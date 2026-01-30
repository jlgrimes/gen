use wasm_bindgen::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct CompileError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Serialize)]
struct Diagnostic {
    message: String,
    line: usize,
    column: usize,
    end_line: usize,
    end_column: usize,
    severity: String,
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

fn error_to_diagnostic(e: gen::GenError, measure_to_line: &HashMap<usize, usize>, source: &str) -> Diagnostic {
    let lines: Vec<&str> = source.lines().collect();

    match e {
        gen::GenError::ParseError { line, column, message } => {
            let line_len = lines.get(line.saturating_sub(1)).map(|l| l.len()).unwrap_or(1);
            Diagnostic {
                message,
                line,
                column,
                end_line: line,
                end_column: line_len + 1,
                severity: "error".to_string(),
            }
        },
        gen::GenError::MetadataError(msg) => {
            Diagnostic {
                message: msg,
                line: 1,
                column: 1,
                end_line: 1,
                end_column: 1,
                severity: "error".to_string(),
            }
        },
        gen::GenError::SemanticError { measure, message } => {
            // Convert measure number (1-indexed) to line number using the mapping
            let line = measure_to_line.get(&(measure - 1)).copied().unwrap_or(1);
            let line_len = lines.get(line.saturating_sub(1)).map(|l| l.len()).unwrap_or(1);
            Diagnostic {
                message,
                line,
                column: 1,
                end_line: line,
                end_column: line_len + 1,
                severity: "error".to_string(),
            }
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
pub fn compile_with_options(source: &str, clef: &str, octave_shift: i8, transpose_key: Option<String>) -> Result<String, JsValue> {
    let transposition = transpose_key.as_deref().and_then(gen::Transposition::for_key);
    gen::compile_with_options(source, clef, octave_shift, transposition)
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

/// Compile Gen source to MusicXML with mod points support for instrument-specific octave shifts
#[wasm_bindgen]
pub fn compile_with_mod_points(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<String>,
    transpose_key: Option<String>,
) -> Result<String, JsValue> {
    gen::compile_with_mod_points(source, clef, octave_shift, instrument_group.as_deref(), transpose_key.as_deref())
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

/// Lint Gen source and return diagnostics as JSON array
/// Returns diagnostics with line/column info for inline editor display
/// Generate playback data for a score
#[wasm_bindgen]
pub fn generate_playback_data(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<String>,
    transpose_key: Option<String>,
) -> Result<String, JsValue> {
    gen::generate_playback_data(source, clef, octave_shift, instrument_group.as_deref(), transpose_key.as_deref())
        .map(|data| serde_json::to_string(&data).unwrap())
        .map_err(|e| JsValue::from_str(&serde_json::to_string(&error_to_compile_error(e)).unwrap()))
}

#[wasm_bindgen]
pub fn lint(source: &str) -> String {
    let mut diagnostics: Vec<Diagnostic> = Vec::new();

    // First, try to parse the source
    match gen::parse(source) {
        Ok(score) => {
            // Build reverse mapping: measure_index -> line_number
            let measure_to_line: HashMap<usize, usize> = score
                .line_to_measure
                .iter()
                .map(|(&line, &measure_idx)| (measure_idx, line))
                .collect();

            // Run semantic validation
            if let Err(e) = gen::validate(&score) {
                diagnostics.push(error_to_diagnostic(e, &measure_to_line, source));
            }
        }
        Err(e) => {
            // Parse error - use empty measure_to_line mapping
            let empty_mapping: HashMap<usize, usize> = HashMap::new();
            diagnostics.push(error_to_diagnostic(e, &empty_mapping, source));
        }
    }

    serde_json::to_string(&diagnostics).unwrap_or_else(|_| "[]".to_string())
}
