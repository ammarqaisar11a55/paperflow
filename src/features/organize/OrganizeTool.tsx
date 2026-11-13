import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  CopyPlus,
  FileOutput,
  FolderOpen,
  Redo2,
  RotateCcw,
  RotateCw,
  Save,
  SquareCheck,
  Trash2,
  Undo2,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { Page } from "@/components/layout/Page";
import { Button, IconButton } from "@/components/ui/Button";
import { Badge } from "@/components/ui/Display";
import { DropZone } from "@/components/files/DropZone";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import { ProcessingPanel } from "@/components/processing/ProcessingPanel";
import { useFileSelection } from "@/hooks/useFileSelection";
import { usePointerSort } from "@/hooks/usePointerSort";
import { toAppError } from "@/services/api";
import { pickFolder, pickPdfs } from "@/services/dialogs";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";
import type { AppError } from "@/types";
import { cn } from "@/utils/cn";
import { pluralize, prettyPath, stem } from "@/utils/format";
import { getTool } from "@/features/registry";
import { useOutputDir } from "@/features/ToolWorkspace";
import {
  commit,
  deleteItems,
  duplicateItems,
  initialState,
  isModified,
  moveItems,
  rangeBetween,
  redo,
  rotateItems,
  toOperationPages,
  undo,
  type OrganizerState,
  type PageItem,
} from "./organizerState";

const ZOOMS = [110, 140, 180, 230];

interface PageCardProps {
  item: PageItem;
  index: number;
  path: string;
  width: number;
  selected: boolean;
  dragging: boolean;
  sortProps: Record<string, unknown>;
  onSelect: (e: React.MouseEvent) => void;
  onRotate: (delta: number) => void;
  onDuplicate: () => void;
  onDelete: () => void;
}

function PageCard({ item, index, path, width, selected, dragging, sortProps, onSelect, onRotate, onDuplicate, onDelete }: PageCardProps) {
  return (
    <div
      {...sortProps}
      role="option"
      aria-selected={selected}
      aria-label={`Position ${index + 1}, page ${item.page}${item.rotate ? `, rotated ${item.rotate} degrees` : ""}`}
      tabIndex={-1}
      onClick={onSelect}
      className={cn(
        "group relative flex cursor-grab flex-col items-center gap-2 rounded-lg p-2.5 transition-[background-color,box-shadow,opacity] duration-100",
        selected ? "bg-accent-soft ring-2 ring-accent" : "hover:bg-surface-2",
        dragging && "z-10 opacity-60 ring-2 ring-accent ring-offset-2 ring-offset-bg",
      )}
    >
      <div className="flex items-center justify-center" style={{ width, height: width * 1.42 }}>
        <PdfThumbnail
          path={path}
          page={item.page}
          width={Math.round(width)}
          rotation={item.rotate}
          className={cn("max-h-full w-full rounded-[2px] ring-1 ring-black/8", selected ? "shadow-raised" : "shadow-card")}
        />
      </div>
      <div className="flex items-center gap-1.5">
        <span className={cn("tabular text-caption font-medium", selected ? "text-accent" : "text-fg/80")}>{index + 1}</span>
        {item.page !== index + 1 && <span className="tabular text-caption text-muted">· p.{item.page}</span>}
      </div>

      {selected && (
        <span className="absolute left-1.5 top-1.5 flex h-[18px] w-[18px] items-center justify-center rounded-full bg-accent text-white shadow-card">
          <svg viewBox="0 0 12 12" className="h-2.5 w-2.5" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M2.5 6.5l2.2 2L9.5 3.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </span>
      )}

      <div
        data-no-drag
        className="absolute right-1.5 top-1.5 flex gap-0.5 rounded-md border border-line bg-surface/95 p-0.5 opacity-0 shadow-raised transition-opacity group-hover:opacity-100 group-focus-within:opacity-100"
      >
        <IconButton icon={RotateCcw} size="sm" label="Rotate left" onClick={(e) => (e.stopPropagation(), onRotate(-90))} />
        <IconButton icon={RotateCw} size="sm" label="Rotate right" onClick={(e) => (e.stopPropagation(), onRotate(90))} />
        <IconButton icon={CopyPlus} size="sm" label="Duplicate" onClick={(e) => (e.stopPropagation(), onDuplicate())} />
        <IconButton icon={Trash2} size="sm" label="Delete" className="hover:!text-danger" onClick={(e) => (e.stopPropagation(), onDelete())} />
      </div>
    </div>
  );
}

