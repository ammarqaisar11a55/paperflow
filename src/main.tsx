import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource-variable/inter";
import "./styles/index.css";
import { App } from "./app/App";

// Apply the cached theme before the first paint to avoid a flash.
try {
  const pref = localStorage.getItem("paperflow.theme");
  const dark = pref === "dark" || (pref !== "light" && window.matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
} catch {
  /* storage unavailable: the settings load sets the theme shortly */
}

// A desktop app has no use for the browser's context menu, except in text fields.
document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement;
  if (!target.closest("input, textarea, .selectable")) e.preventDefault();
});

// `vite dev` in a plain browser tab: answer IPC with Tauri's mock (dev only).
if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
  const preview = await import("./dev/browserPreview");
  preview.installBrowserPreview();
  setTimeout(() => void preview.applyPreviewHash(), 300);
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
