import { useState } from "react";
import { Info } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Notice } from "@/components/ui/Display";
import { Switch } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool, type ToolId } from "@/features/registry";
import type { InputKind } from "@/features/fileKinds";
import { stem } from "@/utils/format";

interface OfficeToolProps {
  toolId: ToolId;
  accepts: InputKind;
  /** What to tell people about how faithful the conversion is. */
  note: string;
  sheets?: boolean;
}

/** Word, PowerPoint, Excel and HTML to PDF share one page: LibreOffice does the work. */
function OfficeTool({ toolId, accepts, note, sheets }: OfficeToolProps) {
  const selection = useFileSelection({ accepts });
  const [pdfA, setPdfA] = useState(false);
  const [onePagePerSheet, setOnePagePerSheet] = useState(false);

  return (
    <ToolWorkspace
      tool={getTool(toolId)!}
      selection={selection}
      defaultName={(files) => stem(files[0].path)}
      buildOptions={() => ({ pdfA, onePagePerSheet: sheets ? onePagePerSheet : false })}
      options={
        <>
          {sheets && (
            <Switch
              checked={onePagePerSheet}
              onChange={setOnePagePerSheet}
              label="Each sheet on one page"
              description="Scales every sheet to fit a single page instead of splitting it."
            />
          )}
          <Switch
            checked={pdfA}
            onChange={setPdfA}
            label="PDF/A for archiving"
            description="A long-term archival format, often required for official records."
          />
          <Notice tone="neutral" icon={Info}>
            {note} Converted on this computer with LibreOffice; macros never run and nothing is downloaded.
          </Notice>
        </>
      }
    />
  );
}

export function WordToPdfTool() {
  return (
    <OfficeTool
      toolId="word-to-pdf"
      accepts="word"
      note="Layout matches Word closely; fonts that aren't installed are replaced with similar ones."
    />
  );
}

export function PowerPointToPdfTool() {
  return (
    <OfficeTool
      toolId="powerpoint-to-pdf"
      accepts="presentation"
      note="Each slide becomes a page. Animations and embedded videos are not included."
    />
  );
}

export function ExcelToPdfTool() {
  return (
    <OfficeTool
      toolId="excel-to-pdf"
      accepts="spreadsheet"
      sheets
      note="Every sheet is included, using each sheet's print settings."
    />
  );
}

export function HtmlToPdfTool() {
  return (
    <OfficeTool
      toolId="html-to-pdf"
      accepts="html"
      note="Best for documents and saved pages. Scripts don't run, and complex web layouts may differ from a browser."
    />
  );
}
