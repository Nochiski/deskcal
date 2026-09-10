//! Tauri commands exposed to the frontend (see src/lib/api.ts) plus shared helpers.
use crate::apple::{self, AppleCreds};
use crate::feed;
use crate::google;
use crate::model::{AccountInfo, EventInput, IcsFeed, Provider, Settings, SyncResult, WindowMode};
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
    let apple_connected = a.apple_id.is_some() && store::get_secret(store::SECRET_APPLE_PASSWORD).is_some();
    let mut out: Vec<AccountInfo> = a
        .google_accounts
        .iter()
        .map(|g| AccountInfo {
            provider: Provider::Google,
            id: g.id.clone(),
            label: g.email.clone(),
            connected: store::get_secret(&store::google_secret_key(&g.id)).is_some(),
        })
        .collect();
    out.push(AccountInfo {
        provider: Provider::Apple,
        id: "apple".into(),
        label: a.apple_id.clone().unwrap_or_default(),
        connected: apple_connected,
    });
    out.push(AccountInfo {
        provider: Provider::Ics,
        id: "ics".into(),
        label: if a.ics_feeds.is_empty() { String::new() } else { format!("{}개 구독", a.ics_feeds.len()) },
        connected: !a.ics_feeds.is_empty(),
    });
    out
}

async fn resync_last(app: &AppHandle) -> SyncResult {
    let (s, e) = *app.state::<AppState>().last_range.lock().unwrap();
    crate::sync::sync_range(app, s, e).await
}

/// Resolves (provider, account id, provider-side calendar id) from a CalendarInfo id.
/// Google ids are `google:<accountId>:<calendarId>`; Apple ids are `apple:<url>`.
fn split_calendar_id(calendar_id: &str) -> Result<(Provider, String, String), String> {
    if let Some(rest) = calendar_id.strip_prefix("google:") {
        let (account, cal) = rest
            .split_once(':')
            .ok_or_else(|| "이전 버전의 캘린더 ID입니다. 동기화 후 다시 시도해 주세요.".to_string())?;
        Ok((Provider::Google, account.to_string(), cal.to_string()))
    } else if let Some(rest) = calendar_id.strip_prefix("apple:") {
        Ok((Provider::Apple, "apple".to_string(), rest.to_string()))
    } else if calendar_id.starts_with("ics:") {
        Err("iCal 구독 캘린더는 읽기 전용입니다.".into())
    } else {
        Err(format!("알 수 없는 캘린더: {calendar_id}"))
    }
}

async fn google_access_token(state: &AppState, account_id: &str) -> Result<String, String> {
    let refresh = store::get_secret(&store::google_secret_key(account_id))
        .ok_or("이 Google 계정의 로그인 정보가 없습니다. 설정에서 다시 로그인해 주세요.")?;
    let s = state.settings();
    crate::sync::google_token(state, &state.http, &s.google.client_id, &s.google.client_secret, account_id, &refresh)
        .await
}

fn apple_creds(state: &AppState) -> Result<AppleCreds, String> {
    let a = state.accounts();
    let apple_id = a.apple_id.ok_or("iCloud 계정이 연결되어 있지 않습니다.")?;
    let password = store::get_secret(store::SECRET_APPLE_PASSWORD).ok_or("iCloud 앱 암호를 찾을 수 없습니다.")?;
    Ok(AppleCreds { apple_id, password })
}

fn validate_input(input: &EventInput) -> Result<(), String> {
    if input.title.trim().is_empty() {
        return Err("제목을 입력해 주세요.".into());
    }
    let s = crate::vevent::parse_input_when(&input.start, input.all_day)?;
    let e = crate::vevent::parse_input_when(&input.end, input.all_day)?;
    if crate::vevent::when_diff_secs(&s, &e) <= 0 {
        return Err("종료 시각은 시작 시각 이후여야 합니다.".into());
    }
    Ok(())
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
    // Abort any previous pending login, then arm a fresh cancel flag for this one.
    cancel_pending_login(&state);
    state.login_cancel.store(false, std::sync::atomic::Ordering::SeqCst);
    let cancel = state.login_cancel.clone();
    let port_slot = app.clone();
    let result = google::login(
        &state.http,
        &s.google.client_id,
        &s.google.client_secret,
        cancel,
        move |port| {
            *port_slot.state::<AppState>().login_port.lock().unwrap() = Some(port);
        },
        move |url| {
            opener
                .opener()
                .open_url(url, None::<&str>)
                .map_err(|e| format!("브라우저 열기 실패: {e}"))
        },
    )
    .await;
    *state.login_port.lock().unwrap() = None;
    let result = result?;
    let mut a = state.accounts();
    // Re-linking an already linked email replaces its token instead of duplicating the account.
    let id = match a.google_accounts.iter().find(|g| g.email.eq_ignore_ascii_case(&result.email)) {
        Some(g) => g.id.clone(),
        None => {
            let id = format!("{:x}", chrono::Utc::now().timestamp_millis());
            a.google_accounts.push(crate::model::GoogleAccount { id: id.clone(), email: result.email.clone() });
            id
        }
    };
    store::set_secret(&store::google_secret_key(&id), &result.refresh_token)?;
    state.google_token.lock().unwrap().insert(
        id.clone(),
        google::AccessToken {
            token: result.access_token,
            expires_at: chrono::Utc::now() + chrono::Duration::seconds(result.expires_in - 60),
        },
    );
    state.set_accounts(a);
    Ok(AccountInfo { provider: Provider::Google, id, label: result.email, connected: true })
}

