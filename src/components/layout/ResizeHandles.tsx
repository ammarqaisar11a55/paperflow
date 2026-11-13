import type { ResizeDirection } from "@/hooks/useWindowState";

const EDGE = 5;
const CORNER = 12;

const handles: { dir: ResizeDirection; style: React.CSSProperties; cursor: string }[] = [
  { dir: "North", style: { top: 0, left: CORNER, right: CORNER, height: EDGE }, cursor: "ns-resize" },
  { dir: "South", style: { bottom: 0, left: CORNER, right: CORNER, height: EDGE }, cursor: "ns-resize" },
  { dir: "West", style: { left: 0, top: CORNER, bottom: CORNER, width: EDGE }, cursor: "ew-resize" },
  { dir: "East", style: { right: 0, top: CORNER, bottom: CORNER, width: EDGE }, cursor: "ew-resize" },
  { dir: "NorthWest", style: { top: 0, left: 0, width: CORNER, height: CORNER }, cursor: "nwse-resize" },
  { dir: "NorthEast", style: { top: 0, right: 0, width: CORNER, height: CORNER }, cursor: "nesw-resize" },
  { dir: "SouthWest", style: { bottom: 0, left: 0, width: CORNER, height: CORNER }, cursor: "nesw-resize" },
  { dir: "SouthEast", style: { bottom: 0, right: 0, width: CORNER, height: CORNER }, cursor: "nwse-resize" },
];

/**
 * Invisible strips along the window edges. A frameless window has no system
 * border to grab, so these start a native resize drag in the right direction.
 */
export function ResizeHandles({ onResize }: { onResize: (dir: ResizeDirection) => void }) {
  return (
    <>
      {handles.map(({ dir, style, cursor }) => (
        <div
          key={dir}
          aria-hidden="true"
          data-resize={dir}
          className="fixed z-[900]"
          style={{ ...style, cursor }}
          onMouseDown={(e) => {
            if (e.button !== 0) return;
            e.preventDefault();
            onResize(dir);
          }}
        />
      ))}
    </>
  );
}
