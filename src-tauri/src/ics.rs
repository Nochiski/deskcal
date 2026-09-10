//! Minimal iCalendar (RFC 5545) VEVENT parser — enough for CalDAV responses.
use chrono::{DateTime, Duration, FixedOffset, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Prop {
    pub name: String,
    pub params: HashMap<String, String>,
    pub value: String,
}

#[derive(Debug, Default, Clone)]
pub struct VEvent {
    pub props: Vec<Prop>,
    /// Reminder lead times (minutes before start) from VALARM/TRIGGER.
    pub alarms: Vec<i64>,
}

impl VEvent {
    pub fn get(&self, name: &str) -> Option<&Prop> {
        self.props.iter().find(|p| p.name == name)
    }
    pub fn value(&self, name: &str) -> Option<String> {
        self.get(name).map(|p| unescape(&p.value))
    }
}

/// Unfold continuation lines (CRLF followed by SPACE/HTAB).
pub fn unfold(input: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    for raw in input.split('\n') {
        let line = raw.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix([' ', '\t']) {
            if let Some(last) = lines.last_mut() {
                last.push_str(rest);
                continue;
            }
        }
        lines.push(line.to_string());
    }
    lines
}

fn parse_line(line: &str) -> Option<Prop> {
    // NAME;PARAM=VALUE;PARAM2="quoted":value
    let mut in_quotes = false;
    let mut colon_idx = None;
    for (i, ch) in line.char_indices() {
        match ch {
            '"' => in_quotes = !in_quotes,
            ':' if !in_quotes => {
                colon_idx = Some(i);
                break;
            }
            _ => {}
        }
    }
    let colon = colon_idx?;
    let (head, value) = (&line[..colon], &line[colon + 1..]);
    let mut parts = head.split(';');
    let name = parts.next()?.trim().to_uppercase();
    let mut params = HashMap::new();
    for p in parts {
        if let Some((k, v)) = p.split_once('=') {
            params.insert(k.trim().to_uppercase(), v.trim().trim_matches('"').to_string());
        }
    }
    Some(Prop { name, params, value: value.to_string() })
}

pub fn unescape(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    let mut chars = v.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') | Some('N') => out.push('\n'),
                Some(other) => out.push(other),
                None => {}
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse all VEVENTs in an iCalendar text.
pub fn parse_events(text: &str) -> Vec<VEvent> {
    let mut events = Vec::new();
    let mut current: Option<VEvent> = None;
    let mut in_alarm = false;
    let mut depth_other = 0usize;
    for line in unfold(text) {
        let Some(prop) = parse_line(&line) else { continue };
        match (prop.name.as_str(), prop.value.to_uppercase().as_str()) {
            ("BEGIN", "VEVENT") => {
                current = Some(VEvent::default());
                in_alarm = false;
                depth_other = 0;
            }
            ("END", "VEVENT") => {
                if let Some(ev) = current.take() {
                    events.push(ev);
                }
            }
            ("BEGIN", "VALARM") if current.is_some() => in_alarm = true,
            ("END", "VALARM") if current.is_some() => in_alarm = false,
            ("BEGIN", _) if current.is_some() => depth_other += 1,
            ("END", _) if current.is_some() => depth_other = depth_other.saturating_sub(1),
            _ => {
                if let Some(ev) = current.as_mut() {
                    if in_alarm {
                        if prop.name == "TRIGGER" {
                            if let Some(min) = parse_trigger_minutes(&prop) {
                                ev.alarms.push(min);
                            }
                        }
                    } else if depth_other == 0 {
                        ev.props.push(prop);
                    }
                }
            }
        }
    }
    events
}

/// Parses "-PT15M", "-P1D", "-PT1H30M", "PT0S" into minutes-before-start (positive number).
fn parse_trigger_minutes(p: &Prop) -> Option<i64> {
    if p.params.get("VALUE").map(|v| v == "DATE-TIME").unwrap_or(false) {
        return None; // absolute triggers are not supported
    }
    if p.params.get("RELATED").map(|v| v == "END").unwrap_or(false) {
        return None;
    }
    let secs = parse_duration_secs(&p.value)?;
    if secs > 0 {
        return None; // after start
    }
    Some(-secs / 60)
}

/// ISO-8601 duration subset: [+-]P[nW][nD][T[nH][nM][nS]] → seconds (signed).
pub fn parse_duration_secs(s: &str) -> Option<i64> {
    let s = s.trim();
    let (neg, rest) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s.strip_prefix('+').unwrap_or(s)),
    };
    let rest = rest.strip_prefix('P')?;
    let mut total: i64 = 0;
    let mut num = String::new();
    let mut in_time = false;
    for c in rest.chars() {
        match c {
            'T' => in_time = true,
            d if d.is_ascii_digit() => num.push(d),
            unit => {
                let n: i64 = num.parse().ok()?;
                num.clear();
                total += match (unit, in_time) {
                    ('W', false) => n * 7 * 86400,
                    ('D', false) => n * 86400,
                    ('H', true) => n * 3600,
                    ('M', true) => n * 60,
                    ('S', true) => n,
                    _ => return None,
                };
            }
        }
    }
    Some(if neg { -total } else { total })
}

