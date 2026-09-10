//! Plain iCalendar subscription feeds (read-only). Works with Google Calendar's
//! "비공개 주소(iCal 형식)" URL, public holiday feeds, webcal:// links, etc. No login needed.
use crate::ics;
use crate::model::{CalEvent, CalendarInfo, IcsFeed, Provider};
use crate::vevent;
use chrono::NaiveDate;

pub fn calendar_id(feed_id: &str) -> String {
    format!("ics:{feed_id}")
}

pub fn normalize_url(url: &str) -> Result<String, String> {
    let u = url.trim();
    let u = if let Some(rest) = u.strip_prefix("webcal://") {
        format!("https://{rest}")
    } else if let Some(rest) = u.strip_prefix("webcals://") {
        format!("https://{rest}")
    } else {
        u.to_string()
    };
    if !(u.starts_with("https://") || u.starts_with("http://")) {
        return Err("http(s):// 또는 webcal:// 로 시작하는 URL을 입력해 주세요.".into());
    }
    url::Url::parse(&u).map_err(|e| format!("잘못된 URL: {e}"))?;
    Ok(u)
}

async fn fetch_text(http: &reqwest::Client, url: &str) -> Result<String, String> {
    let resp = http
        .get(url)
        .header("User-Agent", "DeskCal/0.1")
        .send()
        .await
        .map_err(|e| format!("피드 다운로드 실패: {e}"))?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("피드 다운로드 실패 ({status})"));
    }
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !text.to_uppercase().contains("BEGIN:VCALENDAR") {
        return Err("iCalendar(.ics) 형식이 아닙니다.".into());
    }
    Ok(text)
}

/// Top-level calendar properties (X-WR-CALNAME, X-APPLE-CALENDAR-COLOR) of a feed.
fn calendar_props(text: &str) -> (Option<String>, Option<String>) {
    let mut name = None;
    let mut color = None;
    for line in text.lines().take(200) {
        let line = line.trim_end_matches('\r');
        if line.eq_ignore_ascii_case("BEGIN:VEVENT") {
            break;
        }
        if let Some(v) = line.strip_prefix("X-WR-CALNAME:") {
            name = Some(ics::unescape(v.trim()));
        } else if let Some(v) = line.strip_prefix("X-APPLE-CALENDAR-COLOR:") {
            let v = v.trim();
            color = Some(if v.len() >= 7 { v[..7].to_string() } else { v.to_string() });
        }
    }
    (name, color)
}

/// Validates a feed URL by downloading it once; returns (calendar name, color) hints.
pub async fn probe(http: &reqwest::Client, url: &str) -> Result<(Option<String>, Option<String>), String> {
    let text = fetch_text(http, url).await?;
    Ok(calendar_props(&text))
}

pub async fn fetch(
    http: &reqwest::Client,
    feed: &IcsFeed,
    range_start: NaiveDate,
    range_end: NaiveDate,
) -> Result<(CalendarInfo, Vec<CalEvent>), String> {
    let text = fetch_text(http, &feed.url).await?;
    let (cal_name, cal_color) = calendar_props(&text);
    let name = if feed.name.trim().is_empty() {
        cal_name.unwrap_or_else(|| "iCal 구독".into())
    } else {
        feed.name.clone()
    };
    let color = if feed.color.is_empty() {
        cal_color.unwrap_or_else(|| "#7986cb".into())
    } else {
        feed.color.clone()
    };
    let id = calendar_id(&feed.id);
    let is_holiday = name.contains("휴일") || name.to_lowercase().contains("holiday");
    let cal = CalendarInfo {
        id: id.clone(),
        provider: Provider::Ics,
        remote_id: feed.url.clone(),
        name,
        color,
        owned: false,
        is_holiday,
        can_edit: false,
        default_reminders: vec![],
    };
    let vevents = ics::parse_events(&text);
    let events = vevent::vevents_to_events(&vevents, &id, None, false, range_start, range_end, true);
    Ok((cal, events))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Network test: `cargo test --lib feed -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn fetches_public_korean_holidays() {
        let http = reqwest::Client::new();
        let feed = IcsFeed {
            id: "test".into(),
            name: String::new(),
            url: "webcal://calendar.google.com/calendar/ical/ko.south_korea%23holiday%40group.v.calendar.google.com/public/basic.ics".into(),
            color: String::new(),
        };
        let url = normalize_url(&feed.url).unwrap();
        assert!(url.starts_with("https://"));
        let feed = IcsFeed { url, ..feed };
        let (cal, events) = fetch(
            &http,
            &feed,
            NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
            NaiveDate::from_ymd_opt(2026, 10, 15).unwrap(),
        )
        .await
        .unwrap();
        println!("calendar: {} ({})", cal.name, cal.color);
        for e in &events {
            println!("{} .. {}  {}", e.start, e.end, e.title);
        }
        assert!(cal.is_holiday);
        assert!(events.iter().any(|e| e.title.contains("추석")));
        assert!(events.iter().any(|e| e.title.contains("개천절")));
        assert!(events.iter().all(|e| e.all_day && !e.editable));
    }
}
