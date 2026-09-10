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

const calendars: CalendarInfo[] = [
  { id: "google:me", provider: "google", remoteId: "me", name: "한상목", color: "#4285f4", owned: true, isHoliday: false, canEdit: true },
  { id: "google:tasks", provider: "google", remoteId: "tasks", name: "Tasks", color: "#4285f4", owned: true, isHoliday: false, canEdit: true },
  { id: "google:bday", provider: "google", remoteId: "bday", name: "생일", color: "#0b8043", owned: true, isHoliday: false, canEdit: false },
  { id: "google:michelo", provider: "google", remoteId: "michelo", name: "Michelo Robotics", color: "#e67c73", owned: false, isHoliday: false, canEdit: true },
  { id: "google:kr", provider: "google", remoteId: "ko.south_korea#holiday", name: "대한민국의 휴일", color: "#0b8043", owned: false, isHoliday: true, canEdit: false },
];

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
  mk("google:michelo", "🌴 [최성원] 휴가", day(Y, M, 1), day(Y, M, 2)),
  mk("google:michelo", "[미켈로] 전월 법인카드 고위드", day(Y, M, 1), day(Y, M, 2)),
  mk("google:michelo", "[AI사업융합] 협의체 회의", day(Y, M, 2), day(Y, M, 3)),
  mk("google:michelo", "🌴 [윤여상] 휴가", day(Y, M, 2), day(Y, M, 5)),
  mk("google:michelo", "🌴 [이건우] 휴가", day(Y, M, 3), day(Y, M, 4)),
  mk("google:michelo", "🌴 [한상목] 휴가", day(Y, M, 3), day(Y, M, 4)),
  mk("google:bday", "🎂 축 정태준 탄신일", day(Y, M, 3), day(Y, M, 4)),
  mk("google:michelo", "오전 8:30 안영사 출장", day(Y, M, 4), day(Y, M, 6)),
  mk("google:michelo", "[서울형TIPS] 연구성과 입력", iso(Y, M, 4, 9, 0), iso(Y, M, 4, 10, 0), { location: "회의실 A" }),
  mk("google:michelo", "위더스21 정비창고 방문", iso(Y, M, 4, 10, 0), iso(Y, M, 4, 11, 30)),
  mk("google:michelo", "[연구팀] 랩미팅", iso(Y, M, 4, 16, 0), iso(Y, M, 4, 17, 0)),
  mk("google:michelo", "[UR&성원] 하종원", iso(Y, M, 4, 18, 0), iso(Y, M, 4, 19, 0)),
  mk("google:me", "핑거 홍승표대표님 미팅", iso(Y, M, 1, 11, 30), iso(Y, M, 1, 12, 30)),
  mk("google:me", "프론트엔드 팀 미팅", iso(Y, M, 1, 13, 0), iso(Y, M, 1, 14, 0), { description: "주간 진행 상황 공유", htmlLink: "https://calendar.google.com" }),
  mk("google:me", "프론트엔드 팀 미팅", iso(Y, M, 8, 13, 0), iso(Y, M, 8, 14, 0)),
  mk("google:me", "프론트엔드 팀 미팅", iso(Y, M, 15, 13, 0), iso(Y, M, 15, 14, 0)),
  mk("google:me", "[바른창호] 현장", iso(Y, M, 8, 10, 30), iso(Y, M, 8, 12, 0)),
  mk("google:tasks", "건강검진 예약", iso(Y, M, 8, 9, 0), iso(Y, M, 8, 9, 30)),
  mk("google:me", "[신동해] 질의 및 답변", iso(Y, M, 9, 14, 45), iso(Y, M, 9, 15, 30)),
  mk("google:me", "[슈미트] 김현준 미팅", iso(Y, M, 9, 18, 30), iso(Y, M, 9, 19, 30)),
  mk("google:michelo", "제이코어 출장", day(Y, M, 11), day(Y, M, 12)),
  mk("google:michelo", "🌴 [문희진] 휴가", day(Y, M, 11), day(Y, M, 13)),
  mk("google:michelo", "🌴 [남궁민호] 휴가", day(Y, M, 10), day(Y, M, 12)),
  mk("google:michelo", "[국방벤처]1차 정산 처리", day(Y, M, 14), day(Y, M, 15)),
  mk("google:michelo", "[박상용] 건강검진", day(Y, M, 14), day(Y, M, 15)),
  mk("google:michelo", "🌴 [박상용] 휴가", day(Y, M, 14), day(Y, M, 15)),
  mk("google:michelo", "🌴 [장명근] 휴가", day(Y, M, 14), day(Y, M, 15)),
  mk("google:michelo", "[AI가상융합] 격주 회의", iso(Y, M, 15, 10, 0), iso(Y, M, 15, 11, 0)),
  mk("google:michelo", "[경영 지원] 1차 면접", iso(Y, M, 15, 10, 0), iso(Y, M, 15, 12, 0)),
  mk("google:michelo", "[스톤브릿지] 조현우", iso(Y, M, 15, 18, 0), iso(Y, M, 15, 19, 0)),
  mk("google:michelo", "[연구팀] 랩미팅", iso(Y, M, 17, 16, 0), iso(Y, M, 17, 17, 0)),
  mk("google:michelo", "[서울형TIPS]최종보고서 제출", day(Y, M, 18), day(Y, M, 19)),
  mk("google:kr", "추석 연휴", day(Y, M, 24), day(Y, M, 27)),
  mk("google:kr", "추석", day(Y, M, 25), day(Y, M, 26)),
  mk("google:michelo", "[미켈로] 주간 정기", iso(Y, M, 25, 17, 0), iso(Y, M, 25, 18, 0)),
  mk("google:michelo", "[미켈로] 주간 정기", iso(Y, M, 26, 17, 0), iso(Y, M, 26, 18, 0)),
  mk("google:kr", "개천절", day(Y, M + 1, 3), day(Y, M + 1, 4)),
  mk("google:kr", "국군의날", day(Y, M + 1, 1), day(Y, M + 1, 2)),
];

let settings: Settings = {
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

let accounts: AccountInfo[] = [
  { provider: "google", label: "sangmok@example.com", connected: true },
  { provider: "apple", label: "", connected: false },
  { provider: "ics", label: "", connected: false },
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
    accounts = accounts.map((a) => (a.provider === "google" ? { ...a, connected: true, label: "sangmok@example.com" } : a));
    return accounts[0];
  },
  disconnectGoogle: async () => {
    accounts = accounts.map((a) => (a.provider === "google" ? { ...a, connected: false, label: "" } : a));
  },
  connectApple: async (appleId: string, _pw: string) => {
    await delay(800);
    accounts = accounts.map((a) => (a.provider === "apple" ? { ...a, connected: true, label: appleId } : a));
    return accounts[1];
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