export function OrganizeTool() {
  const tool = getTool("organize")!;
  const selection = useFileSelection({ accepts: "pdf", single: true });
  const file = selection.files[0];
  const takeHandoff = useNavigation((s) => s.takeHandoff);
  const overwrite = useSettings((s) => s.settings.overwrite);
  const startJob = useJobs((s) => s.start);
  const output = useOutputDir(file?.path ?? null);

  const [state, setState] = useState<OrganizerState>({ items: [], past: [], future: [] });
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [anchor, setAnchor] = useState<string | null>(null);
  const [zoom, setZoom] = useState(1);
  const [name, setName] = useState("");
  const [jobIds, setJobIds] = useState<string[] | null>(null);
  const [startErrors, setStartErrors] = useState<AppError[]>([]);
  const gridRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handed = takeHandoff();
    if (handed.length) void selection.add(handed.slice(0, 1));
  }, []);

  useEffect(() => {
    if (file?.pageCount) {
      setState(initialState(file.pageCount));
      setSelected(new Set());
      setName(`${stem(file.path)} - organized`);
    }
  }, [file]);

  const items = state.items;
  const apply = useCallback((fn: (items: PageItem[]) => PageItem[]) => setState((s) => commit(s, fn(s.items))), []);
  const targets = useCallback(
    (uid?: string) => (uid && !selected.has(uid) ? new Set([uid]) : selected),
    [selected],
  );

  const rotate = useCallback((delta: number, uid?: string) => apply((it) => rotateItems(it, targets(uid), delta)), [apply, targets]);
  const remove = useCallback(
    (uid?: string) => {
      const t = targets(uid);
      apply((it) => (it.length - t.size < 1 ? it : deleteItems(it, t)));
      setSelected(new Set());
    },
    [apply, targets],
  );
  const duplicate = useCallback(
    (uid?: string) => {
      const t = targets(uid);
      let copies: string[] = [];
      apply((it) => {
        const r = duplicateItems(it, t);
        copies = r.copies;
        return r.items;
      });
      if (copies.length) setSelected(new Set(copies));
    },
    [apply, targets],
  );

  const sort = usePointerSort({
    ids: items.map((i) => i.uid),
    dragGroup: (uid) => (selected.has(uid) ? items.filter((i) => selected.has(i.uid)).map((i) => i.uid) : [uid]),
    onReorder: (moving, index) => apply((it) => moveItems(it, moving, index)),
  });

  const onSelect = (uid: string) => (e: React.MouseEvent) => {
    if (e.shiftKey && anchor) {
      setSelected(new Set(rangeBetween(items, anchor, uid)));
    } else if (e.ctrlKey || e.metaKey) {
      setSelected((s) => {
        const n = new Set(s);
        if (n.has(uid)) n.delete(uid);
        else n.add(uid);
        return n;
      });
      setAnchor(uid);
    } else {
      setSelected(new Set([uid]));
      setAnchor(uid);
    }
  };

  useEffect(() => {
    if (!file || jobIds) return;
    const onKey = (e: KeyboardEvent) => {
      if ((e.target as HTMLElement).closest("input, textarea, select")) return;
      const mod = e.ctrlKey || e.metaKey;
      if (mod && e.key.toLowerCase() === "z") {
        e.preventDefault();
        setState((s) => (e.shiftKey ? redo(s) : undo(s)));
      } else if (mod && e.key.toLowerCase() === "y") {
        e.preventDefault();
        setState(redo);
      } else if (mod && e.key.toLowerCase() === "a") {
        e.preventDefault();
        setSelected(new Set(items.map((i) => i.uid)));
      } else if (mod && e.key.toLowerCase() === "d" && selected.size) {
        e.preventDefault();
        duplicate();
      } else if ((e.key === "Delete" || e.key === "Backspace") && selected.size) {
        e.preventDefault();
        remove();
      } else if (e.key.toLowerCase() === "r" && !mod && selected.size) {
        e.preventDefault();
        rotate(e.shiftKey ? -90 : 90);
      } else if (e.key === "Escape") {
        setSelected(new Set());
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [file, jobIds, items, selected, duplicate, remove, rotate]);

  const start = async (pages: PageItem[], outputName: string) => {
    if (!file || !output.dir) return;
    try {
      const id = await startJob({
        type: "organize",
        inputFiles: [file.path],
        outputDir: output.dir,
        outputName,
        options: { pages: toOperationPages(pages) },
        overwrite,
      });
      setStartErrors([]);
      setJobIds([id]);
    } catch (e) {
      setStartErrors([toAppError(e)]);
      setJobIds([]);
    }
  };

  const modified = file?.pageCount ? isModified(items, file.pageCount) : false;
  const selectedItems = useMemo(() => items.filter((i) => selected.has(i.uid)), [items, selected]);
  const thumbWidth = ZOOMS[zoom];

  if (!file) {
    return (
      <Page title={tool.title} description={tool.description} back="Back to tools">
        <DropZone
          accepts="pdf"
          multiple={false}
          onFiles={(p) => void selection.add(p.slice(0, 1))}
          onBrowse={async () => void selection.add(await pickPdfs(false))}
          dropLabel="Drop a PDF to organize its pages"
        />
        {selection.rejected.map((r) => (
          <p key={r} className="mt-3 text-secondary text-warning">
            {r}
          </p>
        ))}
      </Page>
    );
  }

  if (file.encrypted) {
    return (
      <Page title={tool.title} description={tool.description} back="Back to tools">
        <p className="text-body">“{file.name}” is password-protected. Unlock it first with Unlock PDF.</p>
        <Button className="mt-4" onClick={selection.clear}>
          Choose another file
        </Button>
      </Page>
    );
  }

  if (jobIds !== null) {
    return (
      <Page title={tool.title} description={tool.description} back="Back to tools">
        <ProcessingPanel
          jobIds={jobIds}
          startErrors={startErrors}
          onBack={() => setJobIds(null)}
          onRetry={() => {
            setJobIds(null);
            void start(items, name);
          }}
          onAnother={() => {
            setJobIds(null);
            selection.clear();
            output.reset();
          }}
        />
      </Page>
    );
  }

  return (
    <div className="flex h-full flex-col">
      {/* Toolbar */}
      <div className="flex shrink-0 flex-wrap items-center gap-1 border-b border-line bg-surface px-4 py-2">
        <div className="mr-3 flex min-w-0 items-center gap-2">
          <span className="max-w-[260px] truncate text-body font-semibold" title={file.path}>
            {file.name}
          </span>
          <Badge>{pluralize(items.length, "page")}</Badge>
          {modified && <Badge tone="accent">Edited</Badge>}
        </div>
        <IconButton icon={Undo2} label="Undo" shortcut="Ctrl Z" disabled={!state.past.length} onClick={() => setState(undo)} />
        <IconButton icon={Redo2} label="Redo" shortcut="Ctrl Shift Z" disabled={!state.future.length} onClick={() => setState(redo)} />
        <div className="mx-1.5 h-5 w-px bg-line" />
        <IconButton icon={RotateCcw} label="Rotate left" shortcut="Shift R" disabled={!selected.size} onClick={() => rotate(-90)} />
        <IconButton icon={RotateCw} label="Rotate right" shortcut="R" disabled={!selected.size} onClick={() => rotate(90)} />
        <IconButton icon={CopyPlus} label="Duplicate" shortcut="Ctrl D" disabled={!selected.size} onClick={() => duplicate()} />
        <IconButton
          icon={Trash2}
          label="Delete"
          shortcut="Del"
          disabled={!selected.size || selected.size >= items.length}
          onClick={() => remove()}
        />
        <div className="mx-1.5 h-5 w-px bg-line" />
        <IconButton
          icon={selected.size === items.length ? X : SquareCheck}
          label={selected.size === items.length ? "Clear selection" : "Select all"}
          shortcut={selected.size === items.length ? "Esc" : "Ctrl A"}
          onClick={() => setSelected(selected.size === items.length ? new Set() : new Set(items.map((i) => i.uid)))}
        />
        <Button
          size="sm"
          variant="ghost"
          icon={FileOutput}
          disabled={!selected.size}
          onClick={() => void start(selectedItems, `${stem(file.path)} - extract`)}
        >
          Extract {selected.size ? pluralize(selected.size, "page") : "pages"}
        </Button>
        <div className="ml-auto flex items-center gap-1">
          <IconButton icon={ZoomOut} label="Smaller thumbnails" disabled={zoom === 0} onClick={() => setZoom((z) => z - 1)} />
          <IconButton icon={ZoomIn} label="Larger thumbnails" disabled={zoom === ZOOMS.length - 1} onClick={() => setZoom((z) => z + 1)} />
          <div className="mx-1.5 h-5 w-px bg-line" />
          <IconButton icon={X} label="Close document" onClick={selection.clear} />
        </div>
      </div>

      {/* Pages */}
      <div
        ref={gridRef}
        role="listbox"
        aria-multiselectable="true"
        aria-label="Pages"
        className="min-h-0 flex-1 overflow-y-auto px-6 py-6"
        onClick={(e) => e.target === e.currentTarget && setSelected(new Set())}
      >
        <div
          className="mx-auto grid max-w-[1400px] gap-2"
          style={{ gridTemplateColumns: `repeat(auto-fill, minmax(${thumbWidth + 24}px, 1fr))` }}
          onClick={(e) => e.target === e.currentTarget && setSelected(new Set())}
        >
          {items.map((item, i) => (
            <PageCard
              key={item.uid}
              item={item}
              index={i}
              path={file.path}
              width={thumbWidth}
              selected={selected.has(item.uid)}
              dragging={sort.isDragging(item.uid)}
              sortProps={sort.itemProps(item.uid)}
              onSelect={onSelect(item.uid)}
              onRotate={(d) => rotate(d, item.uid)}
              onDuplicate={() => duplicate(item.uid)}
              onDelete={() => remove(item.uid)}
            />
          ))}
        </div>
        <p className="mt-6 text-center text-caption text-muted">
          Drag to reorder · Click to select, Ctrl-click to add, Shift-click for a range
        </p>
      </div>

      {/* Save bar */}
      <div className="flex shrink-0 items-center gap-3 border-t border-line bg-surface px-4 py-2.5">
        <label className="flex items-center gap-2 text-secondary text-muted">
          Save as
          <span className="flex items-center rounded-md border border-line bg-surface focus-within:border-accent hover:border-line-strong">
            <input
              value={name}
              aria-label="Output file name"
              spellCheck={false}
              onChange={(e) => setName(e.target.value)}
              className="h-7 w-[260px] bg-transparent px-2 text-body text-fg focus:outline-none"
            />
            <span className="pr-2 text-body">.pdf</span>
          </span>
        </label>
        <span className="text-secondary text-muted">in</span>
        <button
          type="button"
          title={output.dir ?? undefined}
          onClick={async () => {
            const d = await pickFolder(output.dir);
            if (d) output.setDir(d);
          }}
          className="flex max-w-[320px] items-center gap-1.5 truncate rounded-md px-2 py-1 text-secondary text-fg hover:bg-surface-2"
        >
          <FolderOpen size={14} className="shrink-0 text-muted" />
          <span className="truncate">{output.dir ? prettyPath(output.dir) : "Choose folder"}</span>
        </button>
        <Button
          variant="primary"
          icon={Save}
          className="ml-auto"
          disabled={!output.dir || !name.trim() || !items.length}
          onClick={() => void start(items, name)}
        >
          Save document
        </Button>
      </div>
    </div>
  );
}

