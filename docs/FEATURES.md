# Features

Every tool runs on your computer. Each one shares the same layout: add files (drag them onto the
window or browse), adjust the settings, choose where to save, and start. Progress, results and
errors appear in the same place, and the status bar shows every running job.

Originals are never modified. Results get a new name, and an existing file is never replaced
unless you allow it in Settings; even then, an input file is never overwritten.

## Organize

| Tool | What it does | Options |
| --- | --- | --- |
| **Merge PDF** | Combines several PDFs into one, in the order shown. | Drag rows or use the arrows to reorder; sort by name. |
| **Split PDF** | Creates new PDFs from parts of one document, saved in a new folder. | By ranges (`1-3, 4-6, 7-`), every page, or files of a fixed number of pages. Shows how many files will be created. |
| **Organize PDF** | A visual page workspace with real page thumbnails. | Drag to reorder (multi-page selections move together), rotate, duplicate, delete, extract the selection, undo and redo, thumbnail size. Keyboard: `R` / `Shift+R` rotate, `Ctrl+D` duplicate, `Del` delete, `Ctrl+A` select all, `Ctrl+Z` / `Ctrl+Shift+Z` undo and redo. |
| **Rotate PDF** | Turns all pages or selected ones. Everything else in the document (bookmarks, links, forms) is kept. | Left, right or 180°; page range; live preview of which pages turn. |
| **Extract Pages** | Copies chosen pages, in the order given, into a new PDF. | Type a range or click thumbnails. |

## Optimize

| Tool | What it does | Options |
| --- | --- | --- |
| **Compress PDF** | Reduces file size with Ghostscript and reports the original size, new size and percentage saved. If a file cannot get smaller, PaperFlow says so and keeps an unchanged copy rather than a larger one. | Extreme (72 dpi images), Recommended (150 dpi), Light (300 dpi); convert to grayscale. |

## Convert to PDF

Word, PowerPoint, Excel and HTML files are converted by LibreOffice, running on your computer
with a fresh private profile for every file. Macros never run, and linked content from the
internet is never downloaded.

| Tool | What it does | Options |
| --- | --- | --- |
| **Word to PDF** | Converts DOCX, DOC, DOCM, DOTX, ODT and RTF documents. Fonts that aren't installed are replaced with metric-compatible ones (Carlito for Calibri, Caladea for Cambria, Liberation for Arial, Times New Roman and Courier New), so line breaks match closely. | PDF/A for archiving. |
| **PowerPoint to PDF** | Converts PPTX, PPT, PPSX and ODP presentations, one slide per page. Animations and embedded video are not included. | PDF/A for archiving. |
| **Excel to PDF** | Converts XLSX, XLS, XLSM, ODS and CSV spreadsheets, every sheet, using each sheet's print settings. | Each sheet on one page; PDF/A for archiving. |
| **HTML to PDF** | Converts saved web pages and HTML documents. Scripts don't run; complex web layouts may differ from a browser's rendering. | PDF/A for archiving. |
| **Images to PDF** | See below. | |

The result takes the document's name (`Report.docx` becomes `Report.pdf`).

## Convert from PDF

| Tool | What it does | Options |
| --- | --- | --- |
| **PDF to Markdown** | Rebuilds the document's structure as Markdown: headings from larger text (`#`, `##`, `###`), paragraphs with hyphenated words rejoined, bullet and numbered lists, bold, italic and links. Running headers, footers and page numbers repeated on most pages are dropped. Tables come out as plain text in reading order; scanned pages need OCR PDF first. | Page range; detect headings; separate pages with a rule. |
| **PDF to Images** | Renders pages to images in a new folder. | PNG or JPEG, 72–600 dpi, JPEG quality, page range. |
| **Images to PDF** (Convert to PDF) | Turns images into one PDF, one image per page. JPEG photos are embedded unchanged (no quality loss) and EXIF rotation is respected; transparency is kept. | Page size (same as image, A4, Letter, Legal), orientation, margin. Accepts PNG, JPEG, WebP, BMP, TIFF and GIF. |
| **Extract Images** | Saves the pictures embedded in a PDF at their original resolution and format. | Page range. |
| **PDF to Text** | Exports text to a `.txt` file, with pages separated by form feeds. Warns when pages have no text (likely scans). | Keep layout or plain reading order; page range. |

