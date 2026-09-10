//! Google: OAuth 2.0 (PKCE + loopback redirect) and Calendar API v3 (read-only).
use crate::model::{CalEvent, CalendarInfo, Provider};
use base64::Engine;
use chrono::{DateTime, NaiveDate, Utc};
use rand::RngCore;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPES: &str = "openid email https://www.googleapis.com/auth/calendar.readonly https://www.googleapis.com/auth/calendar.events";

const PAGE_OK: &str = "<html><head><meta charset=\"utf-8\"><title>DeskCal</title></head><body style=\"font-family:system-ui;text-align:center;padding-top:80px\"><h2>로그인 완료</h2><p>이 창을 닫고 DeskCal로 돌아가세요.</p></body></html>";
const PAGE_FAIL: &str = "<html><head><meta charset=\"utf-8\"><title>DeskCal</title></head><body style=\"font-family:system-ui;text-align:center;padding-top:80px\"><h2>로그인 실패</h2><p>DeskCal에서 다시 시도해 주세요.</p></body></html>";

fn b64url(bytes: &[u8]) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

pub struct LoginResult {
    pub email: String,
    pub refresh_token: String,
    pub access_token: String,
    pub expires_in: i64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: Option<i64>,
    #[serde(default)]
    id_token: Option<String>,
}

/// Waits for the OAuth redirect on the given listener and returns (code, state).
fn wait_for_code(listener: TcpListener, timeout: Duration) -> Result<(String, String), String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        if std::time::Instant::now() > deadline {
            return Err("로그인 시간이 초과되었습니다.".into());
        }
        let (mut stream, _) = match listener.accept() {
            Ok(v) => v,
            Err(e) => return Err(e.to_string()),
        };
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).unwrap_or(0);
        let req = String::from_utf8_lossy(&buf[..n]);
        let first = req.lines().next().unwrap_or("");
        // "GET /?code=...&state=... HTTP/1.1"
        let path = first.split_whitespace().nth(1).unwrap_or("/");
        if path.starts_with("/favicon") {
            let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
            continue;
        }
        let url = url::Url::parse(&format!("http://127.0.0.1{path}")).map_err(|e| e.to_string())?;
        let mut code = None;
        let mut state = None;
        let mut error = None;
        for (k, v) in url.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.to_string()),
                "state" => state = Some(v.to_string()),
                "error" => error = Some(v.to_string()),
                _ => {}
            }
        }
        let (status, body) = if code.is_some() { ("200 OK", PAGE_OK) } else { ("400 Bad Request", PAGE_FAIL) };
        let _ = stream.write_all(
            format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .as_bytes(),
        );
        let _ = stream.flush();
        if let Some(err) = error {
            return Err(format!("Google 로그인 거부: {err}"));
        }
        if let (Some(c), Some(s)) = (code, state) {
            return Ok((c, s));
        }
    }
}

/// Full interactive login. `open` is called with the URL to open in the browser.
pub async fn login(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    open: impl FnOnce(String) -> Result<(), String>,
) -> Result<LoginResult, String> {
    if client_id.trim().is_empty() {
        return Err(
            "Google OAuth 클라이언트 ID가 설정되지 않았습니다. 설정 > 계정 > 고급에서 입력해 주세요.".into(),
        );
    }
    let mut verifier_bytes = [0u8; 48];
    rand::thread_rng().fill_bytes(&mut verifier_bytes);
    let verifier = b64url(&verifier_bytes);
    let challenge = b64url(&Sha256::digest(verifier.as_bytes()));
    let mut state_bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut state_bytes);
    let state = b64url(&state_bytes);

    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("로컬 포트 열기 실패: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}");

    let mut auth = url::Url::parse(AUTH_URL).unwrap();
    auth.query_pairs_mut()
        .append_pair("client_id", client_id.trim())
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("access_type", "offline")
        .append_pair("prompt", "consent")
        .append_pair("state", &state);

    open(auth.to_string())?;

    let expected_state = state.clone();
    let (code, got_state) =
        tokio::task::spawn_blocking(move || wait_for_code(listener, Duration::from_secs(300)))
            .await
            .map_err(|e| e.to_string())??;
    if got_state != expected_state {
        return Err("OAuth state 불일치".into());
    }

    let mut form = vec![
        ("code", code),
        ("client_id", client_id.trim().to_string()),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code".to_string()),
        ("code_verifier", verifier),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret.trim().to_string()));
    }
    let resp = http.post(TOKEN_URL).form(&form).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("토큰 교환 실패 ({status}): {text}"));
    }
    let tok: TokenResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let refresh_token = tok
        .refresh_token
        .ok_or("refresh_token을 받지 못했습니다. 다시 시도해 주세요.")?;

    let email = match fetch_email(http, &tok.access_token).await {
        Some(e) => e,
        None => tok
            .id_token
            .as_deref()
            .and_then(email_from_id_token)
            .unwrap_or_else(|| "Google 계정".into()),
    };

    Ok(LoginResult {
        email,
        refresh_token,
        access_token: tok.access_token,
        expires_in: tok.expires_in.unwrap_or(3600),
    })
}