fn cancel_pending_login(state: &AppState) {
    state.login_cancel.store(true, std::sync::atomic::Ordering::SeqCst);
    if let Some(port) = *state.login_port.lock().unwrap() {
        google::cancel_login(port);
    }
}

/// Aborts a Google login that is waiting for the browser (e.g. the user closed the tab).
#[tauri::command]
pub fn cancel_google_login(state: State<'_, AppState>) -> Result<(), String> {
    cancel_pending_login(&state);
    Ok(())
}

#[tauri::command]
pub fn disconnect_google(state: State<'_, AppState>, account_id: String) -> Result<(), String> {
    store::delete_secret(&store::google_secret_key(&account_id));
    state.google_token.lock().unwrap().remove(&account_id);
    let mut a = state.accounts();
    a.google_accounts.retain(|g| g.id != account_id);
    state.set_accounts(a);
    let prefix = format!("google:{account_id}:");
    let mut c = state.cache.lock().unwrap().clone();
    c.calendars.retain(|cal| !cal.id.starts_with(&prefix));
    c.events.retain(|e| !e.calendar_id.starts_with(&prefix));
    state.set_cache(c);
    let mut s = state.settings();
    s.calendars.retain(|k, _| !k.starts_with(&prefix));
    state.set_settings(s);
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
    Ok(AccountInfo { provider: Provider::Apple, id: "apple".into(), label: apple_id, connected: true })
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

// ── event writes ──

#[tauri::command]
pub async fn create_event(app: AppHandle, state: State<'_, AppState>, input: EventInput) -> Result<SyncResult, String> {
    validate_input(&input)?;
    let (provider, account, remote_cal) = split_calendar_id(&input.calendar_id)?;
    match provider {
        Provider::Google => {
            let token = google_access_token(&state, &account).await?;
            google::create_event(&state.http, &token, &remote_cal, &input).await?;
        }
        Provider::Apple => {
            let creds = apple_creds(&state)?;
            apple::create_event(&state.http, &creds, &remote_cal, &input).await?;
        }
        Provider::Ics => unreachable!(),
    }
    Ok(resync_last(&app).await)
}

#[tauri::command]
pub async fn update_event(
    app: AppHandle,
    state: State<'_, AppState>,
    calendar_id: String,
    remote_id: String,
    input: EventInput,
) -> Result<SyncResult, String> {
    validate_input(&input)?;
    if input.calendar_id != calendar_id {
        return Err("일정을 다른 캘린더로 옮기는 기능은 아직 지원하지 않습니다.".into());
    }
    let (provider, account, remote_cal) = split_calendar_id(&calendar_id)?;
    match provider {
        Provider::Google => {
            let token = google_access_token(&state, &account).await?;
            google::update_event(&state.http, &token, &remote_cal, &remote_id, &input).await?;
        }
        Provider::Apple => {
            let creds = apple_creds(&state)?;
            apple::update_event(&state.http, &creds, &remote_id, &input).await?;
        }
        Provider::Ics => unreachable!(),
    }
    Ok(resync_last(&app).await)
}

#[tauri::command]
pub async fn delete_event(
    app: AppHandle,
    state: State<'_, AppState>,
    calendar_id: String,
    remote_id: String,
) -> Result<SyncResult, String> {
    let (provider, account, remote_cal) = split_calendar_id(&calendar_id)?;
    match provider {
        Provider::Google => {
            let token = google_access_token(&state, &account).await?;
            google::delete_event(&state.http, &token, &remote_cal, &remote_id).await?;
        }
        Provider::Apple => {
            let creds = apple_creds(&state)?;
            apple::delete_event(&state.http, &creds, &remote_id).await?;
        }
        Provider::Ics => unreachable!(),
    }
    Ok(resync_last(&app).await)
}

// ── iCal subscription feeds ──

#[tauri::command]
pub fn list_ics_feeds(state: State<'_, AppState>) -> Vec<IcsFeed> {
    state.accounts().ics_feeds
}

#[tauri::command]
pub async fn add_ics_feed(state: State<'_, AppState>, name: String, url: String) -> Result<IcsFeed, String> {
    let url = feed::normalize_url(&url)?;
    let (cal_name, cal_color) = feed::probe(&state.http, &url).await?;
    let mut a = state.accounts();
    if a.ics_feeds.iter().any(|f| f.url == url) {
        return Err("이미 추가된 피드입니다.".into());
    }
    let id = format!("{:x}", chrono::Utc::now().timestamp_millis());
    let feed = IcsFeed {
        id,
        name: if name.trim().is_empty() { cal_name.unwrap_or_else(|| "iCal 구독".into()) } else { name.trim().to_string() },
        url,
        color: cal_color.unwrap_or_default(),
    };
    a.ics_feeds.push(feed.clone());
    state.set_accounts(a);
    Ok(feed)
}

#[tauri::command]
pub fn remove_ics_feed(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let mut a = state.accounts();
    a.ics_feeds.retain(|f| f.id != id);
    state.set_accounts(a);
    let cal_id = feed::calendar_id(&id);
    let mut c = state.cache.lock().unwrap().clone();
    c.calendars.retain(|cal| cal.id != cal_id);
    c.events.retain(|e| e.calendar_id != cal_id);
    state.set_cache(c);
    Ok(())
}

#[tauri::command]
pub fn open_external(app: AppHandle, url: String) -> Result<(), String> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("http(s) URL만 열 수 있습니다.".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}
