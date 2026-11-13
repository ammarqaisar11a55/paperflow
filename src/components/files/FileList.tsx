import { ArrowDown, ArrowDownAZ, ArrowUp, GripVertical, LockKeyhole, Trash2, X } from "lucide-react";
import { IconButton } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Display";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import type { FileSelection } from "@/hooks/useFileSelection";
import { usePointerSort } from "@/hooks/usePointerSort";
import { cn } from "@/utils/cn";
import { ImagePreview } from "./ImagePreview";
import { KINDS, type InputKind } from "@/features/fileKinds";
import type { FileKind } from "@/types";

/** Documents without a thumbnail show their type's icon. */
function KindIcon({ kind }: { kind: FileKind }) {
  const Icon = kind in KINDS ? KINDS[kind as InputKind].icon : KINDS.pdf.icon;
  return (
    <div className="flex h-full items-center justify-center text-[#9aa1ad]">
      <Icon size={15} />
    </div>
  );
}
import { formatBytes, pluralize } from "@/utils/format";

interface FileListProps {
  selection: FileSelection;
  /** Show ordering controls (for tools where order matters). */
  orderable?: boolean;
  title?: string;
}

export function FileList({ selection, orderable, title = "Selected files" }: FileListProps) {
  const { files, remove, move, reorder, clear, sortByName, rejected, dismissRejected } = selection;
  const sort = usePointerSort({ ids: files.map((f) => f.path), onReorder: reorder, axis: "y", disabled: !orderable });
  const totalPages = files.reduce((n, f) => n + (f.pageCount ?? 0), 0);

  return (
    <div className="flex flex-col gap-3">
      {rejected.length > 0 && (
        <div role="alert" className="flex items-start gap-2 rounded-lg bg-warning-soft px-3 py-2 text-secondary text-warning">
          <div className="flex-1">
            {rejected.map((r) => (
              <p key={r}>{r}</p>
            ))}
          </div>
          <button type="button" aria-label="Dismiss" onClick={dismissRejected} className="rounded p-0.5 hover:bg-black/5">
            <X size={14} />
          </button>
        </div>
      )}
      {files.length > 0 && (
        <div className="overflow-hidden rounded-xl border border-line bg-surface shadow-card">
          <div className="flex items-center justify-between border-b border-line px-4 py-2.5">
            <p className="text-secondary font-semibold">
              {title}
              <span className="ml-2 font-normal text-muted">
                {pluralize(files.length, "file")}
                {totalPages > 0 && ` · ${pluralize(totalPages, "page")}`}
              </span>
            </p>
            <div className="flex items-center gap-1">
              {orderable && files.length > 1 && <IconButton icon={ArrowDownAZ} label="Sort by name" size="sm" onClick={sortByName} />}
              <IconButton icon={Trash2} label="Remove all" size="sm" onClick={clear} />
            </div>
          </div>
          <ul className="max-h-[420px] divide-y divide-line overflow-y-auto">
            {files.map((f, i) => (
              <li
                key={f.path}
                {...sort.itemProps(f.path)}
                className={cn(
                  "group flex items-center gap-3 px-3 py-2 transition-colors",
                  orderable && "cursor-grab",
                  sort.isDragging(f.path) ? "relative z-10 bg-accent-soft shadow-raised" : "hover:bg-surface-2/60",
                )}
              >
                {orderable && <GripVertical size={14} className="shrink-0 text-muted/60 group-hover:text-muted" />}
                {orderable && <span className="tabular w-5 shrink-0 text-right text-caption text-muted">{i + 1}</span>}
                <div className="h-11 w-9 shrink-0 overflow-hidden rounded-[3px] border border-line bg-page">
                  {f.kind === "pdf" ? (
                    f.encrypted ? (
                      <div className="flex h-full items-center justify-center text-[#9aa1ad]">
                        <LockKeyhole size={14} />
                      </div>
                    ) : (
                      <PdfThumbnail path={f.path} page={1} width={72} className="h-full w-full" />
                    )
                  ) : f.kind === "image" ? (
                    <ImagePreview path={f.path} />
                  ) : (
                    <KindIcon kind={f.kind} />
                  )}
                </div>
                <div className="flex min-w-0 flex-1 flex-col">
                  <span className="truncate text-body font-medium" title={f.path}>
                    {f.name}
                  </span>
                  <span className="flex items-center gap-2 text-caption text-muted">
                    {f.pageCount !== null && <span>{pluralize(f.pageCount, "page")}</span>}
                    <span>{formatBytes(f.size)}</span>
                    {f.encrypted && (
                      <Badge tone="warning" icon={LockKeyhole}>
                        Protected
                      </Badge>
                    )}
                  </span>
                </div>
                <div className="flex shrink-0 items-center gap-0.5 opacity-70 group-hover:opacity-100">
                  {orderable && (
                    <>
                      <IconButton icon={ArrowUp} label="Move up" size="sm" disabled={i === 0} onClick={() => move(f.path, -1)} />
                      <IconButton icon={ArrowDown} label="Move down" size="sm" disabled={i === files.length - 1} onClick={() => move(f.path, 1)} />
                    </>
                  )}
                  <IconButton icon={X} label="Remove" size="sm" onClick={() => remove(f.path)} />
                </div>
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
}
