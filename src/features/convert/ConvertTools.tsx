import { useState } from "react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Field, Segmented, Select, Slider, Switch } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { PageRangeField, rangeError } from "@/features/shared/controls";
import { useSettings } from "@/stores/settings";

const DPI_OPTIONS = [72, 96, 150, 200, 300, 600];

export function PdfToImagesTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const settings = useSettings((s) => s.settings);
  const [format, setFormat] = useState<"png" | "jpeg">(settings.imageFormat);
  const [dpi, setDpi] = useState(settings.imageDpi);
  const [quality, setQuality] = useState(90);
  const [pages, setPages] = useState("");
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;

  return (
    <ToolWorkspace
      tool={getTool("pdf-to-images")!}
      selection={selection}
      extension={null}
      blocker={error}
      buildOptions={() => ({ format, dpi, quality, pages: single ? pages : "" })}
      options={
        <>
          <Segmented
            label="Image format"
            value={format}
            onChange={setFormat}
            options={[
              { value: "png", label: "PNG" },
              { value: "jpeg", label: "JPEG" },
            ]}
          />
          <Field label="Resolution" hint="Higher values give sharper, larger images.">
            {(id, d) => (
              <Select id={id} aria-describedby={d} value={dpi} onChange={(e) => setDpi(Number(e.target.value))}>
                {[...new Set([...DPI_OPTIONS, dpi])].sort((a, b) => a - b).map((v) => (
                  <option key={v} value={v}>
                    {v} dpi{v === 150 ? " (screen)" : v === 300 ? " (print)" : ""}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          {format === "jpeg" && <Slider label="JPEG quality" value={quality} min={40} max={100} onChange={setQuality} format={(v) => `${v}%`} />}
          {single ? (
            <PageRangeField value={pages} onChange={setPages} pageCount={count} />
          ) : (
            <p className="text-caption text-muted">All pages of each file are converted.</p>
          )}
        </>
      }
    />
  );
}

export function ImagesToPdfTool() {
  const selection = useFileSelection({ accepts: "image" });
  const [pageSize, setPageSize] = useState("a4");
  const [orientation, setOrientation] = useState<"auto" | "portrait" | "landscape">("auto");
  const [margin, setMargin] = useState<"none" | "small" | "large">("small");

  return (
    <ToolWorkspace
      tool={getTool("images-to-pdf")!}
      selection={selection}
      defaultName={(files) => (files.length === 1 ? files[0].name.replace(/\.[^.]+$/, "") : "Images")}
      buildOptions={() => ({ pageSize, orientation, margin })}
      options={
        <>
          <Field label="Page size">
            {(id) => (
              <Select id={id} value={pageSize} onChange={(e) => setPageSize(e.target.value)}>
                <option value="fit">Same as image</option>
                <option value="a4">A4 (210 × 297 mm)</option>
                <option value="letter">US Letter (8.5 × 11 in)</option>
                <option value="legal">US Legal (8.5 × 14 in)</option>
              </Select>
            )}
          </Field>
          {pageSize !== "fit" && (
            <Segmented
              label="Orientation"
              value={orientation}
              onChange={setOrientation}
              options={[
                { value: "auto", label: "Auto" },
                { value: "portrait", label: "Portrait" },
                { value: "landscape", label: "Landscape" },
              ]}
            />
          )}
          <div className="flex flex-col gap-1.5">
            <span className="text-secondary font-medium">Margin</span>
            <Segmented
              label="Margin"
              value={margin}
              onChange={setMargin}
              options={[
                { value: "none", label: "None" },
                { value: "small", label: "Small" },
                { value: "large", label: "Large" },
              ]}
            />
          </div>
          <p className="text-caption text-muted">One image per page, in the order shown. JPEG photos are kept at full quality.</p>
        </>
      }
    />
  );
}

export function ExtractImagesTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [pages, setPages] = useState("");
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;
  return (
    <ToolWorkspace
      tool={getTool("extract-images")!}
      selection={selection}
      extension={null}
      blocker={error}
      buildOptions={() => ({ pages: single ? pages : "" })}
      options={
        <>
          {single && <PageRangeField value={pages} onChange={setPages} pageCount={count} />}
          <p className="text-caption text-muted">
            Images are saved in their original format and resolution. To save whole pages as pictures, use PDF to Images.
          </p>
        </>
      }
    />
  );
}

export function PdfToTextTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [pages, setPages] = useState("");
  const [layout, setLayout] = useState(true);
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;
  return (
    <ToolWorkspace
      tool={getTool("pdf-to-text")!}
      selection={selection}
      extension="txt"
      blocker={error}
      buildOptions={() => ({ pages: single ? pages : "", layout })}
      options={
        <>
          {single && <PageRangeField value={pages} onChange={setPages} pageCount={count} />}
          <Switch
            checked={layout}
            onChange={setLayout}
            label="Keep layout"
            description="Preserve columns and spacing. Turn off for plain reading order."
          />
          <p className="text-caption text-muted">Scanned pages have no text. Run OCR PDF on them first.</p>
        </>
      }
    />
  );
}
