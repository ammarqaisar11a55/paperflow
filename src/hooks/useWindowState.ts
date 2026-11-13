import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow, type Window } from "@tauri-apps/api/window";

export type ResizeDirection = "East" | "North" | "NorthEast" | "NorthWest" | "South" | "SouthEast" | "SouthWest" | "West";

/** True inside the Tauri webview (false in unit tests and a plain browser). */
export const inTauri = () => typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

function currentWindow(): Window | null {
  if (!inTauri()) return null;
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
}

/**
 * Window state and controls for the custom title bar. The window is
 * frameless, so PaperFlow draws its own controls and resize edges.
 */
export function useWindowState() {
  const [maximized, setMaximized] = useState(false);
  const [fullscreen, setFullscreen] = useState(false);
  const [focused, setFocused] = useState(true);

  useEffect(() => {
    const win = currentWindow();
    if (!win) return;
    let disposed = false;
    const unlisteners: Array<() => void> = [];
    const refresh = async () => {
      const [m, f] = await Promise.all([win.isMaximized(), win.isFullscreen()]);
      if (!disposed) {
        setMaximized(m);
        setFullscreen(f);
      }
    };
    void refresh();
    void win.onResized(() => void refresh()).then((u) => (disposed ? u() : unlisteners.push(u)));
    void win.onFocusChanged(({ payload }) => setFocused(payload)).then((u) => (disposed ? u() : unlisteners.push(u)));
    return () => {
      disposed = true;
      unlisteners.forEach((u) => u());
    };
  }, []);

  const minimize = useCallback(() => void currentWindow()?.minimize(), []);
  const toggleMaximize = useCallback(() => void currentWindow()?.toggleMaximize(), []);
  const close = useCallback(() => void currentWindow()?.close(), []);
  const startResize = useCallback((direction: ResizeDirection) => void currentWindow()?.startResizeDragging(direction), []);
  const setFullscreenMode = useCallback(async (on: boolean) => {
    await currentWindow()?.setFullscreen(on);
    setFullscreen(on);
  }, []);

  return { maximized, fullscreen, focused, minimize, toggleMaximize, close, startResize, setFullscreen: setFullscreenMode };
}
