use std::env;
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("scores.rs");

    let mut code = String::new();
    code.push_str("/// Embedded score files\n");
    code.push_str("pub static SCORES: &[(&str, &str)] = &[\n");

    let scores_dir = Path::new("examples");

    if scores_dir.exists() {
        for entry in WalkDir::new(scores_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "gen"))
        {
            let path = entry.path();
            let relative_path = path.strip_prefix(scores_dir).unwrap();
            let name = relative_path.to_string_lossy();

            // Read the file content
            if let Ok(content) = fs::read_to_string(path) {
                let escaped = content.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                code.push_str(&format!("    (\"{}\", \"{}\"),\n", name, escaped));
            }
        }
    }

    code.push_str("];\n");

    fs::write(&dest_path, code).unwrap();

    println!("cargo:rerun-if-changed=examples");
}
