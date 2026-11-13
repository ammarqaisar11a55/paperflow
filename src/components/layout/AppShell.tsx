import type { ReactNode } from "react";
import { FileUp } from "lucide-react";
import { useWindowState } from "@/hooks/useWindowState";
import { useDrop } from "@/stores/drop";
import { cn } from "@/utils/cn";
import { ResizeHandles } from "./ResizeHandles";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import { TitleBar } from "./TitleBar";

function DropOverlay() {
  const dragging = useDrop((s) => s.dragging);
  const label = useDrop((s) => s.handlers[s.handlers.length - 1]?.label);
  if (!dragging || !label) return null;
  return (
    <div className="pointer-events-none fixed inset-0 z-[800] flex items-center justify-center bg-accent/8 p-6">
      <div className="animate-in flex flex-col items-center gap-2 rounded-2xl border-2 border-dashed border-accent bg-surface px-10 py-8 shadow-pop">
        <FileUp size={28} className="text-accent" />
        <p className="text-card font-semibold">{label}</p>
      </div>
    </div>
  );
}

/** Title bar, sidebar, content and status bar inside the frameless window. */
export function AppShell({ children }: { children: ReactNode }) {
  const win = useWindowState();
  const framed = !win.maximized && !win.fullscreen;
  return (
    <div
      className={cn(
        "flex h-full flex-col overflow-hidden bg-bg",
        // A hairline edge gives the frameless window definition on any wallpaper.
        framed && "ring-1 ring-inset ring-line-strong",
      )}
    >
      {!win.fullscreen && (
        <TitleBar
          maximized={win.maximized}
          focused={win.focused}
          onMinimize={win.minimize}
          onToggleMaximize={win.toggleMaximize}
          onClose={win.close}
        />
      )}
      <div className="flex min-h-0 flex-1">
        {!win.fullscreen && <Sidebar />}
        <main className="relative min-w-0 flex-1 overflow-hidden">{children}</main>
      </div>
      {!win.fullscreen && <StatusBar />}
      {framed && <ResizeHandles onResize={win.startResize} />}
      <DropOverlay />
    </div>
  );
}
