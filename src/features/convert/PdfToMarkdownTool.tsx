import { useState } from "react";
import { Info } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Notice } from "@/components/ui/Display";
import { Switch } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { PageRangeField, rangeError } from "@/features/shared/controls";
import { stem } from "@/utils/format";

export function PdfToMarkdownTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [pages, setPages] = useState("");
  const [detectHeadings, setDetectHeadings] = useState(true);
  const [pageBreaks, setPageBreaks] = useState(false);
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;

  return (
    <ToolWorkspace
      tool={getTool("pdf-to-markdown")!}
      selection={selection}
      extension="md"
      blocker={error}
      defaultName={(files) => stem(files[0].path)}
      buildOptions={() => ({ pages: single ? pages : "", detectHeadings, pageBreaks })}
      options={
        <>
          {single && <PageRangeField value={pages} onChange={setPages} pageCount={count} />}
          <Switch
            checked={detectHeadings}
            onChange={setDetectHeadings}
            label="Detect headings"
            description="Larger text becomes #, ## and ### headings."
          />
          <Switch
            checked={pageBreaks}
            onChange={setPageBreaks}
            label="Separate pages"
            description="Adds a horizontal rule between pages."
          />
          <Notice tone="neutral" icon={Info}>
            Keeps headings, paragraphs, lists, bold, italic and links, and drops repeated headers and page numbers.
            Tables come out as plain text, and scanned pages need OCR PDF first.
          </Notice>
        </>
      }
    />
  );
}
