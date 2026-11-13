import { useCallback, useEffect, useRef, useState } from "react";

interface Options {
  ids: string[];
  /** Ids that move together when `id` is dragged (e.g. the selection). */
  dragGroup?: (id: string) => string[];
  /** Move `moving` so they start at `index` in the list without them. */
  onReorder: (moving: string[], index: number) => void;
  axis?: "x" | "y" | "grid";
  disabled?: boolean;
}

/**
 * Pointer-driven reordering for lists and grids. HTML5 drag-and-drop is not
 * used because the window also receives native file drops.
 */
export function usePointerSort({ ids, dragGroup, onReorder, axis = "grid", disabled }: Options) {
  const [dragging, setDragging] = useState<string[] | null>(null);
  const start = useRef<{ x: number; y: number; id: string } | null>(null);
  const last = useRef<string>("");
  const idsRef = useRef(ids);
  idsRef.current = ids;

  const onMove = useCallback(
    (e: PointerEvent) => {
      const s = start.current;
      if (!s) return;
      if (!dragging) {
        if (Math.hypot(e.clientX - s.x, e.clientY - s.y) < 6) return;
        setDragging(dragGroup ? dragGroup(s.id) : [s.id]);
        return;
      }
      const el = document.elementFromPoint(e.clientX, e.clientY)?.closest<HTMLElement>("[data-sort-id]");
      const target = el?.dataset.sortId;
      if (!el || !target || dragging.includes(target)) return;
      const r = el.getBoundingClientRect();
      const after =
        axis === "y" ? e.clientY > r.top + r.height / 2 : axis === "x" ? e.clientX > r.left + r.width / 2 : e.clientX > r.left + r.width / 2;
      const key = `${target}:${after}`;
      if (key === last.current) return;
      last.current = key;
      const rest = idsRef.current.filter((id) => !dragging.includes(id));
      const index = rest.indexOf(target) + (after ? 1 : 0);
      onReorder(dragging, index);
    },
    [dragging, dragGroup, onReorder, axis],
  );

  const end = useCallback(() => {
    start.current = null;
    last.current = "";
    setDragging(null);
  }, []);

  useEffect(() => {
    window.addEventListener("pointermove", onMove);
    window.addEventListener("pointerup", end);
    window.addEventListener("pointercancel", end);
    return () => {
      window.removeEventListener("pointermove", onMove);
      window.removeEventListener("pointerup", end);
      window.removeEventListener("pointercancel", end);
    };
  }, [onMove, end]);

  const itemProps = useCallback(
    (id: string) => ({
      "data-sort-id": id,
      onPointerDown: (e: React.PointerEvent) => {
        if (disabled || e.button !== 0) return;
        if ((e.target as HTMLElement).closest("button, input, select, a, [data-no-drag]")) return;
        start.current = { x: e.clientX, y: e.clientY, id };
      },
    }),
    [disabled],
  );

  return { dragging, itemProps, isDragging: (id: string) => !!dragging?.includes(id) };
}

/** Returns `list` with the `moving` ids relocated to `index` (in the list without them). */
export function moveIds(list: string[], moving: string[], index: number): string[] {
  const set = new Set(moving);
  const rest = list.filter((id) => !set.has(id));
  const block = list.filter((id) => set.has(id));
  const at = Math.max(0, Math.min(index, rest.length));
  return [...rest.slice(0, at), ...block, ...rest.slice(at)];
}
