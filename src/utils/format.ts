export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes < 0) return "—";
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value < 10 ? value.toFixed(1) : Math.round(value)} ${units[i]}`;
}

/** Percentage saved, e.g. 1000 → 400 is 60. Negative when the file grew. */
export function reduction(before: number, after: number): number {
  if (before <= 0) return 0;
  return Math.round((1 - after / before) * 100);
}

export function pluralize(n: number, one: string, many = `${one}s`): string {
  return `${n} ${n === 1 ? one : many}`;
}

export function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] || path;
}

export function dirname(path: string): string {
  const i = Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\"));
  return i > 0 ? path.slice(0, i) : path;
}

export function stem(path: string): string {
  const name = basename(path);
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
}

/** Shortens /home/user/... to ~/... for display. */
export function prettyPath(path: string, home?: string | null): string {
  const h = home ?? guessHome(path);
  if (h && path.startsWith(h)) return `~${path.slice(h.length)}`;
  return path;
}

function guessHome(path: string): string | null {
  const m = /^\/home\/[^/]+/.exec(path);
  return m ? m[0] : null;
}

export function relativeTime(timestamp: number, now = Date.now()): string {
  const seconds = Math.round((now - timestamp) / 1000);
  if (seconds < 45) return "Just now";
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes} min ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours} h ago`;
  const days = Math.round(hours / 24);
  if (days === 1) return "Yesterday";
  if (days < 7) return `${days} days ago`;
  return new Date(timestamp).toLocaleDateString(undefined, { day: "numeric", month: "short", year: "numeric" });
}

/**
 * Validates a page range like "1-3, 5, 8-" against a page count. Mirrors the
 * Rust parser so mistakes show up while typing. Returns an error message or
 * null when valid. An empty string means "all pages" when allowEmpty is set.
 */
export function validateRanges(spec: string, total: number | null, allowEmpty = true): string | null {
  const s = spec.trim();
  if (!s || s.toLowerCase() === "all") return allowEmpty ? null : "Enter at least one page, for example 1-3, 5.";
  const parts = s.split(/[,;]/).map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return "Enter at least one page, for example 1-3, 5.";
  for (const part of parts) {
    const m = /^(\d+|end|last)?\s*(-\s*(\d+|end|last)?)?$/i.exec(part);
    if (!m || (!m[1] && !m[2])) return `“${part}” is not a valid page range.`;
    const toNum = (v: string | undefined, fallback: number) =>
      v === undefined ? fallback : /^(end|last)$/i.test(v) ? (total ?? Number.MAX_SAFE_INTEGER) : Number(v);
    const a = toNum(m[1], 1);
    const b = m[2] ? toNum(m[3], total ?? Number.MAX_SAFE_INTEGER) : a;
    if (a < 1 || b < 1) return "Pages start at 1.";
    if (total !== null && (a > total || b > total)) return `This document has ${pluralize(total, "page")}.`;
  }
  return null;
}

/** Expands a valid range spec into page numbers (deduplicated, in order). */
export function expandRanges(spec: string, total: number): number[] {
  const s = spec.trim();
  if (!s || s.toLowerCase() === "all") return Array.from({ length: total }, (_, i) => i + 1);
  const seen = new Set<number>();
  const out: number[] = [];
  for (const part of s.split(/[,;]/).map((p) => p.trim()).filter(Boolean)) {
    const [rawA, rawB] = part.includes("-") ? part.split("-").map((x) => x.trim()) : [part, part];
    const num = (v: string, fallback: number) => (v === "" ? fallback : /^(end|last)$/i.test(v) ? total : Number(v));
    const a = num(rawA, 1);
    const b = num(rawB, total);
    const step = a <= b ? 1 : -1;
    for (let p = a; step > 0 ? p <= b : p >= b; p += step) {
      if (p >= 1 && p <= total && !seen.has(p)) {
        seen.add(p);
        out.push(p);
      }
    }
  }
  return out;
}

/** "1-3, 5" for [1,2,3,5]. */
export function describePages(pages: number[]): string {
  const sorted = [...pages];
  const parts: string[] = [];
  for (let i = 0; i < sorted.length; i++) {
    const start = sorted[i];
    let end = start;
    while (i + 1 < sorted.length && sorted[i + 1] === end + 1) {
      end++;
      i++;
    }
    parts.push(start === end ? `${start}` : `${start}-${end}`);
  }
  return parts.join(", ");
}
