// Shared mocks for component tests. The backend is replaced at the module
// boundary (src/services/api.ts); PDF.js rendering is replaced because jsdom
// has no canvas.
import { vi } from "vitest";
import type { FileInfo, PdfOperation } from "@/types";

export function pdfInfo(name: string, pages = 3, extra: Partial<FileInfo> = {}): FileInfo {
  return {
    path: `/home/test/${name}`,
    name,
    size: 1024 * pages,
    kind: "pdf",
    pageCount: pages,
    encrypted: false,
    error: null,
    ...extra,
  };
}

export const apiMock = {
  inspectFiles: vi.fn(async (paths: string[]) => paths.map((p) => pdfInfo(p.split("/").pop()!))),
  readFileBytes: vi.fn(async () => new ArrayBuffer(0)),
  readMetadata: vi.fn(async () => ({ title: "", author: "", subject: "", keywords: "", creator: "", producer: "" })),
  suggestOutputDir: vi.fn(async () => "/home/test/Documents"),
  startJob: vi.fn(async (_operation: PdfOperation) => "job-1"),
  cancelJob: vi.fn(async () => true),
  listJobs: vi.fn(async () => []),
  clearFinishedJobs: vi.fn(async () => undefined),
  getSettings: vi.fn(async () => {
    const { DEFAULT_SETTINGS } = await import("@/stores/settings");
    return DEFAULT_SETTINGS;
  }),
  saveSettings: vi.fn(async (s: unknown) => s),
  getActivity: vi.fn(async () => []),
  removeActivity: vi.fn(async () => undefined),
  clearActivity: vi.fn(async () => undefined),
  engineStatus: vi.fn(async () => []),
  ocrLanguages: vi.fn(async () => ["eng"]),
  openPath: vi.fn(async () => undefined),
  revealPath: vi.fn(async (_path: string) => undefined),
  tempUsage: vi.fn(async () => 0),
  clearTemp: vi.fn(async () => 0),
  appInfo: vi.fn(async () => ({ name: "PaperFlow", version: "0.1.0", identifier: "", repository: "", license: "MIT", sandboxed: false, tessdata: null })),
};
