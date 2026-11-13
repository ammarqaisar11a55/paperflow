import { useEffect, useState } from "react";
import { api } from "@/services/api";
import { cn } from "@/utils/cn";

/** Shows a local image file (read through the backend, never by URL). */
export function ImagePreview({ path, className, fit = "cover" }: { path: string; className?: string; fit?: "cover" | "contain" }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    let created: string | null = null;
    let cancelled = false;
    api
      .readFileBytes(path)
      .then((bytes) => {
        if (cancelled) return;
        created = URL.createObjectURL(new Blob([bytes]));
        setUrl(created);
      })
      .catch(() => setUrl(null));
    return () => {
      cancelled = true;
      if (created) URL.revokeObjectURL(created);
    };
  }, [path]);
  if (!url) return null;
  return (
    <img
      src={url}
      alt=""
      draggable={false}
      className={cn("h-full w-full", fit === "cover" ? "object-cover" : "object-contain", className)}
    />
  );
}
