import { useEffect, useLayoutEffect, useRef, useState, type ReactNode } from "react";
import { countRender } from "../lib/perf";

export interface AnchorRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface Props {
  anchor: AnchorRect;
  onClose: () => void;
  width?: number;
  children: ReactNode;
  className?: string;
}

/** Floating panel anchored beside an element, clamped to the viewport. */
export default function Popover({ anchor, onClose, width = 300, children, className }: Props) {
  countRender("Popover");
  const ref = useRef<HTMLDivElement>(null);
  const [pos, setPos] = useState<{ left: number; top: number }>({ left: -9999, top: -9999 });

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    const margin = 8;
    const vw = window.innerWidth;
    const vh = window.innerHeight;
    const h = el.offsetHeight;
    let left = anchor.left + anchor.width + margin;
    if (left + width > vw - margin) left = anchor.left - width - margin;
    if (left < margin) left = Math.max(margin, Math.min(vw - width - margin, anchor.left));
    let top = anchor.top;
    if (top + h > vh - margin) top = Math.max(margin, vh - h - margin);
    setPos({ left, top });
  }, [anchor, width]);

  useEffect(() => {
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) onClose();
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    // defer so the opening click doesn't immediately close it
    const t = setTimeout(() => {
      document.addEventListener("mousedown", onDown);
      document.addEventListener("keydown", onKey);
    }, 0);
    return () => {
      clearTimeout(t);
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [onClose]);

  return (
    <div
      ref={ref}
      className={`popover ${className ?? ""}`}
      style={{ left: pos.left, top: pos.top, width }}
      role="dialog"
    >
      {children}
    </div>
  );
}
