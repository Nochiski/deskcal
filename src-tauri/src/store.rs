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
            // Write to a temp file, flush to disk, then atomically replace — so a crash or a
            // hard kill never leaves a truncated / zero-filled JSON behind.
            let tmp = path.with_extension("json.tmp");
            let result = (|| -> std::io::Result<()> {
                use std::io::Write;
                let mut f = fs::File::create(&tmp)?;
                f.write_all(s.as_bytes())?;
                f.sync_all()?;
                drop(f);
                fs::rename(&tmp, path)
            })();
            if let Err(e) = result {
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
    log::info!(
        "google oauth client: {} (built-in default {})",
        if s.google.client_id.is_empty() { "not configured" } else { "configured" },
        if option_env!("DESKCAL_GOOGLE_CLIENT_ID").map(|v| !v.is_empty()).unwrap_or(false) { "present" } else { "absent" }
    );
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

/// Legacy single-account key (migrated to the per-account key on first load).
pub const SECRET_GOOGLE_REFRESH: &str = "google_refresh_token";
pub const SECRET_APPLE_PASSWORD: &str = "apple_app_password";

pub fn google_secret_key(account_id: &str) -> String {
    format!("google_refresh_token:{account_id}")
}

/// Migrates pre-multi-account data: the single `google_email` + legacy keyring entry become a
/// Google account with id "legacy", and calendar prefs keyed `google:<cal>` become
/// `google:legacy:<cal>`. Cached Google events are dropped (they re-sync with the new ids).
pub fn migrate_accounts(p: &Paths, accounts: &mut Accounts, settings: &mut Settings, cache: &mut SyncResult) {
    let Some(email) = accounts.google_email.take() else { return };
    let legacy_id = "legacy".to_string();
    if let Some(token) = get_secret(SECRET_GOOGLE_REFRESH) {
        if set_secret(&google_secret_key(&legacy_id), &token).is_ok() {
            delete_secret(SECRET_GOOGLE_REFRESH);
        }
        if !accounts.google_accounts.iter().any(|a| a.id == legacy_id) {
            accounts
                .google_accounts
                .push(crate::model::GoogleAccount { id: legacy_id.clone(), email });
        }
    }
    save_accounts(p, accounts);

    let prefs: Vec<(String, crate::model::CalendarPrefs)> = settings
        .calendars
        .iter()
        .filter(|(k, _)| k.starts_with("google:") && !k.starts_with("google:legacy:"))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if !prefs.is_empty() {
        for (k, v) in prefs {
            let rest = &k["google:".len()..];
            settings.calendars.remove(&k);
            settings.calendars.insert(format!("google:{legacy_id}:{rest}"), v);
        }
        save_settings(p, settings);
    }
    cache.calendars.retain(|c| c.provider != crate::model::Provider::Google);
    cache.events.retain(|e| !e.calendar_id.starts_with("google:"));
    save_cache(p, cache);
    log::info!("migrated legacy google account to multi-account storage");
}

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
