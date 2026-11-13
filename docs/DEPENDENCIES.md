# Dependencies and licenses

PaperFlow itself is released under the MIT License. It uses the components below.

## Processing engines

These are separate programs that PaperFlow runs; the snap includes them unchanged from Ubuntu
24.04, and the `.deb` depends on the distribution's packages. They are not linked into
PaperFlow, so their licenses apply to the programs themselves.

| Engine | Used for | License | Notes |
| --- | --- | --- | --- |
| [Ghostscript](https://www.ghostscript.com) (`gs`) | Compression | AGPL-3.0-or-later | Distributed as an unmodified, separate program, with its source available from Ubuntu (`apt source ghostscript`) and upstream. |
| [Poppler](https://poppler.freedesktop.org) (`pdftoppm`, `pdftotext`, `pdfimages`) | PDF to images and text, image extraction, OCR rendering | GPL-2.0-or-later | Separate, unmodified programs. |
| [poppler-data](https://poppler.freedesktop.org) | Character maps for Asian fonts in Poppler | BSD-3-Clause, GPL-2.0, MIT and Adobe terms | |
| [qpdf](https://qpdf.sourceforge.io) | Password protection and unlocking | Apache-2.0 | |
| [Tesseract](https://github.com/tesseract-ocr/tesseract) | OCR | Apache-2.0 | |
| [Tesseract language data](https://github.com/tesseract-ocr/tessdata) | OCR languages (17 plus orientation) | Apache-2.0 | |
| [LibreOffice](https://www.libreoffice.org) (`soffice`, Writer, Calc and Impress without a desktop interface) | Word, PowerPoint, Excel and HTML to PDF | MPL-2.0 | Separate, unmodified program; its configuration is made relocatable when bundled. |
| [Liberation fonts](https://github.com/liberationfonts/liberation-fonts) | Metric-compatible replacements for Arial, Times New Roman and Courier New | OFL-1.1 | |
| [Carlito](https://github.com/googlefonts/carlito) and [Caladea](https://github.com/huertatipografica/Caladea) | Metric-compatible replacements for Calibri and Cambria | OFL-1.1 and Apache-2.0 | |
| [DejaVu fonts](https://dejavu-fonts.github.io) | General fallback font | Bitstream Vera licence (permissive) | |
| [URW base35 fonts](https://github.com/ArtifexSoftware/urw-base35-fonts) | Fonts Ghostscript needs | AGPL-3.0 with font exception | |

## Rust crates (linked into PaperFlow)

| Crate | Purpose | License |
| --- | --- | --- |
| [tauri](https://tauri.app) and `tauri-build` | Desktop framework | Apache-2.0 OR MIT |
| `tauri-plugin-dialog`, `tauri-plugin-opener` | File dialogs, opening results | Apache-2.0 OR MIT |
| [lopdf](https://crates.io/crates/lopdf) | PDF reading and writing | MIT |
| [image](https://crates.io/crates/image) | Decoding images for Images to PDF and watermarks | MIT OR Apache-2.0 |
| [flate2](https://crates.io/crates/flate2) | Compressing embedded images | MIT OR Apache-2.0 |
| [quick-xml](https://crates.io/crates/quick-xml) | Reading Poppler's layout for PDF to Markdown | MIT |
| [libc](https://crates.io/crates/libc) | Stopping an engine's whole process group on cancel | MIT OR Apache-2.0 |
| serde, serde_json | Serialization | MIT OR Apache-2.0 |
| chrono | Dates in metadata and activity | MIT OR Apache-2.0 |
| uuid | Job ids | MIT OR Apache-2.0 |
| log | Logging (never document contents) | MIT OR Apache-2.0 |
| tempfile (tests only) | Temporary folders in tests | MIT OR Apache-2.0 |

## JavaScript packages (bundled into the interface)

| Package | Purpose | License |
| --- | --- | --- |
| [react](https://react.dev), react-dom | Interface | MIT |
| [pdfjs-dist](https://mozilla.github.io/pdf.js/) | Thumbnails and the viewer, with its character maps, standard fonts and decoders | Apache-2.0 |
| @tauri-apps/api and plugins | Talking to the backend | Apache-2.0 OR MIT |
| [zustand](https://github.com/pmndrs/zustand) | State | MIT |
| [lucide-react](https://lucide.dev) | Icons | ISC |
| [@fontsource-variable/inter](https://fontsource.org/fonts/inter) | Inter typeface | OFL-1.1 |

Build and test tools (not shipped): Vite, Tailwind CSS (MIT), TypeScript (Apache-2.0), Vitest,
Testing Library and jsdom (MIT).

## Design

The colour theme is shared with Dayweave by the same author. The PaperFlow icon is original.

## Checking

```bash
npm ls --omit=dev
```

```bash
cargo tree --manifest-path src-tauri/Cargo.toml -e normal
```