#[derive(Debug, Clone)]
pub enum When {
    Date(NaiveDate),
    DateTime(DateTime<FixedOffset>),
}

/// Parse a DTSTART/DTEND property into a concrete point in time.
pub fn parse_when(p: &Prop) -> Option<When> {
    let v = p.value.trim();
    let is_date = p.params.get("VALUE").map(|x| x == "DATE").unwrap_or(false) || v.len() == 8;
    if is_date {
        return NaiveDate::parse_from_str(v, "%Y%m%d").ok().map(When::Date);
    }
    if let Some(utc) = v.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(utc, "%Y%m%dT%H%M%S").ok()?;
        let dt = Utc.from_utc_datetime(&naive);
        return Some(When::DateTime(dt.fixed_offset()));
    }
    let naive = NaiveDateTime::parse_from_str(v, "%Y%m%dT%H%M%S").ok()?;
    if let Some(tzid) = p.params.get("TZID") {
        if let Ok(tz) = tzid.parse::<chrono_tz::Tz>() {
            if let Some(dt) = tz.from_local_datetime(&naive).earliest() {
                return Some(When::DateTime(dt.fixed_offset()));
            }
        }
        // Common Windows/Apple aliases fall through to local time.
    }
    let local = Local.from_local_datetime(&naive).earliest()?;
    Some(When::DateTime(local.fixed_offset()))
}

impl When {
    pub fn add(&self, secs: i64) -> When {
        match self {
            When::Date(d) => When::Date(*d + Duration::seconds(secs)),
            When::DateTime(dt) => When::DateTime(*dt + Duration::seconds(secs)),
        }
    }
    pub fn to_iso(&self) -> String {
        match self {
            When::Date(d) => d.format("%Y-%m-%d").to_string(),
            When::DateTime(dt) => dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, false),
        }
    }
    pub fn is_date(&self) -> bool {
        matches!(self, When::Date(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_basic_event() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:1\r\nSUMMARY:Hello\\, world\r\n  continued\r\nDTSTART;TZID=Asia/Seoul:20260901T090000\r\nDTEND;TZID=Asia/Seoul:20260901T100000\r\nBEGIN:VALARM\r\nTRIGGER:-PT15M\r\nEND:VALARM\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let evs = parse_events(ics);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].value("SUMMARY").unwrap(), "Hello, world continued");
        assert_eq!(evs[0].alarms, vec![15]);
        let w = parse_when(evs[0].get("DTSTART").unwrap()).unwrap();
        assert_eq!(w.to_iso(), "2026-09-01T09:00:00+09:00");
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration_secs("-PT15M"), Some(-900));
        assert_eq!(parse_duration_secs("-P1D"), Some(-86400));
        assert_eq!(parse_duration_secs("PT1H30M"), Some(5400));
    }
}
