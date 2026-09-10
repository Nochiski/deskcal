import { useCallback, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import type { AccountInfo, CalEvent, CalendarInfo, Settings, SyncResult, WindowMode } from "./lib/types";
import {
  getAccounts,
  getCached,
  getSettings,
  hideWindow,
  onEventsUpdated,
  onOpenSettings,
  onWindowModeChanged,
  saveSettings,
  syncNow,
} from "./lib/api";
import { addMonths, dayKey, eventStart, isAllDayString, parseDayKey } from "./lib/dates";
import TopBar from "./components/TopBar";
import MonthGrid from "./components/MonthGrid";
import EventDetail from "./components/EventDetail";
import DayMore from "./components/DayMore";
import SettingsModal from "./components/Settings";
import type { AnchorRect } from "./components/Popover";
import { HOLIDAY_GREEN } from "./components/EventChip";

const DEFAULT_SETTINGS: Settings = {
  windowMode: "floating",
  opacity: 0.96,
  theme: "system",
  weekStart: 0,
  syncIntervalMin: 15,
  autostart: false,
  defaultReminderMin: 10,
  notificationsEnabled: true,
  allDayReminderTime: "09:00",
  google: { clientId: "", clientSecret: "" },
  calendars: {},
};

/** Dev-only URL params (browser mock mode): ?settings&tab=display&theme=dark */
const DEV_PARAMS = new URLSearchParams(typeof location !== "undefined" ? location.search : "");

interface Range {
  start: string;
  end: string;
}

/** Sync range for a view month: [first of prev month, first of month+2). */
function rangeFor(viewMonth: Date): Range {
  return { start: dayKey(addMonths(viewMonth, -1)), end: dayKey(addMonths(viewMonth, 2)) };
}

function rangeCovers(loaded: Range | null, view: Date): boolean {
  if (!loaded) return false;
  const gridStart = dayKey(addMonths(view, 0));
  const gridEnd = dayKey(addMonths(view, 1));
  // the grid may spill up to 6 days before/after; be conservative with a 7-day margin
  const s = parseDayKey(loaded.start).getTime() + 7 * 86_400_000;
  const e = parseDayKey(loaded.end).getTime() - 7 * 86_400_000;
  return parseDayKey(gridStart).getTime() >= s && parseDayKey(gridEnd).getTime() <= e;
}

type Popup =
  | { kind: "event"; ev: CalEvent; anchor: AnchorRect }
  | { kind: "day"; day: Date; anchor: AnchorRect }
  | null;

function rectOf(el: HTMLElement): AnchorRect {
  const r = el.getBoundingClientRect();
  return { left: r.left, top: r.top, width: r.width, height: r.height };
}

export default function App() {
  const [settings, setSettings] = useState<Settings>(DEFAULT_SETTINGS);
  const [settingsLoaded, setSettingsLoaded] = useState(false);
  const [accounts, setAccounts] = useState<AccountInfo[]>([]);
  const [calendars, setCalendars] = useState<CalendarInfo[]>([]);
  const [events, setEvents] = useState<CalEvent[]>([]);
  const [syncedAt, setSyncedAt] = useState<string | null>(null);
  const [syncErrors, setSyncErrors] = useState<SyncResult["errors"]>([]);
  const [syncing, setSyncing] = useState(false);
  const [viewMonth, setViewMonth] = useState(() => addMonths(new Date(), 0));
  const [popup, setPopup] = useState<Popup>(null);
  const [settingsOpen, setSettingsOpen] = useState(() => DEV_PARAMS.has("settings"));
  const loadedRange = useRef<Range | null>(null);
  const syncSeq = useRef(0);

  // ── apply a sync result ──
  const applyResult = useCallback((r: SyncResult, range: Range | null) => {
    setCalendars(r.calendars);
    setSyncedAt(r.syncedAt);
    setSyncErrors(r.errors ?? []);
    setEvents((prev) => {
      if (!range) {
        // background update: merge by id (keeps events outside the backend's range)
        const map = new Map(prev.map((e) => [e.id, e]));
        for (const e of r.events) map.set(e.id, e);
        return [...map.values()];
      }
      // explicit range sync: replace everything inside the range, keep the rest
      const s = parseDayKey(range.start).getTime();
      const e = parseDayKey(range.end).getTime();
      const inRange = (ev: CalEvent) => {
        const t = (isAllDayString(ev.start) ? parseDayKey(ev.start) : eventStart(ev)).getTime();
        return t >= s && t < e;
      };
      const kept = prev.filter((ev) => !inRange(ev));
      const ids = new Set(kept.map((k) => k.id));
      for (const ev of r.events) if (!ids.has(ev.id)) kept.push(ev);
      return kept;
    });
  }, []);

  const doSync = useCallback(
    async (month: Date) => {
      const range = rangeFor(month);
      const seq = ++syncSeq.current;
      setSyncing(true);
      try {
        const r = await syncNow(range.start, range.end);
        if (seq !== syncSeq.current) return;
        loadedRange.current = range;
        applyResult(r, range);
      } catch (e) {
        if (seq === syncSeq.current) setSyncErrors([{ provider: "google", message: String(e) }]);
      } finally {
        if (seq === syncSeq.current) setSyncing(false);
      }
    },
    [applyResult],
  );

  const refreshAccounts = useCallback(async () => {
    try {
      setAccounts(await getAccounts());
    } catch {
      /* ignore */
    }
  }, []);

  // ── initial load ──
  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const s = await getSettings();
        const theme = DEV_PARAMS.get("theme") as Settings["theme"] | null;
        if (!cancelled)
          setSettings({
            ...DEFAULT_SETTINGS,
            ...s,
            ...(theme ? { theme } : {}),
            google: { ...DEFAULT_SETTINGS.google, ...s.google },
          });
      } catch {
        /* keep defaults */
      } finally {
        if (!cancelled) setSettingsLoaded(true);
      }
      await refreshAccounts();
      try {
        const cached = await getCached();
        if (!cancelled) applyResult(cached, null);
      } catch {
        /* ignore */
      }
      if (!cancelled) void doSync(addMonths(new Date(), 0));
    })();

    const unlisten: Promise<() => void>[] = [
      onEventsUpdated((r) => applyResult(r, null)),
      onWindowModeChanged((m: WindowMode) => setSettings((prev) => ({ ...prev, windowMode: m }))),
      onOpenSettings(() => setSettingsOpen(true)),
    ];
    return () => {
      cancelled = true;
      unlisten.forEach((p) => p.then((u) => u()).catch(() => {}));
    };
  }, [applyResult, doSync, refreshAccounts]);

  // ── re-sync when navigating out of the loaded range ──
  useEffect(() => {
    if (!rangeCovers(loadedRange.current, viewMonth) && accounts.some((a) => a.connected)) {
      void doSync(viewMonth);
    }
  }, [viewMonth, accounts, doSync]);

  // ── theme / opacity / mode on the document ──
  useEffect(() => {
    const root = document.documentElement;
    const apply = () => {
      const dark =
        settings.theme === "dark" ||
        (settings.theme === "system" && window.matchMedia("(prefers-color-scheme: dark)").matches);
      root.dataset.theme = dark ? "dark" : "light";
    };
    apply();
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", apply);
    return () => mq.removeEventListener("change", apply);
  }, [settings.theme]);

  useEffect(() => {
    const root = document.documentElement;
    root.style.setProperty("--opacity", String(settings.opacity));
    root.dataset.mode = settings.windowMode;
  }, [settings.opacity, settings.windowMode]);

  // ── settings persistence (debounced) ──
  const saveTimer = useRef<number | null>(null);
  const updateSettings = useCallback((next: Settings) => {
    setSettings(next);
    if (saveTimer.current) window.clearTimeout(saveTimer.current);
    saveTimer.current = window.setTimeout(() => {
      saveSettings(next).catch((e) => console.error("saveSettings failed", e));
    }, 250);
  }, []);

  // ── derived ──
  const calMap = useMemo(() => new Map(calendars.map((c) => [c.id, c])), [calendars]);
  const colorOf = useCallback(
    (id: string) => {
      const c = calMap.get(id);
      if (c?.isHoliday) return HOLIDAY_GREEN;
      return settings.calendars[id]?.color ?? c?.color ?? "#4285f4";
    },
    [calMap, settings.calendars],
  );
  const visibleEvents = useMemo(
    () => events.filter((e) => settings.calendars[e.calendarId]?.visible !== false),
    [events, settings.calendars],
  );
  const connected = accounts.some((a) => a.connected);
  const compact = settings.windowMode !== "floating";

  const onEventClick = useCallback((ev: CalEvent, e: MouseEvent<HTMLElement>) => {
    e.stopPropagation();
    setPopup({ kind: "event", ev, anchor: rectOf(e.currentTarget) });
  }, []);
  const onMoreClick = useCallback((day: Date, e: MouseEvent<HTMLElement>) => {
    e.stopPropagation();
    setPopup({ kind: "day", day, anchor: rectOf(e.currentTarget) });
  }, []);
  const closePopup = useCallback(() => setPopup(null), []);

  const errorText = syncErrors.length
    ? syncErrors.map((e) => `${e.provider === "google" ? "Google" : "Apple"}: ${e.message}`).join(" · ")
    : null;

  return (
    <div className={`app mode-${settings.windowMode}`}>
      <TopBar
        viewMonth={viewMonth}
        syncing={syncing}
        syncedAt={syncedAt}
        compact={compact}
        onPrev={() => setViewMonth((m) => addMonths(m, -1))}
        onNext={() => setViewMonth((m) => addMonths(m, 1))}
        onToday={() => setViewMonth(addMonths(new Date(), 0))}
        onSync={() => void doSync(viewMonth)}
        onSettings={() => setSettingsOpen(true)}
        onHide={() => void hideWindow()}
      />

      {errorText && (
        <div className="banner" title={errorText}>
          ⚠ {errorText}
        </div>
      )}

      <main className="content">
        <MonthGrid
          viewMonth={viewMonth}
          weekStart={settings.weekStart}
          events={visibleEvents}
          calendars={calMap}
          colorOf={colorOf}
          onEventClick={onEventClick}
          onMoreClick={onMoreClick}
        />
        {settingsLoaded && !connected && events.length === 0 && (
          <div className="empty">
            <div className="empty-card">
              <div className="empty-icon">📅</div>
              <div className="empty-title">계정을 연결해 일정을 불러오세요</div>
              <div className="empty-desc">Google 캘린더 또는 iCloud 캘린더를 연결할 수 있습니다.</div>
              <button type="button" className="btn btn-primary" onClick={() => setSettingsOpen(true)}>
                계정 연결
              </button>
            </div>
          </div>
        )}
      </main>

      {popup?.kind === "event" && (
        <EventDetail
          ev={popup.ev}
          calendar={calMap.get(popup.ev.calendarId)}
          color={colorOf(popup.ev.calendarId)}
          anchor={popup.anchor}
          onClose={closePopup}
        />
      )}
      {popup?.kind === "day" && (
        <DayMore
          day={popup.day}
          events={visibleEvents}
          calendars={calMap}
          colorOf={colorOf}
          anchor={popup.anchor}
          onClose={closePopup}
          onEventClick={onEventClick}
        />
      )}

      {settingsOpen && (
        <SettingsModal
          settings={settings}
          accounts={accounts}
          calendars={calendars}
          onChange={updateSettings}
          onAccountsChanged={() => {
            void refreshAccounts().then(() => doSync(viewMonth));
          }}
          onClose={() => setSettingsOpen(false)}
        />
      )}
    </div>
  );
}
