//! Tauri commands exposed to the frontend (see src/lib/api.ts) plus shared helpers.
use crate::apple::{self, AppleCreds};
use crate::google;
use crate::model::{AccountInfo, Provider, Settings, SyncResult, WindowMode};
use crate::state::AppState;
use crate::store;
use crate::tray::TrayItems;
use crate::window_mode;
use chrono::NaiveDate;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_opener::OpenerExt;

// ── helpers shared with the tray ──

pub fn toggle_main_window(app: &AppHandle) {
    let Some(w) = app.get_webview_window("main") else { return };
    match w.is_visible() {
        Ok(true) => {
            let _ = w.hide();
        }
        _ => {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

/// Applies a window mode, persists it, updates the tray checks and notifies the UI.
pub fn apply_window_mode(app: &AppHandle, mode: WindowMode) {
    let Some(w) = app.get_webview_window("main") else { return };
    let _ = w.show();
    if let Err(e) = window_mode::apply(&w, mode) {
        log::error!("window mode {mode:?} failed: {e}");
        return;
    }
    let state = app.state::<AppState>();
    let mut s = state.settings();
    if s.window_mode != mode {
        s.window_mode = mode;
        state.set_settings(s);
    }
    if let Some(items) = app.try_state::<TrayItems>() {
        items.set_mode(mode);
    }
    let _ = app.emit("window-mode-changed", mode);
}

fn apply_autostart(app: &AppHandle, enabled: bool) {
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    if let Err(e) = r {
        log::warn!("autostart update failed: {e}");
    }
}

fn accounts_of(state: &AppState) -> Vec<AccountInfo> {
    let a = state.accounts();
    let google_connected = a.google_email.is_some() && store::get_secret(store::SECRET_GOOGLE_REFRESH).is_some();
    let apple_connected = a.apple_id.is_some() && store::get_secret(store::SECRET_APPLE_PASSWORD).is_some();
    vec![
        AccountInfo {
            provider: Provider::Google,
            label: a.google_email.clone().unwrap_or_default(),
            connected: google_connected,
        },
        AccountInfo {
            provider: Provider::Apple,
            label: a.apple_id.clone().unwrap_or_default(),
            connected: apple_connected,
        },
    ]
}

fn parse_date(s: &str) -> Result<NaiveDate, String> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|e| format!("잘못된 날짜 '{s}': {e}"))
}

// ── commands ──

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<(), String> {
    let prev = state.settings();
    state.set_settings(settings.clone());
    if prev.autostart != settings.autostart {
        apply_autostart(&app, settings.autostart);
    }
    if prev.window_mode != settings.window_mode {
        apply_window_mode(&app, settings.window_mode);
    }
    Ok(())
}

#[tauri::command]
pub fn get_accounts(state: State<'_, AppState>) -> Vec<AccountInfo> {
    accounts_of(&state)
}

#[tauri::command]
pub async fn connect_google(app: AppHandle, state: State<'_, AppState>) -> Result<AccountInfo, String> {
    let s = state.settings();
    let opener = app.clone();
    let result = google::login(&state.http, &s.google.client_id, &s.google.client_secret, move |url| {
        opener
            .opener()
            .open_url(url, None::<&str>)
            .map_err(|e| format!("브라우저 열기 실패: {e}"))
    })
    .await?;
    store::set_secret(store::SECRET_GOOGLE_REFRESH, &result.refresh_token)?;
    *state.google_token.lock().unwrap() = Some(google::AccessToken {
        token: result.access_token,
        expires_at: chrono::Utc::now() + chrono::Duration::seconds(result.expires_in - 60),
    });
    let mut a = state.accounts();
    a.google_email = Some(result.email.clone());
    state.set_accounts(a);
    Ok(AccountInfo { provider: Provider::Google, label: result.email, connected: true })
}

#[tauri::command]
pub fn disconnect_google(state: State<'_, AppState>) -> Result<(), String> {
    store::delete_secret(store::SECRET_GOOGLE_REFRESH);
    *state.google_token.lock().unwrap() = None;
    let mut a = state.accounts();
    a.google_email = None;
    state.set_accounts(a);
    let mut c = state.cache.lock().unwrap().clone();
    c.calendars.retain(|cal| cal.provider != Provider::Google);
    c.events.retain(|e| !e.calendar_id.starts_with("google:"));
    state.set_cache(c);
    Ok(())
}

#[tauri::command]
pub async fn connect_apple(
    state: State<'_, AppState>,
    apple_id: String,
    app_password: String,
) -> Result<AccountInfo, String> {
    let apple_id = apple_id.trim().to_string();
    let password = app_password.trim().replace('-', "").replace(' ', "");
    if apple_id.is_empty() || password.is_empty() {
        return Err("Apple ID와 앱 암호를 모두 입력해 주세요.".into());
    }
    let creds = AppleCreds { apple_id: apple_id.clone(), password: password.clone() };
    let home = apple::discover_home(&state.http, &creds).await?;
    store::set_secret(store::SECRET_APPLE_PASSWORD, &password)?;
    let mut a = state.accounts();
    a.apple_id = Some(apple_id.clone());
    a.apple_home_url = Some(home);
    state.set_accounts(a);
    Ok(AccountInfo { provider: Provider::Apple, label: apple_id, connected: true })
}

#[tauri::command]
pub fn disconnect_apple(state: State<'_, AppState>) -> Result<(), String> {
    store::delete_secret(store::SECRET_APPLE_PASSWORD);
    let mut a = state.accounts();
    a.apple_id = None;
    a.apple_home_url = None;
    state.set_accounts(a);
    let mut c = state.cache.lock().unwrap().clone();
    c.calendars.retain(|cal| cal.provider != Provider::Apple);
    c.events.retain(|e| !e.calendar_id.starts_with("apple:"));
    state.set_cache(c);
    Ok(())
}

#[tauri::command]
pub fn get_cached(state: State<'_, AppState>) -> SyncResult {
    state.cache.lock().unwrap().clone()
}

#[tauri::command]
pub async fn sync_now(app: AppHandle, range_start: String, range_end: String) -> Result<SyncResult, String> {
    let s = parse_date(&range_start)?;
    let e = parse_date(&range_end)?;
    if e <= s {
        return Err("종료일이 시작일보다 앞섭니다.".into());
    }
    Ok(crate::sync::sync_range(&app, s, e).await)
}

#[tauri::command]
pub fn set_window_mode(app: AppHandle, mode: WindowMode) -> Result<(), String> {
    apply_window_mode(&app, mode);
    Ok(())
}

#[tauri::command]
pub fn hide_window(app: AppHandle) -> Result<(), String> {
    if let Some(w) = app.get_webview_window("main") {
        w.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("http(s) URL만 열 수 있습니다.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
