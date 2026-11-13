import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ChevronDown,
  ChevronLeft,
  ChevronRight,
  ChevronUp,
  Expand,
  FileOutput,
  FolderOpen,
  LoaderCircle,
  Minimize,
  PanelLeft,
  Search,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import { Button, IconButton } from "@/components/ui/Button";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import { acquirePdf, pdfjs, releasePdf, type PDFDocumentProxy } from "@/services/pdf";
import { cn } from "@/utils/cn";
import { describePages, pluralize } from "@/utils/format";

type ZoomMode = "fit-width" | "fit-page" | number;
const ZOOM_STEPS = [0.5, 0.67, 0.75, 0.9, 1, 1.1, 1.25, 1.5, 1.75, 2, 2.5, 3, 4];
const GAP = 16;
const PAD = 24;

interface Match {
  page: number;
  /** Occurrence index within the page. */
  index: number;
}

interface PageViewProps {
  doc: PDFDocumentProxy;
  number: number;
  scale: number;
  size: { width: number; height: number };
  query: string;
  currentMatch: Match | null;
}

/** One page: canvas plus a selectable text layer, rendered when near view. */
function PageView({ doc, number, scale, size, query, currentMatch }: PageViewProps) {
  const ref = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const textRef = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [rendered, setRendered] = useState(0);

  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    const io = new IntersectionObserver(([e]) => setVisible(e.isIntersecting), { rootMargin: "600px 0px" });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    let task: { cancel: () => void } | null = null;
    let textLayer: { cancel: () => void } | null = null;
    (async () => {
      const page = await doc.getPage(number);
      if (cancelled) return;
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const viewport = page.getViewport({ scale });
      const canvas = canvasRef.current!;
      canvas.width = Math.floor(viewport.width * ratio);
      canvas.height = Math.floor(viewport.height * ratio);
      const render = page.render({
        canvas,
        viewport,
        transform: ratio !== 1 ? [ratio, 0, 0, ratio, 0, 0] : undefined,
        background: "#ffffff",
      });
      task = render;
      await render.promise;
      if (cancelled) return;
      const container = textRef.current!;
      container.replaceChildren();
      container.style.setProperty("--total-scale-factor", String(scale));
      const layer = new pdfjs.TextLayer({ textContentSource: page.streamTextContent(), container, viewport });
      textLayer = layer;
      await layer.render();
      if (!cancelled) setRendered((n) => n + 1);
    })().catch(() => undefined);
    return () => {
      cancelled = true;
      task?.cancel();
      textLayer?.cancel();
    };
  }, [visible, doc, number, scale]);

  // Highlight matches in the text layer (span level).
  useEffect(() => {
    const container = textRef.current;
    if (!container) return;
    const q = query.trim().toLowerCase();
    let occurrence = 0;
    for (const span of Array.from(container.querySelectorAll<HTMLSpanElement>("span:not(.markedContent)"))) {
      span.classList.remove("pf-hit", "pf-hit-current");
      if (!q) continue;
      const text = (span.textContent ?? "").toLowerCase();
      let from = 0;
      let hit = false;
      let current = false;
      while ((from = text.indexOf(q, from)) !== -1) {
        hit = true;
        if (currentMatch?.page === number && currentMatch.index === occurrence) current = true;
        occurrence++;
        from += q.length;
      }
      if (hit) span.classList.add("pf-hit");
      if (current) {
        span.classList.add("pf-hit-current");
        span.scrollIntoView({ block: "center", behavior: "smooth" });
      }
    }
  }, [query, currentMatch, number, rendered]);

  return (
    <div
      ref={ref}
      data-page={number}
      className="relative mx-auto bg-white shadow-raised ring-1 ring-black/5"
      style={{ width: size.width * scale, height: size.height * scale }}
    >
      <canvas ref={canvasRef} className="absolute inset-0 h-full w-full" />
      <div ref={textRef} className="textLayer" />
      {!visible && <span className="absolute inset-0 flex items-center justify-center text-caption text-[#9aa1ad]">{number}</span>}
    </div>
  );
}

interface PdfViewerProps {
  path: string;
  fullscreen: boolean;
  onFullscreen: (on: boolean) => void;
  onClose: () => void;
  onExtract?: (pages: number[]) => void;
  onOpenAnother: () => void;
}

