import type { CalEvent } from "./types";
import { addDays, diffDays, eventFirstDay, eventLastDay, eventStart } from "./dates";

export interface PlacedEvent {
  ev: CalEvent;
  /** Column range within the week, inclusive. */
  startCol: number;
  endCol: number;
  /** true if drawn as a spanning bar (all-day or multi-day). */
  bar: boolean;
  /** Continues from the previous week / into the next week. */
  contLeft: boolean;
  contRight: boolean;
  /** Assigned vertical slot (0-based). */
  slot: number;
  /** Visible after overflow calculation. */
  visible: boolean;
}

export interface WeekLayout {
  placed: PlacedEvent[];
  /** Per column: number of hidden events ("N개 더보기"). */
  hidden: number[];
  /** Per column: slot index where the "더보기" link sits (== limit). */
  limit: number[];
}

export function isBar(ev: CalEvent): boolean {
  if (ev.allDay) return true;
  return diffDays(eventFirstDay(ev), eventLastDay(ev)) >= 1;
}

/**
 * Lay out events for one week (7 days from `weekStartDay`).
 * `maxSlots` = how many chip rows fit into a cell; the last one becomes the "더보기" row on overflow.
 */
export function layoutWeek(events: CalEvent[], weekStartDay: Date, maxSlots: number): WeekLayout {
  const weekEndDay = addDays(weekStartDay, 6);

  const items: PlacedEvent[] = [];
  for (const ev of events) {
    const first = eventFirstDay(ev);
    const last = eventLastDay(ev);
    if (last < weekStartDay || first > weekEndDay) continue;
    const sIdx = diffDays(weekStartDay, first);
    const eIdx = diffDays(weekStartDay, last);
    items.push({
      ev,
      startCol: Math.max(0, sIdx),
      endCol: Math.min(6, eIdx),
      bar: isBar(ev),
      contLeft: sIdx < 0,
      contRight: eIdx > 6,
      slot: 0,
      visible: true,
    });
  }

  items.sort((a, b) => {
    if (a.bar !== b.bar) return a.bar ? -1 : 1;
    if (a.bar) {
      const fa = eventFirstDay(a.ev).getTime();
      const fb = eventFirstDay(b.ev).getTime();
      if (fa !== fb) return fa - fb;
      const la = a.endCol - a.startCol;
      const lb = b.endCol - b.startCol;
      if (la !== lb) return lb - la;
      return a.ev.title.localeCompare(b.ev.title, "ko");
    }
    const ta = eventStart(a.ev).getTime();
    const tb = eventStart(b.ev).getTime();
    if (ta !== tb) return ta - tb;
    return a.ev.title.localeCompare(b.ev.title, "ko");
  });

  // first-fit slot assignment
  const occupancy: boolean[][] = [];
  for (const it of items) {
    let slot = 0;
    for (;;) {
      if (!occupancy[slot]) occupancy[slot] = new Array(7).fill(false);
      let free = true;
      for (let c = it.startCol; c <= it.endCol; c++) {
        if (occupancy[slot][c]) {
          free = false;
          break;
        }
      }
      if (free) break;
      slot++;
    }
    for (let c = it.startCol; c <= it.endCol; c++) occupancy[slot][c] = true;
    it.slot = slot;
  }

  // overflow: fixed point on per-column limit
  const max = Math.max(1, maxSlots);
  const limit: number[] = new Array(7).fill(max);
  const hidden: number[] = new Array(7).fill(0);
  for (let iter = 0; iter < 10; iter++) {
    for (const it of items) {
      let vis = true;
      for (let c = it.startCol; c <= it.endCol; c++) {
        if (it.slot >= limit[c]) {
          vis = false;
          break;
        }
      }
      it.visible = vis;
    }
    hidden.fill(0);
    for (const it of items) {
      if (!it.visible) for (let c = it.startCol; c <= it.endCol; c++) hidden[c]++;
    }
    let changed = false;
    for (let c = 0; c < 7; c++) {
      const want = hidden[c] > 0 ? max - 1 : max;
      if (limit[c] !== want) {
        limit[c] = want;
        changed = true;
      }
    }
    if (!changed) break;
  }

  return { placed: items, hidden, limit };
}

/** Events that occupy the given day, sorted like the layout (bars first, then by time). */
export function eventsOnDay(events: CalEvent[], day: Date): CalEvent[] {
  const list = events.filter((ev) => {
    const f = eventFirstDay(ev);
    const l = eventLastDay(ev);
    return f <= day && day <= l;
  });
  list.sort((a, b) => {
    const ba = isBar(a);
    const bb = isBar(b);
    if (ba !== bb) return ba ? -1 : 1;
    const ta = eventStart(a).getTime();
    const tb = eventStart(b).getTime();
    if (ta !== tb) return ta - tb;
    return a.title.localeCompare(b.title, "ko");
  });
  return list;
}
