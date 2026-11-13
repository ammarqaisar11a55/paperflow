import { useState } from "react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Segmented } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { NumberInput, PageRangeField, rangeError } from "@/features/shared/controls";
import { pluralize } from "@/utils/format";

type Mode = "ranges" | "every" | "chunks";

export function splitOutputCount(mode: Mode, pages: number | null, ranges: string, chunk: number): number | null {
  if (pages === null) return null;
  if (mode === "every") return pages;
  if (mode === "chunks") return Math.ceil(pages / Math.max(1, chunk));
  if (rangeError(ranges, pages, false)) return null;
  return ranges.split(/[,;]/).filter((p) => p.trim()).length;
}

export function SplitTool() {
  const selection = useFileSelection({ accepts: "pdf", single: true });
  const [mode, setMode] = useState<Mode>("ranges");
  const [ranges, setRanges] = useState("");
  const [chunk, setChunk] = useState(1);
  const pages = selection.files[0]?.pageCount ?? null;
  const count = splitOutputCount(mode, pages, ranges, chunk);
  const blocker =
    mode === "ranges" ? (ranges.trim() ? rangeError(ranges, pages, false) : "Enter the page ranges to split into.") : null;

  return (
    <ToolWorkspace
      tool={getTool("split")!}
      selection={selection}
      extension={null}
      blocker={blocker}
      buildOptions={() => ({ mode, ranges, chunkSize: chunk })}
      options={
        <>
          <Segmented<Mode>
            label="Split mode"
            value={mode}
            onChange={setMode}
            options={[
              { value: "ranges", label: "By ranges" },
              { value: "every", label: "Every page" },
              { value: "chunks", label: "Fixed size" },
            ]}
          />
          {mode === "ranges" && (
            <PageRangeField
              label="Ranges"
              value={ranges}
              onChange={setRanges}
              pageCount={pages}
              allowEmpty={false}
              placeholder="1-3, 4-6, 7-"
              hint="Each comma-separated range becomes its own file."
            />
          )}
          {mode === "chunks" && (
            <NumberInput label="Pages per file" value={chunk} min={1} max={Math.max(1, pages ?? 9999)} onChange={setChunk} />
          )}
          {mode === "every" && <p className="text-secondary text-muted">Every page is saved as a separate PDF.</p>}
          {count !== null && (
            <p className="rounded-md bg-surface-2 px-3 py-2 text-secondary">
              Creates <span className="font-semibold">{pluralize(count, "file")}</span> in a new folder.
            </p>
          )}
        </>
      }
    />
  );
}
