import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles/index.css";

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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
