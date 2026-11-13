import { useEffect } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { useDrop } from "@/stores/drop";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import { resolveTheme, useSettings } from "@/stores/settings";
import { inTauri } from "./useWindowState";

/** Applies the theme preference to <html>, following the OS when "system". */
export function useThemeEffect() {
  const theme = useSettings((s) => s.settings.theme);
  useEffect(() => {
    const media = window.matchMedia?.("(prefers-color-scheme: dark)");
    const apply = () => {
      document.documentElement.dataset.theme = resolveTheme(theme, media?.matches ?? false);
    };
    apply();
    media?.addEventListener("change", apply);
    return () => media?.removeEventListener("change", apply);
  }, [theme]);
}

/** Loads settings and jobs, and listens for files dropped on the window. */
export function useAppBootstrap() {
  useEffect(() => {
    void useSettings.getState().load();
    let unlistenJobs: (() => void) | undefined;
    void useJobs
      .getState()
      .init()
      .then((u) => (unlistenJobs = u));

    let unlistenDrop: (() => void) | undefined;
    let disposed = false;
    if (inTauri()) {
      void getCurrentWebview()
        .onDragDropEvent((event) => {
          const { setDragging, dispatch } = useDrop.getState();
          const p = event.payload;
          if (p.type === "enter" || p.type === "over") setDragging(true);
          else if (p.type === "leave") setDragging(false);
          else if (p.type === "drop") dispatch(p.paths);
        })
        .then((u) => (disposed ? u() : (unlistenDrop = u)));
    }
    return () => {
      disposed = true;
      unlistenJobs?.();
      unlistenDrop?.();
    };
  }, []);
}

/** Application-wide keyboard shortcuts. */
export function useGlobalShortcuts() {
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      const nav = useNavigation.getState();
      if (mod && e.key === ",") {
        e.preventDefault();
        nav.navigate({ view: "settings" });
      } else if (mod && e.key.toLowerCase() === "b") {
        e.preventDefault();
        const s = useSettings.getState();
        void s.update({ sidebarCollapsed: !s.settings.sidebarCollapsed });
      } else if (e.altKey && e.key === "ArrowLeft" && nav.history.length) {
        e.preventDefault();
        nav.back();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);
}
