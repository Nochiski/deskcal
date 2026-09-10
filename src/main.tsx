import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/index.css";
import { PERF, perfDiff, perfSnapshot } from "./lib/perf";

// Disable the WebView2 context menu / text selection outside inputs for an app-like feel.
document.addEventListener("contextmenu", (e) => {
  const t = e.target as HTMLElement | null;
  if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA")) return;
  e.preventDefault();
});

// Browser-only preview: simulate a desktop wallpaper behind the translucent root.
// (Inside Tauri the window itself is transparent/acrylic, so nothing is applied.)
if (!("__TAURI_INTERNALS__" in window)) {
  const wp = new URLSearchParams(location.search).get("wallpaper");
  if (wp) {
    document.body.style.background =
      wp === "dark"
        ? "radial-gradient(1200px 700px at 20% 10%, #3b3f8f 0%, #1c1d3a 45%, #0b0c1a 100%)"
        : "radial-gradient(1100px 650px at 15% 10%, #ffd6a5 0%, #9ad0ec 40%, #5b7dd8 75%, #2b3f8f 100%)";
    document.body.style.padding = "16px";
    document.body.style.boxSizing = "border-box";
  }
}

// Browser-only debugging aid: ?probe=<selector> dumps computed styles of matching elements into
// a <pre id="probe"> so `msedge --headless --dump-dom` can read them.
if (!("__TAURI_INTERNALS__" in window)) {
  const sel = new URLSearchParams(location.search).get("probe");
  if (sel) {
    window.setTimeout(() => {
      const pre = document.createElement("pre");
      pre.id = "probe";
      const props = ["background-color", "background-image", "color", "box-shadow", "border", "opacity", "outline"];
      pre.textContent =
        `viewport ${window.innerWidth}x${window.innerHeight} dpr ${window.devicePixelRatio}\n` +
        [...document.querySelectorAll(sel)]
        .map((el, i) => {
          const cs = getComputedStyle(el);
          return `#${i} <${el.tagName.toLowerCase()} class="${(el as HTMLElement).className}">\n` + props.map((p) => `  ${p}: ${cs.getPropertyValue(p)}`).join("\n");
        })
        .join("\n");
      document.body.appendChild(pre);
    }, 1500);
  }
}

// Browser-only: ?resync=N clicks the 동기화 button N times and reports whether chip DOM nodes
// kept their identity (no re-mount) and whether the grid's layout box moved.
if (!("__TAURI_INTERNALS__" in window)) {
  const n = Number(new URLSearchParams(location.search).get("resync") ?? 0);
  if (n > 0) {
    window.setTimeout(async () => {
      const sleep = (ms: number) => new Promise((r) => window.setTimeout(r, ms));
      const chips = [...document.querySelectorAll<HTMLElement>(".chip")];
      chips.forEach((c) => (c.dataset.mark = "1"));
      const grid = document.querySelector(".grid")!;
      const before = grid.getBoundingClientRect().toJSON();
      const rects = chips.map((c) => c.getBoundingClientRect().toJSON());
      let shifts = 0;
      const ro = new ResizeObserver(() => shifts++);
      chips.forEach((c) => ro.observe(c));
      for (let i = 0; i < n; i++) {
        (document.querySelector<HTMLButtonElement>('[aria-label="동기화"]') ?? { click() {} }).click();
        await sleep(400);
      }
      await sleep(500);
      const after = [...document.querySelectorAll<HTMLElement>(".chip")];
      const kept = after.filter((c) => c.dataset.mark === "1").length;
      const moved = after.filter((c, i) => JSON.stringify(c.getBoundingClientRect().toJSON()) !== JSON.stringify(rects[i])).length;
      const pre = document.createElement("pre");
      pre.id = "resync";
      pre.textContent = `resync x${n}: chips before=${chips.length} after=${after.length} identityKept=${kept} moved=${moved} resizeEvents=${shifts} gridSame=${JSON.stringify(before) === JSON.stringify(grid.getBoundingClientRect().toJSON())}`;
      document.body.appendChild(pre);
    }, 1500);
  }
}

