import type { ReactNode } from "react";
import { Tooltip } from "@/components/ui/Tooltip";
import { cn } from "@/utils/cn";

interface ControlProps {
  label: string;
  onClick: () => void;
  danger?: boolean;
  children: ReactNode;
}

function Control({ label, onClick, danger, children }: ControlProps) {
  return (
    <Tooltip label={label}>
      <button
        type="button"
        aria-label={label}
        onClick={onClick}
        // Keep the double-click of the drag region from firing on the buttons.
        onDoubleClick={(e) => e.stopPropagation()}
        className={cn(
          "flex h-7 w-7 items-center justify-center rounded-full text-fg/80 transition-colors duration-100",
          "bg-surface-2 hover:bg-surface-3 hover:text-fg active:bg-line-strong",
          danger && "hover:bg-danger hover:text-white active:bg-danger active:brightness-90",
        )}
      >
        {children}
      </button>
    </Tooltip>
  );
}

// Hand-drawn 10px glyphs: crisper than icon-font strokes at this size.
const glyph = "h-2.5 w-2.5";

interface WindowControlsProps {
  maximized: boolean;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onClose: () => void;
}

export function WindowControls({ maximized, onMinimize, onToggleMaximize, onClose }: WindowControlsProps) {
  return (
    <div className="flex items-center gap-2" role="group" aria-label="Window controls">
      <Control label="Minimize" onClick={onMinimize}>
        <svg viewBox="0 0 10 10" className={glyph} aria-hidden="true">
          <path d="M1.5 5.5h7" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
        </svg>
      </Control>
      <Control label={maximized ? "Restore" : "Maximize"} onClick={onToggleMaximize}>
        {maximized ? (
          <svg viewBox="0 0 10 10" className={glyph} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.2">
            <rect x="1.5" y="3" width="5.5" height="5.5" rx="1" />
            <path d="M3.5 1.5h4a1 1 0 0 1 1 1v4" strokeLinecap="round" />
          </svg>
        ) : (
          <svg viewBox="0 0 10 10" className={glyph} aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.2">
            <rect x="1.75" y="1.75" width="6.5" height="6.5" rx="1" />
          </svg>
        )}
      </Control>
      <Control label="Close" onClick={onClose} danger>
        <svg viewBox="0 0 10 10" className={glyph} aria-hidden="true">
          <path d="M2 2l6 6M8 2l-6 6" stroke="currentColor" strokeWidth="1.3" strokeLinecap="round" />
        </svg>
      </Control>
    </div>
  );
}
