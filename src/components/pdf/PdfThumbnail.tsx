import { useEffect, useRef, useState } from "react";
import { FileText, ImageOff } from "lucide-react";
import { renderThumbnail } from "@/services/pdf";
import { cn } from "@/utils/cn";

interface PdfThumbnailProps {
  path: string;
  page: number;
  /** Render width in CSS pixels. */
  width: number;
  /** Extra clockwise rotation applied with CSS (no re-render). */
  rotation?: number;
  className?: string;
  alt?: string;
}

/**
 * A real rendering of a PDF page, produced lazily when it scrolls into view.
 */
export function PdfThumbnail({ path, page, width, rotation = 0, className, alt }: PdfThumbnailProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [src, setSrc] = useState<string | null>(null);
  const [failed, setFailed] = useState(false);
  const [aspect, setAspect] = useState(1.414);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === "undefined") {
      setVisible(true);
      return;
    }
    const io = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && setVisible(true), {
      rootMargin: "300px",
    });
    io.observe(el);
    return () => io.disconnect();
  }, []);

  useEffect(() => {
    if (!visible) return;
    let cancelled = false;
    setFailed(false);
    renderThumbnail(path, page, width)
      .then((url) => {
        if (cancelled) return;
        const img = new Image();
        img.onload = () => !cancelled && setAspect(img.naturalHeight / img.naturalWidth || 1.414);
        img.src = url;
        setSrc(url);
      })
      .catch(() => !cancelled && setFailed(true));
    return () => {
      cancelled = true;
    };
  }, [visible, path, page, width]);

  const quarter = (((rotation % 360) + 360) % 360) / 90;
  const sideways = quarter % 2 === 1;
  // When turned sideways the box takes the page's rotated proportions.
  const boxAspect = sideways ? 1 / aspect : aspect;

  return (
    <div
      ref={ref}
      className={cn("relative flex items-center justify-center overflow-hidden bg-page", className)}
      style={{ aspectRatio: `1 / ${boxAspect}` }}
    >
      {src ? (
        <img
          src={src}
          alt={alt ?? `Page ${page}`}
          draggable={false}
          className="absolute max-w-none transition-transform duration-150"
          style={{
            width: sideways ? `${100 * boxAspect}%` : "100%",
            transform: `rotate(${rotation}deg)`,
          }}
        />
      ) : failed ? (
        <ImageOff size={20} className="text-[#9aa1ad]" />
      ) : (
        <FileText size={20} className="animate-pulse text-[#c3c8d0]" />
      )}
    </div>
  );
}
