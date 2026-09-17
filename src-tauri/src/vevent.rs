//! VEVENT ↔ app model: conversion (with local RRULE expansion) and serialization for CalDAV writes.
//! Shared by the iCloud CalDAV provider and plain iCalendar feeds.
use crate::ics::{parse_duration_secs, parse_when, unfold, Prop, VEvent, When};
use crate::model::{CalEvent, EventInput};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, Utc, Weekday};
use std::collections::HashSet;

/// Converts parsed VEVENTs into app events for `[range_start, range_end)`.
/// `remote_id`: explicit provider id (CalDAV href); `None` → the VEVENT UID is used.
/// `expand_locally`: expand RRULEs here (feeds, or CalDAV servers that ignore `<expand>`).
pub fn vevents_to_events(
    vevents: &[VEvent],
    cal_id: &str,
    remote_id: Option<&str>,
    editable_base: bool,
    range_start: NaiveDate,
    range_end: NaiveDate,
    expand_locally: bool,
) -> Vec<CalEvent> {
    let mut out = Vec::new();
    let overridden: HashSet<String> = vevents
        .iter()
        .filter_map(|v| v.get("RECURRENCE-ID").and_then(parse_when).map(|w| w.to_iso()))
        .collect();
    for ev in vevents {
        let Some(start_prop) = ev.get("DTSTART") else { continue };
        let Some(start) = parse_when(start_prop) else { continue };
        let end = match ev.get("DTEND").and_then(parse_when) {
            Some(e) => e,
            None => match ev.value("DURATION").and_then(|d| parse_duration_secs(&d)) {
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
        let rid = remote_id.map(|s| s.to_string()).unwrap_or_else(|| uid.clone());
        let is_recurring = ev.get("RRULE").is_some() || ev.get("RECURRENCE-ID").is_some();
        let base = CalEvent {
            id: String::new(),
            calendar_id: cal_id.to_string(),
            remote_id: rid,
            editable: editable_base && !is_recurring,
            title: ev.value("SUMMARY").unwrap_or_else(|| "(제목 없음)".into()),
            start: start.to_iso(),
            end: end.to_iso(),
            all_day: start.is_date(),
            location: ev.value("LOCATION").filter(|s| !s.is_empty()),
            description: ev.value("DESCRIPTION").filter(|s| !s.is_empty()),
            reminders: ev.alarms.clone(),
            html_link: None,
            attendees: Vec::new(),
            organizer: None,
            attendees_omitted: false,
            response_status: None,
            can_respond: false,
            recurring: is_recurring,
        };
        let rrule = ev.value("RRULE");
        if expand_locally && rrule.is_some() && ev.get("RECURRENCE-ID").is_none() {
            let dur_secs = when_diff_secs(&start, &end);
            for occ in expand_rrule(&rrule.unwrap(), &start, ev, range_start, range_end) {
                let iso = occ.to_iso();
                if overridden.contains(&iso) {
                    continue;
                }
                let mut e = base.clone();
                e.start = iso.clone();
                e.end = occ.add(dur_secs).to_iso();
                e.id = format!("{cal_id}:{uid}:{iso}");
                out.push(e);
            }
        } else {
            if !overlaps(&start, &end, range_start, range_end) {
                continue;
            }
            let mut e = base;
            e.id = format!("{cal_id}:{uid}:{}", e.start);
            out.push(e);
        }
    }
    out
}

fn overlaps(start: &When, end: &When, range_start: NaiveDate, range_end: NaiveDate) -> bool {
    let s = when_date(start);
    let e = when_date(end).max(s);
    s < range_end && e >= range_start
}

pub fn when_diff_secs(a: &When, b: &When) -> i64 {
    match (a, b) {
        (When::Date(x), When::Date(y)) => (*y - *x).num_seconds(),
        (When::DateTime(x), When::DateTime(y)) => (*y - *x).num_seconds(),
        _ => 0,
    }
}

pub fn when_date(w: &When) -> NaiveDate {
    match w {
        When::Date(d) => *d,
        When::DateTime(dt) => dt.with_timezone(&Local).date_naive(),
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

/// Local expansion for the common RRULE subset (DAILY/WEEKLY/MONTHLY/YEARLY, INTERVAL, COUNT,
/// UNTIL, BYDAY for WEEKLY, EXDATE).
pub fn expand_rrule(
    rrule: &str,
    start: &When,
    ev: &VEvent,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Vec<When> {
    let mut freq = String::new();
    let mut interval: i64 = 1;
    let mut count: Option<usize> = None;
    let mut until: Option<NaiveDate> = None;
    let mut bydays: Vec<Weekday> = Vec::new();
    for part in rrule.split(';') {
        let Some((k, v)) = part.split_once('=') else { continue };
        match k.to_uppercase().as_str() {
            "FREQ" => freq = v.to_uppercase(),
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
                    let pp = Prop { name: "EXDATE".into(), params: p.params.clone(), value: v.into() };
                    parse_when(&pp).map(|w| when_date(&w))
                })
                .collect::<Vec<_>>()
        })
        .collect();

    let start_date = when_date(start);
    let mut out = Vec::new();
    let mut produced = 0usize;
    let hard_stop = range_end.min(until.unwrap_or(range_end));
    let mut cursor = start_date;
    let mut guard = 0;
    while cursor <= hard_stop && guard < 5000 {
        guard += 1;
        let candidates: Vec<NaiveDate> = match freq.as_str() {
            "WEEKLY" if !bydays.is_empty() => (0..7)
                .map(|i| cursor + Duration::days(i))
                .filter(|d| bydays.contains(&d.weekday()))
                .collect(),
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

// ─────────────────────────────────────────────────────────────
// Serialization (for CalDAV writes)
// ─────────────────────────────────────────────────────────────

/// Parses an EventInput start/end string into a `When`.
pub fn parse_input_when(s: &str, all_day: bool) -> Result<When, String> {
    if all_day {
        NaiveDate::parse_from_str(s, "%Y-%m-%d")
            .map(When::Date)
            .map_err(|_| format!("잘못된 날짜: {s}"))
    } else {
        DateTime::parse_from_rfc3339(s)
            .map(When::DateTime)
            .map_err(|_| format!("잘못된 일시: {s}"))
    }
}

fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            ';' => out.push_str("\\;"),
            ',' => out.push_str("\\,"),
            '\n' => out.push_str("\\n"),
            '\r' => {}
            _ => out.push(c),
        }
    }
    out
}

/// Folds a content line at 75 octets (RFC 5545 §3.1) on UTF-8 boundaries.
fn fold(line: &str) -> String {
    let mut out = String::new();
    let bytes = line.as_bytes();
    let mut cur = 0usize;
    let mut first = true;
    while cur < bytes.len() {
        let limit = if first { 75 } else { 74 };
        let mut end = (cur + limit).min(bytes.len());
        while end > cur && end < bytes.len() && (bytes[end] & 0xC0) == 0x80 {
            end -= 1;
        }
        if !first {
            out.push_str("\r\n ");
        }
        out.push_str(&line[cur..end]);
        cur = end;
        first = false;
    }
    out
}

fn fmt_when(name: &str, w: &When) -> String {
    match w {
        When::Date(d) => format!("{name};VALUE=DATE:{}", d.format("%Y%m%d")),
        When::DateTime(dt) => format!("{name}:{}", dt.with_timezone(&Utc).format("%Y%m%dT%H%M%SZ")),
    }
}

/// Property lines (unfolded, without CRLF) that describe `input`.
fn input_lines(input: &EventInput) -> Result<Vec<String>, String> {
    let start = parse_input_when(&input.start, input.all_day)?;
    let end = parse_input_when(&input.end, input.all_day)?;
    let mut lines = vec![
        format!("DTSTAMP:{}", Utc::now().format("%Y%m%dT%H%M%SZ")),
        fmt_when("DTSTART", &start),
        fmt_when("DTEND", &end),
        format!("SUMMARY:{}", escape_text(input.title.trim())),
    ];
    if let Some(loc) = input.location.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        lines.push(format!("LOCATION:{}", escape_text(loc)));
    }
    if let Some(desc) = input.description.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        lines.push(format!("DESCRIPTION:{}", escape_text(desc)));
    }
    for m in &input.reminders {
        let trigger = if *m <= 0 {
            "PT0S".to_string()
        } else if m % 1440 == 0 {
            format!("-P{}D", m / 1440)
        } else if m % 60 == 0 {
            format!("-PT{}H", m / 60)
        } else {
            format!("-PT{m}M")
        };
        lines.push("BEGIN:VALARM".into());
        lines.push("ACTION:DISPLAY".into());
        lines.push("DESCRIPTION:Reminder".into());
        lines.push(format!("TRIGGER:{trigger}"));
        lines.push("END:VALARM".into());
    }
    Ok(lines)
}

const REPLACED: &[&str] = &[
    "DTSTAMP",
    "DTSTART",
    "DTEND",
    "DURATION",
    "SUMMARY",
    "LOCATION",
    "DESCRIPTION",
    "LAST-MODIFIED",
];

/// Builds a full VCALENDAR document. When `existing` is given, the first VEVENT's editable
/// properties are replaced and everything else (UID, custom X- props, …) is preserved.
pub fn build_ics(uid: &str, input: &EventInput, existing: Option<&str>) -> Result<String, String> {
    let new_lines = input_lines(input)?;
    let mut out: Vec<String> = Vec::new();
    match existing {
        Some(text) => {
            let mut in_event = false;
            let mut done = false;
            let mut in_alarm = false;
            for line in unfold(text) {
                let upper = line.to_uppercase();
                if !done && upper == "BEGIN:VEVENT" {
                    in_event = true;
                    out.push(line);
                    continue;
                }
                if in_event {
                    if upper == "BEGIN:VALARM" {
                        in_alarm = true;
                        continue;
                    }
                    if upper == "END:VALARM" {
                        in_alarm = false;
                        continue;
                    }
                    if in_alarm {
                        continue;
                    }
                    if upper == "END:VEVENT" {
                        out.extend(new_lines.iter().cloned());
                        out.push(line);
                        in_event = false;
                        done = true;
                        continue;
                    }
                    let name = line.split([';', ':']).next().unwrap_or("").to_uppercase();
                    if REPLACED.contains(&name.as_str()) {
                        continue;
                    }
                }
                out.push(line);
            }
            if !done {
                return Err("기존 일정 데이터를 해석할 수 없습니다.".into());
            }
        }
        None => {
            out.push("BEGIN:VCALENDAR".into());
            out.push("VERSION:2.0".into());
            out.push("PRODID:-//DeskCal//DeskCal 0.1//KO".into());
            out.push("CALSCALE:GREGORIAN".into());
            out.push("BEGIN:VEVENT".into());
            out.push(format!("UID:{uid}"));
            out.push(format!("CREATED:{}", Utc::now().format("%Y%m%dT%H%M%SZ")));
            out.extend(new_lines);
            out.push("END:VEVENT".into());
            out.push("END:VCALENDAR".into());
        }
    }
    let mut s = String::new();
    for l in out {
        if l.is_empty() {
            continue;
        }
        s.push_str(&fold(&l));
        s.push_str("\r\n");
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::parse_events;

    fn input() -> EventInput {
        EventInput {
            calendar_id: "apple:x".into(),
            title: "회의; 준비, 자료".into(),
            start: "2026-09-10T15:00:00+09:00".into(),
            end: "2026-09-10T16:00:00+09:00".into(),
            all_day: false,
            location: Some("본사".into()),
            description: None,
            reminders: vec![10],
        }
    }

    #[test]
    fn builds_new_document_and_round_trips() {
        let s = build_ics("abc", &input(), None).unwrap();
        assert!(s.contains("UID:abc\r\n"));
        assert!(s.contains("DTSTART:20260910T060000Z\r\n"));
        assert!(s.contains("TRIGGER:-PT10M\r\n"));
        let back = parse_events(&s);
        assert_eq!(back[0].value("SUMMARY").unwrap(), "회의; 준비, 자료");
        assert_eq!(back[0].alarms, vec![10]);
    }

    #[test]
    fn patches_existing_document_preserving_uid_and_x_props() {
        let existing = "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nBEGIN:VEVENT\r\nUID:keep-me\r\nX-CUSTOM:1\r\nSUMMARY:old\r\nDTSTART:20260101T000000Z\r\nDTEND:20260101T010000Z\r\nBEGIN:VALARM\r\nTRIGGER:-PT1H\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let s = build_ics("ignored", &input(), Some(existing)).unwrap();
        assert!(s.contains("UID:keep-me"));
        assert!(s.contains("X-CUSTOM:1"));
        assert!(!s.contains("SUMMARY:old"));
        assert!(!s.contains("TRIGGER:-PT1H"));
        assert!(s.contains("TRIGGER:-PT10M"));
        assert_eq!(s.matches("BEGIN:VEVENT").count(), 1);
    }

    #[test]
    fn expands_weekly_rrule_locally() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:r\r\nSUMMARY:주간\r\nDTSTART;TZID=Asia/Seoul:20260901T170000\r\nDTEND;TZID=Asia/Seoul:20260901T180000\r\nRRULE:FREQ=WEEKLY;BYDAY=TU,TH\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let evs = vevents_to_events(
            &parse_events(ics),
            "ics:f",
            None,
            false,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 9, 15).unwrap(),
            true,
        );
        let days: Vec<String> = evs.iter().map(|e| e.start[..10].to_string()).collect();
        assert_eq!(days, vec!["2026-09-01", "2026-09-03", "2026-09-08", "2026-09-10"]);
        assert!(evs.iter().all(|e| !e.editable));
    }

    #[test]
    fn folds_long_lines_on_utf8_boundaries() {
        let long = format!("DESCRIPTION:{}", "가".repeat(60));
        let f = fold(&long);
        for piece in f.split("\r\n") {
            assert!(piece.len() <= 75);
        }
        assert_eq!(unfold(&f)[0], long);
    }
}
