use super::*;
use serde_json::{json, Value};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn calendar() -> CalendarInfo {
    CalendarInfo {
        id: "google:account:me@example.com".into(),
        provider: Provider::Google,
        remote_id: "me@example.com".into(),
        name: "내 캘린더".into(),
        color: "#4285f4".into(),
        owned: true,
        is_holiday: false,
        can_edit: true,
        account: "me@example.com".into(),
        default_reminders: vec![10],
    }
}

fn invitation() -> Value {
    json!({
        "id": "invitation_20260922T010000Z",
        "summary": "파트너 미팅",
        "status": "confirmed",
        "start": { "dateTime": "2026-09-22T10:00:00+09:00" },
        "end": { "dateTime": "2026-09-22T11:30:00+09:00" },
        "organizer": { "email": "host@example.com", "displayName": "주최자" },
        "attendees": [
            { "email": "host@example.com", "organizer": true, "responseStatus": "accepted" },
            { "email": "colleague@example.com", "displayName": "동료", "responseStatus": "tentative" },
            { "email": "alias@example.com", "self": true, "responseStatus": "needsAction" }
        ],
        "recurringEventId": "invitation"
    })
}

fn convert(value: Value, cal: &CalendarInfo) -> Option<CalEvent> {
    to_event(serde_json::from_value(value).unwrap(), cal)
}

#[test]
fn pending_invitation_preserves_guests_and_can_respond_without_editing() {
    let e = convert(invitation(), &calendar()).unwrap();
    assert_eq!(e.title, "파트너 미팅");
    assert_eq!(e.attendees.len(), 3);
    assert_eq!(e.attendees[1].display_name.as_deref(), Some("동료"));
    assert_eq!(e.attendees[1].response_status, ResponseStatus::Tentative);
    assert!(e.can_respond);
    assert!(!e.editable);
    assert!(e.recurring);
    assert_eq!(e.response_status, Some(ResponseStatus::NeedsAction));
    assert_eq!(e.reminders, vec![10]);
    // Google's self flag, rather than the signed-in email, identifies the response target.
    assert_ne!(e.attendees[2].email.as_deref(), Some(calendar().account.as_str()));
    let cached: CalEvent = serde_json::from_value(serde_json::to_value(&e).unwrap()).unwrap();
    assert!(cached.attendees[2].is_self);
    assert_eq!(cached.response_status, e.response_status);
}

#[test]
fn all_response_states_remain_visible() {
    for status in ["needsAction", "accepted", "declined", "tentative"] {
        let mut value = invitation();
        value["attendees"][2]["responseStatus"] = json!(status);
        let event = convert(value, &calendar()).unwrap();
        assert_eq!(serde_json::to_value(event.response_status).unwrap(), json!(status));
        assert!(event.can_respond);
    }
}

#[test]
fn organizer_readonly_and_non_attendee_copies_cannot_rsvp() {
    let mut value = invitation();
    value["organizer"]["self"] = json!(true);
    let hosted = convert(value, &calendar()).unwrap();
    assert!(!hosted.can_respond);
    assert!(hosted.editable);

    let mut readonly = calendar();
    readonly.can_edit = false;
    let shared = convert(invitation(), &readonly).unwrap();
    assert!(!shared.can_respond);
    assert!(!shared.editable);
    assert_eq!(shared.attendees.len(), 3);

    let mut value = invitation();
    value["attendees"][2]["self"] = json!(false);
    let other = convert(value, &calendar()).unwrap();
    assert!(!other.can_respond);
    assert_eq!(other.response_status, None);

    let mut value = invitation();
    value["guestsCanModify"] = json!(true);
    assert!(convert(value, &calendar()).unwrap().editable);
}

#[test]
fn cancelled_events_are_removed_and_partial_guest_lists_are_marked() {
    let mut value = invitation();
    value["status"] = json!("cancelled");
    assert!(convert(value, &calendar()).is_none());
    let mut value = invitation();
    value["attendeesOmitted"] = json!(true);
    value["attendees"] = json!([{ "email": "alias@example.com", "self": true }]);
    let e = convert(value, &calendar()).unwrap();
    assert!(e.attendees_omitted);
    assert_eq!(e.attendees.len(), 1);
    assert_eq!(e.response_status, Some(ResponseStatus::NeedsAction));
}

#[test]
fn legacy_cache_loads_without_invitation_fields() {
    let e: CalEvent = serde_json::from_value(json!({
        "id": "old", "calendarId": "google:old", "title": "기존 일정",
        "start": "2026-09-22", "end": "2026-09-23", "allDay": true
    })).unwrap();
    assert!(e.attendees.is_empty());
    assert!(e.organizer.is_none());
    assert!(!e.can_respond);
    assert_eq!(e.response_status, None);
}

