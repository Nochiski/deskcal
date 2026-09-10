import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountInfo, EventInput, IcsFeed, Settings, SyncResult, WindowMode } from "./types";
import { mock } from "./mock";

/** true when running inside the Tauri webview; false in a plain browser (vite dev). */
export const isTauri: boolean =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

// ── Tauri commands (implemented in src-tauri/src/commands.rs) ──

/** Load persisted settings (defaults are filled in by the backend). */
export const getSettings = () =>
  isTauri ? invoke<Settings>("get_settings") : mock.getSettings();
/** Persist the whole settings object. Backend applies side effects (autostart, window mode, opacity…). */
export const saveSettings = (settings: Settings) =>
  isTauri ? invoke<void>("save_settings", { settings }) : mock.saveSettings(settings);

/** Connected accounts. */
export const getAccounts = () =>
  isTauri ? invoke<AccountInfo[]>("get_accounts") : mock.getAccounts();
/**
 * Opens the browser for Google OAuth (PKCE, loopback). Resolves when login finishes.
 * Can be called repeatedly to link additional Google accounts (re-linking an email replaces its token).
 */
export const connectGoogle = () =>
  isTauri ? invoke<AccountInfo>("connect_google") : mock.connectGoogle();
/** Unlinks one Google account by `AccountInfo.id`. */
export const disconnectGoogle = (accountId: string) =>
  isTauri ? invoke<void>("disconnect_google", { accountId }) : mock.disconnectGoogle(accountId);
/** iCloud CalDAV with Apple ID + app-specific password (https://appleid.apple.com → 앱 암호). */
export const connectApple = (appleId: string, appPassword: string) =>
  isTauri
    ? invoke<AccountInfo>("connect_apple", { appleId, appPassword })
    : mock.connectApple(appleId, appPassword);
export const disconnectApple = () =>
  isTauri ? invoke<void>("disconnect_apple") : mock.disconnectApple();

/**
 * Returns the last cached sync result immediately (may be empty on first run).
 * Use `syncNow` to force a network refresh.
 */
export const getCached = () =>
  isTauri ? invoke<SyncResult>("get_cached") : mock.getCached();
/** Fetch events for [rangeStart, rangeEnd) (ISO dates "YYYY-MM-DD") from all connected accounts. */
export const syncNow = (rangeStart: string, rangeEnd: string) =>
  isTauri
    ? invoke<SyncResult>("sync_now", { rangeStart, rangeEnd })
    : mock.syncNow(rangeStart, rangeEnd);

export const setWindowMode = (mode: WindowMode) =>
  isTauri ? invoke<void>("set_window_mode", { mode }) : mock.setWindowMode(mode);
export const hideWindow = () =>
  isTauri ? invoke<void>("hide_window") : mock.hideWindow();
export const openUrl = (url: string) =>
  isTauri ? invoke<void>("open_external", { url }) : mock.openUrl(url);

// ── Events emitted by the backend ──

/** Emitted after a background sync completes (payload = SyncResult). */
export const onEventsUpdated = (cb: (r: SyncResult) => void): Promise<UnlistenFn> =>
  isTauri ? listen<SyncResult>("events-updated", (e) => cb(e.payload)) : mock.noopListen();
/** Emitted when the window mode changes (from tray menu). */
export const onWindowModeChanged = (cb: (m: WindowMode) => void): Promise<UnlistenFn> =>
  isTauri ? listen<WindowMode>("window-mode-changed", (e) => cb(e.payload)) : mock.noopListen();
/** Emitted when the tray asks the UI to open settings. */
export const onOpenSettings = (cb: () => void): Promise<UnlistenFn> =>
  isTauri ? listen("open-settings", () => cb()) : mock.noopListen();

// ── Event write operations (Google OAuth / iCloud only; ICS feeds are read-only) ──
// Each returns a fresh SyncResult for the last synced range so the UI can apply it as a range replace.

export const createEvent = (input: EventInput) =>
  isTauri ? invoke<SyncResult>("create_event", { input }) : mock.createEvent(input);
export const updateEvent = (calendarId: string, remoteId: string, input: EventInput) =>
  isTauri
    ? invoke<SyncResult>("update_event", { calendarId, remoteId, input })
    : mock.updateEvent(calendarId, remoteId, input);
export const deleteEvent = (calendarId: string, remoteId: string) =>
  isTauri ? invoke<SyncResult>("delete_event", { calendarId, remoteId }) : mock.deleteEvent(calendarId, remoteId);

// ── iCal subscription feeds (read-only, no login needed) ──

export const listIcsFeeds = () => (isTauri ? invoke<IcsFeed[]>("list_ics_feeds") : mock.listIcsFeeds());
/** Validates by fetching the URL once. `webcal://` is accepted. */
export const addIcsFeed = (name: string, url: string) =>
  isTauri ? invoke<IcsFeed>("add_ics_feed", { name, url }) : mock.addIcsFeed(name, url);
export const removeIcsFeed = (id: string) =>
  isTauri ? invoke<void>("remove_ics_feed", { id }) : mock.removeIcsFeed(id);
