import { useEffect, useMemo, useRef, useState } from "react";
import { countRender } from "../lib/perf";
import type { CalEvent, CalendarInfo, EventInput, SyncResult } from "../lib/types";
import { createEvent, deleteEvent, updateEvent } from "../lib/api";
import {
  addDays,
  combineDayTime,
  dayKey,
  eventEnd,
  eventStart,
  isAllDayString,
  parseDayKey,
  timeKey,
  toLocalIso,
} from "../lib/dates";
import { HOLIDAY_GREEN } from "./EventChip";

/** Initial values when creating. */
export interface EditorDraft {
  /** Local start; for all-day only the date part matters. */
  start: Date;
  /** Local exclusive end. */
  end: Date;
  allDay: boolean;
  calendarId?: string;
}

interface Props {
  calendars: CalendarInfo[];
  colorOf: (calendarId: string) => string;
  /** Existing event to edit; omit when creating. */
  event?: CalEvent;
  draft?: EditorDraft;
  onSaved: (r: SyncResult) => void;
  onClose: () => void;
}

const REMINDER_CHOICES: { v: number; label: string }[] = [
  { v: 0, label: "정각" },
  { v: 5, label: "5분 전" },
  { v: 10, label: "10분 전" },
  { v: 15, label: "15분 전" },
  { v: 30, label: "30분 전" },
  { v: 60, label: "1시간 전" },
  { v: 1440, label: "1일 전" },
];

function labelOfReminder(m: number): string {
  const hit = REMINDER_CHOICES.find((c) => c.v === m);
  if (hit) return hit.label;
  if (m % 1440 === 0) return `${m / 1440}일 전`;
  if (m % 60 === 0) return `${m / 60}시간 전`;
  return `${m}분 전`;
}

