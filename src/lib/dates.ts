import type { CalEvent } from "./types";

/** "YYYY-MM-DD" in local time. */
export function dayKey(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

/** Parse "YYYY-MM-DD" as a local midnight Date. */
export function parseDayKey(key: string): Date {
  const [y, m, d] = key.slice(0, 10).split("-").map(Number);
  return new Date(y, m - 1, d);
}

export function startOfDay(d: Date): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate());
}

export function addDays(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth(), d.getDate() + n);
}

export function addMonths(d: Date, n: number): Date {
  return new Date(d.getFullYear(), d.getMonth() + n, 1);
}

export function isSameDay(a: Date, b: Date): boolean {
  return (
    a.getFullYear() === b.getFullYear() &&
    a.getMonth() === b.getMonth() &&
    a.getDate() === b.getDate()
  );
}

/** Days between two local-midnight dates (b - a). */
export function diffDays(a: Date, b: Date): number {
  const ms = startOfDay(b).getTime() - startOfDay(a).getTime();
  return Math.round(ms / 86_400_000);
}

export function isAllDayString(s: string): boolean {
  return s.length === 10;
}

/** Event start as a local Date (all-day → local midnight). */
export function eventStart(ev: CalEvent): Date {
  return ev.allDay || isAllDayString(ev.start) ? parseDayKey(ev.start) : new Date(ev.start);
}

/** Event end (exclusive) as a local Date. */
export function eventEnd(ev: CalEvent): Date {
  if (ev.allDay || isAllDayString(ev.end)) return parseDayKey(ev.end);
  const e = new Date(ev.end);
  return isNaN(e.getTime()) ? eventStart(ev) : e;
}

/** First day (local midnight) the event occupies. */
export function eventFirstDay(ev: CalEvent): Date {
  return startOfDay(eventStart(ev));
}

/**
 * Last day (local midnight, inclusive) the event occupies.
 * For an exclusive end exactly at midnight the previous day is the last day.
 */
export function eventLastDay(ev: CalEvent): Date {
  const s = eventFirstDay(ev);
  const e = eventEnd(ev);
  if (e.getTime() <= s.getTime()) return s;
  const eDay = startOfDay(e);
  if (eDay.getTime() === e.getTime()) {
    const prev = addDays(eDay, -1);
    return prev.getTime() < s.getTime() ? s : prev;
  }
  return eDay;
}

/** "오전 9시" / "오후 1:30" style. */
export function fmtTimeShort(d: Date): string {
  const h = d.getHours();
  const m = d.getMinutes();
  const ampm = h < 12 ? "오전" : "오후";
  const hh = h % 12 || 12;
  return m === 0 ? `${ampm} ${hh}시` : `${ampm} ${hh}:${String(m).padStart(2, "0")}`;
}

/** "오전 9:00" style (always with minutes). */
export function fmtTimeFull(d: Date): string {
  const h = d.getHours();
  const m = d.getMinutes();
  const ampm = h < 12 ? "오전" : "오후";
  const hh = h % 12 || 12;
  return `${ampm} ${hh}:${String(m).padStart(2, "0")}`;
}

export const WEEKDAY_LABELS = ["일", "월", "화", "수", "목", "금", "토"];

/** "9월 1일 (화)" */
export function fmtDateKo(d: Date, withYear = false): string {
  const base = `${d.getMonth() + 1}월 ${d.getDate()}일 (${WEEKDAY_LABELS[d.getDay()]})`;
  return withYear ? `${d.getFullYear()}년 ${base}` : base;
}

export function fmtMonthTitle(d: Date): string {
  return `${d.getFullYear()}년 ${d.getMonth() + 1}월`;
}

/** Human readable range for the detail popover. */
export function fmtEventRange(ev: CalEvent): string {
  const s = eventStart(ev);
  if (ev.allDay || isAllDayString(ev.start)) {
    const last = eventLastDay(ev);
    if (isSameDay(s, last)) return fmtDateKo(s);
    return `${fmtDateKo(s)} – ${fmtDateKo(last)}`;
  }
  const e = eventEnd(ev);
  if (isSameDay(s, e) || e.getTime() <= s.getTime()) {
    return `${fmtDateKo(s)} · ${fmtTimeFull(s)} – ${fmtTimeFull(e)}`;
  }
  return `${fmtDateKo(s)} ${fmtTimeFull(s)} – ${fmtDateKo(e)} ${fmtTimeFull(e)}`;
}

export function fmtSyncedAt(iso: string): string {
  const d = new Date(iso);
  if (isNaN(d.getTime())) return "";
  return `${d.getMonth() + 1}/${d.getDate()} ${fmtTimeFull(d)}`;
}

/**
 * Compute the grid of weeks for a month view.
 * Returns an array of weeks, each week an array of 7 local-midnight Dates.
 */
export function monthGrid(viewMonth: Date, weekStart: 0 | 1): Date[][] {
  const first = new Date(viewMonth.getFullYear(), viewMonth.getMonth(), 1);
  const lastOfMonth = new Date(viewMonth.getFullYear(), viewMonth.getMonth() + 1, 0);
  const offset = (first.getDay() - weekStart + 7) % 7;
  const gridStart = addDays(first, -offset);
  const totalDays = offset + lastOfMonth.getDate();
  const weeks = Math.ceil(totalDays / 7);
  const rows: Date[][] = [];
  for (let w = 0; w < weeks; w++) {
    const row: Date[] = [];
    for (let i = 0; i < 7; i++) row.push(addDays(gridStart, w * 7 + i));
    rows.push(row);
  }
  return rows;
}
