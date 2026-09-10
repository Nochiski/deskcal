import { useEffect, useState } from "react";
import type { AccountInfo, CalendarInfo, CalendarPrefs, IcsFeed, Settings, WindowMode } from "../lib/types";
import {
  addIcsFeed,
  connectApple,
  connectGoogle,
  disconnectApple,
  disconnectGoogle,
  listIcsFeeds,
  removeIcsFeed,
  saveSettings,
  setWindowMode,
} from "../lib/api";
import { HOLIDAY_GREEN } from "./EventChip";

type Tab = "accounts" | "calendars" | "notify" | "display";

interface Props {
  settings: Settings;
  accounts: AccountInfo[];
  calendars: CalendarInfo[];
  onChange: (next: Settings) => void;
  onAccountsChanged: () => void;
  onClose: () => void;
}

const TABS: { id: Tab; label: string }[] = [
  { id: "accounts", label: "계정" },
  { id: "calendars", label: "캘린더" },
  { id: "notify", label: "알림" },
  { id: "display", label: "표시" },
];

export default function SettingsModal({ settings, accounts, calendars, onChange, onAccountsChanged, onClose }: Props) {
  const [tab, setTab] = useState<Tab>(() => {
    const q = new URLSearchParams(location.search).get("tab") as Tab | null;
    if (q && TABS.some((t) => t.id === q)) return q;
    return accounts.some((a) => a.connected) ? "calendars" : "accounts";
  });

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  const patch = (p: Partial<Settings>) => onChange({ ...settings, ...p });

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" role="dialog" aria-label="설정">
        <div className="modal-head">
          <h2>설정</h2>
          <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
            ×
          </button>
        </div>
        <div className="modal-body">
          <nav className="tabs">
            {TABS.map((t) => (
              <button
                key={t.id}
                type="button"
                className={`tab${tab === t.id ? " active" : ""}`}
                onClick={() => setTab(t.id)}
              >
                {t.label}
              </button>
            ))}
          </nav>
          <div className="pane">
            {tab === "accounts" && (
              <AccountsPane settings={settings} accounts={accounts} patch={patch} onAccountsChanged={onAccountsChanged} />
            )}
            {tab === "calendars" && <CalendarsPane settings={settings} calendars={calendars} patch={patch} />}
            {tab === "notify" && <NotifyPane settings={settings} patch={patch} />}
            {tab === "display" && <DisplayPane settings={settings} patch={patch} />}
          </div>
        </div>
      </div>
    </div>
  );
}

// ───────────────────────── 계정 ─────────────────────────

