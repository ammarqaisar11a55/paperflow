// Typed wrappers around the Rust commands. This is the only module that
// talks to the backend, which keeps components testable with a mock.
import { invoke } from "@tauri-apps/api/core";
import type {
  ActivityEntry,
  AppError,
  AppInfo,
  EngineInfo,
  FileInfo,
  JobSnapshot,
  PdfMetadata,
  PdfOperation,
  Settings,
} from "@/types";

/** Normalises anything thrown by `invoke` into an AppError. */
export function toAppError(err: unknown): AppError {
  if (err && typeof err === "object" && "message" in err && "kind" in err) {
    return err as AppError;
  }
  const message = err instanceof Error ? err.message : typeof err === "string" ? err : "Something went wrong.";
  return { kind: "processing", message, hint: null, detail: null };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (err) {
    throw toAppError(err);
  }
}

export const api = {
  inspectFiles: (paths: string[]) => call<FileInfo[]>("inspect_files", { paths }),
  readFileBytes: (path: string) => call<ArrayBuffer>("read_file_bytes", { path }),
  readMetadata: (path: string) => call<PdfMetadata>("read_metadata", { path }),
  suggestOutputDir: (input?: string | null) => call<string>("suggest_output_dir", { input: input ?? null }),

  startJob: (operation: PdfOperation) => call<string>("start_job", { operation }),
  cancelJob: (id: string) => call<boolean>("cancel_job", { id }),
  listJobs: () => call<JobSnapshot[]>("list_jobs"),
  clearFinishedJobs: () => call<void>("clear_finished_jobs"),

  getSettings: () => call<Settings>("get_settings"),
  saveSettings: (settings: Settings) => call<Settings>("save_settings", { settings }),
  getActivity: () => call<ActivityEntry[]>("get_activity"),
  removeActivity: (id: string) => call<void>("remove_activity", { id }),
  clearActivity: () => call<void>("clear_activity"),

  engineStatus: () => call<EngineInfo[]>("engine_status"),
  ocrLanguages: () => call<string[]>("ocr_languages"),
  openPath: (path: string) => call<void>("open_path", { path }),
  revealPath: (path: string) => call<void>("reveal_path", { path }),
  tempUsage: () => call<number>("temp_usage"),
  clearTemp: () => call<number>("clear_temp"),
  appInfo: () => call<AppInfo>("app_info"),
};

export type Api = typeof api;
