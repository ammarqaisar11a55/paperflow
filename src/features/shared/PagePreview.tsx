import { useEffect, useState, type ReactNode } from "react";
import { PdfThumbnail } from "@/components/pdf/PdfThumbnail";
import { acquirePdf, releasePdf } from "@/services/pdf";

/** Page size of page 1 in points, for scaling previews. */
export function usePageSize(path: string | null, page = 1) {
  const [size, setSize] = useState<{ width: number; height: number } | null>(null);
  useEffect(() => {
    if (!path) return;
    let cancelled = false;
    acquirePdf(path)
      .then(async (doc) => {
        const p = await doc.getPage(Math.min(page, doc.numPages));
        const v = p.getViewport({ scale: 1 });
        if (!cancelled) setSize({ width: v.width, height: v.height });
      })
      .catch(() => undefined)
      .finally(() => releasePdf(path));
    return () => {
      cancelled = true;
    };
  }, [path, page]);
  return size;
}

interface PagePreviewProps {
  path: string;
  page?: number;
  width?: number;
  /** Overlay drawn in page coordinates: receives points→pixels scale. */
  overlay?: (scale: number) => ReactNode;
  caption?: string;
}

/** A large rendering of a page with an optional live overlay (watermark, numbers). */
export function PagePreview({ path, page = 1, width = 300, overlay, caption }: PagePreviewProps) {
  const size = usePageSize(path, page);
  const scale = size ? width / size.width : 0;
  return (
    <figure className="flex flex-col items-center gap-2">
      <div className="relative overflow-hidden rounded-[3px] shadow-raised ring-1 ring-black/5" style={{ width }}>
        <PdfThumbnail path={path} page={page} width={width} />
        {size && overlay && <div className="pointer-events-none absolute inset-0">{overlay(scale)}</div>}
      </div>
      {caption && <figcaption className="text-caption text-muted">{caption}</figcaption>}
    </figure>
  );
}
