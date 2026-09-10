// In-browser mock of the Tauri backend, used when running `vite` outside Tauri.
import type {
  AccountInfo,
  CalEvent,
  CalendarInfo,
  EventInput,
  IcsFeed,
  Settings,
  SyncResult,
  WindowMode,
} from "./types";

const iso = (y: number, m: number, d: number, h = 0, min = 0) => {
  const dt = new Date(y, m - 1, d, h, min);
  const off = -dt.getTimezoneOffset();
  const sign = off >= 0 ? "+" : "-";
  const pad = (n: number) => String(Math.abs(n)).padStart(2, "0");
  return (
    `${y}-${pad(m)}-${pad(d)}T${pad(h)}:${pad(min)}:00` +
    `${sign}${pad(Math.floor(Math.abs(off) / 60))}:${pad(Math.abs(off) % 60)}`
  );
};
const day = (y: number, m: number, d: number) => {
  const dt = new Date(y, m - 1, d);
  return `${dt.getFullYear()}-${String(dt.getMonth() + 1).padStart(2, "0")}-${String(dt.getDate()).padStart(2, "0")}`;
};

const ACCT_A = { id: "a1", email: "me@example.com" };
const ACCT_B = { id: "b2", email: "work@example.com" };

const gcal = (
  acct: { id: string; email: string },
  remoteId: string,
  name: string,
  color: string,
  owned: boolean,
  canEdit: boolean,
  isHoliday = false,
): CalendarInfo => ({
  id: `google:${acct.id}:${remoteId}`,
  provider: "google",
  remoteId,
  name,
  color,
  owned,
  isHoliday,
  canEdit,
  account: acct.email,
});

const calendarsA: CalendarInfo[] = [
  gcal(ACCT_A, "me", "내 캘린더", "#4285f4", true, true),
  gcal(ACCT_A, "tasks", "Tasks", "#4285f4", true, true),
  gcal(ACCT_A, "bday", "생일", "#0b8043", true, false),
  gcal(ACCT_A, "michelo", "Michelo Robotics", "#e67c73", false, true),
  gcal(ACCT_A, "ko.south_korea#holiday", "대한민국의 휴일", "#0b8043", false, false, true),
];
const calendarsB: CalendarInfo[] = [
  gcal(ACCT_B, "work", "업무", "#8e24aa", true, true),
  gcal(ACCT_B, "team", "팀 공유", "#f6bf26", false, true),
];
let calendars: CalendarInfo[] = [...calendarsA, ...calendarsB];

const now = new Date();
const Y = now.getFullYear();
const M = now.getMonth() + 1;

let idc = 0;
const mk = (calendarId: string, title: string, start: string, end: string, extra: Partial<CalEvent> = {}): CalEvent => {
  const remoteId = String(++idc);
  return {
    id: `${calendarId}:${remoteId}:${start}`,
    calendarId,
    remoteId,
    editable: calendars.find((c) => c.id === calendarId)?.canEdit ?? false,
    title,
    start,
    end,
    allDay: start.length === 10,
    reminders: [],
    ...extra,
  };
};