## Edit

| Tool | What it does | Options |
| --- | --- | --- |
| **Watermark PDF** | Draws text or an image over pages, upright on rotated pages too, with a live preview. | Text, font, size and colour, or an image and its width; nine positions; opacity; rotation; page range. |
| **Page Numbers** | Adds numbers with a live preview. | Six positions, `1` or `1 of 12`, prefix and suffix, starting number, font, size, colour, which pages to number. |
| **Metadata Editor** | Shows and edits the document information. | Title, author, subject, keywords, creator, producer. Empty fields are removed. |

Watermarks and page numbers use the PDF standard fonts (Helvetica, Times and Courier), which
every reader has. They cover Western European characters; others are replaced by `?` with a
warning.

## Security

| Tool | What it does | Options |
| --- | --- | --- |
| **Protect PDF** | Encrypts with a password using qpdf. | AES-256 or AES-128; owner password (or a random one); printing (allowed, low resolution, not allowed); copying, editing, comments and form filling. |
| **Unlock PDF** | Removes protection from a PDF whose password you know. | The document's open or owner password. |

PaperFlow cannot recover or guess passwords and does not try. Tools other than Unlock refuse
protected files and point to Unlock PDF instead.

## OCR

**OCR PDF** makes scanned documents searchable and selectable. Each page is rendered, recognized
with Tesseract, and stored as the page image with an invisible text layer.

Options: document language and an optional second language; scan quality of 200, 300 or 400 dpi.
The snap includes English, Arabic, Chinese (simplified and traditional), Dutch, French, German,
Hindi, Italian, Japanese, Korean, Polish, Portuguese, Russian, Spanish, Turkish and Urdu.
Settings › OCR lists what is installed.

## Viewer

Continuous scrolling with fit width, fit page and zoom from 50% to 400%; a thumbnail sidebar;
page number entry; text search with highlighted matches and next/previous; selectable text;
full screen (`F11`). `Ctrl`-click thumbnails to select pages and extract them.

## Batch processing

Most tools (Rotate, Compress, every conversion except Images to PDF, Watermark, Page Numbers,
Protect, Unlock, OCR) accept several files and process each one separately. Two
jobs run at a time; the rest wait in the queue. Queued and running jobs can be cancelled; a
cancelled job leaves no files behind.

## Dashboard, favorites and recent activity

Drop files on the dashboard and PaperFlow suggests tools that fit them: the right converter for
Word, PowerPoint, Excel, HTML and image files, and common tools for PDFs. Star a tool to keep it
under Favorites. Recent activity lists file names, the tool, the outcome, the time and where the
result was saved, with a button to show it in the file manager.

## Settings

- **Appearance:** system, light or dark theme; compact sidebar.
- **Processing:** default output folder (next to the original, or a fixed folder), compression
  preset, image resolution and format, and whether to keep both files or replace an existing
  result.
- **Privacy:** turn recent activity off, clear it, and clear temporary files.
- **OCR:** default language and installed language data.
- **About:** version, license, package type and the status and version of every engine.

## Keyboard shortcuts

| Shortcut | Action |
| --- | --- |
| `Ctrl+K` | Search tools |
| `Ctrl+,` | Settings |
| `Ctrl+B` | Collapse or expand the sidebar |
| `Ctrl+O` | Add files (in a tool) or open a PDF (in the viewer) |
| `Ctrl+Enter` | Start the tool |
| `Alt+←` | Back |
| `Ctrl+F`, `Ctrl+±`, `Ctrl+0`, `F11` | Viewer: search, zoom, fit width, full screen |
