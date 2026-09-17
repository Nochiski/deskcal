import { useRef, useState } from "react";
import type { CalEvent, CalendarInfo, InvitationResponse, ResponseStatus } from "../lib/types";
import { countRender } from "../lib/perf";
import { fmtEventRange } from "../lib/dates";
import { openUrl, respondEvent } from "../lib/api";
import { RESPONSE_ICON, RESPONSE_LABEL } from "../lib/invitations";
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
  onResponded: (ev: CalEvent) => void;
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

export default function EventDetail({ ev, calendar, color, anchor, onClose, onEdit, onResponded }: Props) {
  countRender("EventDetail");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState("");
  const pending = useRef(false);
  const attendees = ev.attendees ?? [];
  const ownAttendee = attendees.find((a) => a.isSelf);
  const counts = attendees.reduce((all, a) => {
    all[a.responseStatus]++;
    return all;
  }, { accepted: 0, declined: 0, tentative: 0, needsAction: 0 });
  const respond = async (status: InvitationResponse) => {
    if (pending.current) return;
    pending.current = true;
    setSaving(true);
    setError(null);
    setNotice("");
    try {
      const updated = await respondEvent(ev.calendarId, ev.remoteId, status);
      onResponded(updated);
      setNotice(`참석 여부를 변경했습니다: ${RESPONSE_LABEL[status]}`);
    } catch (e) {
      setError(String(e));
    } finally {
      pending.current = false;
      setSaving(false);
    }
  };
  const bg = calendar?.isHoliday ? HOLIDAY_GREEN : color;
  const readOnlyNote = ev.editable
    ? null
    : calendar && !calendar.canEdit
      ? "읽기 전용 캘린더"
      : calendar?.provider === "apple"
        ? "반복 일정은 iCloud에서 수정하세요"
        : ev.canRespond
          ? "일정 내용은 주최자만 수정할 수 있습니다"
          : "수정할 수 없는 일정";
  return (
    <Popover anchor={anchor} onClose={onClose} width={380} className="detail">
      <div className="detail-head">
        <span className="detail-swatch" style={{ background: bg }} />
        <div className="detail-title">{ev.title}</div>
        <button type="button" className="btn btn-icon btn-close" onClick={onClose} aria-label="닫기">
          ×
        </button>
      </div>
      <div className="detail-content">
        <div className="detail-row">{fmtEventRange(ev)}</div>
        {calendar && (
          <div className="detail-row detail-muted">
            <span className="detail-dot" style={{ background: bg }} />
            {calendar.name}
          </div>
        )}
        {ev.location && <div className="detail-row">📍 {ev.location}</div>}
        {ev.organizer && (
          <div className="detail-row detail-organizer">
            <span className="detail-muted">주최자</span>
            <span title={ev.organizer.email ?? undefined}>
              {ev.organizer.displayName || ev.organizer.email || "이름 없는 주최자"}
              {ev.organizer.displayName && ev.organizer.email && (
                <span className="detail-person-email">{ev.organizer.email}</span>
              )}
            </span>
          </div>
        )}
        {(attendees.length > 0 || ev.attendeesOmitted) && (
          <section className="detail-guests" aria-label="참석자">
            <div className="detail-guests-title">
              {ev.attendeesOmitted ? "공개된 참석자" : "참석자"} {attendees.length}명
            </div>
            <div className="detail-guest-summary">
              {(Object.keys(counts) as ResponseStatus[]).filter((s) => counts[s] > 0)
                .map((s) => `${RESPONSE_LABEL[s]} ${counts[s]}명`).join(" · ")}
            </div>
            <ul className="detail-guest-list">
              {attendees.map((a, i) => (
                <li className="detail-guest" key={`${a.email ?? "guest"}:${i}`}>
                  <span className={`detail-response-icon response-${a.responseStatus}`} aria-hidden="true">
                    {RESPONSE_ICON[a.responseStatus]}
                  </span>
                  <div className="detail-person">
                    <div className="detail-person-name">{a.displayName || a.email || "이름 없는 참석자"}</div>
                    {a.displayName && a.email && <div className="detail-person-email">{a.email}</div>}
                    {(a.organizer || a.isSelf || a.optional) && (
                      <div className="detail-person-email">
                        {[a.organizer && "주최자", a.isSelf && (a.email?.toLowerCase() === calendar?.account.toLowerCase() ? "나" : "응답 대상"), a.optional && "선택 참석"].filter(Boolean).join(" · ")}
                      </div>
                    )}
                  </div>
                  <span className="detail-guest-status">{RESPONSE_LABEL[a.responseStatus]}</span>
                </li>
              ))}
            </ul>
            {ev.attendeesOmitted && <p className="detail-guest-summary">Google에서 공개한 참석자만 표시됩니다.</p>}
          </section>
        )}
        {ev.description && (
          <div className="detail-row detail-desc" style={{ whiteSpace: "pre-wrap" }}>
            {descriptionToText(ev.description)}
          </div>
        )}
      </div>
      {ev.responseStatus && (
        <div className="detail-rsvp" aria-busy={saving}>
          <div className="detail-rsvp-heading">
            <span>참석 여부</span>
            <span className="detail-muted">{RESPONSE_LABEL[ev.responseStatus]}</span>
          </div>
          {ev.canRespond && <>
            {ownAttendee?.email && <div className="detail-person-email">{ownAttendee.email}</div>}
            {ev.recurring && <div className="detail-guest-summary">반복 일정의 이 날짜에만 적용됩니다.</div>}
            <div className="detail-rsvp-buttons" role="group" aria-label="참석 여부 선택">
              {(["accepted", "declined", "tentative"] as const).map((status) => (
                <button type="button" key={status}
                  className={`btn btn-sm ${ev.responseStatus === status ? "btn-primary" : "btn-outline"}`}
                  aria-pressed={ev.responseStatus === status}
                  disabled={saving || ev.responseStatus === status}
                  onClick={() => void respond(status)}>
                  {RESPONSE_LABEL[status]}
                </button>
              ))}
            </div>
          </>}
          <div className="detail-rsvp-notice" role="status">{saving ? "응답을 보내는 중…" : notice}</div>
          {error && <div className="detail-rsvp-error" role="alert">{error}</div>}
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
