// PDF.js integration: loading documents (bytes come from the Rust side, so
// the webview has no filesystem access) and rendering page thumbnails.
import * as pdfjs from "pdfjs-dist";
import type { PDFDocumentProxy } from "pdfjs-dist";
import workerUrl from "pdfjs-dist/build/pdf.worker.min.mjs?url";
import { api } from "./api";

pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

export type { PDFDocumentProxy };

const ASSETS = "/pdfjs";

interface CachedDoc {
  promise: Promise<PDFDocumentProxy>;
  users: number;
  destroy: () => Promise<void>;
  idle?: number;
}

const documents = new Map<string, CachedDoc>();
/** Documents nobody uses are kept briefly, so moving between screens is instant. */
const IDLE_MS = 60_000;

/** Loads (or reuses) a document. Call `releasePdf` when done. */
export function acquirePdf(path: string): Promise<PDFDocumentProxy> {
  const cached = documents.get(path);
  if (cached) {
    cached.users++;
    window.clearTimeout(cached.idle);
    return cached.promise;
  }
  let task: pdfjs.PDFDocumentLoadingTask | null = null;
  const promise = api.readFileBytes(path).then((bytes) => {
    task = pdfjs.getDocument({
      data: new Uint8Array(bytes),
      cMapUrl: `${ASSETS}/cmaps/`,
      cMapPacked: true,
      standardFontDataUrl: `${ASSETS}/standard_fonts/`,
      wasmUrl: `${ASSETS}/wasm/`,
      iccUrl: `${ASSETS}/iccs/`,
      enableXfa: false,
    });
    return task.promise;
  });
  documents.set(path, { promise, users: 1, destroy: async () => void (await task?.destroy()) });
  promise.catch(() => documents.delete(path));
  return promise;
}

export function releasePdf(path: string) {
  const cached = documents.get(path);
  if (!cached) return;
  cached.users = Math.max(0, cached.users - 1);
  if (cached.users === 0) {
    window.clearTimeout(cached.idle);
    cached.idle = window.setTimeout(() => {
      if (cached.users === 0 && documents.get(path) === cached) {
        documents.delete(path);
        void cached.destroy().catch(() => undefined);
      }
    }, IDLE_MS);
  }
}

/** Forgets everything about a file (e.g. after it changed on disk). */
export function forgetPdf(path: string) {
  const cached = documents.get(path);
  if (cached) {
    documents.delete(path);
    void cached.destroy().catch(() => undefined);
  }
  for (const [key, url] of thumbnails) {
    if (key.startsWith(`${path}#`)) {
      thumbnails.delete(key);
      void url.then((u) => URL.revokeObjectURL(u)).catch(() => undefined);
    }
  }
}

// Thumbnail rendering is queued so a 300-page document does not render every
// page at once; visible thumbnails ask first.
const MAX_PARALLEL = 3;
let active = 0;
const queue: Array<() => void> = [];

function schedule<T>(task: () => Promise<T>): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const run = () => {
      active++;
      task()
        .then(resolve, reject)
        .finally(() => {
          active--;
          queue.shift()?.();
        });
    };
    if (active < MAX_PARALLEL) run();
    else queue.push(run);
  });
}

/** Rendered thumbnails, least recently used first. */
const thumbnails = new Map<string, Promise<string>>();
const MAX_THUMBNAILS = 600;

function remember(key: string, value: Promise<string>) {
  thumbnails.set(key, value);
  while (thumbnails.size > MAX_THUMBNAILS) {
    const [oldest, url] = thumbnails.entries().next().value as [string, Promise<string>];
    thumbnails.delete(oldest);
    void url.then((u) => URL.revokeObjectURL(u)).catch(() => undefined);
  }
}

/**
 * Renders page `pageNumber` (1-based) to an image of the given CSS width and
 * returns an object URL. Results are cached per document.
 */
export function renderThumbnail(path: string, pageNumber: number, width: number): Promise<string> {
  const key = `${path}#${pageNumber}@${width}`;
  const cached = thumbnails.get(key);
  if (cached) {
    // Mark as recently used.
    thumbnails.delete(key);
    thumbnails.set(key, cached);
    return cached;
  }
  const promise = schedule(async () => {
    const doc = await acquirePdf(path);
    try {
      const page = await doc.getPage(pageNumber);
      const base = page.getViewport({ scale: 1 });
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      const viewport = page.getViewport({ scale: (width / base.width) * ratio });
      const canvas = document.createElement("canvas");
      canvas.width = Math.ceil(viewport.width);
      canvas.height = Math.ceil(viewport.height);
      await page.render({ canvas, viewport, background: "#ffffff" }).promise;
      page.cleanup();
      const blob = await new Promise<Blob | null>((r) => canvas.toBlob(r, "image/jpeg", 0.85));
      if (!blob) throw new Error("Thumbnail could not be created.");
      return URL.createObjectURL(blob);
    } finally {
      releasePdf(path);
    }
  });
  remember(key, promise);
  promise.catch(() => thumbnails.delete(key));
  return promise;
}

/** Page size in points, as displayed (rotation applied). */
export async function pageSizes(doc: PDFDocumentProxy): Promise<{ width: number; height: number }[]> {
  const sizes = [];
  for (let i = 1; i <= doc.numPages; i++) {
    const page = await doc.getPage(i);
    const v = page.getViewport({ scale: 1 });
    sizes.push({ width: v.width, height: v.height });
  }
  return sizes;
}

export { pdfjs };
