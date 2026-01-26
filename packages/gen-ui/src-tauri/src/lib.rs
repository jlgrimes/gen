use tauri::command;

#[command]
fn compile_gen(source: &str) -> Result<String, String> {
    gen::compile(source).map_err(|e| e.to_string())
}

#[command]
fn compile_gen_unchecked(source: &str) -> Result<String, String> {
    gen::compile_unchecked(source).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![compile_gen, compile_gen_unchecked])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
