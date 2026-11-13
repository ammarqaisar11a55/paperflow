import { useState } from "react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Card } from "@/components/ui/Display";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { PageRangeField, rangeError } from "@/features/shared/controls";
import { cn } from "@/utils/cn";
import { describePages, expandRanges, pluralize } from "@/utils/format";

export function ExtractPagesTool() {
  const selection = useFileSelection({ accepts: "pdf", single: true });
  const [pages, setPages] = useState("");
  const file = selection.files[0];
  const count = file?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count, false) : "Select the pages to extract.";
  const selected = count && !rangeError(pages, count, false) && pages.trim() ? expandRanges(pages, count) : [];
  const selectedSet = new Set(selected);

  const toggle = (p: number) => {
    const next = selectedSet.has(p) ? selected.filter((x) => x !== p) : [...selected, p].sort((a, b) => a - b);
    setPages(describePages(next));
  };

  return (
    <ToolWorkspace
      tool={getTool("extract-pages")!}
      selection={selection}
      blocker={error}
      buildOptions={() => ({ pages })}
      main={
        file &&
        count && (
          <Card className="p-4">
            <p className="mb-3 text-secondary font-semibold">
              Click pages to select them{" "}
              <span className="font-normal text-muted">· {selected.length ? pluralize(selected.length, "page") + " selected" : "none selected"}</span>
            </p>
            <div className="grid max-h-[560px] grid-cols-[repeat(auto-fill,minmax(104px,1fr))] gap-3 overflow-y-auto p-0.5">
              {Array.from({ length: count }, (_, i) => i + 1).map((p) => {
                const on = selectedSet.has(p);
                return (
                  <button
                    key={p}
                    type="button"
                    aria-pressed={on}
                    onClick={() => toggle(p)}
                    className={cn(
                      "flex flex-col items-center gap-1.5 rounded-lg p-2 transition-colors",
                      on ? "bg-accent-soft ring-2 ring-accent" : "hover:bg-surface-2",
                    )}
                  >
                    <PdfThumbnail path={file.path} page={p} width={88} className="w-[84px] shadow-card" />
                    <span className={cn("tabular text-caption", on ? "font-semibold text-accent-text" : "text-muted")}>{p}</span>
                  </button>
                );
              })}
            </div>
          </Card>
        )
      }
      options={<PageRangeField value={pages} onChange={setPages} pageCount={count} allowEmpty={false} placeholder="1-3, 5" />}
    />
  );
}
