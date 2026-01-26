use serde::Serialize;
use tauri::command;

#[command]
fn compile_gen(source: &str) -> Result<String, String> {
    gen::compile(source).map_err(|e| e.to_string())
}

#[command]
fn compile_gen_unchecked(source: &str) -> Result<String, String> {
    gen::compile_unchecked(source).map_err(|e| e.to_string())
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
        .invoke_handler(tauri::generate_handler![
            compile_gen,
            compile_gen_unchecked,
            list_scores,
            get_score
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
