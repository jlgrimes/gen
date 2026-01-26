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

#[derive(Serialize)]
struct ScoreInfo {
    name: String,
    content: String,
}

#[command]
fn list_scores() -> Vec<ScoreInfo> {
    gen_scores::get_all_scores()
        .into_iter()
        .map(|s| ScoreInfo {
            name: s.name.to_string(),
            content: s.content.to_string(),
        })
        .collect()
}

#[command]
fn get_score(name: &str) -> Option<ScoreInfo> {
    gen_scores::get_score(name).map(|s| ScoreInfo {
        name: s.name.to_string(),
        content: s.content.to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![
            compile_gen,
            compile_gen_unchecked,
            list_scores,
            get_score
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
