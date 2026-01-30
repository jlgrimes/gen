use serde::Serialize;
use tauri::command;

#[derive(Serialize)]
struct CompileError {
    message: String,
    line: Option<usize>,
    column: Option<usize>,
}

#[derive(Serialize)]
#[serde(tag = "status")]
enum CompileResult {
    #[serde(rename = "success")]
    Success { xml: String },
    #[serde(rename = "error")]
    Error { error: CompileError },
}

#[command]
fn compile_gen(source: &str) -> CompileResult {
    match gen::compile(source) {
        Ok(xml) => CompileResult::Success { xml },
        Err(e) => CompileResult::Error {
            error: error_to_compile_error(e),
        },
    }
}

#[command]
fn compile_gen_unchecked(source: &str) -> CompileResult {
    match gen::compile_unchecked(source) {
        Ok(xml) => CompileResult::Success { xml },
        Err(e) => CompileResult::Error {
            error: error_to_compile_error(e),
        },
    }
}

#[command]
fn compile_gen_with_options(source: &str, clef: &str, octave_shift: i8, transpose_key: Option<&str>) -> CompileResult {
    let transposition = transpose_key.and_then(gen::Transposition::for_key);
    match gen::compile_with_options(source, clef, octave_shift, transposition) {
        Ok(xml) => CompileResult::Success { xml },
        Err(e) => CompileResult::Error {
            error: error_to_compile_error(e),
        },
    }
}

#[command]
fn compile_gen_with_mod_points(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
    transpose_key: Option<&str>,
) -> CompileResult {
    match gen::compile_with_mod_points(source, clef, octave_shift, instrument_group, transpose_key) {
        Ok(xml) => CompileResult::Success { xml },
        Err(e) => CompileResult::Error {
            error: error_to_compile_error(e),
        },
    }
}

#[derive(Serialize)]
#[serde(tag = "status")]
enum PlaybackResult {
    #[serde(rename = "success")]
    Success { data: gen::PlaybackData },
    #[serde(rename = "error")]
    Error { error: CompileError },
}

#[command]
fn generate_playback_data(
    source: &str,
    clef: &str,
    octave_shift: i8,
    instrument_group: Option<&str>,
    transpose_key: Option<&str>,
) -> PlaybackResult {
    match gen::generate_playback_data(source, clef, octave_shift, instrument_group, transpose_key) {
        Ok(data) => PlaybackResult::Success { data },
        Err(e) => PlaybackResult::Error {
            error: error_to_compile_error(e),
        },
    }
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

#[command]
fn open_external_url(url: &str) -> Result<(), String> {
    open::that(url).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            compile_gen,
            compile_gen_unchecked,
            compile_gen_with_options,
            compile_gen_with_mod_points,
            generate_playback_data,
            open_external_url,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
