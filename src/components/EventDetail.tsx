import type { CalEvent, CalendarInfo } from "../lib/types";
import { countRender } from "../lib/perf";
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
  /** Opens the editor for this event (only offered when `ev.editable`). */
  onEdit?: () => void;
}

/**
 * Google (and some CalDAV clients) store descriptions as HTML. Render them as plain text with
 * line breaks preserved and entities decoded, instead of showing raw tags.
 */
export function descriptionToText(raw: string): string {
  if (!/<[a-z!/][^>]*>/i.test(raw) && !/&[a-z#0-9]+;/i.test(raw)) return raw;
  const doc = new DOMParser().parseFromString(
    raw
      .replace(/<br\s*\/?>/gi, "\n")
      .replace(/<\/(p|div|li|h[1-6]|tr)>/gi, "\n")
      .replace(/<li[^>]*>/gi, "• "),
    "text/html",
  );
  // Show link targets that differ from their text so nothing is lost.
  doc.querySelectorAll("a[href]").forEach((a) => {
    const href = a.getAttribute("href") ?? "";
    if (href && a.textContent?.trim() !== href) a.textContent = `${a.textContent} (${href})`;
  });
  return (doc.body.textContent ?? "").replace(/\n{3,}/g, "\n\n").trim();
}

export default function EventDetail({ ev, calendar, color, anchor, onClose, onEdit }: Props) {
  countRender("EventDetail");
  const bg = calendar?.isHoliday ? HOLIDAY_GREEN : color;
  const readOnlyNote = ev.editable
    ? null
    : calendar && !calendar.canEdit
      ? "읽기 전용 캘린더"
      : calendar?.provider === "apple"
        ? "반복 일정은 iCloud에서 수정하세요"
        : "수정할 수 없는 일정";
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
      {ev.description && (
        <div className="detail-row detail-desc" style={{ whiteSpace: "pre-wrap" }}>
          {descriptionToText(ev.description)}
        </div>
      )}
      <div className="detail-actions">
        {ev.editable && onEdit ? (
          <button type="button" className="btn btn-outline btn-sm" onClick={onEdit}>
            수정
          </button>
        ) : (
          readOnlyNote && <span className="detail-readonly">🔒 {readOnlyNote}</span>
        )}
        <span className="editor-spacer" />
        {ev.htmlLink && (
          <button type="button" className="btn btn-link" onClick={() => openUrl(ev.htmlLink!)}>
            웹에서 열기 ↗
          </button>
        )}
      </div>
    </Popover>
  );
}
