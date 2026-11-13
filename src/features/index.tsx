import type { ComponentType } from "react";
import { CompressTool } from "./compress/CompressTool";
import { ExtractImagesTool, ImagesToPdfTool, PdfToImagesTool, PdfToTextTool } from "./convert/ConvertTools";
import { MergeTool } from "./merge/MergeTool";
import { MetadataTool } from "./metadata/MetadataTool";
import { OcrTool } from "./ocr/OcrTool";
import { ExtractPagesTool } from "./organize/ExtractPagesTool";
import { OrganizeTool } from "./organize/OrganizeTool";
import { RotateTool } from "./organize/RotateTool";
import type { ToolId } from "./registry";
import { ProtectTool, UnlockTool } from "./security/SecurityTools";
import { SplitTool } from "./split/SplitTool";
import { PageNumbersTool } from "./watermark/PageNumbersTool";
import { WatermarkTool } from "./watermark/WatermarkTool";

export const TOOL_SCREENS: Record<ToolId, ComponentType> = {
  merge: MergeTool,
  split: SplitTool,
  organize: OrganizeTool,
  rotate: RotateTool,
  "extract-pages": ExtractPagesTool,
  compress: CompressTool,
  "pdf-to-images": PdfToImagesTool,
  "images-to-pdf": ImagesToPdfTool,
  "extract-images": ExtractImagesTool,
  "pdf-to-text": PdfToTextTool,
  watermark: WatermarkTool,
  "page-numbers": PageNumbersTool,
  metadata: MetadataTool,
  protect: ProtectTool,
  unlock: UnlockTool,
  ocr: OcrTool,
};
