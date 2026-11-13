import type { LucideIcon } from "lucide-react";
import { FileCode, FilePlus2, FileSpreadsheet, FileType, ImagePlus, Presentation } from "lucide-react";

/** The kinds of file a tool can take. Mirrors the backend's file inspection. */
export type InputKind = "pdf" | "image" | "word" | "presentation" | "spreadsheet" | "html";

export interface KindInfo {
  /** "a PDF", "an image"… */
  one: string;
  /** "PDF files", "images"… */
  many: string;
  /** Shown under the drop zone. */
  types: string;
  extensions: string[];
  icon: LucideIcon;
  /** Name of the file-dialog filter. */
  filter: string;
}

export const KINDS: Record<InputKind, KindInfo> = {
  pdf: {
    one: "a PDF",
    many: "PDF files",
    types: "PDF",
    extensions: ["pdf"],
    icon: FilePlus2,
    filter: "PDF documents",
  },
  image: {
    one: "an image",
    many: "images",
    types: "PNG, JPEG, WebP, TIFF, BMP, GIF",
    extensions: ["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif"],
    icon: ImagePlus,
    filter: "Images",
  },
  word: {
    one: "a Word document",
    many: "Word documents",
    types: "DOCX, DOC, ODT, RTF",
    extensions: ["docx", "doc", "docm", "dot", "dotx", "odt", "fodt", "rtf"],
    icon: FileType,
    filter: "Word documents",
  },
  presentation: {
    one: "a PowerPoint presentation",
    many: "presentations",
    types: "PPTX, PPT, ODP",
    extensions: ["pptx", "ppt", "pptm", "pps", "ppsx", "odp", "fodp"],
    icon: Presentation,
    filter: "Presentations",
  },
  spreadsheet: {
    one: "an Excel spreadsheet",
    many: "spreadsheets",
    types: "XLSX, XLS, ODS, CSV",
    extensions: ["xlsx", "xls", "xlsm", "ods", "fods", "csv"],
    icon: FileSpreadsheet,
    filter: "Spreadsheets",
  },
  html: {
    one: "an HTML page",
    many: "HTML pages",
    types: "HTML, HTM",
    extensions: ["html", "htm", "xhtml"],
    icon: FileCode,
    filter: "Web pages",
  },
};

/** Dialog filter, with upper-case variants for case-sensitive filesystems. */
export function dialogFilter(kind: InputKind) {
  const info = KINDS[kind];
  return { name: info.filter, extensions: [...info.extensions, ...info.extensions.map((e) => e.toUpperCase())] };
}

/** Every kind PaperFlow can work with, for the dashboard's file picker. */
export const ALL_KINDS: InputKind[] = ["pdf", "word", "presentation", "spreadsheet", "html", "image"];
