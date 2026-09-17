//! Shared data model. Keep in sync with `src/lib/types.ts`.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Google,
    Apple,
    Ics,
}

impl Provider {
    pub fn as_str(&self) -> &'static str {
        match self {
            Provider::Google => "google",
            Provider::Apple => "apple",
            Provider::Ics => "ics",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarInfo {
    pub id: String,
    pub provider: Provider,
    pub remote_id: String,
    pub name: String,
    pub color: String,
    pub owned: bool,
    pub is_holiday: bool,
    #[serde(default)]
    pub can_edit: bool,
    /// Owning account label (Google email, Apple ID, or "iCal"). Lets the UI group by account.
    #[serde(default)]
    pub account: String,
    /// Default reminder minutes supplied by the provider for this calendar.
    #[serde(default)]
    pub default_reminders: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalendarPrefs {
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default = "default_true")]
    pub notify: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for CalendarPrefs {
    fn default() -> Self {
        Self { visible: true, notify: true, color: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ResponseStatus {
    #[default]
    NeedsAction,
    Accepted,
    Declined,
    Tentative,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventOrganizer {
    pub email: Option<String>,
    pub display_name: Option<String>,
    #[serde(default, alias = "self")]
    pub is_self: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventAttendee {
    pub email: Option<String>,
    pub display_name: Option<String>,
    #[serde(default, alias = "self")]
    pub is_self: bool,
    #[serde(default)]
    pub organizer: bool,
    #[serde(default)]
    pub optional: bool,
    #[serde(default)]
    pub response_status: ResponseStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalEvent {
    pub id: String,
    pub calendar_id: String,
    #[serde(default)]
    pub remote_id: String,
    #[serde(default)]
    pub editable: bool,
    pub title: String,
    /// ISO-8601 with offset, or YYYY-MM-DD for all-day.
    pub start: String,
    pub end: String,
    pub all_day: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default)]
    pub reminders: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub html_link: Option<String>,
    #[serde(default)]
    pub attendees: Vec<EventAttendee>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organizer: Option<EventOrganizer>,
    #[serde(default)]
    pub attendees_omitted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_status: Option<ResponseStatus>,
    /// Responding to an invitation does not require permission to edit the event's details.
    #[serde(default)]
    pub can_respond: bool,
    #[serde(default)]
    pub recurring: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountInfo {
    pub provider: Provider,
    /// Stable account id. Google: random id per linked account; Apple: "apple"; iCal: "ics".
    pub id: String,
    pub label: String,
    pub connected: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WindowMode {
    #[default]
    Floating,
    Desktop,
    Wallpaper,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GoogleClient {
    #[serde(default)]
    pub client_id: String,
    #[serde(default)]
    pub client_secret: String,
}

impl Default for GoogleClient {
    fn default() -> Self {
        Self {
            client_id: option_env!("DESKCAL_GOOGLE_CLIENT_ID").unwrap_or("").to_string(),
            client_secret: option_env!("DESKCAL_GOOGLE_CLIENT_SECRET").unwrap_or("").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub window_mode: WindowMode,
    pub opacity: f64,
    pub theme: String,
    /// "glass" (liquid glass) or "flat".
    pub style: String,
    pub week_start: u8,
    pub sync_interval_min: u64,
    pub autostart: bool,
    /// When launched at logon, show the window right away instead of starting hidden in the tray.
    pub autostart_visible: bool,
    pub default_reminder_min: i64,
    pub notifications_enabled: bool,
    pub all_day_reminder_time: String,
    pub google: GoogleClient,
    pub calendars: HashMap<String, CalendarPrefs>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            window_mode: WindowMode::Floating,
            opacity: 0.96,
            theme: "system".into(),
            style: "glass".into(),
            week_start: 0,
            sync_interval_min: 15,
            autostart: false,
            autostart_visible: true,
            default_reminder_min: 10,
            notifications_enabled: true,
            all_day_reminder_time: "09:00".into(),
            google: GoogleClient::default(),
            calendars: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncError {
    pub provider: Provider,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SyncResult {
    pub calendars: Vec<CalendarInfo>,
    pub events: Vec<CalEvent>,
    #[serde(default)]
    pub synced_at: String,
    #[serde(default)]
    pub errors: Vec<SyncError>,
    /// The date range this result covers (YYYY-MM-DD, end exclusive).
    #[serde(default)]
    pub range_start: String,
    #[serde(default)]
    pub range_end: String,
}

/// Non-secret account metadata (secrets live in the OS keyring).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Accounts {
    /// Legacy single-account field; migrated into `google_accounts` on load.
    pub google_email: Option<String>,
    /// Linked Google accounts (refresh tokens live in the keyring under `google_refresh_token:{id}`).
    pub google_accounts: Vec<GoogleAccount>,
    pub apple_id: Option<String>,
    /// Discovered CalDAV calendar-home URL for the Apple account.
    pub apple_home_url: Option<String>,
    /// Subscribed iCalendar feeds (read-only).
    pub ics_feeds: Vec<IcsFeed>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GoogleAccount {
    pub id: String,
    pub email: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct IcsFeed {
    pub id: String,
    pub name: String,
    pub url: String,
    pub color: String,
}

/// Fields the user can set when creating or editing an event (mirrors EventInput in types.ts).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventInput {
    pub calendar_id: String,
    pub title: String,
    pub start: String,
    pub end: String,
    pub all_day: bool,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub reminders: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowGeometry {
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}
