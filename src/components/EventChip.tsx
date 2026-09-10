import { memo, type MouseEvent } from "react";
import { countRender } from "../lib/perf";
import type { CalEvent } from "../lib/types";
import { eventStart, fmtTimeShort } from "../lib/dates";

interface Props {
  ev: CalEvent;
  color: string;
  holiday: boolean;
  bar: boolean;
  contLeft?: boolean;
  contRight?: boolean;
  onClick: (ev: CalEvent, e: MouseEvent<HTMLElement>) => void;
  /** Full-width list style (used in popovers). */
  listStyle?: boolean;
}

export const HOLIDAY_GREEN = "#0b8043";

// Memoized: a week row that re-lays out (e.g. slot count changed) only re-renders the chips whose
// event / colour / placement actually changed.
const EventChip = memo(function EventChip({ ev, color, holiday, bar, contLeft, contRight, onClick, listStyle }: Props) {
  countRender("EventChip");
  const bg = holiday ? HOLIDAY_GREEN : color;
  if (bar) {
    return (
      <button
        type="button"
        className={`chip chip-bar${contLeft ? " cont-l" : ""}${contRight ? " cont-r" : ""}${listStyle ? " chip-list" : ""}`}
        style={{ background: bg }}
        title={ev.title}
        onClick={(e) => onClick(ev, e)}
      >
        <span className="chip-title">{ev.title}</span>
      </button>
    );
  }
  const t = fmtTimeShort(eventStart(ev));
  return (
    <button
      type="button"
      className={`chip chip-timed${listStyle ? " chip-list" : ""}`}
      title={`${t} ${ev.title}`}
      onClick={(e) => onClick(ev, e)}
    >
      <span className="chip-dot" style={{ background: bg }} />
      <span className="chip-time">{t}</span>
      <span className="chip-title">{ev.title}</span>
    </button>
  );
});
export default EventChip;
