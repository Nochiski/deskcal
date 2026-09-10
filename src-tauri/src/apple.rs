//! Apple iCloud Calendar via CalDAV (Apple ID + app-specific password).
//!
//! "Sign in with Apple" only provides identity; calendar data is only reachable through CalDAV,
//! which requires an app-specific password generated at https://appleid.apple.com.
use crate::ics::{self, When};
use crate::model::{CalEvent, CalendarInfo, Provider};
use chrono::{Datelike, Duration, NaiveDate, Weekday};
use std::collections::HashSet;

const DISCOVERY_URL: &str = "https://caldav.icloud.com/";

pub struct AppleCreds {
    pub apple_id: String,
    pub password: String,
}

fn method(name: &str) -> reqwest::Method {
    reqwest::Method::from_bytes(name.as_bytes()).unwrap()
}

async fn dav(
    http: &reqwest::Client,
    creds: &AppleCreds,
    m: &str,
    url: &str,
    depth: &str,
    body: &str,
) -> Result<(reqwest::StatusCode, String), String> {
    let resp = http
        .request(method(m), url)
        .basic_auth(&creds.apple_id, Some(&creds.password))
        .header("Depth", depth)
        .header("Content-Type", "application/xml; charset=utf-8")
        .header("User-Agent", "DeskCal/0.1")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| format!("iCloud 연결 실패: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    Ok((status, text))
}

fn resolve(base: &str, href: &str) -> String {
    match url::Url::parse(base).and_then(|b| b.join(href)) {
        Ok(u) => u.to_string(),
        Err(_) => href.to_string(),
    }
}

/// Finds the first element with the given local name below `node` and returns its text.
fn find_text<'a>(node: roxmltree::Node<'a, 'a>, local: &str) -> Option<String> {
    node.descendants()
        .find(|n| n.is_element() && n.tag_name().name() == local)
        .and_then(|n| n.text().map(|t| t.trim().to_string()))
}

fn find_href_in<'a>(node: roxmltree::Node<'a, 'a>, local: &str) -> Option<String> {
    let el = node
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == local)?;
    find_text(el, "href")
}

fn check_auth(status: reqwest::StatusCode) -> Result<(), String> {
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(
            "iCloud 인증 실패: Apple ID 또는 앱 암호를 확인해 주세요. (appleid.apple.com > 로그인 및 보안 > 앱 암호)".into(),
        );
    }
    Ok(())
}

/// Discovers the calendar-home URL for the account. Also validates credentials.
pub async fn discover_home(http: &reqwest::Client, creds: &AppleCreds) -> Result<String, String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:current-user-principal/></d:prop></d:propfind>"#;
    let (status, text) = dav(http, creds, "PROPFIND", DISCOVERY_URL, "0", body).await?;
    check_auth(status)?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("iCloud 응답 파싱 실패: {e}"))?;
    let principal = find_href_in(doc.root(), "current-user-principal")
        .ok_or_else(|| format!("principal을 찾을 수 없습니다 ({status})"))?;
    let principal_url = resolve(DISCOVERY_URL, &principal);

    let body = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><c:calendar-home-set/></d:prop></d:propfind>"#;
    let (status, text) = dav(http, creds, "PROPFIND", &principal_url, "0", body).await?;
    check_auth(status)?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("iCloud 응답 파싱 실패: {e}"))?;
    let home = find_href_in(doc.root(), "calendar-home-set")
        .ok_or_else(|| format!("calendar-home-set을 찾을 수 없습니다 ({status})"))?;
    Ok(resolve(&principal_url, &home))
}

pub fn calendar_id(remote: &str) -> String {
    format!("apple:{remote}")
}