export function PdfViewer({ path, fullscreen, onFullscreen, onClose, onExtract, onOpenAnother }: PdfViewerProps) {
  const [doc, setDoc] = useState<PDFDocumentProxy | null>(null);
  const [sizes, setSizes] = useState<{ width: number; height: number }[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [zoom, setZoom] = useState<ZoomMode>("fit-width");
  const [current, setCurrent] = useState(1);
  const [pageInput, setPageInput] = useState("1");
  const [sidebar, setSidebar] = useState(true);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const [query, setQuery] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [matches, setMatches] = useState<Match[]>([]);
  const [matchIndex, setMatchIndex] = useState(0);
  const [searching, setSearching] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);
  const searchRef = useRef<HTMLInputElement>(null);
  const [viewport, setViewport] = useState({ width: 800, height: 600 });
  const textCache = useRef<Map<number, string[]>>(new Map());

  useEffect(() => {
    let cancelled = false;
    setDoc(null);
    setError(null);
    textCache.current = new Map();
    acquirePdf(path)
      .then(async (d) => {
        const s: { width: number; height: number }[] = [];
        for (let i = 1; i <= d.numPages; i++) {
          const v = (await d.getPage(i)).getViewport({ scale: 1 });
          s.push({ width: v.width, height: v.height });
        }
        if (!cancelled) {
          setSizes(s);
          setDoc(d);
        }
      })
      .catch((e) => !cancelled && setError(e?.message ?? "This PDF could not be opened."));
    return () => {
      cancelled = true;
      releasePdf(path);
    };
  }, [path]);

  useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const ro = new ResizeObserver(() => setViewport({ width: el.clientWidth, height: el.clientHeight }));
    ro.observe(el);
    return () => ro.disconnect();
  }, [doc]);

  const maxWidth = Math.max(...sizes.map((s) => s.width), 1);
  const scale = useMemo(() => {
    if (typeof zoom === "number") return zoom;
    const s0 = sizes[current - 1] ?? sizes[0] ?? { width: 612, height: 792 };
    const byWidth = (viewport.width - PAD * 2) / maxWidth;
    if (zoom === "fit-width") return Math.max(0.25, Math.min(byWidth, 4));
    return Math.max(0.25, Math.min(byWidth, (viewport.height - PAD * 2) / s0.height));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [zoom, viewport, maxWidth, sizes]);

  // Track the page in view.
  const onScroll = useCallback(() => {
    const el = scrollRef.current;
    if (!el || !sizes.length) return;
    const mid = el.scrollTop + el.clientHeight / 3;
    let y = PAD;
    for (let i = 0; i < sizes.length; i++) {
      y += sizes[i].height * scale + GAP;
      if (y > mid) {
        if (current !== i + 1) {
          setCurrent(i + 1);
          setPageInput(String(i + 1));
        }
        return;
      }
    }
  }, [sizes, scale, current]);

  const goTo = useCallback(
    (n: number) => {
      const el = scrollRef.current;
      if (!el || !sizes.length) return;
      const page = Math.max(1, Math.min(sizes.length, n));
      const top = PAD + sizes.slice(0, page - 1).reduce((y, s) => y + s.height * scale + GAP, 0);
      el.scrollTo({ top: top - 8 });
      setCurrent(page);
      setPageInput(String(page));
    },
    [sizes, scale],
  );

  // Keep the current page in place when zooming.
  const prevScale = useRef(scale);
  useEffect(() => {
    if (prevScale.current !== scale) {
      prevScale.current = scale;
      goTo(current);
    }
  }, [scale, current, goTo]);

  const zoomBy = (dir: 1 | -1) => {
    const idx = ZOOM_STEPS.findIndex((z) => z >= scale - 0.001);
    const next = dir > 0 ? ZOOM_STEPS.find((z) => z > scale + 0.01) : [...ZOOM_STEPS].reverse().find((z) => z < scale - 0.01);
    setZoom(next ?? ZOOM_STEPS[Math.max(0, Math.min(ZOOM_STEPS.length - 1, idx + dir))]);
  };

  // Search: page text is read once, then matched in memory.
  const runSearch = useCallback(
    async (q: string) => {
      if (!doc) return;
      const needle = q.trim().toLowerCase();
      if (!needle) {
        setMatches([]);
        return;
      }
      setSearching(true);
      const found: Match[] = [];
      for (let p = 1; p <= doc.numPages; p++) {
        let items = textCache.current.get(p);
        if (!items) {
          const content = await (await doc.getPage(p)).getTextContent();
          items = content.items.map((it) => ("str" in it ? it.str : ""));
          textCache.current.set(p, items);
        }
        let index = 0;
        for (const s of items) {
          const t = s.toLowerCase();
          let from = 0;
          while ((from = t.indexOf(needle, from)) !== -1) {
            found.push({ page: p, index: index++ });
            from += needle.length;
          }
        }
      }
      setMatches(found);
      setMatchIndex(0);
      setSearching(false);
      if (found.length) goTo(found[0].page);
    },
    [doc, goTo],
  );

  useEffect(() => {
    const t = window.setTimeout(() => void runSearch(query), 250);
    return () => window.clearTimeout(t);
  }, [query, runSearch]);

  const stepMatch = (dir: 1 | -1) => {
    if (!matches.length) return;
    const next = (matchIndex + dir + matches.length) % matches.length;
    setMatchIndex(next);
    goTo(matches[next].page);
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.ctrlKey || e.metaKey;
      const typing = (e.target as HTMLElement).closest("input");
      if (mod && e.key.toLowerCase() === "o") {
        e.preventDefault();
        onOpenAnother();
      } else if (mod && e.key.toLowerCase() === "f") {
        e.preventDefault();
        setSearchOpen(true);
        setTimeout(() => searchRef.current?.select(), 0);
      } else if (mod && (e.key === "=" || e.key === "+")) {
        e.preventDefault();
        zoomBy(1);
      } else if (mod && e.key === "-") {
        e.preventDefault();
        zoomBy(-1);
      } else if (mod && e.key === "0") {
        e.preventDefault();
        setZoom("fit-width");
      } else if (e.key === "F11") {
        e.preventDefault();
        onFullscreen(!fullscreen);
      } else if (e.key === "Escape" && fullscreen) {
        onFullscreen(false);
      } else if (!typing && (e.key === "PageDown" || (e.key === "ArrowRight" && !mod))) {
        e.preventDefault();
        goTo(current + 1);
      } else if (!typing && (e.key === "PageUp" || (e.key === "ArrowLeft" && !mod))) {
        e.preventDefault();
        goTo(current - 1);
      } else if (!typing && e.key === "Home") {
        goTo(1);
      } else if (!typing && e.key === "End") {
        goTo(sizes.length);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  const toggleSelect = (p: number, e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) {
      setSelected((s) => {
        const n = new Set(s);
        if (n.has(p)) n.delete(p);
        else n.add(p);
        return n;
      });
    } else {
      setSelected(new Set());
      goTo(p);
    }
  };

  if (error) {
    return (
      <div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
        <p className="text-card font-semibold">This PDF could not be opened</p>
        <p className="max-w-md text-secondary text-muted">{error}</p>
        <div className="flex gap-2">
          <Button onClick={onClose}>Close</Button>
          <Button variant="primary" icon={FolderOpen} onClick={onOpenAnother}>
            Open another file
          </Button>
        </div>
      </div>
    );
  }

  const zoomLabel = typeof zoom === "number" ? `${Math.round(zoom * 100)}%` : zoom === "fit-width" ? "Fit width" : "Fit page";

  return (
    <div className="flex h-full flex-col bg-surface-2">
      {/* Toolbar */}
      <div className="flex h-11 shrink-0 items-center gap-1 border-b border-line bg-surface px-3">
        <IconButton icon={PanelLeft} label={sidebar ? "Hide thumbnails" : "Show thumbnails"} active={sidebar} onClick={() => setSidebar((s) => !s)} />
        <IconButton icon={FolderOpen} label="Open another PDF" shortcut="Ctrl O" onClick={onOpenAnother} />
        <div className="mx-1.5 h-5 w-px bg-line" />
        <IconButton icon={ChevronLeft} label="Previous page" disabled={current <= 1} onClick={() => goTo(current - 1)} />
        <form
          className="flex items-center gap-1.5 text-secondary text-muted"
          onSubmit={(e) => {
            e.preventDefault();
            const n = Number(pageInput);
            if (Number.isFinite(n)) goTo(n);
          }}
        >
          <input
            aria-label="Page number"
            value={pageInput}
            onChange={(e) => setPageInput(e.target.value.replace(/\D/g, ""))}
            onBlur={() => setPageInput(String(current))}
            className="tabular h-7 w-11 rounded-md border border-line bg-surface text-center text-body text-fg focus:border-accent focus:outline-none"
          />
          <span className="tabular">/ {sizes.length || "–"}</span>
        </form>
        <IconButton icon={ChevronRight} label="Next page" disabled={current >= sizes.length} onClick={() => goTo(current + 1)} />
        <div className="mx-1.5 h-5 w-px bg-line" />
        <IconButton icon={ZoomOut} label="Zoom out" shortcut="Ctrl −" onClick={() => zoomBy(-1)} />
        <select
          aria-label="Zoom"
          value={typeof zoom === "number" ? "custom" : zoom}
          onChange={(e) => setZoom(e.target.value === "custom" ? scale : (e.target.value as ZoomMode))}
          className="h-7 rounded-md border border-line bg-surface px-1.5 text-secondary text-fg focus:border-accent focus:outline-none"
        >
          <option value="fit-width">Fit width</option>
          <option value="fit-page">Fit page</option>
          <option value="custom">{typeof zoom === "number" ? zoomLabel : `${Math.round(scale * 100)}%`}</option>
        </select>
        <IconButton icon={ZoomIn} label="Zoom in" shortcut="Ctrl +" onClick={() => zoomBy(1)} />

        <div className="ml-auto flex items-center gap-1">
          {searchOpen ? (
            <div className="flex items-center gap-1 rounded-md border border-line bg-surface pl-2 focus-within:border-accent">
              <Search size={14} className="text-muted" />
              <input
                ref={searchRef}
                autoFocus
                value={query}
                aria-label="Search in document"
                placeholder="Search in document"
                onChange={(e) => setQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter") stepMatch(e.shiftKey ? -1 : 1);
                  if (e.key === "Escape") {
                    setSearchOpen(false);
                    setQuery("");
                  }
                }}
                className="h-7 w-44 bg-transparent text-body focus:outline-none"
              />
              <span className="tabular min-w-12 text-right text-caption text-muted">
                {searching ? <LoaderCircle size={12} className="ml-auto animate-spin" /> : query ? (matches.length ? `${matchIndex + 1}/${matches.length}` : "0") : ""}
              </span>
              <IconButton icon={ChevronUp} size="sm" label="Previous match" disabled={!matches.length} onClick={() => stepMatch(-1)} />
              <IconButton icon={ChevronDown} size="sm" label="Next match" disabled={!matches.length} onClick={() => stepMatch(1)} />
              <IconButton
                icon={X}
                size="sm"
                label="Close search"
                onClick={() => {
                  setSearchOpen(false);
                  setQuery("");
                }}
              />
            </div>
          ) : (
            <IconButton icon={Search} label="Search" shortcut="Ctrl F" onClick={() => setSearchOpen(true)} />
          )}
          <IconButton icon={fullscreen ? Minimize : Expand} label={fullscreen ? "Exit full screen" : "Full screen"} shortcut="F11" onClick={() => onFullscreen(!fullscreen)} />
          <IconButton icon={X} label="Close document" onClick={onClose} />
        </div>
      </div>

      <div className="flex min-h-0 flex-1">
        {sidebar && (
          <aside className="flex w-[184px] shrink-0 flex-col border-r border-line bg-surface">
            <ul aria-label="Pages" className="flex-1 space-y-3 overflow-y-auto p-3">
              {doc &&
                sizes.map((_, i) => {
                  const p = i + 1;
                  const isSel = selected.has(p);
                  return (
                    <li key={p}>
                      <button
                        type="button"
                        onClick={(e) => toggleSelect(p, e)}
                        aria-current={current === p ? "page" : undefined}
                        aria-pressed={isSel}
                        className={cn(
                          "flex w-full flex-col items-center gap-1 rounded-md p-1.5",
                          isSel ? "bg-accent-soft ring-2 ring-accent" : current === p ? "bg-surface-2" : "hover:bg-surface-2",
                        )}
                      >
                        <PdfThumbnail path={path} page={p} width={120} className={cn("w-[120px] shadow-card", current === p && "ring-2 ring-accent")} />
                        <span className={cn("tabular text-caption", current === p ? "font-semibold text-accent-text" : "text-muted")}>{p}</span>
                      </button>
                    </li>
                  );
                })}
            </ul>
            {selected.size > 0 && onExtract && (
              <div className="flex flex-col gap-1.5 border-t border-line p-3">
                <p className="text-caption text-muted">
                  {pluralize(selected.size, "page")} selected: {describePages([...selected].sort((a, b) => a - b))}
                </p>
                <Button size="sm" variant="primary" icon={FileOutput} onClick={() => onExtract([...selected].sort((a, b) => a - b))}>
                  Extract pages
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setSelected(new Set())}>
                  Clear selection
                </Button>
              </div>
            )}
          </aside>
        )}

        <div ref={scrollRef} onScroll={onScroll} className="selectable min-w-0 flex-1 overflow-auto" style={{ padding: PAD }}>
          {!doc ? (
            <div className="flex h-full items-center justify-center gap-2 text-secondary text-muted">
              <LoaderCircle size={16} className="animate-spin" /> Opening document…
            </div>
          ) : (
            <div className="flex flex-col" style={{ gap: GAP }}>
              {sizes.map((size, i) => (
                <PageView
                  key={i}
                  doc={doc}
                  number={i + 1}
                  scale={scale}
                  size={size}
                  query={query}
                  currentMatch={matches[matchIndex] ?? null}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
