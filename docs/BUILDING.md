# Building, testing and packaging

## Requirements

- Node.js 22 and npm
- Rust (stable) from [rustup](https://rustup.rs)
- The Tauri system libraries:

```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev libssl-dev build-essential pkg-config
```

- The processing engines, for the tools that use them:

```bash
sudo apt install ghostscript poppler-utils qpdf tesseract-ocr tesseract-ocr-eng libreoffice-writer-nogui libreoffice-calc-nogui libreoffice-impress-nogui fonts-crosextra-carlito fonts-crosextra-caladea
```

Without root, `scripts/dev-engines.sh` downloads the Ubuntu packages for any missing engine into
`~/.cache/paperflow-engines` (nothing is installed system-wide) and prints the variables
PaperFlow reads to find them:

```bash
eval "$(scripts/dev-engines.sh)"
```

## Running

```bash
npm ci
```

```bash
npm run tauri dev
```

The interface alone also runs in a browser with `npm run dev` (http://localhost:1420). Outside
the desktop app, backend calls are answered by Tauri's IPC mock with the sample document
`tests/fixtures/sample-report.pdf`; processing itself only works in the desktop app. The URL hash
opens a screen directly, for example `#tool=organize&file&theme=dark`.

## Testing

| Suite | Command | Covers |
| --- | --- | --- |
| Frontend | `npm test` | Page ranges, organizer history, tool search, title bar window controls, form controls, tool validation and job submission, processing, completion and error states, navigation (Vitest, React Testing Library). |
| Type checking | `npm run typecheck` | The whole interface, tests included. |
| Backend | `npm run test:rust` | Path safety, output naming, option parsing, every PDF operation on generated documents (page order, rotation, text content, page counts), the job queue and cancellation, settings persistence, and the IPC contract on Tauri's mock runtime. |
| Engines | included in `npm run test:rust` | Compression, rendering, text extraction, image extraction, encryption and wrong passwords, OCR of a rendered scan, Word, PowerPoint, Excel and HTML to PDF (with a remote image that must not be fetched), PDF to Markdown, and awkward file names, all against the real programs. Each test is skipped, with a note, when its engine is missing. |

Lint and format the Rust code the way CI does:

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml --all --check
```

```bash
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

## Building packages

### .deb

```bash
npm run tauri build -- --bundles deb
```

The package is written to `src-tauri/target/release/bundle/deb/`. It depends on the
distribution's engine packages.

### Snap

The snap (`snap/snapcraft.yaml`) uses `core24` and the GNOME extension, runs under strict
confinement, and bundles the engines, the URW fonts Ghostscript needs, Poppler's data,
Tesseract language data, LibreOffice (Writer, Calc and Impress without a desktop interface) and
Office-compatible fonts. The build runs `scripts/relink-libreoffice.sh` so LibreOffice works from
`$SNAP`. Its plugs are `home`, `removable-media` and `browser-support` (for
WebKitGTK's web process sandbox), plus those of the GNOME extension. It has no `network` plug.

Building locally needs LXD:

```bash
sudo snap install lxd
```

```bash
sudo lxd init --auto
```

```bash
snapcraft pack
```

Then install and try the result:

```bash
sudo snap install --dangerous paperflow_*.snap
```

Check that every engine converts documents inside the installed snap (the Snap and Release
workflows run the same check, and a release is not published if it fails):

```bash
cp scripts/snap-smoke-test.sh ~/ && snap run --shell paperflow -c "bash $HOME/snap-smoke-test.sh"
```

CI builds the snap on every change to its inputs (`.github/workflows/snap.yml`), installs it and
checks that every engine and the English language data are present.

### Checking an installed snap

1. Launch PaperFlow from the application menu (desktop entry and icon).
2. Settings › About lists every engine as available, with its version.
3. Settings › OCR lists the bundled languages.
4. Open a PDF in your home folder and on a USB drive; run Compress, Protect, PDF to Images and
   OCR on it; use "Show in folder" and "Open file" on the results.
5. Turn off the network and repeat: everything still works.

## Releasing

1. Set the same version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json`, and add a release entry to
   `packaging/io.github.ammarqaisar11a55.paperflow.metainfo.xml`.
2. Tag the commit `vX.Y.Z` and push the tag.
3. `.github/workflows/release.yml` checks the versions, runs the tests, builds the `.deb` and the
   snap, publishes a GitHub release with both and a `SHA256SUMS` file, and uploads the snap to
   the Snap Store's stable channel.

The store upload needs the `SNAPCRAFT_STORE_CREDENTIALS` repository secret. Create it once from
an account that owns the `paperflow` name (the credentials go straight into GitHub without being
shown):

```bash
snapcraft export-login --snaps=paperflow --acls=package_access,package_push,package_update,package_release - | gh secret set SNAPCRAFT_STORE_CREDENTIALS --repo ammarqaisar11a55/paperflow
```

Without the secret, the release is still published and the store step only leaves a notice; upload
the snap from the release by hand instead:

```bash
snapcraft upload --release=stable paperflow_X.Y.Z_amd64.snap
```

## Documentation screenshots

`docs/screenshots` is regenerated from the browser preview with
`node scripts/capture-screenshots.mjs` while `npm run dev` runs. See
[screenshots/README.md](screenshots/README.md).

## Troubleshooting

| Problem | Fix |
| --- | --- |
| A tool says Ghostscript, qpdf, Poppler or Tesseract "is not available" | Install the package named in the message, or use the snap, which includes them. Settings › About shows what PaperFlow found. |
| "Language data for … is not installed" | Install `tesseract-ocr-<language>` (outside the snap), or choose a listed language. |
| Compression fails with a permission error outside the snap | Update PaperFlow: engines now work on private copies in `/tmp`, which Ubuntu's AppArmor profiles allow. |
| The window opens blank in development | Run `npm run dev` once so `public/pdfjs` is created, or use `npm run tauri dev`, which does it for you. |
| Files on another drive cannot be opened in the snap | Connect the removable-media interface: `sudo snap connect paperflow:removable-media`. |
| A protected PDF is refused by every tool | Unlock it first with Unlock PDF and its password. |
| Office or HTML conversion says LibreOffice "is not available" | Install the packages listed under Requirements, or use the snap. |
| Word, PowerPoint, Excel or HTML to PDF fails in snap 0.2.0 with "Fatal exception: Signal 6" | LibreOffice could not find its configuration in that build. Fixed in 0.2.1; update with `sudo snap refresh paperflow`. |
| A converted Word document's lines break differently | A font it uses isn't installed. Install the font, or `fonts-crosextra-carlito` and `fonts-crosextra-caladea` for Calibri and Cambria. |