function AccountsPane({
  settings,
  accounts,
  patch,
  onAccountsChanged,
}: {
  settings: Settings;
  accounts: AccountInfo[];
  patch: (p: Partial<Settings>) => void;
  onAccountsChanged: () => void;
}) {
  const googles = accounts.filter((a) => a.provider === "google");
  const apple = accounts.find((a) => a.provider === "apple");
  const [gBusy, setGBusy] = useState(false);
  /** id of the Google account row currently being unlinked */
  const [gRowBusy, setGRowBusy] = useState<string | null>(null);
  const [gErr, setGErr] = useState<string | null>(null);
  const [advOpen, setAdvOpen] = useState(false);
  const [aBusy, setABusy] = useState(false);
  const [aErr, setAErr] = useState<string | null>(null);
  const [appleId, setAppleId] = useState("");
  const [appPw, setAppPw] = useState("");

  const doGoogle = async () => {
    setGBusy(true);
    setGErr(null);
    try {
      // Flush any just-typed OAuth client id/secret before the backend reads them.
      await saveSettings(settings);
      await connectGoogle();
      onAccountsChanged();
    } catch (e) {
      setGErr(String(e));
    } finally {
      setGBusy(false);
    }
  };
  const undoGoogle = async (accountId: string) => {
    setGRowBusy(accountId);
    setGErr(null);
    try {
      await disconnectGoogle(accountId);
      onAccountsChanged();
    } catch (e) {
      setGErr(String(e));
    } finally {
      setGRowBusy(null);
    }
  };
  const doApple = async () => {
    if (!appleId.trim() || !appPw.trim()) {
      setAErr("Apple ID와 앱 암호를 입력하세요.");
      return;
    }
    setABusy(true);
    setAErr(null);
    try {
      await connectApple(appleId.trim(), appPw.trim());
      setAppPw("");
      onAccountsChanged();
    } catch (e) {
      setAErr(String(e));
    } finally {
      setABusy(false);
    }
  };
  const undoApple = async () => {
    setABusy(true);
    try {
      await disconnectApple();
      onAccountsChanged();
    } catch (e) {
      setAErr(String(e));
    } finally {
      setABusy(false);
    }
  };

  return (
    <>
      <section className="sec">
        <h3>
          <GoogleLogo /> Google 캘린더
        </h3>
        {googles.length > 0 && (
          <ul className="acctlist">
            {googles.map((g) => (
              <li key={g.id} className="acctrow">
                <GoogleLogo />
                <span className="acct-label" title={g.label}>
                  {g.label || "Google 계정"}
                  {!g.connected && <span className="acct-warn"> · 다시 로그인 필요</span>}
                </span>
                <button
                  type="button"
                  className="btn btn-outline btn-sm"
                  onClick={() => void undoGoogle(g.id)}
                  disabled={gBusy || gRowBusy !== null}
                >
                  {gRowBusy === g.id ? "해제 중…" : "연결 해제"}
                </button>
              </li>
            ))}
          </ul>
        )}
        <div className="row">
          <button type="button" className={`btn ${googles.length ? "btn-outline" : "btn-primary"}`} onClick={doGoogle} disabled={gBusy}>
            {gBusy ? "브라우저에서 로그인 중…" : googles.length ? "+ Google 계정 추가" : "Google로 로그인"}
          </button>
        </div>
        {googles.length > 0 && <p className="help">여러 Google 계정을 동시에 연결할 수 있습니다. 같은 계정으로 다시 로그인하면 권한이 갱신됩니다.</p>}
        {gErr && <div className="err">{gErr}</div>}
        <button type="button" className="btn btn-link disclosure" onClick={() => setAdvOpen((v) => !v)}>
          {advOpen ? "▾" : "▸"} 고급: OAuth 클라이언트 ID / 시크릿
        </button>
        {advOpen && (
          <div className="adv">
            <p className="help">
              Google Cloud Console → API 및 서비스 → 사용자 인증 정보에서 <b>'데스크톱 앱'</b> 유형의 OAuth 클라이언트를
              만들고, Google Calendar API를 사용 설정한 뒤 값을 입력하세요.
            </p>
            <label className="field">
              <span>클라이언트 ID</span>
              <input
                type="text"
                value={settings.google.clientId}
                onChange={(e) => patch({ google: { ...settings.google, clientId: e.target.value } })}
                placeholder="xxxx.apps.googleusercontent.com"
                spellCheck={false}
              />
            </label>
            <label className="field">
              <span>클라이언트 시크릿</span>
              <input
                type="password"
                value={settings.google.clientSecret}
                onChange={(e) => patch({ google: { ...settings.google, clientSecret: e.target.value } })}
                placeholder="GOCSPX-…"
                spellCheck={false}
              />
            </label>
          </div>
        )}
      </section>

      <section className="sec">
        <h3>
          <AppleLogo /> Apple (iCloud) 캘린더
        </h3>
        <p className="help">
          iCloud 캘린더 연동에는 <b>appleid.apple.com</b> → 로그인 및 보안 → <b>앱 암호</b>에서 발급한 앱 전용 암호가
          필요합니다. (일반 Apple ID 비밀번호는 사용할 수 없습니다.)
        </p>
        {apple?.connected ? (
          <div className="row">
            <span className="acct-label">{apple.label || "연결됨"}</span>
            <button type="button" className="btn btn-outline" onClick={undoApple} disabled={aBusy}>
              연결 해제
            </button>
          </div>
        ) : (
          <>
            <label className="field">
              <span>Apple ID</span>
              <input
                type="email"
                value={appleId}
                onChange={(e) => setAppleId(e.target.value)}
                placeholder="you@icloud.com"
                autoComplete="username"
              />
            </label>
            <label className="field">
              <span>앱 암호</span>
              <input
                type="password"
                value={appPw}
                onChange={(e) => setAppPw(e.target.value)}
                placeholder="xxxx-xxxx-xxxx-xxxx"
                autoComplete="current-password"
                onKeyDown={(e) => e.key === "Enter" && doApple()}
              />
            </label>
            <div className="row">
              <button type="button" className="btn btn-primary" onClick={doApple} disabled={aBusy}>
                {aBusy ? "연결 중…" : "iCloud 연결"}
              </button>
            </div>
          </>
        )}
        {aErr && <div className="err">{aErr}</div>}
      </section>

      <IcsPane onAccountsChanged={onAccountsChanged} />
    </>
  );
}