pub async fn list_calendars(
    http: &reqwest::Client,
    creds: &AppleCreds,
    home_url: &str,
) -> Result<Vec<CalendarInfo>, String> {
    let body = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:a="http://apple.com/ns/ical/"><d:prop><d:displayname/><d:resourcetype/><a:calendar-color/><c:supported-calendar-component-set/></d:prop></d:propfind>"#;
    let (status, text) = dav(http, creds, "PROPFIND", home_url, "1", body).await?;
    check_auth(status)?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("iCloud 응답 파싱 실패: {e}"))?;
    let mut out = Vec::new();
    for resp in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "response")
    {
        let Some(href) = find_text(resp, "href") else { continue };
        let url = resolve(home_url, &href);
        if url.trim_end_matches('/') == home_url.trim_end_matches('/') {
            continue;
        }
        let Some(rt) = resp
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "resourcetype")
        else {
            continue;
        };
        let types: Vec<String> = rt
            .children()
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name().to_string())
            .collect();
        if !types.iter().any(|t| t == "calendar") {
            continue;
        }
        if let Some(comp) = resp
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "supported-calendar-component-set")
        {
            let comps: Vec<String> = comp
                .children()
                .filter(|n| n.is_element())
                .filter_map(|n| n.attribute("name").map(|s| s.to_string()))
                .collect();
            if !comps.is_empty() && !comps.iter().any(|c| c == "VEVENT") {
                continue;
            }
        }
        let name = find_text(resp, "displayname").unwrap_or_else(|| "캘린더".into());
        let color = find_text(resp, "calendar-color")
            .map(|c| if c.len() >= 7 { c[..7].to_string() } else { c })
            .unwrap_or_else(|| "#ff9500".into());
        let owned = !types.iter().any(|t| t == "shared" || t == "subscribed");
        let is_holiday = name.contains("휴일") || name.to_lowercase().contains("holiday");
        out.push(CalendarInfo {
            id: calendar_id(&url),
            provider: Provider::Apple,
            remote_id: url,
            name,
            color,
            owned,
            is_holiday,
            default_reminders: vec![],
        });
    }
    Ok(out)
}

fn fmt_utc(d: NaiveDate) -> String {
    format!("{}T000000Z", d.format("%Y%m%d"))
}

pub async fn list_events(
    http: &reqwest::Client,
    creds: &AppleCreds,
    cal: &CalendarInfo,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Result<Vec<CalEvent>, String> {
    let start = fmt_utc(range_start - Duration::days(1));
    let end = fmt_utc(range_end + Duration::days(1));
    let query = |expand: bool| {
        let data = if expand {
            format!(r#"<c:calendar-data><c:expand start="{start}" end="{end}"/></c:calendar-data>"#)
        } else {
            "<c:calendar-data/>".to_string()
        };
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?><c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav"><d:prop><d:getetag/>{data}</d:prop><c:filter><c:comp-filter name="VCALENDAR"><c:comp-filter name="VEVENT"><c:time-range start="{start}" end="{end}"/></c:comp-filter></c:comp-filter></c:filter></c:calendar-query>"#
        )
    };
    let (mut status, mut text) = dav(http, creds, "REPORT", &cal.remote_id, "1", &query(true)).await?;
    check_auth(status)?;
    let mut expanded = true;
    if !status.is_success() {
        // Some servers reject <expand>; fall back to raw data and expand locally.
        let (s2, t2) = dav(http, creds, "REPORT", &cal.remote_id, "1", &query(false)).await?;
        status = s2;
        text = t2;
        expanded = false;
    }
    if !status.is_success() {
        return Err(format!("'{}' 일정 조회 실패 ({status})", cal.name));
    }
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("iCloud 응답 파싱 실패: {e}"))?;
    let mut out = Vec::new();
    for data in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "calendar-data")
    {
        let Some(ics_text) = data.text() else { continue };
        let vevents = ics::parse_events(ics_text);
        // Overridden instances (RECURRENCE-ID) replace the generated one when expanding locally.
        let overridden: HashSet<String> = vevents
            .iter()
            .filter_map(|v| v.get("RECURRENCE-ID").and_then(ics::parse_when).map(|w| w.to_iso()))
            .collect();
        for ev in &vevents {
            let Some(start_prop) = ev.get("DTSTART") else { continue };
            let Some(start) = ics::parse_when(start_prop) else { continue };
            let end = match ev.get("DTEND").and_then(ics::parse_when) {
                Some(e) => e,
                None => match ev.value("DURATION").and_then(|d| ics::parse_duration_secs(&d)) {
                    Some(secs) => start.add(secs),
                    None => {
                        if start.is_date() {
                            start.add(86400)
                        } else {
                            start.clone()
                        }
                    }
                },
            };
            let uid = ev.value("UID").unwrap_or_default();
            let title = ev.value("SUMMARY").unwrap_or_else(|| "(제목 없음)".into());
            let base = CalEvent {
                id: String::new(),
                calendar_id: cal.id.clone(),
                title,
                start: start.to_iso(),
                end: end.to_iso(),
                all_day: start.is_date(),
                location: ev.value("LOCATION").filter(|s| !s.is_empty()),
                description: ev.value("DESCRIPTION").filter(|s| !s.is_empty()),
                reminders: ev.alarms.clone(),
                html_link: None,
            };
            let rrule = ev.value("RRULE");
            if !expanded && rrule.is_some() && ev.get("RECURRENCE-ID").is_none() {
                let dur_secs = when_diff_secs(&start, &end);
                for occ in expand_rrule(&rrule.unwrap(), &start, ev, range_start, range_end) {
                    let iso = occ.to_iso();
                    if overridden.contains(&iso) {
                        continue;
                    }
                    let mut e = base.clone();
                    e.start = iso.clone();
                    e.end = occ.add(dur_secs).to_iso();
                    e.id = format!("{}:{}:{}", cal.id, uid, iso);
                    out.push(e);
                }
            } else {
                let mut e = base;
                e.id = format!("{}:{}:{}", cal.id, uid, e.start);
                out.push(e);
            }
        }
    }
    Ok(out)
}

