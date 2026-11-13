# Screenshots

Screens of PaperFlow used in the documentation, in the light and dark themes.
They show the sample document `tests/fixtures/sample-report.pdf`.

| File | Screen |
| --- | --- |
| `dashboard-light.png`, `dashboard-dark.png` | Dashboard |
| `all-tools-light.png` | All tools, with favorites marked |
| `organize-light.png`, `organize-dark.png` | Page organizer |
| `viewer-light.png` | PDF viewer |
| `watermark-light.png` | Watermark PDF with live preview |
| `page-numbers-dark.png` | Page Numbers with live preview |
| `compress-dark.png` | Compress PDF |
| `settings-light.png` | Settings |

## Regenerating

The screenshots come from the development browser preview, which renders the
real interface and the sample PDF with Tauri's IPC mock. Start it, then run
the capture script (it needs Google Chrome or Chromium):

```bash
npm run dev
```

```bash
node scripts/capture-screenshots.mjs
```
