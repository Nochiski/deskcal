import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./styles.css";

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

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