fn when_diff_secs(a: &When, b: &When) -> i64 {
    match (a, b) {
        (When::Date(x), When::Date(y)) => (*y - *x).num_seconds(),
        (When::DateTime(x), When::DateTime(y)) => (*y - *x).num_seconds(),
        _ => 0,
    }
}

fn when_date(w: &When) -> NaiveDate {
    match w {
        When::Date(d) => *d,
        When::DateTime(dt) => dt.date_naive(),
    }
}

fn weekday_from(s: &str) -> Option<Weekday> {
    let s = s.trim_start_matches(|c: char| c.is_ascii_digit() || c == '-' || c == '+');
    Some(match s {
        "MO" => Weekday::Mon,
        "TU" => Weekday::Tue,
        "WE" => Weekday::Wed,
        "TH" => Weekday::Thu,
        "FR" => Weekday::Fri,
        "SA" => Weekday::Sat,
        "SU" => Weekday::Sun,
        _ => return None,
    })
}

/// Local fallback expansion for the common RRULE subset (DAILY/WEEKLY/MONTHLY/YEARLY,
/// INTERVAL, COUNT, UNTIL, BYDAY for WEEKLY, EXDATE).
fn expand_rrule(
    rrule: &str,
    start: &When,
    ev: &ics::VEvent,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<When> {
    let mut freq = "";
    let mut interval: i64 = 1;
    let mut count: Option<usize> = None;
    let mut until: Option<NaiveDate> = None;
    let mut bydays: Vec<Weekday> = Vec::new();
    for part in rrule.split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        match k.to_uppercase().as_str() {
            "FREQ" => freq = v,
            "INTERVAL" => interval = v.parse().unwrap_or(1).max(1),
            "COUNT" => count = v.parse().ok(),
            "UNTIL" => until = NaiveDate::parse_from_str(&v[..8.min(v.len())], "%Y%m%d").ok(),
            "BYDAY" => bydays = v.split(',').filter_map(weekday_from).collect(),
            _ => {}
        }
    }
    let exdates: HashSet<NaiveDate> = ev
        .props
        .iter()
        .filter(|p| p.name == "EXDATE")
        .flat_map(|p| {
            p.value
                .split(',')
                .filter_map(|v| {
                    let pp = ics::Prop { name: "EXDATE".into(), params: p.params.clone(), value: v.into() };
                    ics::parse_when(&pp).map(|w| when_date(&w))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let start_date = when_date(start);
    let mut out = Vec::new();
    let mut produced = 0usize;
    let hard_stop = range_end.min(until.unwrap_or(range_end));
    let freq = freq.to_uppercase();
    let mut cursor = start_date;
    let mut guard = 0;
    while cursor <= hard_stop && guard < 5000 {
        guard += 1;
        let candidates: Vec<NaiveDate> = match freq.as_str() {
            "WEEKLY" if !bydays.is_empty() => {
                // Week starting at cursor (same weekday as start).
                (0..7)
                    .map(|i| cursor + Duration::days(i))
                    .filter(|d| bydays.contains(&d.weekday()))
                    .collect()
            }
            _ => vec![cursor],
        };
        for d in candidates {
            if d < start_date {
                continue;
            }
            if let Some(c) = count {
                if produced >= c {
                    return out;
                }
            }
            if let Some(u) = until {
                if d > u {
                    return out;
                }
            }
            produced += 1;
            if d >= range_start && d < range_end && !exdates.contains(&d) {
                let secs = (d - start_date).num_days() * 86400;
                out.push(start.add(secs));
            }
        }
        cursor = match freq.as_str() {
            "DAILY" => cursor + Duration::days(interval),
            "WEEKLY" => cursor + Duration::weeks(interval),
            "MONTHLY" => add_months(cursor, interval as u32),
            "YEARLY" => add_months(cursor, 12 * interval as u32),
            _ => break,
        };
    }
    out
}

fn add_months(d: NaiveDate, months: u32) -> NaiveDate {
    let total = d.month0() + months;
    let year = d.year() + (total / 12) as i32;
    let month = total % 12 + 1;
    let mut day = d.day();
    loop {
        if let Some(nd) = NaiveDate::from_ymd_opt(year, month, day) {
            return nd;
        }
        day -= 1;
    }
}
