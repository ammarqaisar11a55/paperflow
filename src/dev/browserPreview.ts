// Development-only: lets the interface run in an ordinary browser tab (for
// layout and styling work) by answering Tauri IPC calls with Tauri's official
// mock. It is loaded only by `vite dev` outside the desktop app and is never
// part of a production build. It does not imitate processing: starting a job
// reports that processing needs the desktop app.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { FileInfo, Settings } from "@/types";

const FIXTURE = "/home/preview/Documents/Quarterly Report.pdf";
let settings: Settings | null = null;

/**
 * Opens a screen from the URL hash, e.g. `#tool=organize&file` or
 * `#view=settings&theme=light`, so individual screens can be captured.
 */
export async function applyPreviewHash() {
  const params = new URLSearchParams(location.hash.slice(1));
  const { useNavigation } = await import("@/stores/navigation");
  const { useSettings } = await import("@/stores/settings");
  const theme = params.get("theme");
  if (theme === "light" || theme === "dark") await useSettings.getState().update({ theme });
  const files = params.has("file") ? [FIXTURE] : undefined;
  const tool = params.get("tool");
  const view = params.get("view");
  if (tool) useNavigation.getState().openTool(tool as never, files);
  else if (view) useNavigation.getState().navigate({ view: view as never }, files);
}

export function installBrowserPreview() {
  mockWindows("main");
  mockIPC(async (cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "get_settings":
        return settings ?? (settings = (await import("@/stores/settings")).DEFAULT_SETTINGS);
      case "save_settings":
        settings = a.settings as Settings;
        return settings;
      case "get_activity":
      case "list_jobs":
        return [];
      case "inspect_files":
        return (a.paths as string[]).map<FileInfo>((path) => ({
          path,
          name: path.split("/").pop() ?? path,
          size: 18_342,
          kind: path.endsWith(".pdf") ? "pdf" : "image",
          pageCount: 6,
          encrypted: false,
          error: null,
        }));
      case "read_file_bytes":
        return (await fetch("/tests/fixtures/sample-report.pdf")).arrayBuffer();
      case "read_metadata":
        return { title: "Quarterly Report", author: "", subject: "", keywords: "", creator: "", producer: "Ghostscript" };
      case "suggest_output_dir":
        return "/home/preview/Documents";
      case "ocr_languages":
        return ["deu", "eng", "fra"];
      case "engine_status":
        return [];
      case "temp_usage":
        return 0;
      case "app_info":
        return { name: "PaperFlow", version: "0.1.0 (browser preview)", identifier: "", repository: "", license: "MIT", sandboxed: false, tessdata: null };
      case "plugin:dialog|open":
        return [FIXTURE];
      case "start_job":
        throw { kind: "processing", message: "Processing runs in the desktop app.", hint: "Start PaperFlow with npm run tauri dev.", detail: null };
      default:
        return null;
    }
  });
}