// Local HTTP fixture verifies the real reqwest request without touching a Google account.
async fn server(responses: Vec<(u16, Value)>) -> (String, tokio::task::JoinHandle<Vec<String>>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/events/instance", listener.local_addr().unwrap());
    let handle = tokio::spawn(async move {
        let mut requests = Vec::new();
        for (status, body) in responses {
            let (mut socket, _) = tokio::time::timeout(Duration::from_secs(5), listener.accept()).await.unwrap().unwrap();
            let mut bytes = Vec::new();
            let (header_end, length) = loop {
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
                if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                    let headers = String::from_utf8_lossy(&bytes[..i]);
                    let length = headers.lines().filter_map(|line| line.split_once(':'))
                        .find(|(key, _)| key.eq_ignore_ascii_case("content-length"))
                        .map(|(_, value)| value.trim().parse::<usize>().unwrap()).unwrap_or(0);
                    break (i + 4, length);
                }
            };
            while bytes.len() < header_end + length {
                let mut buf = [0u8; 4096];
                let n = socket.read(&mut buf).await.unwrap();
                assert!(n > 0);
                bytes.extend_from_slice(&buf[..n]);
            }
            requests.push(String::from_utf8(bytes).unwrap());
            let body = body.to_string();
            let reply = format!("HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            socket.write_all(reply.as_bytes()).await.unwrap();
        }
        requests
    });
    (url, handle)
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().timeout(Duration::from_secs(5)).build().unwrap()
}

#[tokio::test]
async fn every_page_requests_hidden_invitations_and_keeps_pending_events() {
    let (url, task) = server(vec![
        (200, json!({ "items": [], "nextPageToken": "second" })),
        (200, json!({ "items": [invitation()] })),
    ]).await;
    let events = list_events_at(&client(), "test-token", &calendar(),
        NaiveDate::from_ymd_opt(2026, 9, 1).unwrap(),
        NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(), &url).await.unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].response_status, Some(ResponseStatus::NeedsAction));
    let requests = task.await.unwrap();
    assert_eq!(requests.len(), 2);
    for request in &requests {
        assert!(request.contains("showHiddenInvitations=true"));
        assert!(request.contains("singleEvents=true"));
        assert!(!request.contains("maxAttendees="));
    }
    assert!(requests[1].contains("pageToken=second"));
}

#[tokio::test]
async fn rsvp_patches_only_the_actual_self_attendee_for_each_response() {
    for status in [ResponseStatus::Accepted, ResponseStatus::Declined, ResponseStatus::Tentative] {
        let (url, task) = server(vec![(200, invitation()), (200, json!({}))]).await;
        let event = respond_event_at(&client(), "test-token", &calendar(), status, &url).await.unwrap();
        assert_eq!(event.response_status, Some(status));
        assert_eq!(event.attendees.len(), 3);
        assert_eq!(event.attendees[0].response_status, ResponseStatus::Accepted);
        assert_eq!(event.attendees[1].response_status, ResponseStatus::Tentative);
        assert_eq!(event.attendees[2].response_status, status);
        let requests = task.await.unwrap();
        assert!(requests[0].starts_with("GET /events/instance "));
        assert!(requests[1].starts_with("PATCH /events/instance?sendUpdates=all "));
        let body: Value = serde_json::from_str(requests[1].split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(body, json!({
            "attendeesOmitted": true,
            "attendees": [{ "email": "alias@example.com", "responseStatus": status }]
        }));
    }
}

#[tokio::test]
async fn rsvp_errors_do_not_report_success_or_patch_someone_elses_event() {
    for code in [403, 404, 410, 500] {
        let (url, task) = server(vec![(200, invitation()), (code, json!({}))]).await;
        assert!(respond_event_at(&client(), "test-token", &calendar(), ResponseStatus::Accepted, &url).await.is_err());
        assert_eq!(task.await.unwrap().len(), 2);
    }
    let mut value = invitation();
    value["attendees"][2]["self"] = json!(false);
    let (url, task) = server(vec![(200, value)]).await;
    assert!(respond_event_at(&client(), "test-token", &calendar(), ResponseStatus::Accepted, &url).await.is_err());
    assert_eq!(task.await.unwrap().len(), 1);
    assert!(respond_event_at(&client(), "test-token", &calendar(), ResponseStatus::NeedsAction, "http://127.0.0.1:1").await.is_err());
}
