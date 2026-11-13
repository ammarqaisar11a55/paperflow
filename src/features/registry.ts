import type { LucideIcon } from "lucide-react";
import {
  Combine,
  FileImage,
  FileOutput,
  FileText,
  Hash,
  Images,
  LayoutGrid,
  LockKeyhole,
  LockOpen,
  RotateCw,
  ScanText,
  Scissors,
  Shrink,
  Stamp,
  Tags,
  ImageDown,
} from "lucide-react";
import type { PdfOperationType } from "@/types";

export type ToolCategory = "Organize" | "Optimize" | "Convert" | "Edit" | "Security" | "OCR";

export type ToolId =
  | "merge"
  | "split"
  | "organize"
  | "rotate"
  | "extract-pages"
  | "compress"
  | "pdf-to-images"
  | "images-to-pdf"
  | "extract-images"
  | "pdf-to-text"
  | "watermark"
  | "page-numbers"
  | "metadata"
  | "protect"
  | "unlock"
  | "ocr";

export interface ToolDefinition {
  id: ToolId;
  operation: PdfOperationType;
  title: string;
  description: string;
  icon: LucideIcon;
  category: ToolCategory;
  /** What the tool takes as input. */
  accepts: "pdf" | "image";
  /** "many" combines files into one result, "batch" runs once per file, "one" takes a single file. */
  inputs: "many" | "batch" | "one";
  /** Verb for the primary button. */
  action: string;
  keywords: string[];
}

