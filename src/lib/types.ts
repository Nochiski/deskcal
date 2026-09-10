// ─────────────────────────────────────────────────────────────
// Shared contract between the React frontend and the Rust backend.
// Keep in sync with src-tauri/src/model.rs
// ─────────────────────────────────────────────────────────────

export type Provider = "google" | "apple";

/** A calendar (e.g. "한상목", "Tasks", "대한민국의 휴일") coming from a provider. */
export interface CalendarInfo {
  /** Stable id: `${provider}:${remoteId}` */
  id: string;
  provider: Provider;
  remoteId: string;
  name: string;
  /** Hex color like "#f6bf26". Provider-supplied, may be overridden in settings. */
  color: string;
  /** true = 내 캘린더, false = 다른 캘린더 (구독/공유) */
  owned: boolean;
  /** Holiday-ish calendars (e.g. 대한민국의 휴일) are flagged by the backend. */
  isHoliday: boolean;
}

/** Per-calendar user preferences (persisted in settings). */
export interface CalendarPrefs {
  /** Show events from this calendar. */
  visible: boolean;
  /** Fire reminders/notifications for this calendar. */
  notify: boolean;
  /** Optional color override (hex). */
  color?: string;
}

/** One occurrence of an event, already expanded (no recurrence rules). */
export interface CalEvent {
  /** Unique per occurrence: `${calendarId}:${remoteId}:${startIso}` */
  id: string;
  calendarId: string;
  title: string;
  /** ISO-8601 with offset (e.g. "2026-09-01T09:00:00+09:00") or "YYYY-MM-DD" for all-day. */
  start: string;
  /** Exclusive end. Same format as start. For all-day events end is the day AFTER the last day. */
  end: string;
  allDay: boolean;
  location?: string;
  description?: string;
  /** Reminder lead times in minutes before start (from provider). Empty = use calendar/global default. */
  reminders: number[];
  /** Original web link if available. */
  htmlLink?: string;
}

export interface AccountInfo {
  provider: Provider;
  /** Email / Apple ID shown in settings. */
  label: string;
  connected: boolean;
}

export type WindowMode = "floating" | "desktop" | "wallpaper";
//  floating  : normal always-available window (no taskbar button, tray only)
//  desktop   : pinned onto the desktop, above icons, under every other window, survives Win+D. Interactive.
//  wallpaper : behind the desktop icons (like Wallpaper Engine). View-only.

export interface Settings {
  windowMode: WindowMode;
  /** 0..1 background opacity of the calendar panel. */
  opacity: number;
  /** "light" | "dark" | "system" */
  theme: "light" | "dark" | "system";
  /** 0 = Sunday, 1 = Monday */
  weekStart: 0 | 1;
  /** Minutes between background syncs. */
  syncIntervalMin: number;
  autostart: boolean;
  /** Global default reminder (minutes before start) used when an event has no explicit reminder. */
  defaultReminderMin: number;
  /** Master switch for notifications. */
  notificationsEnabled: boolean;
  /** Fire a reminder for all-day events at this local time (HH:mm), e.g. "09:00". */
  allDayReminderTime: string;
  google: { clientId: string; clientSecret: string };
  /** Keyed by CalendarInfo.id */
  calendars: Record<string, CalendarPrefs>;
}

export interface SyncResult {
  calendars: CalendarInfo[];
  events: CalEvent[];
  /** ISO timestamp of this sync. */
  syncedAt: string;
  /** Non-fatal per-provider errors (e.g. token expired). */
  errors: { provider: Provider; message: string }[];
}
