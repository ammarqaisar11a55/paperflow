import { useEffect } from "react";
import { FilePlus2 } from "lucide-react";
import { KINDS, type InputKind } from "@/features/fileKinds";
import { useDrop } from "@/stores/drop";
import { cn } from "@/utils/cn";

interface DropZoneProps {
  /** A file kind, or "any" for every supported kind. */
  accepts: InputKind | "any";
  multiple?: boolean;
  onFiles: (paths: string[]) => void;
  onBrowse: () => void;
  /** Shown in the full-window overlay while dragging. */
  dropLabel: string;
  compact?: boolean;
  className?: string;
}

/**
 * Where files come in: click (or Enter/Space) to browse, or drop files
 * anywhere on the window while this zone is shown.
 */
export function DropZone({ accepts, multiple = true, onFiles, onBrowse, dropLabel, compact, className }: DropZoneProps) {
  const register = useDrop((s) => s.register);
  const dragging = useDrop((s) => s.dragging);
  useEffect(() => register(onFiles, dropLabel), [register, onFiles, dropLabel]);

  const info = accepts === "any" ? null : KINDS[accepts];
  const Icon = info?.icon ?? FilePlus2;
  const noun = info ? (multiple ? info.many : info.one) : "files";
  const types = info ? info.types : "PDF, Word, PowerPoint, Excel, HTML, images";

  return (
    <button
      type="button"
      onClick={onBrowse}
      className={cn(
        "group flex w-full items-center rounded-xl border border-dashed bg-surface text-left transition-colors",
        "border-line-strong hover:border-accent hover:bg-accent-soft/40",
        dragging && "border-accent bg-accent-soft/50",
        compact ? "gap-3 px-4 py-3" : "flex-col justify-center gap-2 px-6 py-9 text-center",
        className,
      )}
    >
      <span
        className={cn(
          "flex items-center justify-center rounded-lg bg-accent-soft text-accent-text transition-transform group-hover:scale-[1.03]",
          compact ? "h-9 w-9" : "mb-1 h-11 w-11",
        )}
      >
        <Icon size={compact ? 18 : 21} strokeWidth={1.8} />
      </span>
      <span className="flex flex-col">
        <span className="text-card font-semibold text-fg">{compact ? `Add ${noun}` : `Drop ${noun} here`}</span>
        <span className="text-secondary text-muted">
          {compact ? `Drop here or click to browse · ${types}` : (
            <>
              or <span className="font-medium text-accent-text group-hover:underline">choose files</span> · {types}
            </>
          )}
        </span>
      </span>
    </button>
  );
}