export const TOOLS: ToolDefinition[] = [
  {
    id: "merge",
    operation: "merge",
    title: "Merge PDF",
    description: "Combine multiple PDFs into one document, in the order you choose.",
    icon: Combine,
    category: "Organize",
    accepts: "pdf",
    inputs: "many",
    action: "Merge PDFs",
    keywords: ["combine", "join", "append"],
  },
  {
    id: "split",
    operation: "split",
    title: "Split PDF",
    description: "Separate pages into new files by ranges, single pages or chunks.",
    icon: Scissors,
    category: "Organize",
    accepts: "pdf",
    inputs: "one",
    action: "Split PDF",
    keywords: ["separate", "divide", "cut"],
  },
  {
    id: "organize",
    operation: "organize",
    title: "Organize PDF",
    description: "Reorder, rotate, duplicate and delete pages visually.",
    icon: LayoutGrid,
    category: "Organize",
    accepts: "pdf",
    inputs: "one",
    action: "Save document",
    keywords: ["reorder", "sort", "pages", "delete", "arrange", "thumbnails"],
  },
  {
    id: "rotate",
    operation: "rotate",
    title: "Rotate PDF",
    description: "Turn all pages, or just the ones you pick.",
    icon: RotateCw,
    category: "Organize",
    accepts: "pdf",
    inputs: "batch",
    action: "Rotate",
    keywords: ["turn", "orientation", "landscape"],
  },
  {
    id: "extract-pages",
    operation: "extractPages",
    title: "Extract Pages",
    description: "Copy selected pages into a new PDF.",
    icon: FileOutput,
    category: "Organize",
    accepts: "pdf",
    inputs: "one",
    action: "Extract pages",
    keywords: ["select", "copy", "subset"],
  },
  {
    id: "compress",
    operation: "compress",
    title: "Compress PDF",
    description: "Reduce file size while keeping the quality you need.",
    icon: Shrink,
    category: "Optimize",
    accepts: "pdf",
    inputs: "batch",
    action: "Compress",
    keywords: ["shrink", "reduce", "size", "optimize", "smaller"],
  },
  {
    id: "pdf-to-images",
    operation: "pdfToImages",
    title: "PDF to Images",
    description: "Save pages as PNG or JPEG images.",
    icon: FileImage,
    category: "Convert",
    accepts: "pdf",
    inputs: "batch",
    action: "Convert to images",
    keywords: ["png", "jpg", "jpeg", "export", "render"],
  },
  {
    id: "images-to-pdf",
    operation: "imagesToPdf",
    title: "Images to PDF",
    description: "Turn photos and scans into a single PDF.",
    icon: Images,
    category: "Convert",
    accepts: "image",
    inputs: "many",
    action: "Create PDF",
    keywords: ["jpg", "png", "photo", "scan", "convert"],
  },
  {
    id: "extract-images",
    operation: "extractImages",
    title: "Extract Images",
    description: "Save the pictures embedded in a PDF at their original quality.",
    icon: ImageDown,
    category: "Convert",
    accepts: "pdf",
    inputs: "batch",
    action: "Extract images",
    keywords: ["pictures", "photos", "embedded"],
  },
  {
    id: "pdf-to-text",
    operation: "pdfToText",
    title: "PDF to Text",
    description: "Export the text of a PDF to a plain text file.",
    icon: FileText,
    category: "Convert",
    accepts: "pdf",
    inputs: "batch",
    action: "Export text",
    keywords: ["txt", "extract", "copy"],
  },
  {
    id: "watermark",
    operation: "watermark",
    title: "Watermark PDF",
    description: "Stamp text or an image over your pages.",
    icon: Stamp,
    category: "Edit",
    accepts: "pdf",
    inputs: "batch",
    action: "Add watermark",
    keywords: ["stamp", "confidential", "draft", "logo"],
  },
  {
    id: "page-numbers",
    operation: "pageNumbers",
    title: "Page Numbers",
    description: "Add page numbers in the position and style you like.",
    icon: Hash,
    category: "Edit",
    accepts: "pdf",
    inputs: "batch",
    action: "Add page numbers",
    keywords: ["numbering", "footer", "header", "pagination"],
  },
  {
    id: "metadata",
    operation: "metadata",
    title: "Metadata Editor",
    description: "Edit the title, author, subject and keywords.",
    icon: Tags,
    category: "Edit",
    accepts: "pdf",
    inputs: "one",
    action: "Save metadata",
    keywords: ["properties", "title", "author", "info"],
  },
  {
    id: "protect",
    operation: "protect",
    title: "Protect PDF",
    description: "Encrypt with a password and restrict printing or copying.",
    icon: LockKeyhole,
    category: "Security",
    accepts: "pdf",
    inputs: "batch",
    action: "Protect",
    keywords: ["password", "encrypt", "secure", "lock", "permissions"],
  },
  {
    id: "unlock",
    operation: "unlock",
    title: "Unlock PDF",
    description: "Remove a password you know from a protected PDF.",
    icon: LockOpen,
    category: "Security",
    accepts: "pdf",
    inputs: "batch",
    action: "Unlock",
    keywords: ["decrypt", "password", "remove"],
  },
  {
    id: "ocr",
    operation: "ocr",
    title: "OCR PDF",
    description: "Make scanned documents searchable and selectable.",
    icon: ScanText,
    category: "OCR",
    accepts: "pdf",
    inputs: "batch",
    action: "Run OCR",
    keywords: ["scan", "recognize", "searchable", "text", "tesseract"],
  },
];

export const CATEGORIES: ToolCategory[] = ["Organize", "Optimize", "Convert", "Edit", "Security", "OCR"];

export const QUICK_ACTIONS: ToolId[] = ["merge", "compress", "split", "organize"];

export function getTool(id: ToolId | string | undefined): ToolDefinition | undefined {
  return TOOLS.find((t) => t.id === id);
}

export function searchTools(query: string): ToolDefinition[] {
  const q = query.trim().toLowerCase();
  if (!q) return TOOLS;
  const terms = q.split(/\s+/);
  return TOOLS.filter((t) => {
    const haystack = [t.title, t.description, t.category, ...t.keywords].join(" ").toLowerCase();
    return terms.every((term) => haystack.includes(term));
  });
}
