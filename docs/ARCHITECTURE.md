# Architecture

PaperFlow is a [Tauri 2](https://tauri.app) application: a Rust backend that owns every file and
every PDF operation, and a React interface that runs in the system WebKitGTK webview.

```text
┌──────────────────────── WebKitGTK webview ────────────────────────┐
│ React + TypeScript + Tailwind                                      │
│  screens · tool pages · organizer · viewer (PDF.js)                │
│  stores (zustand): settings, navigation, jobs, drag-and-drop       │
│  services/api.ts — the only module that calls the backend          │
└───────────────┬───────────────────────────────▲────────────────────┘
       invoke   │ JSON (PdfOperation, …)        │ "job-update" events
┌───────────────▼───────────────────────────────┴────────────────────┐
│ Rust                                                               │
│  commands/   files · jobs · settings · system                      │
│  services/   operations (validate → execute → finalize)            │
│              jobs (queue, threads, progress, cancellation)         │
│              engines (locate, run, cancel external programs)       │
│              store (settings.json, activity.json)                  │
│  pdf/        lopdf: assembly, overlays, fonts, images, ranges      │
│  pdf/ops/    one module per tool group                             │
│  filesystem/ safe names, unique paths, per-job temporary folders   │
└───────────────┬────────────────────────────────────────────────────┘
                │ private copies of inputs in /tmp/paperflow-<uid>/jobs/<id>
     Ghostscript · Poppler · qpdf · Tesseract
```

## Processing engines

| Work | Engine | Why |
| --- | --- | --- |
| Merge, split, organize, rotate, extract, watermark, page numbers, metadata, images to PDF | Rust ([lopdf](https://crates.io/crates/lopdf), [image](https://crates.io/crates/image)) | No process start-up, fully testable, no extra runtime dependency. |
| Compression | Ghostscript (`pdfwrite`) | The reference implementation for re-distilling PDFs and downsampling images. |
| PDF to images, PDF to text, extract images, OCR rendering | Poppler (`pdftoppm`, `pdftotext`, `pdfimages`) | Accurate rendering and text extraction. |
| Protect and unlock | qpdf | The reference implementation of PDF encryption. |
| OCR | Tesseract | Mature, offline OCR that writes searchable PDF pages. |

`services/engines.rs` finds each program in `$SNAP/usr/bin`, next to the executable,
`PAPERFLOW_ENGINE_DIR` (development) or `PATH`. Engines run with a fixed locale, can be
cancelled at any time (the process is killed), and report progress by their output where they
provide it (Ghostscript prints each page). Inside the Snap, Ghostscript is pointed at its bundled
support files with `GS_LIB`, and Tesseract at its language data with `TESSDATA_PREFIX`.

## The operation pipeline

The interface sends one request shape for every tool:

```ts
interface PdfOperation {
  type: PdfOperationType;      // "merge", "compress", …
  inputFiles: string[];        // absolute paths chosen by the user
  outputDir: string;
  outputName?: string;
  options: Record<string, unknown>;
  overwrite?: "rename" | "overwrite";
}
```

1. **Validate** (`services/operations.rs`), before queueing: inputs exist, are regular files of
   the right type (PDF magic bytes or a supported image), and the output folder exists. Mistakes
   come back immediately as an error with a message and a hint.
2. **Queue** (`services/jobs.rs`): the job gets an id and a worker thread; at most two run at
   once. Every state change is emitted to the interface as a `job-update` event, throttled to
   about 12 per second.
3. **Execute**: the operation parses its typed options and works only inside the job's private
   temporary folder. Inputs that go to an external engine are copied there first, so engines never
   touch the user's files, odd file names cannot be misread as options, and distribution security
   profiles (Ubuntu's AppArmor profiles for `gs` and `qpdf`) are satisfied.
4. **Verify**: every PDF output is opened again and its page count checked before it is accepted.
5. **Finalize**: results are moved to the output folder under a unique name (or into a new folder
   for multi-file results). An input is never a valid destination.
6. **Clean up**: the temporary folder is deleted when the job ends, whatever happened. Leftovers
   from a crash are removed at the next start.

Results use the `ProcessingResult` shape (`success`, `outputFiles`, `errors`, `warnings`, sizes
and page count). Errors use `AppError`: a `kind`, a plain-language `message`, an optional `hint`
and an optional technical `detail` that the interface hides behind "Technical details".

### Page assembly

Merge, split, organize, extract and the OCR output are all built by one function,
`pdf/assemble.rs`: given source documents and a list of `(source, page, extra rotation)`, it
renumbers objects so sources cannot collide, copies inherited attributes (resources, media and
crop boxes, rotation) onto each page because the old page trees are dropped, duplicates page
objects (and their annotations) when a page appears twice, builds a new page tree, keeps
bookmarks, named destinations and forms when the output comes from a single document, and
prunes everything no longer referenced.

### Overlays

Watermarks and page numbers (`pdf/overlay.rs`, `pdf/ops/stamp.rs`) wrap the page's existing
content in `q … Q` so its graphics state cannot leak, add their resources under names that cannot
clash, and draw in the page's displayed orientation using the transform in `pdf/geometry.rs`, so
they appear upright on rotated pages. Text uses the standard 14 fonts with width tables generated
from the URW metrics (`scripts/gen-font-metrics.py`), so text is centred exactly.

## Interface

- **Shell:** a frameless window with a custom title bar (drag region, double-click to maximize,
  minimize/maximize/close with tooltips), invisible resize edges, a collapsible sidebar and a
  status bar with the job tray.
- **Tool pages** share `features/ToolWorkspace.tsx`: drop zone and file list, a settings panel
  with output name and location, validation messages, and the processing view
  (`components/processing`). Tools with a single combined result (merge, images to PDF) send one
  job; batch tools send one job per file.
- **PDF.js** renders thumbnails and the viewer. Bytes are read through the `read_file_bytes`
  command, so the webview has no filesystem access. Documents are cached and destroyed after a
  minute unused; thumbnails are rendered lazily, at most three at a time, and kept in a bounded
  cache.
- **Drag and drop** uses Tauri's native drop event (it provides real paths); the screen shown
  registers the handler. Reordering uses pointer events.
- **Design tokens** live in `src/styles/index.css` (the Dayweave colour theme) and are exposed as
  Tailwind utilities; components never use raw colours.

## Security boundaries

- The webview has no shell, no network and no filesystem permission. Its capabilities
  (`src-tauri/capabilities/default.json`) allow only window controls and the open-file dialog.
- The content security policy allows only the app's own scripts, styles and fonts.
- Every path from the interface must be absolute and is validated by type before use.
- Passwords reach qpdf through standard input, never the command line, and are never stored
  or logged. `PdfOperation` deliberately does not implement `Debug`.
- Settings and activity are written atomically (temporary file and rename) and corrupt files fall
  back to defaults.

## Data on disk

| What | Where (outside the Snap) |
| --- | --- |
| Settings | `~/.config/io.github.ammarqaisar11a55.paperflow/settings.json` |
| Recent activity | `~/.local/share/io.github.ammarqaisar11a55.paperflow/activity.json` |
| Job folders (deleted after each job) | `/tmp/paperflow-<uid>/jobs/` |

Inside the Snap the first two live under `~/snap/paperflow/`, and `/tmp` is private to the snap.

## Source layout

```text
src/                     interface
  app/                   App and screens (dashboard, library, viewer, settings)
  components/            ui, layout, files, pdf, processing, tools
  features/              one folder per tool group, registry, ToolWorkspace
  hooks/ services/ stores/ styles/ types/ utils/ test/ dev/
src-tauri/src/
  commands/              Tauri commands
  services/              operations, jobs, engines, store, context
  pdf/                   document, assemble, overlay, geometry, fonts, images, ranges
  pdf/ops/               organize, stamp, metadata, convert, compress, security, ocr
  filesystem/            paths, temp
  ipc_tests.rs           IPC contract tests on Tauri's mock runtime
snap/ packaging/         Snap and AppStream metadata
scripts/ tests/fixtures/ docs/
```
