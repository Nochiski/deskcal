//! Persistence: settings / cache JSON files in the app config dir, secrets in the OS keyring.
use crate::model::{Accounts, Settings, SyncResult, WindowGeometry};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

const KEYRING_SERVICE: &str = "DeskCal";

pub struct Paths {
    pub settings: PathBuf,
    pub accounts: PathBuf,
    pub cache: PathBuf,
    pub window: PathBuf,
}

impl Paths {
    pub fn new(app: &AppHandle) -> Self {
        let dir = app
            .path()
            .app_config_dir()
            .unwrap_or_else(|_| PathBuf::from("."));
        let _ = fs::create_dir_all(&dir);
        Self {
            settings: dir.join("settings.json"),
            accounts: dir.join("accounts.json"),
            cache: dir.join("cache.json"),
            window: dir.join("window.json"),
        }
    }
}

fn read_json<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            log::warn!("failed to parse {}: {e}", path.display());
            T::default()
        }),
        Err(_) => T::default(),
    }
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            let tmp = path.with_extension("json.tmp");
            if let Err(e) = fs::write(&tmp, s).and_then(|_| fs::rename(&tmp, path)) {
                log::error!("failed to write {}: {e}", path.display());
            }
        }
        Err(e) => log::error!("serialize {}: {e}", path.display()),
    }
}

pub fn load_settings(p: &Paths) -> Settings {
    let mut s: Settings = read_json(&p.settings);
    // Fill in build-time defaults when the user hasn't set a client id.
    if s.google.client_id.is_empty() {
        s.google = crate::model::GoogleClient::default();
    }
    s
}
pub fn save_settings(p: &Paths, s: &Settings) {
    write_json(&p.settings, s)
}
pub fn load_accounts(p: &Paths) -> Accounts {
    read_json(&p.accounts)
}
pub fn save_accounts(p: &Paths, a: &Accounts) {
    write_json(&p.accounts, a)
}
pub fn load_cache(p: &Paths) -> SyncResult {
    read_json(&p.cache)
}
pub fn save_cache(p: &Paths, c: &SyncResult) {
    write_json(&p.cache, c)
}
pub fn load_window(p: &Paths) -> WindowGeometry {
    read_json(&p.window)
}
pub fn save_window(p: &Paths, w: &WindowGeometry) {
    write_json(&p.window, w)
}

// ── secrets ──

pub const SECRET_GOOGLE_REFRESH: &str = "google_refresh_token";
pub const SECRET_APPLE_PASSWORD: &str = "apple_app_password";

pub fn get_secret(key: &str) -> Option<String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key).ok()?;
    match entry.get_password() {
        Ok(v) => Some(v),
        Err(keyring::Error::NoEntry) => None,
        Err(e) => {
            log::warn!("keyring read {key}: {e}");
            None
        }
    }
}

pub fn set_secret(key: &str, value: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, key).map_err(|e| e.to_string())?;
    entry
        .set_password(value)
        .map_err(|e| format!("자격 증명 저장 실패: {e}"))
}

pub fn delete_secret(key: &str) {
    if let Ok(entry) = keyring::Entry::new(KEYRING_SERVICE, key) {
        let _ = entry.delete_credential();
    }
}