let events: CalEvent[] = [
  mk("google:a1:michelo", "🌴 [최성원] 휴가", day(Y, M, 1), day(Y, M, 2)),
  mk("google:a1:michelo", "[미켈로] 전월 법인카드 고위드", day(Y, M, 1), day(Y, M, 2)),
  mk("google:a1:michelo", "[AI사업융합] 협의체 회의", day(Y, M, 2), day(Y, M, 3)),
  mk("google:a1:michelo", "🌴 [윤여상] 휴가", day(Y, M, 2), day(Y, M, 5)),
  mk("google:a1:michelo", "🌴 [이건우] 휴가", day(Y, M, 3), day(Y, M, 4)),
  mk("google:a1:michelo", "🌴 [홍길동] 휴가", day(Y, M, 3), day(Y, M, 4)),
  mk("google:a1:bday", "🎂 축 정태준 탄신일", day(Y, M, 3), day(Y, M, 4)),
  mk("google:a1:michelo", "오전 8:30 안영사 출장", day(Y, M, 4), day(Y, M, 6)),
  mk("google:a1:michelo", "[서울형TIPS] 연구성과 입력", iso(Y, M, 4, 9, 0), iso(Y, M, 4, 10, 0), { location: "회의실 A" }),
  mk("google:a1:michelo", "위더스21 정비창고 방문", iso(Y, M, 4, 10, 0), iso(Y, M, 4, 11, 30)),
  mk("google:a1:michelo", "[연구팀] 랩미팅", iso(Y, M, 4, 16, 0), iso(Y, M, 4, 17, 0)),
  mk("google:a1:michelo", "[UR&성원] 하종원", iso(Y, M, 4, 18, 0), iso(Y, M, 4, 19, 0)),
  mk("google:a1:me", "핑거 홍승표대표님 미팅", iso(Y, M, 1, 11, 30), iso(Y, M, 1, 12, 30)),
  mk("google:a1:me", "프론트엔드 팀 미팅", iso(Y, M, 1, 13, 0), iso(Y, M, 1, 14, 0), { description: "주간 진행 상황 공유", htmlLink: "https://calendar.google.com" }),
  mk("google:a1:me", "프론트엔드 팀 미팅", iso(Y, M, 8, 13, 0), iso(Y, M, 8, 14, 0)),
  mk("google:a1:me", "프론트엔드 팀 미팅", iso(Y, M, 15, 13, 0), iso(Y, M, 15, 14, 0)),
  mk("google:a1:me", "[바른창호] 현장", iso(Y, M, 8, 10, 30), iso(Y, M, 8, 12, 0)),
  mk("google:a1:tasks", "건강검진 예약", iso(Y, M, 8, 9, 0), iso(Y, M, 8, 9, 30)),
  mk("google:a1:me", "[신동해] 질의 및 답변", iso(Y, M, 9, 14, 45), iso(Y, M, 9, 15, 30)),
  mk("google:a1:me", "[슈미트] 김현준 미팅", iso(Y, M, 9, 18, 30), iso(Y, M, 9, 19, 30)),
  mk("google:a1:michelo", "제이코어 출장", day(Y, M, 11), day(Y, M, 12)),
  mk("google:a1:michelo", "🌴 [문희진] 휴가", day(Y, M, 11), day(Y, M, 13)),
  mk("google:a1:michelo", "🌴 [남궁민호] 휴가", day(Y, M, 10), day(Y, M, 12)),
  mk("google:a1:michelo", "[국방벤처]1차 정산 처리", day(Y, M, 14), day(Y, M, 15)),
  mk("google:a1:michelo", "[박상용] 건강검진", day(Y, M, 14), day(Y, M, 15)),
  mk("google:a1:michelo", "🌴 [박상용] 휴가", day(Y, M, 14), day(Y, M, 15)),
  mk("google:a1:michelo", "🌴 [장명근] 휴가", day(Y, M, 14), day(Y, M, 15)),
  mk("google:a1:michelo", "[AI가상융합] 격주 회의", iso(Y, M, 15, 10, 0), iso(Y, M, 15, 11, 0)),
  mk("google:a1:michelo", "[경영 지원] 1차 면접", iso(Y, M, 15, 10, 0), iso(Y, M, 15, 12, 0)),
  mk("google:a1:michelo", "[스톤브릿지] 조현우", iso(Y, M, 15, 18, 0), iso(Y, M, 15, 19, 0)),
  mk("google:a1:michelo", "[연구팀] 랩미팅", iso(Y, M, 17, 16, 0), iso(Y, M, 17, 17, 0)),
  mk("google:a1:michelo", "[서울형TIPS]최종보고서 제출", day(Y, M, 18), day(Y, M, 19)),
  mk("google:a1:ko.south_korea#holiday", "추석 연휴", day(Y, M, 24), day(Y, M, 27)),
  mk("google:a1:ko.south_korea#holiday", "추석", day(Y, M, 25), day(Y, M, 26)),
  mk("google:a1:michelo", "[미켈로] 주간 정기", iso(Y, M, 25, 17, 0), iso(Y, M, 25, 18, 0)),
  mk("google:a1:michelo", "[미켈로] 주간 정기", iso(Y, M, 26, 17, 0), iso(Y, M, 26, 18, 0)),
  mk("google:a1:ko.south_korea#holiday", "개천절", day(Y, M + 1, 3), day(Y, M + 1, 4)),
  mk("google:a1:ko.south_korea#holiday", "국군의날", day(Y, M + 1, 1), day(Y, M + 1, 2)),
  mk("google:b2:work", "주간 업무 보고", iso(Y, M, 2, 9, 30), iso(Y, M, 2, 10, 0)),
  mk("google:b2:work", "분기 리뷰", iso(Y, M, 16, 14, 0), iso(Y, M, 16, 15, 30), { location: "본사 12층" }),
  mk("google:b2:team", "팀 워크숍", day(Y, M, 19), day(Y, M, 20)),
];

let settings: Settings = {
  windowMode: "floating",
  opacity: 0.96,
  theme: "system",
  style: "glass",
  weekStart: 0,
  syncIntervalMin: 15,
  autostart: false,
  autostartVisible: true,
  defaultReminderMin: 10,
  notificationsEnabled: true,
  allDayReminderTime: "09:00",
  google: { clientId: "", clientSecret: "" },
  calendars: {},
};

let accounts: AccountInfo[] = [
  { provider: "google", id: ACCT_A.id, label: ACCT_A.email, connected: true },
  { provider: "google", id: ACCT_B.id, label: ACCT_B.email, connected: true },
  { provider: "apple", id: "apple", label: "", connected: false },
  { provider: "ics", id: "ics", label: "", connected: false },
];

