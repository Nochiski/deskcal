import { useMemo, type MouseEvent } from "react";
import { countRender } from "../lib/perf";
import type { CalEvent, CalendarInfo } from "../lib/types";
import { WEEKDAY_LABELS } from "../lib/dates";
import { eventsOnDay, isBar } from "../lib/layout";
import Popover, { type AnchorRect } from "./Popover";
import EventChip from "./EventChip";

interface Props {
  day: Date;
  events: CalEvent[];
  calendars: Map<string, CalendarInfo>;
  colorOf: (calendarId: string) => string;
  anchor: AnchorRect;
  onClose: () => void;
  onEventClick: (ev: CalEvent, e: MouseEvent<HTMLElement>) => void;
  onAdd?: () => void;
}

export default function DayMore({ day, events, calendars, colorOf, anchor, onClose, onEventClick, onAdd }: Props) {
  countRender("DayMore");
  const list = useMemo(() => eventsOnDay(events, day), [events, day]);
  return (
    <Popover anchor={anchor} onClose={onClose} width={260} className="daymore">
      <div className="daymore-head">
        <div className="daymore-wd">{WEEKDAY_LABELS[day.getDay()]}</div>
        <div className="daymore-num">{day.getDate()}</div>
        <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
          ×
        </button>
      </div>
      <div className="daymore-list">
        {list.map((ev) => (
          <EventChip
            key={ev.id}
            ev={ev}
            color={colorOf(ev.calendarId)}
            holiday={calendars.get(ev.calendarId)?.isHoliday ?? false}
            bar={isBar(ev)}
            onClick={onEventClick}
            listStyle
          />
        ))}
        {list.length === 0 && <div className="daymore-empty">일정 없음</div>}
      </div>
      {onAdd && (
        <button type="button" className="daymore-add" onClick={onAdd}>
          + 일정 추가
        </button>
      )}
    </Popover>
  );
}
