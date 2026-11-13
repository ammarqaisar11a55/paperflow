import { useState } from "react";
import { ImagePlus, Type } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Button } from "@/components/ui/Button";
import { Card } from "@/components/ui/Display";
import { ColorInput, Field, Segmented, Slider, TextInput } from "@/components/ui/Form";
import { ImagePreview } from "@/components/files/ImagePreview";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { FONT_CSS, FontSelect, PageRangeField, PositionPicker, rangeError, type FontId, type Position } from "@/features/shared/controls";
import { PagePreview } from "@/features/shared/PagePreview";
import { pickImages } from "@/services/dialogs";
import { basename } from "@/utils/format";

const MARGIN = 36;

/** CSS placement mirroring the backend's anchor computation. */
export function anchorStyle(position: Position, scale: number): React.CSSProperties {
  const [v, h] = position === "center" ? ["middle", "center"] : position.split("-");
  const m = MARGIN * scale;
  const style: React.CSSProperties = { position: "absolute" };
  if (h === "left") style.left = m;
  else if (h === "right") style.right = m;
  else style.left = "50%";
  if (v === "top") style.top = m;
  else if (v === "bottom") style.bottom = m;
  else style.top = "50%";
  return style;
}

export function WatermarkTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const [kind, setKind] = useState<"text" | "image">("text");
  const [text, setText] = useState("CONFIDENTIAL");
  const [font, setFont] = useState<FontId>("helveticaBold");
  const [fontSize, setFontSize] = useState(56);
  const [color, setColor] = useState("#b83a2e");
  const [imagePath, setImagePath] = useState<string | null>(null);
  const [imageScale, setImageScale] = useState(40);
  const [position, setPosition] = useState<Position>("center");
  const [opacity, setOpacity] = useState(30);
  const [rotation, setRotation] = useState(45);
  const [pages, setPages] = useState("");

  const single = selection.files.length === 1 ? selection.files[0] : null;
  const count = single?.pageCount ?? null;
  const blocker =
    kind === "text" && !text.trim()
      ? "Enter the watermark text."
      : kind === "image" && !imagePath
        ? "Choose an image for the watermark."
        : pages.trim()
          ? rangeError(pages, count)
          : null;

  const preview = selection.files[0];
  return (
    <ToolWorkspace
      tool={getTool("watermark")!}
      selection={selection}
      blocker={blocker}
      buildOptions={() => ({
        kind,
        text,
        font,
        fontSize,
        color,
        imagePath,
        imageScale: imageScale / 100,
        position,
        opacity: opacity / 100,
        rotation,
        pages: single ? pages : "",
        margin: MARGIN,
      })}
      main={
        preview &&
        !preview.encrypted && (
          <Card className="flex justify-center bg-surface-2/40 p-6">
            <PagePreview
              path={preview.path}
              width={320}
              caption="Preview of the first page"
              overlay={(scale) => {
                const transform = `${position.includes("center") || position === "center" ? "translateX(-50%)" : ""} ${
                  position.startsWith("middle") || position === "center" ? "translateY(-50%)" : ""
                } rotate(${-rotation}deg)`;
                const base = { ...anchorStyle(position, scale), opacity: opacity / 100, transform, transformOrigin: "center" };
                if (kind === "text") {
                  const f = FONT_CSS[font];
                  return (
                    <span
                      style={{ ...base, color, fontFamily: f.family, fontWeight: f.weight, fontSize: fontSize * scale, lineHeight: 1, whiteSpace: "nowrap" }}
                    >
                      {text}
                    </span>
                  );
                }
                return imagePath ? (
                  <div style={{ ...base, width: `${imageScale}%` }}>
                    <ImagePreview path={imagePath} fit="contain" />
                  </div>
                ) : null;
              }}
            />
          </Card>
        )
      }
      options={
        <>
          <Segmented
            label="Watermark type"
            value={kind}
            onChange={setKind}
            options={[
              { value: "text", label: "Text", icon: Type },
              { value: "image", label: "Image", icon: ImagePlus },
            ]}
          />
          {kind === "text" ? (
            <>
              <Field label="Text">{(id) => <TextInput id={id} value={text} onChange={(e) => setText(e.target.value)} />}</Field>
              <div className="grid grid-cols-[1fr_auto] items-end gap-3">
                <FontSelect value={font} onChange={setFont} />
                <ColorInput label="Text colour" value={color} onChange={setColor} />
              </div>
              <Slider label="Size" value={fontSize} min={8} max={160} onChange={setFontSize} format={(v) => `${v} pt`} />
            </>
          ) : (
            <>
              <Field label="Image">
                {() => (
                  <div className="flex items-center gap-2">
                    <span className="min-w-0 flex-1 truncate text-secondary text-muted">{imagePath ? basename(imagePath) : "No image chosen"}</span>
                    <Button
                      size="sm"
                      onClick={async () => {
                        const [p] = await pickImages(false);
                        if (p) setImagePath(p);
                      }}
                    >
                      Choose…
                    </Button>
                  </div>
                )}
              </Field>
              <Slider label="Width" value={imageScale} min={5} max={100} onChange={setImageScale} format={(v) => `${v}% of page`} />
            </>
          )}
          <div className="grid grid-cols-[auto_1fr] gap-5">
            <PositionPicker value={position} onChange={setPosition} />
            <div className="flex flex-col gap-4">
              <Slider label="Opacity" value={opacity} min={5} max={100} onChange={setOpacity} format={(v) => `${v}%`} />
              <Slider label="Rotation" value={rotation} min={-90} max={90} step={5} onChange={setRotation} format={(v) => `${v}°`} />
            </div>
          </div>
          {single && <PageRangeField value={pages} onChange={setPages} pageCount={count} />}
        </>
      }
    />
  );
}
