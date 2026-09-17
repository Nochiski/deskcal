//! Fetches calendars + events from every connected provider and updates the cache.
use crate::apple::{self, AppleCreds};
use crate::google;
use crate::model::{CalEvent, CalendarInfo, Provider, SyncError, SyncResult};
use crate::state::AppState;
use crate::store;
use chrono::{Local, NaiveDate, Utc};
use tauri::{AppHandle, Emitter, Manager};

pub async fn sync_range(app: &AppHandle, range_start: NaiveDate, range_end: NaiveDate) -> SyncResult {
    let state = app.state::<AppState>();
    let _guard = state.sync_lock.lock().await;
    do_sync(app, range_start, range_end).await
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

    // ── Google (one or more linked accounts) ──
    for account in &accounts.google_accounts {
        let Some(refresh) = store::get_secret(&store::google_secret_key(&account.id)) else {
            errors.push(SyncError {
                provider: Provider::Google,
                message: format!("{}: 저장된 로그인 정보가 없습니다. 설정에서 다시 로그인해 주세요.", account.email),
            });
            continue;
        };
        match google_token(
            &state,
            &http,
            &settings.google.client_id,
            &settings.google.client_secret,
            &account.id,
            &refresh,
        )
        .await
        {
            Ok(token) => match google::list_calendars(&http, &token, account).await {
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

    // ── iCal subscription feeds (read-only) ──
    if !accounts.ics_feeds.is_empty() {
        let mut set = tokio::task::JoinSet::new();
        for feed in accounts.ics_feeds.iter().cloned() {
            let id = crate::feed::calendar_id(&feed.id);
            let wanted_feed = match settings.calendars.get(&id) {
                Some(p) => p.visible || p.notify,
                None => true,
            };
            if !wanted_feed {
                continue;
            }
            let http = http.clone();
            set.spawn(async move { crate::feed::fetch(&http, &feed, range_start, range_end).await });
        }
        while let Some(r) = set.join_next().await {
            match r {
                Ok(Ok((cal, evs))) => {
                    calendars.push(cal);
                    events.extend(evs);
                }
                Ok(Err(e)) => errors.push(SyncError { provider: Provider::Ics, message: e }),
                Err(e) => errors.push(SyncError { provider: Provider::Ics, message: e.to_string() }),
            }
        }
    }

    // The same remote calendar (e.g. 대한민국의 휴일) can be subscribed from several linked
    // accounts; show each occurrence once by keying on the provider-side ids.
    {
        // Keep the actionable copy when the same calendar is shared across linked accounts.
        events.sort_by_key(|e| (
            settings.calendars.get(&e.calendar_id).is_some_and(|p| !p.visible),
            !e.can_respond,
            !e.editable,
        ));
        let remote_of: std::collections::HashMap<&str, &str> =
            calendars.iter().map(|c| (c.id.as_str(), c.remote_id.as_str())).collect();
        let mut seen: std::collections::HashSet<(String, String, String)> = std::collections::HashSet::new();
        events.retain(|e| {
            let remote_cal = remote_of.get(e.calendar_id.as_str()).copied().unwrap_or("");
            if remote_cal.is_empty() || e.remote_id.is_empty() {
                return true;
            }
            seen.insert((remote_cal.to_string(), e.remote_id.clone(), e.start.clone()))
        });
    }

    // Timed events that run from midnight to a later midnight are really whole-day events
    // (Google shows them as bars too); normalise them so every view treats them as all-day.
    for e in events.iter_mut() {
        if e.all_day {
            continue;
        }
        let (Ok(s), Ok(en)) = (
            chrono::DateTime::parse_from_rfc3339(&e.start),
            chrono::DateTime::parse_from_rfc3339(&e.end),
        ) else {
            continue;
        };
        let s = s.with_timezone(&Local);
        let en = en.with_timezone(&Local);
        let at_midnight = |d: &chrono::DateTime<Local>| d.time() == chrono::NaiveTime::MIN;
        if at_midnight(&s) && at_midnight(&en) && en > s {
            e.all_day = true;
            e.start = s.date_naive().to_string();
            e.end = en.date_naive().to_string();
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

pub async fn google_token(
    state: &AppState,
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    account_id: &str,
    refresh: &str,
) -> Result<String, String> {
    if let Some(t) = state.google_token.lock().unwrap().get(account_id).cloned() {
        if t.expires_at > Utc::now() {
            return Ok(t.token);
        }
    }
    let t = google::refresh_access_token(http, client_id, client_secret, refresh).await?;
    let token = t.token.clone();
    state.google_token.lock().unwrap().insert(account_id.to_string(), t);
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
