use std::fs;
use std::path::Path;

/// Loads DESKCAL_* values from a `.env` file (project root or src-tauri) so they can be baked into
/// the binary via `option_env!`. Real environment variables take precedence.
fn load_dotenv() {
    for candidate in ["../.env", ".env"] {
        let path = Path::new(candidate);
        println!("cargo:rerun-if-changed={}", path.display());
        let Ok(text) = fs::read_to_string(path) else { continue };
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((k, v)) = line.split_once('=') else { continue };
            let k = k.trim();
            if !k.starts_with("DESKCAL_") {
                continue;
            }
            if std::env::var(k).map(|s| !s.is_empty()).unwrap_or(false) {
                continue;
            }
            let v = v.trim().trim_matches('"').trim_matches('\'');
            println!("cargo:rustc-env={k}={v}");
        }
    }
    println!("cargo:rerun-if-env-changed=DESKCAL_GOOGLE_CLIENT_ID");
    println!("cargo:rerun-if-env-changed=DESKCAL_GOOGLE_CLIENT_SECRET");
}

fn main() {
    load_dotenv();
    tauri_build::build()
}
