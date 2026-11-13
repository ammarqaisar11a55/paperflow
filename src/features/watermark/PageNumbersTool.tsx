import { useState } from "react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Card } from "@/components/ui/Display";
import { ColorInput, Field, Select, Slider, TextInput } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { FONT_CSS, FontSelect, NumberInput, PageRangeField, PositionPicker, rangeError, type FontId, type Position } from "@/features/shared/controls";
import { PagePreview } from "@/features/shared/PagePreview";

const ALLOWED: Position[] = ["top-left", "top-center", "top-right", "bottom-left", "bottom-center", "bottom-right"];
const MARGIN = 28;

export function pageLabel(prefix: string, n: number, total: number, format: "number" | "numberOfTotal", suffix: string) {
  return `${prefix}${format === "numberOfTotal" ? `${n} of ${total}` : n}${suffix}`;
}

export function PageNumbersTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [position, setPosition] = useState<Position>("bottom-center");
  const [format, setFormat] = useState<"number" | "numberOfTotal">("number");
  const [prefix, setPrefix] = useState("");
  const [suffix, setSuffix] = useState("");
  const [start, setStart] = useState(1);
  const [font, setFont] = useState<FontId>("helvetica");
  const [fontSize, setFontSize] = useState(11);
  const [color, setColor] = useState("#20242c");
  const [pages, setPages] = useState("");
  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const error = pages.trim() ? rangeError(pages, count) : null;
  const preview = selection.files[0];
  const total = start + (preview?.pageCount ?? 1) - 1;
  const label = pageLabel(prefix, start, total, format, suffix);

  return (
    <ToolWorkspace
      tool={getTool("page-numbers")!}
      selection={selection}
      blocker={error}
      buildOptions={() => ({ position, format, prefix, suffix, start, font, fontSize, color, pages: single ? pages : "", margin: MARGIN })}
      main={
        preview &&
        !preview.encrypted && (
          <Card className="flex justify-center bg-surface-2/40 p-6">
            <PagePreview
              path={preview.path}
              width={320}
              caption="Preview of the first numbered page"
              overlay={(scale) => {
                const [v, h] = position.split("-");
                const f = FONT_CSS[font];
                const style: React.CSSProperties = {
                  position: "absolute",
                  color,
                  fontFamily: f.family,
                  fontWeight: f.weight,
                  fontSize: Math.max(6, fontSize * scale),
                  lineHeight: 1,
                  whiteSpace: "nowrap",
                  [v === "top" ? "top" : "bottom"]: MARGIN * scale,
                };
                if (h === "left") style.left = MARGIN * scale;
                else if (h === "right") style.right = MARGIN * scale;
                else {
                  style.left = "50%";
                  style.transform = "translateX(-50%)";
                }
                return <span style={style}>{label}</span>;
              }}
            />
          </Card>
        )
      }
      options={
        <>
          <div className="grid grid-cols-[auto_1fr] gap-5">
            <PositionPicker value={position} onChange={setPosition} allowed={ALLOWED} />
            <div className="flex flex-col gap-4">
              <Field label="Format">
                {(id) => (
                  <Select id={id} value={format} onChange={(e) => setFormat(e.target.value as typeof format)}>
                    <option value="number">1, 2, 3</option>
                    <option value="numberOfTotal">1 of 12</option>
                  </Select>
                )}
              </Field>
              <NumberInput label="Start at" value={start} min={0} max={99999} onChange={setStart} />
            </div>
          </div>
          <div className="grid grid-cols-2 gap-3">
            <Field label="Prefix">{(id) => <TextInput id={id} value={prefix} placeholder="Page " onChange={(e) => setPrefix(e.target.value)} />}</Field>
            <Field label="Suffix">{(id) => <TextInput id={id} value={suffix} onChange={(e) => setSuffix(e.target.value)} />}</Field>
          </div>
          <div className="grid grid-cols-[1fr_auto] items-end gap-3">
            <FontSelect value={font} onChange={setFont} />
            <ColorInput label="Number colour" value={color} onChange={setColor} />
          </div>
          <Slider label="Size" value={fontSize} min={6} max={36} onChange={setFontSize} format={(v) => `${v} pt`} />
          {single && <PageRangeField value={pages} onChange={setPages} pageCount={count} label="Pages to number" />}
        </>
      }
    />
  );
}
