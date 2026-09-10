import type { CalEvent, CalendarInfo } from "../lib/types";
import { fmtEventRange } from "../lib/dates";
import { openUrl } from "../lib/api";
import Popover, { type AnchorRect } from "./Popover";
import { HOLIDAY_GREEN } from "./EventChip";

interface Props {
  ev: CalEvent;
  calendar?: CalendarInfo;
  color: string;
  anchor: AnchorRect;
  onClose: () => void;
}

export default function EventDetail({ ev, calendar, color, anchor, onClose }: Props) {
  const bg = calendar?.isHoliday ? HOLIDAY_GREEN : color;
  return (
    <Popover anchor={anchor} onClose={onClose} width={320} className="detail">
      <div className="detail-head">
        <span className="detail-swatch" style={{ background: bg }} />
        <div className="detail-title">{ev.title}</div>
        <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
          ×
        </button>
      </div>
      <div className="detail-row">{fmtEventRange(ev)}</div>
      {calendar && (
        <div className="detail-row detail-muted">
          <span className="detail-dot" style={{ background: bg }} />
          {calendar.name}
        </div>
      )}
      {ev.location && <div className="detail-row">📍 {ev.location}</div>}
      {ev.description && <div className="detail-row detail-desc">{ev.description}</div>}
      {ev.htmlLink && (
        <div className="detail-actions">
          <button type="button" className="btn btn-link" onClick={() => openUrl(ev.htmlLink!)}>
            웹에서 열기 ↗
          </button>
        </div>
      )}
    </Popover>
  );
}
