//! Apple iCloud Calendar via CalDAV (Apple ID + app-specific password). Read + write.
//!
//! "Sign in with Apple" only provides identity; calendar data is only reachable through CalDAV,
//! which requires an app-specific password generated at https://appleid.apple.com.
use crate::ics;
use crate::model::{CalEvent, CalendarInfo, EventInput, Provider};
use crate::vevent;
use chrono::{Duration, NaiveDate};
use rand::RngCore;

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
    let body = r#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav" xmlns:a="http://apple.com/ns/ical/"><d:prop><d:displayname/><d:resourcetype/><a:calendar-color/><c:supported-calendar-component-set/><d:current-user-privilege-set/></d:prop></d:propfind>"#;
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
        let subscribed = types.iter().any(|t| t == "subscribed");
        let owned = !types.iter().any(|t| t == "shared" || t == "subscribed");
        // Writable unless the server says otherwise via current-user-privilege-set.
        let privileges: Vec<String> = resp
            .descendants()
            .filter(|n| n.is_element() && n.tag_name().name() == "current-user-privilege-set")
            .flat_map(|n| n.descendants())
            .filter(|n| n.is_element())
            .map(|n| n.tag_name().name().to_string())
            .collect();
        let can_edit = !subscribed
            && (privileges.is_empty()
                || privileges.iter().any(|p| p == "write" || p == "write-content" || p == "all"));
        let is_holiday = name.contains("휴일") || name.to_lowercase().contains("holiday");
        out.push(CalendarInfo {
            id: calendar_id(&url),
            provider: Provider::Apple,
            remote_id: url,
            name,
            color,
            owned,
            is_holiday,
            can_edit,
            account: creds.apple_id.clone(),
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
    for resp in doc
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "response")
    {
        let Some(href) = find_text(resp, "href") else { continue };
        let href = resolve(&cal.remote_id, &href);
        let Some(data) = resp
            .descendants()
            .find(|n| n.is_element() && n.tag_name().name() == "calendar-data")
        else {
            continue;
        };
        let Some(ics_text) = data.text() else { continue };
        let vevents = ics::parse_events(ics_text);
        out.extend(vevent::vevents_to_events(
            &vevents,
            &cal.id,
            Some(&href),
            cal.can_edit,
            range_start,
            range_end,
            !expanded,
        ));
    }
    Ok(out)
}

// ── writes ──

fn new_uid() -> String {
    let mut b = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut b);
    let hex: String = b.iter().map(|x| format!("{x:02X}")).collect();
    format!("{}-{}-{}-{}-{}", &hex[..8], &hex[8..12], &hex[12..16], &hex[16..20], &hex[20..])
}

async fn put_ics(
    http: &reqwest::Client,
    creds: &AppleCreds,
    href: &str,
    body: String,
    create: bool,
) -> Result<(), String> {
    let mut req = http
        .put(href)
        .basic_auth(&creds.apple_id, Some(&creds.password))
        .header("Content-Type", "text/calendar; charset=utf-8")
        .header("User-Agent", "DeskCal/0.1");
    if create {
        req = req.header("If-None-Match", "*");
    }
    let resp = req.body(body).send().await.map_err(|e| format!("iCloud 연결 실패: {e}"))?;
    let status = resp.status();
    check_auth(status)?;
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        log::warn!("caldav PUT {href} -> {status}: {text}");
        return Err(format!("iCloud 저장 실패 ({status})"));
    }
    Ok(())
}

/// Creates an event in the calendar collection `cal_url`. Returns the new resource href.
pub async fn create_event(
    http: &reqwest::Client,
    creds: &AppleCreds,
    cal_url: &str,
    input: &EventInput,
) -> Result<String, String> {
    let uid = new_uid();
    let href = format!("{}{}.ics", cal_url.trim_end_matches('/').to_string() + "/", uid);
    let body = vevent::build_ics(&uid, input, None)?;
    put_ics(http, creds, &href, body, true).await?;
    Ok(href)
}

/// Replaces the editable properties of the event stored at `href`.
pub async fn update_event(
    http: &reqwest::Client,
    creds: &AppleCreds,
    href: &str,
    input: &EventInput,
) -> Result<(), String> {
    let resp = http
        .get(href)
        .basic_auth(&creds.apple_id, Some(&creds.password))
        .header("User-Agent", "DeskCal/0.1")
        .send()
        .await
        .map_err(|e| format!("iCloud 연결 실패: {e}"))?;
    let status = resp.status();
    check_auth(status)?;
    if !status.is_success() {
        return Err(format!("iCloud 일정 조회 실패 ({status})"));
    }
    let existing = resp.text().await.map_err(|e| e.to_string())?;
    let body = vevent::build_ics("", input, Some(&existing))?;
    put_ics(http, creds, href, body, false).await
}

pub async fn delete_event(http: &reqwest::Client, creds: &AppleCreds, href: &str) -> Result<(), String> {
    let resp = http
        .delete(href)
        .basic_auth(&creds.apple_id, Some(&creds.password))
        .header("User-Agent", "DeskCal/0.1")
        .send()
        .await
        .map_err(|e| format!("iCloud 연결 실패: {e}"))?;
    let status = resp.status();
    check_auth(status)?;
    if !status.is_success() && status != reqwest::StatusCode::NOT_FOUND {
        return Err(format!("iCloud 삭제 실패 ({status})"));
    }
    Ok(())
}