async fn fetch_email(http: &reqwest::Client, access_token: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct UserInfo {
        email: Option<String>,
    }
    let r = http
        .get("https://www.googleapis.com/oauth2/v3/userinfo")
        .bearer_auth(access_token)
        .send()
        .await
        .ok()?;
    r.json::<UserInfo>().await.ok()?.email
}

fn email_from_id_token(id_token: &str) -> Option<String> {
    let payload = id_token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("email")?.as_str().map(|s| s.to_string())
}

#[derive(Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
}

pub async fn refresh_access_token(
    http: &reqwest::Client,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
) -> Result<AccessToken, String> {
    let mut form = vec![
        ("client_id", client_id.trim().to_string()),
        ("refresh_token", refresh_token.to_string()),
        ("grant_type", "refresh_token".to_string()),
    ];
    if !client_secret.trim().is_empty() {
        form.push(("client_secret", client_secret.trim().to_string()));
    }
    let resp = http.post(TOKEN_URL).form(&form).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        log::warn!("google refresh failed: {text}");
        return Err(format!("Google 토큰 갱신 실패 ({status}). 설정에서 다시 로그인해 주세요."));
    }
    let tok: TokenResponse = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    Ok(AccessToken {
        token: tok.access_token,
        expires_at: Utc::now() + chrono::Duration::seconds(tok.expires_in.unwrap_or(3600) - 60),
    })
}

// ── Calendar API ──

#[derive(Deserialize)]
struct CalendarListResponse {
    #[serde(default)]
    items: Vec<CalendarListEntry>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct CalendarListEntry {
    id: String,
    #[serde(default)]
    summary: String,
    #[serde(rename = "summaryOverride")]
    summary_override: Option<String>,
    #[serde(rename = "backgroundColor")]
    background_color: Option<String>,
    #[serde(rename = "accessRole", default)]
    access_role: String,
    #[serde(default)]
    primary: bool,
    #[serde(rename = "defaultReminders", default)]
    default_reminders: Vec<Reminder>,
    #[serde(default)]
    deleted: bool,
}

#[derive(Deserialize, Clone)]
struct Reminder {
    #[serde(default)]
    method: String,
    #[serde(default)]
    minutes: i64,
}

pub fn calendar_id(remote_id: &str) -> String {
    format!("google:{remote_id}")
}

pub async fn list_calendars(http: &reqwest::Client, token: &str) -> Result<Vec<CalendarInfo>, String> {
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut req = http
            .get("https://www.googleapis.com/calendar/v3/users/me/calendarList")
            .bearer_auth(token)
            .query(&[("maxResults", "250"), ("showHidden", "false")]);
        if let Some(p) = &page {
            req = req.query(&[("pageToken", p)]);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("캘린더 목록 조회 실패 ({})", resp.status()));
        }
        let body: CalendarListResponse = resp.json().await.map_err(|e| e.to_string())?;
        for c in body.items {
            if c.deleted {
                continue;
            }
            let name = c.summary_override.clone().unwrap_or(c.summary.clone());
            let lower = c.id.to_lowercase();
            let is_holiday = lower.contains("holiday@group.v.calendar.google.com")
                || name.contains("휴일")
                || name.to_lowercase().contains("holiday");
            let owned = c.primary || c.access_role == "owner";
            let can_edit = c.access_role == "owner" || c.access_role == "writer";
            out.push(CalendarInfo {
                id: calendar_id(&c.id),
                provider: Provider::Google,
                remote_id: c.id.clone(),
                name,
                color: c.background_color.unwrap_or_else(|| "#4285f4".into()),
                owned,
                is_holiday,
                can_edit,
                default_reminders: c
                    .default_reminders
                    .iter()
                    .filter(|r| r.method == "popup")
                    .map(|r| r.minutes)
                    .collect(),
            });
        }
        match body.next_page_token {
            Some(p) => page = Some(p),
            None => break,
        }
    }
    Ok(out)
}

