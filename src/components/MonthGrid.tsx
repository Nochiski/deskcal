import { memo, useEffect, useMemo, useRef, useState, type MouseEvent } from "react";
import type { CalEvent, CalendarInfo } from "../lib/types";
import { WEEKDAY_LABELS, dayKey, monthGrid } from "../lib/dates";
import { layoutWeek } from "../lib/layout";
import EventChip from "./EventChip";

const DAY_HEAD_H = 26;
const SLOT_H = 22;
const CELL_PAD_BOTTOM = 4;

interface Props {
  viewMonth: Date;
  weekStart: 0 | 1;
  events: CalEvent[];
  calendars: Map<string, CalendarInfo>;
  colorOf: (calendarId: string) => string;
  onEventClick: (ev: CalEvent, e: MouseEvent<HTMLElement>) => void;
  onMoreClick: (day: Date, e: MouseEvent<HTMLElement>) => void;
  /** Double-click on the empty part of a day cell → create an event on that day. */
  onDayDoubleClick?: (day: Date) => void;
}

export default function MonthGrid({ viewMonth, weekStart, events, calendars, colorOf, onEventClick, onMoreClick, onDayDoubleClick }: Props) {
  const weeks = useMemo(() => monthGrid(viewMonth, weekStart), [viewMonth, weekStart]);
  const weeksRef = useRef<HTMLDivElement>(null);
  const [rowHeight, setRowHeight] = useState(120);

  useEffect(() => {
    const el = weeksRef.current;
    if (!el) return;
    // Only commit a new row height when it really changed (sub-pixel jitter must not re-layout).
    const update = () => {
      const h = el.clientHeight / weeks.length;
      setRowHeight((prev) => (Math.abs(prev - h) < 0.5 ? prev : h));
    };
    update();
    const ro = new ResizeObserver(update);
    ro.observe(el);
    const onResize = () => requestAnimationFrame(update);
    window.addEventListener("resize", onResize);
    return () => {
      ro.disconnect();
      window.removeEventListener("resize", onResize);
    };
  }, [weeks.length]);

  const maxSlots = Math.max(1, Math.floor((rowHeight - DAY_HEAD_H - CELL_PAD_BOTTOM) / SLOT_H));
  const todayKey = dayKey(new Date());
  const labels = weekStart === 1 ? [...WEEKDAY_LABELS.slice(1), WEEKDAY_LABELS[0]] : WEEKDAY_LABELS;

  return (
    <div className="grid">
      <div className="wd-header">
        {labels.map((l, i) => {
          const dow = (i + weekStart) % 7;
          return (
            <div key={l} className={`wd${dow === 0 ? " sun" : dow === 6 ? " sat" : ""}`}>
              {l}
            </div>
          );
        })}
      </div>
      <div className="weeks" ref={weeksRef}>
        {weeks.map((days) => (
          <WeekRow
            key={days[0].getTime()}
            days={days}
            viewMonth={viewMonth}
            todayKey={todayKey}
            events={events}
            calendars={calendars}
            colorOf={colorOf}
            maxSlots={maxSlots}
            onEventClick={onEventClick}
            onMoreClick={onMoreClick}
            onDayDoubleClick={onDayDoubleClick}
          />
        ))}
      </div>
    </div>
  );
}

interface WeekProps {
  days: Date[];
  viewMonth: Date;
  todayKey: string;
  events: CalEvent[];
  calendars: Map<string, CalendarInfo>;
  colorOf: (calendarId: string) => string;
  maxSlots: number;
  onEventClick: (ev: CalEvent, e: MouseEvent<HTMLElement>) => void;
  onMoreClick: (day: Date, e: MouseEvent<HTMLElement>) => void;
  onDayDoubleClick?: (day: Date) => void;
}

// Memoized: a sync that changes nothing in this week leaves its DOM untouched.
const WeekRow = memo(function WeekRow({ days, viewMonth, todayKey, events, calendars, colorOf, maxSlots, onEventClick, onMoreClick, onDayDoubleClick }: WeekProps) {
  const layout = useMemo(() => layoutWeek(events, days[0], maxSlots), [events, days, maxSlots]);

  return (
    <div
      className="week"
      style={{ gridTemplateRows: `${DAY_HEAD_H}px repeat(${maxSlots}, ${SLOT_H}px)` }}
    >
      {days.map((d, c) => {
        const inMonth = d.getMonth() === viewMonth.getMonth();
        const isToday = dayKey(d) === todayKey;
        const dow = d.getDay();
        const label = d.getDate() === 1 ? `${d.getMonth() + 1}월 1일` : String(d.getDate());
        return (
          <div
            key={c}
            className={`cell${inMonth ? "" : " out"}`}
            style={{ gridColumn: c + 1, gridRow: "1 / -1" }}
            onDoubleClick={(e) => {
              if (e.target === e.currentTarget) onDayDoubleClick?.(d);
            }}
            title={onDayDoubleClick ? "더블클릭: 일정 추가" : undefined}
          >
            <button
              type="button"
              className={`daynum${isToday ? " today" : ""}${dow === 0 ? " sun" : dow === 6 ? " sat" : ""}${d.getDate() === 1 ? " first" : ""}`}
              onClick={(e) => onMoreClick(d, e)}
              title="이 날의 일정 보기"
            >
              {label}
            </button>
          </div>
        );
      })}

      {layout.placed
        .filter((p) => p.visible)
        .map((p) => (
          <div
            key={p.ev.id}
            className="chip-cell"
            style={{ gridColumn: `${p.startCol + 1} / ${p.endCol + 2}`, gridRow: p.slot + 2 }}
          >
            <EventChip
              ev={p.ev}
              color={colorOf(p.ev.calendarId)}
              holiday={calendars.get(p.ev.calendarId)?.isHoliday ?? false}
              bar={p.bar}
              contLeft={p.contLeft}
              contRight={p.contRight}
              onClick={onEventClick}
            />
          </div>
        ))}

      {layout.hidden.map((n, c) =>
        n > 0 ? (
          <div key={`more-${c}`} className="chip-cell" style={{ gridColumn: c + 1, gridRow: layout.limit[c] + 2 }}>
            <button type="button" className="more" onClick={(e) => onMoreClick(days[c], e)}>
              {n}개 더보기
            </button>
          </div>
        ) : null,
      )}
    </div>
  );
});
