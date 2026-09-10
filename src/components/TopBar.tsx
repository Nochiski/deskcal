import { memo } from "react";
import { fmtMonthTitle, fmtSyncedAt } from "../lib/dates";
import { countRender } from "../lib/perf";

interface Props {
  viewMonth: Date;
  syncing: boolean;
  syncedAt: string | null;
  compact: boolean;
  onPrev: () => void;
  onNext: () => void;
  onToday: () => void;
  onSync: () => void;
  onSettings: () => void;
  onHide: () => void;
  onAdd: () => void;
}

// Memoized: only month, sync state or window mode changes re-render the toolbar.
const TopBar = memo(function TopBar(p: Props) {
  countRender("TopBar");
  const syncTitle = p.syncedAt ? `마지막 동기화: ${fmtSyncedAt(p.syncedAt)}` : "동기화";
  return (
    <header className="topbar" data-tauri-drag-region>
      <div className="topbar-left" data-tauri-drag-region>
        <button type="button" className="btn btn-today" onClick={p.onToday}>
          오늘
        </button>
        <button type="button" className="btn btn-icon" onClick={p.onPrev} aria-label="이전 달" title="이전 달">
          <Chevron dir="left" />
        </button>
        <button type="button" className="btn btn-icon" onClick={p.onNext} aria-label="다음 달" title="다음 달">
          <Chevron dir="right" />
        </button>
        <h1 className="topbar-title" data-tauri-drag-region>
          {fmtMonthTitle(p.viewMonth)}
        </h1>
      </div>
      <div className="topbar-right">
        {p.compact ? (
          <button type="button" className="btn btn-icon" onClick={p.onAdd} aria-label="일정 추가" title="일정 추가">
            <PlusIcon />
          </button>
        ) : (
          <button type="button" className="btn btn-add" onClick={p.onAdd} title="일정 추가">
            <PlusIcon />
            <span>일정 추가</span>
          </button>
        )}
        <button
          type="button"
          className={`btn btn-icon${p.syncing ? " spinning" : ""}`}
          onClick={p.onSync}
          disabled={p.syncing}
          aria-label="동기화"
          title={syncTitle}
        >
          <SyncIcon />
        </button>
        <button type="button" className="btn btn-icon" onClick={p.onSettings} aria-label="설정" title="설정">
          <GearIcon />
        </button>
        {!p.compact && (
          <button type="button" className="btn btn-icon" onClick={p.onHide} aria-label="숨기기" title="트레이로 숨기기">
            <MinimizeIcon />
          </button>
        )}
      </div>
    </header>
  );
});
export default TopBar;

function Chevron({ dir }: { dir: "left" | "right" }) {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round">
      {dir === "left" ? <path d="M15 6l-6 6 6 6" /> : <path d="M9 6l6 6-6 6" />}
    </svg>
  );
}

function SyncIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M21 12a9 9 0 0 1-15.5 6.2L3 16" />
      <path d="M3 12a9 9 0 0 1 15.5-6.2L21 8" />
      <path d="M21 3v5h-5" />
      <path d="M3 21v-5h5" />
    </svg>
  );
}

function GearIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" />
    </svg>
  );
}

function PlusIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

function MinimizeIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round">
      <path d="M5 12h14" />
    </svg>
  );
}
