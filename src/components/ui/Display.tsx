import type { ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { cn } from "@/utils/cn";

export function Card({ className, children, ...rest }: React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cn("rounded-xl border border-line bg-surface shadow-card", className)} {...rest}>
      {children}
    </div>
  );
}

export function SectionTitle({ children, action, className }: { children: ReactNode; action?: ReactNode; className?: string }) {
  return (
    <div className={cn("mb-3 flex items-center justify-between", className)}>
      <h2 className="text-caption font-semibold uppercase tracking-[0.06em] text-muted">{children}</h2>
      {action}
    </div>
  );
}

interface ProgressBarProps {
  value: number | null;
  label?: string;
  tone?: "accent" | "success" | "danger";
  className?: string;
}

/** Determinate when `value` is a number (0..1), indeterminate when null. */
export function ProgressBar({ value, label, tone = "accent", className }: ProgressBarProps) {
  const color = tone === "success" ? "bg-success" : tone === "danger" ? "bg-danger" : "bg-accent";
  const pct = value === null ? undefined : Math.round(Math.max(0, Math.min(1, value)) * 100);
  return (
    <div
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={pct}
      className={cn("relative h-1.5 w-full overflow-hidden rounded-full bg-surface-3", className)}
    >
      {value === null ? (
        <div className={cn("absolute inset-y-0 w-2/5 rounded-full", color)} style={{ animation: "pf-indeterminate 1.2s ease-in-out infinite" }} />
      ) : (
        <div className={cn("h-full rounded-full transition-[width] duration-200", color)} style={{ width: `${pct}%` }} />
      )}
    </div>
  );
}

type Tone = "neutral" | "accent" | "marker" | "success" | "warning" | "danger";
const tones: Record<Tone, string> = {
  neutral: "bg-surface-2 text-muted",
  accent: "bg-accent-soft text-accent",
  marker: "bg-marker-soft text-marker-ink",
  success: "bg-success-soft text-success",
  warning: "bg-warning-soft text-warning",
  danger: "bg-danger-soft text-danger",
};

export function Badge({ tone = "neutral", children, icon: Icon }: { tone?: Tone; children: ReactNode; icon?: LucideIcon }) {
  return (
    <span className={cn("inline-flex h-5 items-center gap-1 rounded px-1.5 text-caption font-medium", tones[tone])}>
      {Icon && <Icon size={12} strokeWidth={2} />}
      {children}
    </span>
  );
}

export function Notice({ tone, icon: Icon, title, children }: { tone: Tone; icon: LucideIcon; title?: string; children?: ReactNode }) {
  return (
    <div className={cn("flex gap-2.5 rounded-lg px-3 py-2.5 text-secondary", tones[tone])}>
      <Icon size={16} className="mt-px shrink-0" />
      <div className="flex flex-col gap-0.5">
        {title && <p className="font-semibold">{title}</p>}
        {children && <div className={title ? "text-fg/80" : ""}>{children}</div>}
      </div>
    </div>
  );
}

export function EmptyState({ icon: Icon, title, children, action }: { icon: LucideIcon; title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-12 text-center">
      <div className="mb-1 flex h-11 w-11 items-center justify-center rounded-xl bg-surface-2 text-muted">
        <Icon size={20} />
      </div>
      <p className="text-card font-semibold">{title}</p>
      {children && <p className="max-w-sm text-secondary text-muted">{children}</p>}
      {action && <div className="mt-2">{action}</div>}
    </div>
  );
}

export function Kbd({ children }: { children: ReactNode }) {
  return <kbd className="rounded border border-line bg-surface-2 px-1 font-sans text-[10.5px] text-muted">{children}</kbd>;
}
