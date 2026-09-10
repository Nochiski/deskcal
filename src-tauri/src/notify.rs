//! Reminder scheduler: fires Windows toast notifications when an event's reminder time
//! (relative to the local system clock) is reached. Honors per-calendar `notify` prefs.
use crate::model::{CalEvent, Settings, SyncResult};
use crate::state::AppState;
use chrono::{DateTime, Duration, Local, NaiveDate, NaiveTime, TimeZone};
use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

const TICK_SECS: u64 = 20;
/// A trigger is considered "due" if it falls within this many seconds before now.
const WINDOW_SECS: i64 = 90;

pub fn start(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(TICK_SECS)).await;
            check(&app);
        }
    });
}

fn parse_start(ev: &CalEvent) -> Option<DateTime<Local>> {
    if ev.all_day {
        return None;
    }
    DateTime::parse_from_rfc3339(&ev.start)
        .ok()
        .map(|d| d.with_timezone(&Local))
}

fn all_day_trigger(ev: &CalEvent, settings: &Settings) -> Option<DateTime<Local>> {
    let date = NaiveDate::parse_from_str(&ev.start, "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(&settings.all_day_reminder_time, "%H:%M").ok()?;
    Local.from_local_datetime(&date.and_time(time)).earliest()
}

/// All reminder instants for an event, paired with the lead time (minutes) used.
fn triggers(ev: &CalEvent, settings: &Settings) -> Vec<(DateTime<Local>, i64)> {
    if ev.all_day {
        return all_day_trigger(ev, settings).map(|t| vec![(t, 0)]).unwrap_or_default();
    }
    let Some(start) = parse_start(ev) else { return vec![] };
    let minutes: Vec<i64> = if !ev.reminders.is_empty() {
        ev.reminders.clone()
    } else if settings.default_reminder_min >= 0 {
        vec![settings.default_reminder_min]
    } else {
        vec![]
    };
    minutes
        .into_iter()
        .map(|m| (start - Duration::minutes(m), m))
        .collect()
}

fn format_time(dt: &DateTime<Local>) -> String {
    let (h, m) = (dt.format("%H").to_string().parse::<u32>().unwrap_or(0), dt.format("%M").to_string());
    let ampm = if h < 12 { "오전" } else { "오후" };
    let h12 = match h % 12 {
        0 => 12,
        x => x,
    };
    format!("{ampm} {h12}:{m}")
}

fn body_for(ev: &CalEvent, cal_name: &str, lead_min: i64) -> String {
    let mut parts = Vec::new();
    if ev.all_day {
        parts.push("종일".to_string());
    } else if let Some(start) = parse_start(ev) {
        let when = if lead_min <= 0 {
            "지금 시작".to_string()
        } else if lead_min % 60 == 0 {
            format!("{}시간 후 시작", lead_min / 60)
        } else {
            format!("{lead_min}분 후 시작")
        };
        parts.push(format!("{} · {}", format_time(&start), when));
    }
    if let Some(loc) = &ev.location {
        parts.push(loc.clone());
    }
    parts.push(cal_name.to_string());
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(start: &str, all_day: bool, reminders: Vec<i64>) -> CalEvent {
        CalEvent {
            id: "x".into(),
            calendar_id: "google:a".into(),
            remote_id: "x".into(),
            editable: true,
            title: "t".into(),
            start: start.into(),
            end: start.into(),
            all_day,
            location: None,
            description: None,
            reminders,
            html_link: None,
        }
    }

    #[test]
    fn timed_event_uses_own_reminders() {
        let s = Settings::default();
        let e = ev("2026-09-10T15:00:00+09:00", false, vec![30, 5]);
        let t = triggers(&e, &s);
        assert_eq!(t.len(), 2);
        let start = DateTime::parse_from_rfc3339("2026-09-10T15:00:00+09:00").unwrap();
        assert_eq!(t[0].0, start - Duration::minutes(30));
        assert_eq!(t[1].1, 5);
    }

    #[test]
    fn timed_event_falls_back_to_default_and_can_be_disabled() {
        let mut s = Settings::default();
        s.default_reminder_min = 15;
        let e = ev("2026-09-10T15:00:00+09:00", false, vec![]);
        assert_eq!(triggers(&e, &s).len(), 1);
        s.default_reminder_min = -1;
        assert!(triggers(&e, &s).is_empty());
    }

    #[test]
    fn all_day_fires_at_configured_local_time() {
        let mut s = Settings::default();
        s.all_day_reminder_time = "08:30".into();
        let e = ev("2026-09-10", true, vec![]);
        let t = triggers(&e, &s);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].0.format("%Y-%m-%d %H:%M").to_string(), "2026-09-10 08:30");
    }

    #[test]
    fn body_mentions_lead_time() {
        let e = ev("2026-09-10T15:00:00+09:00", false, vec![10]);
        assert!(body_for(&e, "내 캘린더", 10).contains("10분 후 시작"));
        assert!(body_for(&e, "내 캘린더", 60).contains("1시간 후 시작"));
    }
}

pub fn check(app: &AppHandle) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    if !settings.notifications_enabled {
        return;
    }
    let cache: SyncResult = state.cache.lock().unwrap().clone();
    let now = Local::now();
    let mut to_fire: Vec<(String, String, String)> = Vec::new();

    for ev in &cache.events {
        let notify = settings
            .calendars
            .get(&ev.calendar_id)
            .map(|p| p.notify)
            .unwrap_or(true);
        if !notify {
            continue;
        }
        for (t, lead) in triggers(ev, &settings) {
            let delta = (now - t).num_seconds();
            if delta < 0 || delta > WINDOW_SECS {
                continue;
            }
            let key = format!("{}|{}", ev.id, lead);
            let mut fired = state.fired.lock().unwrap();
            if fired.contains(&key) {
                continue;
            }
            if fired.len() > 5000 {
                fired.clear();
            }
            fired.insert(key);
            let cal_name = cache
                .calendars
                .iter()
                .find(|c| c.id == ev.calendar_id)
                .map(|c| c.name.clone())
                .unwrap_or_default();
            to_fire.push((ev.title.clone(), body_for(ev, &cal_name, lead), ev.id.clone()));
        }
    }

    for (title, body, id) in to_fire {
        log::info!("notify: {title} ({id})");
        if let Err(e) = app.notification().builder().title(&title).body(&body).show() {
            log::warn!("notification failed: {e}");
        }
    }
}