#[derive(Deserialize)]
struct EventsResponse {
    #[serde(default)]
    items: Vec<GEvent>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

#[derive(Deserialize)]
struct GEvent {
    id: String,
    #[serde(default)]
    status: String,
    summary: Option<String>,
    location: Option<String>,
    description: Option<String>,
    #[serde(rename = "htmlLink")]
    html_link: Option<String>,
    start: Option<GTime>,
    end: Option<GTime>,
    reminders: Option<GReminders>,
    #[serde(rename = "eventType")]
    event_type: Option<String>,
}

#[derive(Deserialize)]
struct GTime {
    date: Option<String>,
    #[serde(rename = "dateTime")]
    date_time: Option<String>,
}

#[derive(Deserialize)]
struct GReminders {
    #[serde(rename = "useDefault", default)]
    use_default: bool,
    #[serde(default)]
    overrides: Vec<Reminder>,
}

pub async fn list_events(
    http: &reqwest::Client,
    token: &str,
    cal: &CalendarInfo,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Result<Vec<CalEvent>, String> {
    let time_min = format!("{}T00:00:00Z", range_start - chrono::Duration::days(1));
    let time_max = format!("{}T00:00:00Z", range_end + chrono::Duration::days(1));
    let mut out = Vec::new();
    let mut page: Option<String> = None;
    loop {
        let mut req = http
            .get(format!(
                "https://www.googleapis.com/calendar/v3/calendars/{}/events",
                urlencode(&cal.remote_id)
            ))
            .bearer_auth(token)
            .query(&[
                ("singleEvents", "true"),
                ("orderBy", "startTime"),
                ("maxResults", "2500"),
                ("timeMin", time_min.as_str()),
                ("timeMax", time_max.as_str()),
            ]);
        if let Some(p) = &page {
            req = req.query(&[("pageToken", p)]);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("'{}' 일정 조회 실패 ({})", cal.name, resp.status()));
        }
        let body: EventsResponse = resp.json().await.map_err(|e| e.to_string())?;
        for e in body.items {
            if e.status == "cancelled" {
                continue;
            }
            if e.event_type.as_deref() == Some("workingLocation") {
                continue;
            }
            let (Some(s), Some(en)) = (e.start, e.end) else { continue };
            let (start, end, all_day) = match (s.date, s.date_time, en.date, en.date_time) {
                (Some(sd), _, Some(ed), _) => (sd, ed, true),
                (_, Some(sdt), _, Some(edt)) => (sdt, edt, false),
                (_, Some(sdt), _, None) => (sdt.clone(), sdt, false),
                _ => continue,
            };
            let reminders = match e.reminders {
                Some(r) if !r.use_default => r
                    .overrides
                    .iter()
                    .filter(|x| x.method == "popup")
                    .map(|x| x.minutes)
                    .collect(),
                _ => cal.default_reminders.clone(),
            };
            out.push(CalEvent {
                id: format!("{}:{}:{}", cal.id, e.id, start),
                calendar_id: cal.id.clone(),
                remote_id: e.id.clone(),
                editable: cal.can_edit,
                title: e.summary.unwrap_or_else(|| "(제목 없음)".into()),
                start,
                end,
                all_day,
                location: e.location.filter(|s| !s.is_empty()),
                description: e.description.filter(|s| !s.is_empty()),
                reminders,
                html_link: e.html_link,
            });
        }
        match body.next_page_token {
            Some(p) => page = Some(p),
            None => break,
        }
    }
    Ok(out)
}

fn urlencode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

// ── writes (scope: calendar.events) ──

fn event_body(input: &crate::model::EventInput) -> serde_json::Value {
    use serde_json::json;
    let when = |s: &str| {
        if input.all_day {
            json!({ "date": s })
        } else {
            json!({ "dateTime": s })
        }
    };
    let reminders = if input.reminders.is_empty() {
        json!({ "useDefault": true })
    } else {
        json!({
            "useDefault": false,
            "overrides": input.reminders.iter().map(|m| json!({ "method": "popup", "minutes": m })).collect::<Vec<_>>()
        })
    };
    json!({
        "summary": input.title.trim(),
        "location": input.location.as_deref().map(str::trim).unwrap_or(""),
        "description": input.description.as_deref().map(str::trim).unwrap_or(""),
        "start": when(&input.start),
        "end": when(&input.end),
        "reminders": reminders,
    })
}

async fn check_write(resp: reqwest::Response, what: &str) -> Result<(), String> {
    let status = resp.status();
    if status.is_success() || status == reqwest::StatusCode::GONE {
        return Ok(());
    }
    let text = resp.text().await.unwrap_or_default();
    log::warn!("google {what} failed ({status}): {text}");
    if status == reqwest::StatusCode::FORBIDDEN {
        return Err(format!("{what} 권한이 없습니다. 설정에서 Google 계정을 다시 로그인하면 쓰기 권한을 요청합니다."));
    }
    Err(format!("Google {what} 실패 ({status})"))
}

pub async fn create_event(
    http: &reqwest::Client,
    token: &str,
    cal_remote_id: &str,
    input: &crate::model::EventInput,
) -> Result<(), String> {
    let resp = http
        .post(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events",
            urlencode(cal_remote_id)
        ))
        .bearer_auth(token)
        .json(&event_body(input))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_write(resp, "일정 생성").await
}

pub async fn update_event(
    http: &reqwest::Client,
    token: &str,
    cal_remote_id: &str,
    event_id: &str,
    input: &crate::model::EventInput,
) -> Result<(), String> {
    let resp = http
        .patch(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            urlencode(cal_remote_id),
            urlencode(event_id)
        ))
        .bearer_auth(token)
        .json(&event_body(input))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_write(resp, "일정 수정").await
}

pub async fn delete_event(
    http: &reqwest::Client,
    token: &str,
    cal_remote_id: &str,
    event_id: &str,
) -> Result<(), String> {
    let resp = http
        .delete(format!(
            "https://www.googleapis.com/calendar/v3/calendars/{}/events/{}",
            urlencode(cal_remote_id),
            urlencode(event_id)
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    check_write(resp, "일정 삭제").await
}
