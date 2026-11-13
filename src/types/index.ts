// Types shared with the Rust backend (see src-tauri/src/services).

export type PdfOperationType =
  | "merge"
  | "split"
  | "compress"
  | "organize"
  | "rotate"
  | "extractPages"
  | "pdfToImages"
  | "imagesToPdf"
  | "extractImages"
  | "pdfToText"
  | "watermark"
  | "pageNumbers"
  | "metadata"
  | "protect"
  | "unlock"
  | "ocr";

export type OverwritePolicy = "rename" | "overwrite";

export interface PdfOperation {
  type: PdfOperationType;
  inputFiles: string[];
  outputDir: string;
  outputName?: string;
  options: Record<string, unknown>;
  overwrite?: OverwritePolicy;
}

export interface ProcessingResult {
  success: boolean;
  outputFiles: string[];
  outputFolder: string | null;
  errors: string[];
  warnings: string[];
  inputSize: number;
  outputSize: number;
  pageCount: number | null;
}

export type ErrorKind =
  | "invalidInput"
  | "encrypted"
  | "wrongPassword"
  | "missingEngine"
  | "output"
  | "cancelled"
  | "processing";

export interface AppError {
  kind: ErrorKind;
  message: string;
  hint: string | null;
  detail: string | null;
}

export type JobStatus = "queued" | "running" | "completed" | "failed" | "cancelled";

export interface JobSnapshot {
  id: string;
  type: PdfOperationType;
  title: string;
  inputNames: string[];
  status: JobStatus;
  progress: number | null;
  message: string;
  currentFile: string | null;
  result: ProcessingResult | null;
  error: AppError | null;
  createdAt: number;
  finishedAt: number | null;
}

export type FileKind = "pdf" | "image" | "unsupported";

export interface FileInfo {
  path: string;
  name: string;
  size: number;
  kind: FileKind;
  pageCount: number | null;
  encrypted: boolean;
  error: string | null;
}

export type ThemePreference = "system" | "light" | "dark";
export type OutputLocation = "sameAsSource" | "custom";
export type CompressionPreset = "extreme" | "recommended" | "light";

export interface Settings {
  theme: ThemePreference;
  outputLocation: OutputLocation;
  outputDir: string | null;
  compressionPreset: CompressionPreset;
  imageDpi: number;
  imageFormat: "png" | "jpeg";
  overwrite: OverwritePolicy;
  ocrLanguage: string;
  favorites: string[];
  sidebarCollapsed: boolean;
  recordActivity: boolean;
}

export interface ActivityEntry {
  id: string;
  type: PdfOperationType;
  title: string;
  fileNames: string[];
  status: "completed" | "failed" | "cancelled";
  timestamp: number;
  outputFiles: string[];
  outputFolder: string | null;
  error: string | null;
}

export interface EngineInfo {
  engine: string;
  label: string;
  binary: string;
  available: boolean;
  version: string | null;
}

export interface AppInfo {
  name: string;
  version: string;
  identifier: string;
  repository: string;
  license: string;
  sandboxed: boolean;
  tessdata: string | null;
}

export interface PdfMetadata {
  title: string;
  author: string;
  subject: string;
  keywords: string;
  creator: string;
  producer: string;
}
