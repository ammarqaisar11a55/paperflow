import { useState } from "react";
import { RotateCcw, RotateCw } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Segmented } from "@/components/ui/Form";
import { Card } from "@/components/ui/Display";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { PageRangeField, rangeError } from "@/features/shared/controls";
import { expandRanges } from "@/utils/format";

type Angle = "90" | "180" | "270";
const PREVIEW_LIMIT = 24;

export function RotateTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [angle, setAngle] = useState<Angle>("90");
  const [pages, setPages] = useState("");
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;
  const turned = new Set(single && count && !error ? expandRanges(pages, count) : []);

  return (
    <ToolWorkspace
      tool={getTool("rotate")!}
      selection={selection}
      blocker={error}
      buildOptions={() => ({ angle: Number(angle), pages })}
      main={
        single &&
        count && (
          <Card className="p-4">
            <p className="mb-3 text-secondary font-semibold">
              Preview <span className="font-normal text-muted">· highlighted pages will turn</span>
            </p>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(96px,1fr))] gap-3">
              {Array.from({ length: Math.min(count, PREVIEW_LIMIT) }, (_, i) => i + 1).map((p) => {
                const on = turned.has(p);
                return (
                  <div key={p} className="flex flex-col items-center gap-1">
                    <div
                      className={`flex h-[120px] w-full items-center justify-center rounded-md p-1.5 ${on ? "bg-accent-soft ring-1 ring-accent" : "bg-surface-2"}`}
                    >
                      <PdfThumbnail path={single.path} page={p} width={96} rotation={on ? Number(angle) : 0} className="max-h-full w-[78px] shadow-card" />
                    </div>
                    <span className="tabular text-caption text-muted">{p}</span>
                  </div>
                );
              })}
            </div>
            {count > PREVIEW_LIMIT && <p className="mt-3 text-caption text-muted">Showing the first {PREVIEW_LIMIT} pages.</p>}
          </Card>
        )
      }
      options={
        <>
          <Segmented<Angle>
            label="Rotation"
            value={angle}
            onChange={setAngle}
            options={[
              { value: "270", label: "Left", icon: RotateCcw },
              { value: "180", label: "180°" },
              { value: "90", label: "Right", icon: RotateCw },
            ]}
          />
          <PageRangeField value={pages} onChange={setPages} pageCount={count} />
        </>
      }
    />
  );
}