// Browser-only, dev-only: ?perf runs a scripted set of interactions against the mock backend and
// dumps per-component render counts + paint-cost stats into <pre id="perf">. Tree-shaken in prod.
if (import.meta.env.DEV && PERF && !("__TAURI_INTERNALS__" in window)) {
  window.setTimeout(async () => {
    const sleep = (ms: number) => new Promise((r) => window.setTimeout(r, ms));
    const lines: string[] = [];
    const q = <T extends HTMLElement>(s: string) => document.querySelector<T>(s);
    const click = (s: string) => q<HTMLElement>(s)?.click();
    const tab = (label: string) =>
      [...document.querySelectorAll<HTMLButtonElement>("button.tab")].find((b) => b.textContent?.trim() === label)?.click();
    const setNative = (el: HTMLInputElement, value: string) => {
      const proto = Object.getPrototypeOf(el) as object;
      const desc = Object.getOwnPropertyDescriptor(proto, "value");
      desc?.set?.call(el, value);
      el.dispatchEvent(new Event("input", { bubbles: true }));
    };
    const fmt = (o: Record<string, number>) =>
      Object.entries(o)
        .sort(([a], [b]) => a.localeCompare(b))
        .map(([k, v]) => `${k}=${v}`)
        .join(" ");
    const step = async (name: string, fn: () => void | Promise<void>) => {
      const before = perfSnapshot();
      await fn();
      await sleep(900); // longer than the mock's simulated latency so renders attribute correctly
      lines.push(`${name}: ${perfDiff(before, perfSnapshot()) || "(no renders)"}`);
    };

    lines.push(`initial: ${fmt(perfSnapshot())}`);
    await step("sync-unchanged", () => click('[aria-label="동기화"]'));
    await step("sync-changed", async () => {
      const { mock } = await import("./lib/mock");
      const r = await mock.getCached();
      const ev = r.events.find((e) => e.editable);
      if (ev)
        await mock.updateEvent(ev.calendarId, ev.remoteId, {
          calendarId: ev.calendarId,
          title: ev.title + " (perf)",
          start: ev.start,
          end: ev.end,
          allDay: ev.allDay,
          reminders: ev.reminders,
        });
      click('[aria-label="동기화"]');
    });
    await step("nav-next", () => click('[aria-label="다음 달"]'));
    await step("nav-prev", () => click('[aria-label="이전 달"]'));
    await step("popover-open", () => click(".chip"));
    await step("popover-close", () => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    await step("hover-10-chips", () => {
      [...document.querySelectorAll<HTMLElement>(".chip")].slice(0, 10).forEach((c) => {
        c.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        c.dispatchEvent(new MouseEvent("mouseenter"));
        c.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
      });
    });
    await step("settings-open", () => click('[aria-label="설정"]'));
    await step("settings-type-5", async () => {
      tab("계정");
      await sleep(100);
      const input = q<HTMLInputElement>('input[type="email"]');
      if (input) for (const v of ["a", "ab", "abc", "abcd", "abcde"]) setNative(input, v);
    });
    await step("calendar-toggle", async () => {
      tab("캘린더");
      await sleep(100);
      q<HTMLInputElement>(".calrow .bell")?.click();
    });
    await step("opacity-slider-5", async () => {
      tab("표시");
      await sleep(100);
      const range = q<HTMLInputElement>('input[type="range"]');
      if (range) for (const v of ["0.9", "0.85", "0.8", "0.75", "0.7"]) setNative(range, v);
    });
    await step("settings-close", () => {
      document.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    });
    await step("resize-x3", async () => {
      for (let i = 0; i < 3; i++) {
        window.dispatchEvent(new Event("resize"));
        await sleep(60);
      }
    });

    // paint-cost stats
    const all = [...document.querySelectorAll<HTMLElement>("*")];
    // Include ::before/::after so per-chip pseudo-element layers are counted too.
    const css = all.flatMap((el) => [getComputedStyle(el), getComputedStyle(el, "::before"), getComputedStyle(el, "::after")]);
    const real = (c: CSSStyleDeclaration) => c.content === "none" || c.content === "normal" || c.display !== "none";
    const count = (pred: (cs: CSSStyleDeclaration) => boolean) => css.filter((c) => real(c) && pred(c)).length;
    lines.push(
      `paint: elements=${all.length} backdropFilter=${count((c) => c.backdropFilter !== "none" && c.backdropFilter !== "")} filter=${count((c) => c.filter !== "none")} mixBlend=${count((c) => c.mixBlendMode !== "normal")} willChange=${count((c) => c.willChange !== "auto")} boxShadow=${count((c) => c.boxShadow !== "none")} textShadow=${count((c) => c.textShadow !== "none")} transitions=${count((c) => c.transitionProperty !== "all" && c.transitionDuration !== "0s")} animationsRunning=${document.getAnimations().length} chips=${document.querySelectorAll(".chip").length}`,
    );
    const pre = document.createElement("pre");
    pre.id = "perf";
    pre.textContent = lines.join("\n");
    document.body.appendChild(pre);
  }, 1500);
}

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
