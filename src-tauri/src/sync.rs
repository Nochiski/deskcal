//! Fetches calendars + events from every connected provider and updates the cache.
use crate::apple::{self, AppleCreds};
use crate::google;
use crate::model::{CalEvent, CalendarInfo, Provider, SyncError, SyncResult};
use crate::state::AppState;
use crate::store;
use chrono::{Local, NaiveDate, Utc};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

pub async fn sync_range(app: &AppHandle, range_start: NaiveDate, range_end: NaiveDate) -> SyncResult {
    let state = app.state::<AppState>();
    if state.syncing.swap(true, Ordering::SeqCst) {
        // Another sync is running; return the cache.
        return state.cache.lock().unwrap().clone();
    }
    let result = do_sync(app, range_start, range_end).await;
    state.syncing.store(false, Ordering::SeqCst);
    result
}

async fn do_sync(app: &AppHandle, range_start: NaiveDate, range_end: NaiveDate) -> SyncResult {
    let state = app.state::<AppState>();
    *state.last_range.lock().unwrap() = (range_start, range_end);
    let settings = state.settings();
    let accounts = state.accounts();
    let http = state.http.clone();

    let mut calendars: Vec<CalendarInfo> = Vec::new();
    let mut events: Vec<CalEvent> = Vec::new();
    let mut errors: Vec<SyncError> = Vec::new();

    let wanted = |cal: &CalendarInfo| -> bool {
        match settings.calendars.get(&cal.id) {
            Some(p) => p.visible || p.notify,
            None => true,
        }
    };

    // ── Google ──
    if let Some(refresh) = store::get_secret(store::SECRET_GOOGLE_REFRESH) {
        match google_token(&state, &http, &settings.google.client_id, &settings.google.client_secret, &refresh).await
        {
            Ok(token) => match google::list_calendars(&http, &token).await {
                Ok(cals) => {
                    let mut set = tokio::task::JoinSet::new();
                    for cal in cals.iter().filter(|c| wanted(c)).cloned() {
                        let http = http.clone();
                        let token = token.clone();
                        set.spawn(async move {
                            google::list_events(&http, &token, &cal, range_start, range_end).await
                        });
                    }
                    while let Some(r) = set.join_next().await {
                        match r {
                            Ok(Ok(evs)) => events.extend(evs),
                            Ok(Err(e)) => errors.push(SyncError { provider: Provider::Google, message: e }),
                            Err(e) => errors.push(SyncError { provider: Provider::Google, message: e.to_string() }),
                        }
                    }
                    calendars.extend(cals);
                }
                Err(e) => errors.push(SyncError { provider: Provider::Google, message: e }),
            },
            Err(e) => errors.push(SyncError { provider: Provider::Google, message: e }),
        }
    }

    // ── Apple (iCloud CalDAV) ──
    if let (Some(apple_id), Some(password)) = (
        accounts.apple_id.clone(),
        store::get_secret(store::SECRET_APPLE_PASSWORD),
    ) {
        let creds = std::sync::Arc::new(AppleCreds { apple_id, password });
        let home = match accounts.apple_home_url.clone() {
            Some(h) => Ok(h),
            None => apple::discover_home(&http, &creds).await,
        };
        match home {
            Ok(home) => match apple::list_calendars(&http, &creds, &home).await {
                Ok(cals) => {
                    let mut set = tokio::task::JoinSet::new();
                    for cal in cals.iter().filter(|c| wanted(c)).cloned() {
                        let http = http.clone();
                        let creds = creds.clone();
                        set.spawn(async move {
                            apple::list_events(&http, &creds, &cal, range_start, range_end).await
                        });
                    }
                    while let Some(r) = set.join_next().await {
                        match r {
                            Ok(Ok(evs)) => events.extend(evs),
                            Ok(Err(e)) => errors.push(SyncError { provider: Provider::Apple, message: e }),
                            Err(e) => errors.push(SyncError { provider: Provider::Apple, message: e.to_string() }),
                        }
                    }
                    calendars.extend(cals);
                }
                Err(e) => errors.push(SyncError { provider: Provider::Apple, message: e }),
            },
            Err(e) => errors.push(SyncError { provider: Provider::Apple, message: e }),
        }
    }

    events.sort_by(|a, b| a.start.cmp(&b.start).then(a.title.cmp(&b.title)));
    for e in &errors {
        log::warn!("sync error [{}]: {}", e.provider.as_str(), e.message);
    }
    log::info!(
        "sync {}..{}: {} calendars, {} events, {} errors",
        range_start,
        range_end,
        calendars.len(),
        events.len(),
        errors.len()
    );

    let result = SyncResult {
        calendars,
        events,
        synced_at: Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        errors,
        range_start: range_start.to_string(),
        range_end: range_end.to_string(),
    };
    // Keep the previous good cache if every provider failed and nothing came back.
    let all_failed = result.calendars.is_empty() && !result.errors.is_empty();
    if !all_failed {
        state.set_cache(result.clone());
    }
    let _ = app.emit("events-updated", &result);
    result
}

async fn google_token(
    state: &AppState,
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh: &str,
) -> Result<String, String> {
    if let Some(t) = state.google_token.lock().unwrap().clone() {
        if t.expires_at > Utc::now() {
            return Ok(t.token);
        }
    }
    let t = google::refresh_access_token(http, client_id, client_secret, refresh).await?;
    let token = t.token.clone();
    *state.google_token.lock().unwrap() = Some(t);
    Ok(token)
}

/// Background loop: re-sync the last requested range every `sync_interval_min` minutes.
pub fn start_background(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            let mins = {
                let state = app.state::<AppState>();
                state.settings().sync_interval_min.clamp(1, 24 * 60)
            };
            tokio::time::sleep(std::time::Duration::from_secs(mins * 60)).await;
            let (s, e) = {
                let state = app.state::<AppState>();
                let r = *state.last_range.lock().unwrap();
                r
            };
            // Keep the window roughly centered on today if the UI hasn't asked recently.
            let today = Local::now().date_naive();
            let (s, e) = if today < s || today >= e {
                (today - chrono::Duration::days(45), today + chrono::Duration::days(60))
            } else {
                (s, e)
            };
            let _ = sync_range(&app, s, e).await;
        }
    });
}
