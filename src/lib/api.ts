import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { AccountInfo, Settings, SyncResult, WindowMode } from "./types";
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
/** Opens the browser for Google OAuth (PKCE, loopback). Resolves when login finishes. */
export const connectGoogle = () =>
  isTauri ? invoke<AccountInfo>("connect_google") : mock.connectGoogle();
export const disconnectGoogle = () =>
  isTauri ? invoke<void>("disconnect_google") : mock.disconnectGoogle();
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