export default function EventEditor({ calendars, colorOf, event, draft, onSaved, onClose }: Props) {
  countRender("EventEditor");
  const editing = !!event;
  const editable = useMemo(() => calendars.filter((c) => c.canEdit), [calendars]);
  /** Editable calendars grouped by owning account (only used when several accounts exist). */
  const accountGroups = useMemo(() => {
    const m = new Map<string, CalendarInfo[]>();
    for (const c of editable) {
      const k = c.account || (c.provider === "apple" ? "iCloud" : "Google");
      m.set(k, [...(m.get(k) ?? []), c]);
    }
    return Array.from(m.entries());
  }, [editable]);

  // ── initial state ──
  const init = useMemo(() => {
    if (event) {
      const allDay = event.allDay || isAllDayString(event.start);
      const s = eventStart(event);
      const e = eventEnd(event);
      return {
        calendarId: event.calendarId,
        title: event.title,
        allDay,
        startDay: dayKey(s),
        startTime: allDay ? "09:00" : timeKey(s),
        // all-day end is exclusive → show inclusive last day in the form
        endDay: allDay ? dayKey(addDays(e, e.getTime() > s.getTime() ? -1 : 0)) : dayKey(e),
        endTime: allDay ? "10:00" : timeKey(e),
        location: event.location ?? "",
        description: event.description ?? "",
        reminders: [...event.reminders],
      };
    }
    const s = draft?.start ?? new Date();
    const e = draft?.end ?? new Date(s.getTime() + 3600_000);
    const allDay = draft?.allDay ?? false;
    const owned = editable.find((c) => c.owned) ?? editable[0];
    return {
      calendarId: draft?.calendarId && editable.some((c) => c.id === draft.calendarId) ? draft.calendarId : (owned?.id ?? ""),
      title: "",
      allDay,
      startDay: dayKey(s),
      startTime: timeKey(s),
      endDay: allDay ? dayKey(addDays(e, e.getTime() > s.getTime() ? -1 : 0)) : dayKey(e),
      endTime: timeKey(e),
      location: "",
      description: "",
      reminders: [] as number[],
    };
  }, [event, draft, editable]);

  const [calendarId, setCalendarId] = useState(init.calendarId);
  const [title, setTitle] = useState(init.title);
  const [allDay, setAllDay] = useState(init.allDay);
  const [startDay, setStartDay] = useState(init.startDay);
  const [startTime, setStartTime] = useState(init.startTime);
  const [endDay, setEndDay] = useState(init.endDay);
  const [endTime, setEndTime] = useState(init.endTime);
  const [location, setLocation] = useState(init.location);
  const [description, setDescription] = useState(init.description);
  const [reminders, setReminders] = useState<number[]>(init.reminders);
  const [reminderPick, setReminderPick] = useState("10");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const titleRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    titleRef.current?.focus();
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);

  // ── derived times ──
  const startDate = useMemo(
    () => (allDay ? parseDayKey(startDay) : combineDayTime(startDay, startTime)),
    [allDay, startDay, startTime],
  );
  const endDate = useMemo(
    () => (allDay ? addDays(parseDayKey(endDay), 1) : combineDayTime(endDay, endTime)),
    [allDay, endDay, endTime],
  );
  const invalidRange = allDay ? endDate.getTime() <= startDate.getTime() : endDate.getTime() <= startDate.getTime();

  /** Change start while keeping the current duration. */
  const shiftStart = (nextDay: string, nextTime: string) => {
    const prevStart = allDay ? parseDayKey(startDay) : combineDayTime(startDay, startTime);
    const prevEnd = allDay ? addDays(parseDayKey(endDay), 1) : combineDayTime(endDay, endTime);
    const dur = Math.max(0, prevEnd.getTime() - prevStart.getTime());
    const ns = allDay ? parseDayKey(nextDay) : combineDayTime(nextDay, nextTime);
    const ne = new Date(ns.getTime() + dur);
    setStartDay(nextDay);
    setStartTime(nextTime);
    if (allDay) {
      setEndDay(dayKey(addDays(ne, dur >= 86_400_000 ? -1 : 0)));
    } else {
      setEndDay(dayKey(ne));
      setEndTime(timeKey(ne));
    }
  };

  const toggleAllDay = (v: boolean) => {
    setAllDay(v);
    if (v) {
      // keep the day span; drop times
      const s = parseDayKey(startDay);
      const e = combineDayTime(endDay, endTime);
      const lastDay = e.getTime() <= s.getTime() ? s : addDays(new Date(e.getFullYear(), e.getMonth(), e.getDate()), e.getHours() === 0 && e.getMinutes() === 0 ? -1 : 0);
      setEndDay(dayKey(lastDay.getTime() < s.getTime() ? s : lastDay));
    } else {
      // give a 1h slot at 09:00 on the start day
      setStartTime("09:00");
      setEndDay(startDay);
      setEndTime("10:00");
    }
  };

  const addReminder = () => {
    const v = Number(reminderPick);
    if (Number.isNaN(v)) return;
    if (!reminders.includes(v)) setReminders([...reminders, v].sort((a, b) => a - b));
  };

  const buildInput = (): EventInput => ({
    calendarId,
    title: title.trim(),
    start: allDay ? dayKey(startDate) : toLocalIso(startDate),
    end: allDay ? dayKey(endDate) : toLocalIso(endDate),
    allDay,
    location: location.trim() || undefined,
    description: description.trim() || undefined,
    reminders,
  });

  const save = async () => {
    setError(null);
    if (!title.trim()) {
      setError("제목을 입력하세요.");
      titleRef.current?.focus();
      return;
    }
    if (!calendarId) {
      setError("일정을 저장할 캘린더를 선택하세요.");
      return;
    }
    if (invalidRange) {
      setError("종료 시각이 시작 시각보다 늦어야 합니다.");
      return;
    }
    setBusy(true);
    try {
      const input = buildInput();
      const r = event ? await updateEvent(event.calendarId, event.remoteId, input) : await createEvent(input);
      onSaved(r);
      onClose();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    if (!event) return;
    setBusy(true);
    setError(null);
    try {
      const r = await deleteEvent(event.calendarId, event.remoteId);
      onSaved(r);
      onClose();
    } catch (e) {
      setError(String(e instanceof Error ? e.message : e));
      setBusy(false);
    }
  };

  const cal = calendars.find((c) => c.id === calendarId);
  const swatch = cal?.isHoliday ? HOLIDAY_GREEN : calendarId ? colorOf(calendarId) : "#999";

  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal editor" role="dialog" aria-label={editing ? "일정 수정" : "일정 추가"}>
        <div className="modal-head">
          <h2>{editing ? "일정 수정" : "일정 추가"}</h2>
          <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
            ×
          </button>
        </div>

        <form
          className="editor-body"
          onSubmit={(e) => {
            e.preventDefault();
            void save();
          }}
        >
          <label className="field">
            <input
              ref={titleRef}
              className="editor-title"
              type="text"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              placeholder="제목 추가"
              maxLength={200}
            />
          </label>

          <label className="field inline">
            <span>캘린더</span>
            <span className="editor-swatch" style={{ background: swatch }} />
            <select value={calendarId} onChange={(e) => setCalendarId(e.target.value)} disabled={editing || editable.length === 0}>
              {editable.length === 0 && <option value="">수정 가능한 캘린더 없음</option>}
              {editing && !editable.some((c) => c.id === calendarId) && cal && <option value={cal.id}>{cal.name}</option>}
              {accountGroups.length > 1
                ? accountGroups.map(([acct, cals]) => (
                    <optgroup key={acct} label={acct}>
                      {cals.map((c) => (
                        <option key={c.id} value={c.id}>
                          {c.name}
                        </option>
                      ))}
                    </optgroup>
                  ))
                : editable.map((c) => (
                    <option key={c.id} value={c.id}>
                      {c.name}
                    </option>
                  ))}
            </select>
          </label>

          <label className="field inline">
            <span>종일</span>
            <span className={`switch${allDay ? " on" : ""}`}>
              <input type="checkbox" checked={allDay} onChange={(e) => toggleAllDay(e.target.checked)} />
              <span className="knob" />
            </span>
          </label>

          <div className="field inline">
            <span>시작</span>
            <div className="editor-when">
              <input type="date" value={startDay} onChange={(e) => e.target.value && shiftStart(e.target.value, startTime)} required />
              {!allDay && (
                <input type="time" value={startTime} step={300} onChange={(e) => e.target.value && shiftStart(startDay, e.target.value)} required />
              )}
            </div>
          </div>

          <div className="field inline">
            <span>종료</span>
            <div className="editor-when">
              <input
                type="date"
                value={endDay}
                min={startDay}
                onChange={(e) => e.target.value && setEndDay(e.target.value)}
                className={invalidRange ? "invalid" : ""}
                required
              />
              {!allDay && (
                <input
                  type="time"
                  value={endTime}
                  step={300}
                  onChange={(e) => e.target.value && setEndTime(e.target.value)}
                  className={invalidRange ? "invalid" : ""}
                  required
                />
              )}
            </div>
          </div>

          <label className="field inline">
            <span>위치</span>
            <input type="text" value={location} onChange={(e) => setLocation(e.target.value)} placeholder="위치 추가" />
          </label>

          <label className="field inline align-top">
            <span>설명</span>
            <textarea value={description} onChange={(e) => setDescription(e.target.value)} placeholder="설명 추가" rows={3} />
          </label>

          <div className="field inline align-top">
            <span>알림</span>
            <div className="editor-reminders">
              <div className="reminder-chips">
                {reminders.length === 0 && <span className="reminder-none">캘린더 기본값</span>}
                {reminders.map((m) => (
                  <span key={m} className="reminder-chip">
                    {labelOfReminder(m)}
                    <button
                      type="button"
                      className="reminder-x"
                      onClick={() => setReminders(reminders.filter((x) => x !== m))}
                      aria-label="알림 제거"
                    >
                      ×
                    </button>
                  </span>
                ))}
              </div>
              <div className="reminder-add">
                <select value={reminderPick} onChange={(e) => setReminderPick(e.target.value)}>
                  {REMINDER_CHOICES.map((c) => (
                    <option key={c.v} value={c.v}>
                      {c.label}
                    </option>
                  ))}
                </select>
                <button type="button" className="btn btn-outline btn-sm" onClick={addReminder}>
                  추가
                </button>
                {reminders.length > 0 && (
                  <button type="button" className="btn btn-link btn-sm" onClick={() => setReminders([])}>
                    없음
                  </button>
                )}
              </div>
            </div>
          </div>

          {error && <div className="err">{error}</div>}

          <div className="editor-actions">
            {editing && !confirmDelete && (
              <button type="button" className="btn btn-danger-outline" onClick={() => setConfirmDelete(true)} disabled={busy}>
                삭제
              </button>
            )}
            {editing && confirmDelete && (
              <span className="editor-confirm">
                <span>정말 삭제할까요?</span>
                <button type="button" className="btn btn-danger" onClick={() => void remove()} disabled={busy}>
                  {busy ? "삭제 중…" : "삭제"}
                </button>
                <button type="button" className="btn btn-outline" onClick={() => setConfirmDelete(false)} disabled={busy}>
                  취소
                </button>
              </span>
            )}
            <span className="editor-spacer" />
            <button type="button" className="btn btn-outline" onClick={onClose} disabled={busy}>
              취소
            </button>
            <button type="submit" className="btn btn-primary" disabled={busy || confirmDelete || editable.length === 0}>
              {busy && !confirmDelete ? "저장 중…" : "저장"}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
}

/** Shown when the user tries to add an event but no writable calendar exists. */
export function NoWritableCalendar({ onSettings, onClose }: { onSettings: () => void; onClose: () => void }) {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [onClose]);
  return (
    <div className="modal-backdrop" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal notice" role="dialog" aria-label="안내">
        <div className="modal-head">
          <h2>일정을 작성할 수 없습니다</h2>
          <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
            ×
          </button>
        </div>
        <div className="notice-body">
          <p>
            일정 작성에는 <b>Google 로그인</b> 또는 <b>iCloud 연결</b>이 필요합니다.
            <br />
            iCal 구독(URL) 캘린더는 읽기 전용입니다.
          </p>
          <div className="editor-actions">
            <span className="editor-spacer" />
            <button type="button" className="btn btn-outline" onClick={onClose}>
              닫기
            </button>
            <button
              type="button"
              className="btn btn-primary"
              onClick={() => {
                onClose();
                onSettings();
              }}
            >
              계정 설정 열기
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
