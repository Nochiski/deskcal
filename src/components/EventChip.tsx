import type { MouseEvent } from "react";
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

export default function EventChip({ ev, color, holiday, bar, contLeft, contRight, onClick, listStyle }: Props) {
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
}