// ───────────────────────── iCal 구독 ─────────────────────────

function IcsPane({ onAccountsChanged }: { onAccountsChanged: () => void }) {
  const [feeds, setFeeds] = useState<IcsFeed[] | null>(null);
  const [name, setName] = useState("");
  const [url, setUrl] = useState("");
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const reload = async () => {
    try {
      setFeeds(await listIcsFeeds());
    } catch (e) {
      setErr(String(e));
      setFeeds([]);
    }
  };
  useEffect(() => {
    void reload();
  }, []);

  const add = async () => {
    const u = url.trim();
    if (!u) {
      setErr("URL을 입력하세요.");
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      await addIcsFeed(name.trim(), u);
      setName("");
      setUrl("");
      await reload();
      onAccountsChanged();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };
  const remove = async (id: string) => {
    setBusy(true);
    setErr(null);
    try {
      await removeIcsFeed(id);
      await reload();
      onAccountsChanged();
    } catch (e) {
      setErr(String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="sec">
      <h3>
        <FeedIcon /> iCal 구독 (URL)
      </h3>
      <p className="help">
        로그인 없이 캘린더를 <b>읽기 전용</b>으로 볼 수 있습니다. Google 캘린더 → 설정 → 해당 캘린더 →{" "}
        <b>'비공개 주소(iCal 형식)'</b> URL을 복사해 붙여넣으세요. (<code>webcal://</code> 주소도 가능)
      </p>
      {feeds && feeds.length > 0 && (
        <ul className="feedlist">
          {feeds.map((f) => (
            <li key={f.id} className="feedrow">
              <span className="feed-dot" style={{ background: f.color }} />
              <span className="feed-name">{f.name}</span>
              <span className="feed-url" title={f.url}>
                {f.url}
              </span>
              <button type="button" className="btn btn-link btn-sm" onClick={() => void remove(f.id)} disabled={busy}>
                삭제
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="feed-add">
        <input type="text" value={name} onChange={(e) => setName(e.target.value)} placeholder="이름 (예: 공휴일)" className="feed-name-input" />
        <input
          type="url"
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://calendar.google.com/calendar/ical/…/basic.ics"
          spellCheck={false}
          onKeyDown={(e) => e.key === "Enter" && void add()}
        />
        <button type="button" className="btn btn-primary" onClick={() => void add()} disabled={busy}>
          {busy ? "확인 중…" : "추가"}
        </button>
      </div>
      {err && <div className="err">{err}</div>}
    </section>
  );
}

function FeedIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <path d="M4 11a9 9 0 0 1 9 9" />
      <path d="M4 4a16 16 0 0 1 16 16" />
      <circle cx="5" cy="19" r="1.5" fill="currentColor" stroke="none" />
    </svg>
  );
}

// ───────────────────────── 캘린더 ─────────────────────────

function CalendarsPane({
  settings,
  calendars,
  patch,
}: {
  settings: Settings;
  calendars: CalendarInfo[];
  patch: (p: Partial<Settings>) => void;
}) {
  const prefsOf = (id: string): CalendarPrefs => settings.calendars[id] ?? { visible: true, notify: true };
  const setPrefs = (id: string, p: Partial<CalendarPrefs>) =>
    patch({ calendars: { ...settings.calendars, [id]: { ...prefsOf(id), ...p } } });

  const mine = calendars.filter((c) => c.owned);
  const others = calendars.filter((c) => !c.owned);

  if (calendars.length === 0) {
    return <p className="help">연결된 계정이 없거나 아직 동기화되지 않았습니다. 계정 탭에서 로그인 후 동기화하세요.</p>;
  }

  const accountNames = Array.from(new Set(calendars.map((c) => c.account || "")));
  const multiAccount = accountNames.length > 1;
  const accountLabel = (c: CalendarInfo) =>
    c.account || (c.provider === "apple" ? "iCloud" : c.provider === "ics" ? "iCal" : "Google");

  const Rows = ({ list }: { list: CalendarInfo[] }) => (
        <ul className="callist">
          {list.map((c) => {
            const p = prefsOf(c.id);
            const color = c.isHoliday ? HOLIDAY_GREEN : (p.color ?? c.color);
            return (
              <li key={c.id} className="calrow">
                <label className="calcheck" title={p.visible ? "숨기기" : "표시"}>
                  <input type="checkbox" checked={p.visible} onChange={(e) => setPrefs(c.id, { visible: e.target.checked })} />
                  <span className="calbox" style={{ background: p.visible ? color : "transparent", borderColor: color }}>
                    {p.visible && <CheckMark />}
                  </span>
                </label>
                <span className="calname">
                  {c.name}
                  {!c.canEdit && (
                    <span className="cal-ro" title="읽기 전용 (일정 작성 불가)">
                      🔒
                    </span>
                  )}
                </span>
                <label className="swatch" title="색상 변경" style={{ background: color }}>
                  <input type="color" value={toHex6(color)} onChange={(e) => setPrefs(c.id, { color: e.target.value })} />
                </label>
                <button
                  type="button"
                  className={`btn btn-icon bell${p.notify ? " on" : ""}`}
                  onClick={() => setPrefs(c.id, { notify: !p.notify })}
                  title={p.notify ? "알림 받는 중 (클릭하여 끄기)" : "알림 꺼짐 (클릭하여 켜기)"}
                  aria-label="알림 토글"
                >
                  <BellIcon off={!p.notify} />
                </button>
              </li>
            );
          })}
        </ul>
  );

  const Group = ({ title, list }: { title: string; list: CalendarInfo[] }) => {
    if (list.length === 0) return null;
    if (!multiAccount) {
      return (
        <section className="sec">
          <h3>{title}</h3>
          <Rows list={list} />
        </section>
      );
    }
    const byAccount = new Map<string, CalendarInfo[]>();
    for (const c of list) {
      const k = accountLabel(c);
      byAccount.set(k, [...(byAccount.get(k) ?? []), c]);
    }
    return (
      <section className="sec">
        <h3>{title}</h3>
        {Array.from(byAccount.entries()).map(([acct, cals]) => (
          <div key={acct} className="calgroup">
            <div className="calgroup-head" title={acct}>
              {acct}
            </div>
            <Rows list={cals} />
          </div>
        ))}
      </section>
    );
  };

  return (
    <>
      <Group title="내 캘린더" list={mine} />
      <Group title="다른 캘린더" list={others} />
      <p className="help">체크박스 = 표시 여부, 종 아이콘 = 해당 캘린더의 알림 수신 여부입니다.</p>
    </>
  );
}

// ───────────────────────── 알림 ─────────────────────────

const REMINDER_OPTIONS: { v: number; label: string }[] = [
  { v: -1, label: "없음" },
  { v: 0, label: "정각" },
  { v: 5, label: "5분 전" },
  { v: 10, label: "10분 전" },
  { v: 15, label: "15분 전" },
  { v: 30, label: "30분 전" },
  { v: 60, label: "1시간 전" },
];

function NotifyPane({ settings, patch }: { settings: Settings; patch: (p: Partial<Settings>) => void }) {
  return (
    <section className="sec">
      <Toggle
        label="알림 사용"
        desc="일정 시작 시각에 맞춰 Windows 알림을 표시합니다."
        checked={settings.notificationsEnabled}
        onChange={(v) => patch({ notificationsEnabled: v })}
      />
      <label className="field inline">
        <span>기본 알림 시간</span>
        <select
          value={settings.defaultReminderMin}
          onChange={(e) => patch({ defaultReminderMin: Number(e.target.value) })}
          disabled={!settings.notificationsEnabled}
        >
          {REMINDER_OPTIONS.map((o) => (
            <option key={o.v} value={o.v}>
              {o.label}
            </option>
          ))}
        </select>
      </label>
      <p className="help">일정에 알림이 따로 설정되어 있지 않을 때 사용됩니다.</p>
      <label className="field inline">
        <span>종일 일정 알림 시각</span>
        <input
          type="time"
          value={settings.allDayReminderTime}
          onChange={(e) => patch({ allDayReminderTime: e.target.value })}
          disabled={!settings.notificationsEnabled}
        />
      </label>
      <p className="help">캘린더별 알림 켜기/끄기는 캘린더 탭의 종 아이콘으로 설정합니다.</p>
    </section>
  );
}

// ───────────────────────── 표시 ─────────────────────────

const MODES: { v: WindowMode; label: string; desc: string }[] = [
  { v: "floating", label: "일반 창", desc: "작업표시줄에는 표시되지 않고 트레이 아이콘으로 열고 닫습니다." },
  { v: "desktop", label: "바탕화면 위젯", desc: "바탕화면 위에 고정됩니다. 아이콘 위, 다른 창 아래. 조작 가능. Win+D에도 유지." },
  { v: "wallpaper", label: "배경화면", desc: "바탕화면 아이콘 뒤에 표시됩니다 (조작 불가, 보기 전용)." },
];

function DisplayPane({ settings, patch }: { settings: Settings; patch: (p: Partial<Settings>) => void }) {
  const changeMode = (m: WindowMode) => {
    patch({ windowMode: m });
    void setWindowMode(m);
  };
  return (
    <>
      <section className="sec">
        <h3>창 모드</h3>
        <div className="radios">
          {MODES.map((m) => (
            <label key={m.v} className={`radio${settings.windowMode === m.v ? " active" : ""}`}>
              <input type="radio" name="mode" checked={settings.windowMode === m.v} onChange={() => changeMode(m.v)} />
              <span className="radio-label">{m.label}</span>
              <span className="radio-desc">{m.desc}</span>
            </label>
          ))}
        </div>
      </section>
      <section className="sec">
        <label className="field inline">
          <span>배경 불투명도</span>
          <input
            type="range"
            min={0.2}
            max={1}
            step={0.02}
            value={settings.opacity}
            onChange={(e) => patch({ opacity: Number(e.target.value) })}
          />
          <span className="val">{Math.round(settings.opacity * 100)}%</span>
        </label>
        <label className="field inline">
          <span>테마</span>
          <select value={settings.theme} onChange={(e) => patch({ theme: e.target.value as Settings["theme"] })}>
            <option value="system">시스템</option>
            <option value="light">라이트</option>
            <option value="dark">다크</option>
          </select>
        </label>
        <label className="field inline">
          <span>주 시작 요일</span>
          <select value={settings.weekStart} onChange={(e) => patch({ weekStart: Number(e.target.value) as 0 | 1 })}>
            <option value={0}>일요일</option>
            <option value={1}>월요일</option>
          </select>
        </label>
        <label className="field inline">
          <span>동기화 주기</span>
          <select value={settings.syncIntervalMin} onChange={(e) => patch({ syncIntervalMin: Number(e.target.value) })}>
            {[5, 10, 15, 30, 60].map((n) => (
              <option key={n} value={n}>
                {n}분
              </option>
            ))}
          </select>
        </label>
        <Toggle label="시작 시 자동 실행" desc="Windows 로그인 시 트레이에 자동으로 실행됩니다." checked={settings.autostart} onChange={(v) => patch({ autostart: v })} />
      </section>
    </>
  );
}

// ───────────────────────── bits ─────────────────────────

function Toggle({ label, desc, checked, onChange }: { label: string; desc?: string; checked: boolean; onChange: (v: boolean) => void }) {
  return (
    <label className="toggle-row">
      <span className="toggle-text">
        <span className="toggle-label">{label}</span>
        {desc && <span className="toggle-desc">{desc}</span>}
      </span>
      <span className={`switch${checked ? " on" : ""}`}>
        <input type="checkbox" checked={checked} onChange={(e) => onChange(e.target.checked)} />
        <span className="knob" />
      </span>
    </label>
  );
}

function toHex6(c: string): string {
  if (/^#[0-9a-f]{6}$/i.test(c)) return c;
  if (/^#[0-9a-f]{3}$/i.test(c)) return "#" + c.slice(1).split("").map((x) => x + x).join("");
  return "#4285f4";
}

function CheckMark() {
  return (
    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#fff" strokeWidth="3.5" strokeLinecap="round" strokeLinejoin="round">
      <path d="M5 12l5 5L19 7" />
    </svg>
  );
}

function BellIcon({ off }: { off: boolean }) {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M18 8a6 6 0 0 0-12 0c0 7-3 9-3 9h18s-3-2-3-9" />
      <path d="M13.7 21a2 2 0 0 1-3.4 0" />
      {off && <path d="M3 3l18 18" />}
    </svg>
  );
}

function GoogleLogo() {
  return (
    <svg width="16" height="16" viewBox="0 0 48 48">
      <path fill="#FFC107" d="M43.6 20.5H42V20H24v8h11.3C33.7 32.7 29.2 36 24 36c-6.6 0-12-5.4-12-12s5.4-12 12-12c3.1 0 5.8 1.2 7.9 3l5.7-5.7C34 6.1 29.3 4 24 4 13 4 4 13 4 24s9 20 20 20 20-9 20-20c0-1.3-.1-2.4-.4-3.5z" />
      <path fill="#FF3D00" d="M6.3 14.7l6.6 4.8C14.7 15.1 19 12 24 12c3.1 0 5.8 1.2 7.9 3l5.7-5.7C34 6.1 29.3 4 24 4 16.3 4 9.7 8.3 6.3 14.7z" />
      <path fill="#4CAF50" d="M24 44c5.2 0 9.9-2 13.4-5.2l-6.2-5.2C29.2 35.1 26.7 36 24 36c-5.2 0-9.6-3.3-11.3-8l-6.5 5C9.5 39.6 16.2 44 24 44z" />
      <path fill="#1976D2" d="M43.6 20.5H42V20H24v8h11.3c-.8 2.2-2.2 4.2-4.1 5.6l6.2 5.2C36.9 39.2 44 34 44 24c0-1.3-.1-2.4-.4-3.5z" />
    </svg>
  );
}

function AppleLogo() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor">
      <path d="M16.4 12.7c0-2.5 2-3.6 2.1-3.7-1.2-1.7-3-1.9-3.6-2-1.5-.2-3 .9-3.7.9-.8 0-2-.9-3.2-.9-1.7 0-3.2 1-4.1 2.5-1.8 3-.5 7.6 1.3 10.1.9 1.2 1.9 2.6 3.2 2.6 1.3-.1 1.8-.8 3.3-.8s2 .8 3.3.8c1.4 0 2.2-1.3 3.1-2.5 1-1.4 1.4-2.8 1.4-2.9-.1 0-2.7-1-3.1-4.1zM14 5.5c.7-.8 1.1-2 1-3.1-1 0-2.2.7-2.9 1.5-.6.7-1.2 1.9-1 3 1.1.1 2.2-.6 2.9-1.4z" />
    </svg>
  );
}