let feeds: IcsFeed[] = [];
const feedCalendars = (): CalendarInfo[] =>
  feeds.map((f) => ({
    id: `ics:${f.id}`,
    provider: "ics",
    remoteId: f.id,
    name: f.name,
    color: f.color,
    owned: false,
    isHoliday: false,
    canEdit: false,
    account: "iCal",
  }));
const syncIcsAccount = () => {
  accounts = accounts.map((a) =>
    a.provider === "ics" ? { ...a, connected: feeds.length > 0, label: feeds.length ? `${feeds.length}개 구독` : "" } : a,
  );
};

const delay = (ms: number) => new Promise((r) => setTimeout(r, ms));

const loggedIn = () => accounts.some((a) => a.provider !== "ics" && a.connected);
const result = (): SyncResult => ({
  calendars: [...(loggedIn() ? calendars : []), ...feedCalendars()],
  events: loggedIn() ? events : [],
  syncedAt: new Date().toISOString(),
  errors: [],
  rangeStart: day(Y, M - 1, 1),
  rangeEnd: day(Y, M + 2, 1),
});

export const mock = {
  getSettings: async () => settings,
  saveSettings: async (s: Settings) => {
    settings = s;
  },
  getAccounts: async () => accounts,
  connectGoogle: async () => {
    await delay(800);
    // Link whichever mock account is missing (A first, then B); re-linking refreshes.
    const missing = [ACCT_A, ACCT_B].find((a) => !accounts.some((x) => x.provider === "google" && x.id === a.id));
    const acct = missing ?? ACCT_A;
    const info: AccountInfo = { provider: "google", id: acct.id, label: acct.email, connected: true };
    if (missing) {
      accounts = [...accounts.filter((a) => a.provider !== "google"), ...accounts.filter((a) => a.provider === "google"), info]
        .sort((a, b) => (a.provider === b.provider ? 0 : a.provider === "google" ? -1 : b.provider === "google" ? 1 : 0));
      calendars = [...calendars, ...(acct.id === ACCT_A.id ? calendarsA : calendarsB)];
    } else {
      accounts = accounts.map((a) => (a.id === acct.id ? info : a));
    }
    return info;
  },
  disconnectGoogle: async (accountId: string) => {
    accounts = accounts.filter((a) => !(a.provider === "google" && a.id === accountId));
    const prefix = `google:${accountId}:`;
    calendars = calendars.filter((c) => !c.id.startsWith(prefix));
    events = events.filter((e) => !e.calendarId.startsWith(prefix));
  },
  connectApple: async (appleId: string, _pw: string) => {
    await delay(800);
    accounts = accounts.map((a) => (a.provider === "apple" ? { ...a, connected: true, label: appleId } : a));
    return accounts.find((a) => a.provider === "apple")!;
  },
  disconnectApple: async () => {
    accounts = accounts.map((a) => (a.provider === "apple" ? { ...a, connected: false, label: "" } : a));
  },
  getCached: async () => result(),
  syncNow: async (_s: string, _e: string) => {
    await delay(600);
    return result();
  },
  setWindowMode: async (_m: WindowMode) => {},
  hideWindow: async () => {},
  openUrl: async (url: string) => {
    window.open(url, "_blank");
  },
  noopListen: async () => () => {},

  // ── write ops ──
  createEvent: async (input: EventInput) => {
    await delay(300);
    if (!input.title.trim()) throw new Error("제목을 입력하세요.");
    events = [
      ...events,
      mk(input.calendarId, input.title, input.start, input.end, {
        location: input.location,
        description: input.description,
        reminders: input.reminders,
      }),
    ];
    return result();
  },
  updateEvent: async (calendarId: string, remoteId: string, input: EventInput) => {
    await delay(300);
    events = events.map((e) =>
      e.calendarId === calendarId && e.remoteId === remoteId
        ? {
            ...e,
            id: `${calendarId}:${remoteId}:${input.start}`,
            title: input.title,
            start: input.start,
            end: input.end,
            allDay: input.allDay,
            location: input.location,
            description: input.description,
            reminders: input.reminders,
          }
        : e,
    );
    return result();
  },
  deleteEvent: async (calendarId: string, remoteId: string) => {
    await delay(300);
    events = events.filter((e) => !(e.calendarId === calendarId && e.remoteId === remoteId));
    return result();
  },

  // ── ics feeds ──
  listIcsFeeds: async () => feeds,
  addIcsFeed: async (name: string, url: string) => {
    await delay(500);
    if (!/^(https?|webcal):\/\//i.test(url)) throw new Error("올바른 URL이 아닙니다. (http(s):// 또는 webcal://)");
    const f: IcsFeed = { id: String(Date.now()), name: name || "iCal 구독", url, color: "#7986cb" };
    feeds = [...feeds, f];
    syncIcsAccount();
    return f;
  },
  removeIcsFeed: async (id: string) => {
    feeds = feeds.filter((f) => f.id !== id);
    syncIcsAccount();
  },
};
