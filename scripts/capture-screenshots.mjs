#!/usr/bin/env node
// Captures the documentation screenshots in docs/screenshots.
//
// Uses the development browser preview (`npm run dev`, which answers backend
// calls with Tauri's IPC mock and renders the real sample PDF) and a headless
// Chrome driven over the DevTools protocol. Each screen gets real time to
// finish rendering, so PDF.js thumbnails appear.
//
// Usage:
//   npm run dev            # in another terminal
//   node scripts/capture-screenshots.mjs [chrome-binary]
import { spawn } from "node:child_process";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const CHROME = process.argv[2] ?? process.env.CHROME ?? "google-chrome";
const BASE = process.env.PREVIEW_URL ?? "http://localhost:1420/";
const OUT = join(process.cwd(), "docs", "screenshots");
const PORT = 9333;
const SIZE = { width: 1440, height: 900 };

const SHOTS = [
  ["dashboard-light", "theme=light"],
  ["dashboard-dark", "theme=dark"],
  ["all-tools-light", "theme=light&fav=compress,ocr,watermark&view=tools"],
  ["organize-light", "theme=light&tool=organize&file"],
  ["organize-dark", "theme=dark&tool=organize&file"],
  ["viewer-light", "theme=light&view=viewer&file"],
  ["watermark-light", "theme=light&tool=watermark&file"],
  ["page-numbers-dark", "theme=dark&tool=page-numbers&file"],
  ["compress-dark", "theme=dark&fav=compress&tool=compress&file"],
  ["word-to-pdf-light", "theme=light&tool=word-to-pdf&file=docx"],
  ["pdf-to-markdown-dark", "theme=dark&tool=pdf-to-markdown&file"],
  ["settings-light", "theme=light&view=settings"],
];

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitForChrome() {
  for (let i = 0; i < 50; i++) {
    try {
      return await (await fetch(`http://127.0.0.1:${PORT}/json/version`)).json();
    } catch {
      await sleep(200);
    }
  }
  throw new Error("Chrome did not start");
}

function connect(url) {
  const ws = new WebSocket(url);
  let id = 0;
  const pending = new Map();
  ws.addEventListener("message", (e) => {
    const msg = JSON.parse(e.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      msg.error ? reject(new Error(msg.error.message)) : resolve(msg.result);
    }
  });
  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const n = ++id;
      pending.set(n, { resolve, reject });
      ws.send(JSON.stringify({ id: n, method, params }));
    });
  return new Promise((resolve) => ws.addEventListener("open", () => resolve({ send, close: () => ws.close() })));
}

const profile = mkdtempSync(join(tmpdir(), "paperflow-shots-"));
const chrome = spawn(
  CHROME,
  [
    "--headless=new",
    `--remote-debugging-port=${PORT}`,
    `--user-data-dir=${profile}`,
    "--hide-scrollbars",
    "--no-first-run",
    "--no-default-browser-check",
    `--window-size=${SIZE.width},${SIZE.height}`,
    "about:blank",
  ],
  { stdio: "ignore" },
);

try {
  await waitForChrome();
  mkdirSync(OUT, { recursive: true });
  for (const [name, hash] of SHOTS) {
    const target = await (await fetch(`http://127.0.0.1:${PORT}/json/new?about:blank`, { method: "PUT" })).json();
    const page = await connect(target.webSocketDebuggerUrl);
    await page.send("Emulation.setDeviceMetricsOverride", { ...SIZE, deviceScaleFactor: 1, mobile: false });
    await page.send("Page.enable");
    // A cache-busting query makes every capture a fresh load of the preview.
    await page.send("Page.navigate", { url: `${BASE}?shot=${name}#${hash}` });
    await sleep(4500);
    const { data } = await page.send("Page.captureScreenshot", { format: "png" });
    writeFileSync(join(OUT, `${name}.png`), Buffer.from(data, "base64"));
    console.log(`docs/screenshots/${name}.png`);
    page.close();
    await fetch(`http://127.0.0.1:${PORT}/json/close/${target.id}`).catch(() => undefined);
  }
} finally {
  chrome.kill();
  await sleep(300);
  rmSync(profile, { recursive: true, force: true });
}
