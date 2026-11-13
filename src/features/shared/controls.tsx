import type { LucideIcon } from "lucide-react";
import { Field, Select, TextInput } from "@/components/ui/Form";
import { cn } from "@/utils/cn";
import { pluralize, validateRanges } from "@/utils/format";

interface PageRangeFieldProps {
  value: string;
  onChange: (v: string) => void;
  pageCount: number | null;
  label?: string;
  allowEmpty?: boolean;
  placeholder?: string;
  hint?: string;
}

export function PageRangeField({
  value,
  onChange,
  pageCount,
  label = "Pages",
  allowEmpty = true,
  placeholder = "All pages",
  hint,
}: PageRangeFieldProps) {
  const error = value.trim() || !allowEmpty ? validateRanges(value, pageCount, allowEmpty) : null;
  return (
    <Field
      label={label}
      error={error}
      hint={hint ?? `For example 1-3, 5, 8-${pageCount ? ` · ${pluralize(pageCount, "page")} in total` : ""}`}
    >
      {(id, d) => (
        <TextInput id={id} aria-describedby={d} value={value} invalid={!!error} placeholder={placeholder} onChange={(e) => onChange(e.target.value)} />
      )}
    </Field>
  );
}

export function rangeError(value: string, pageCount: number | null, allowEmpty = true): string | null {
  return validateRanges(value, pageCount, allowEmpty);
}

export type Position =
  | "top-left"
  | "top-center"
  | "top-right"
  | "middle-left"
  | "center"
  | "middle-right"
  | "bottom-left"
  | "bottom-center"
  | "bottom-right";

const GRID: Position[] = [
  "top-left",
  "top-center",
  "top-right",
  "middle-left",
  "center",
  "middle-right",
  "bottom-left",
  "bottom-center",
  "bottom-right",
];

export function PositionPicker({
  value,
  onChange,
  allowed = GRID,
  label = "Position",
}: {
  value: Position;
  onChange: (p: Position) => void;
  allowed?: Position[];
  label?: string;
}) {
  return (
    <div className="flex flex-col gap-1.5">
      <span className="text-secondary font-medium">{label}</span>
      <div role="radiogroup" aria-label={label} className="grid w-[108px] grid-cols-3 gap-1 rounded-lg border border-line bg-surface-2 p-1.5">
        {GRID.map((p) => {
          const enabled = allowed.includes(p);
          const selected = value === p;
          return (
            <button
              key={p}
              type="button"
              role="radio"
              aria-checked={selected}
              aria-label={p.replace("-", " ")}
              disabled={!enabled}
              onClick={() => onChange(p)}
              className={cn(
                "flex h-7 items-center justify-center rounded",
                enabled ? "hover:bg-surface" : "opacity-30",
                selected && "bg-surface shadow-card",
              )}
            >
              <span className={cn("h-2 w-2 rounded-full", selected ? "bg-accent" : "bg-line-strong")} />
            </button>
          );
        })}
      </div>
    </div>
  );
}

export const FONTS = [
  { value: "helvetica", label: "Helvetica" },
  { value: "helveticaBold", label: "Helvetica Bold" },
  { value: "timesRoman", label: "Times" },
  { value: "timesBold", label: "Times Bold" },
  { value: "courier", label: "Courier" },
  { value: "courierBold", label: "Courier Bold" },
] as const;

export type FontId = (typeof FONTS)[number]["value"];

export const FONT_CSS: Record<FontId, { family: string; weight: number }> = {
  helvetica: { family: "Helvetica, Arial, 'Liberation Sans', sans-serif", weight: 400 },
  helveticaBold: { family: "Helvetica, Arial, 'Liberation Sans', sans-serif", weight: 700 },
  timesRoman: { family: "'Times New Roman', Times, 'Liberation Serif', serif", weight: 400 },
  timesBold: { family: "'Times New Roman', Times, 'Liberation Serif', serif", weight: 700 },
  courier: { family: "'Courier New', Courier, 'Liberation Mono', monospace", weight: 400 },
  courierBold: { family: "'Courier New', Courier, 'Liberation Mono', monospace", weight: 700 },
};

export function FontSelect({ value, onChange }: { value: FontId; onChange: (v: FontId) => void }) {
  return (
    <Field label="Font">
      {(id) => (
        <Select id={id} value={value} onChange={(e) => onChange(e.target.value as FontId)}>
          {FONTS.map((f) => (
            <option key={f.value} value={f.value}>
              {f.label}
            </option>
          ))}
        </Select>
      )}
    </Field>
  );
}

interface OptionCardsProps<T extends string> {
  value: T;
  onChange: (v: T) => void;
  label: string;
  options: { value: T; title: string; description: string; icon?: LucideIcon; badge?: string }[];
}

/** Large radio choices with a description, e.g. compression levels. */
export function OptionCards<T extends string>({ value, onChange, label, options }: OptionCardsProps<T>) {
  return (
    <div role="radiogroup" aria-label={label} className="flex flex-col gap-2">
      <span className="text-secondary font-medium">{label}</span>
      {options.map((o) => {
        const selected = o.value === value;
        const Icon = o.icon;
        return (
          <button
            key={o.value}
            type="button"
            role="radio"
            aria-checked={selected}
            onClick={() => onChange(o.value)}
            className={cn(
              "flex items-start gap-3 rounded-lg border px-3 py-2.5 text-left transition-colors",
              selected ? "border-accent bg-accent-soft/50" : "border-line hover:border-line-strong hover:bg-surface-2/50",
            )}
          >
            <span
              className={cn(
                "mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full border",
                selected ? "border-accent" : "border-line-strong",
              )}
            >
              {selected && <span className="h-2 w-2 rounded-full bg-accent" />}
            </span>
            <span className="flex flex-col">
              <span className="flex items-center gap-2 text-body font-medium">
                {Icon && <Icon size={14} />}
                {o.title}
                {o.badge && <span className="rounded bg-accent-soft px-1.5 text-caption font-semibold text-accent">{o.badge}</span>}
              </span>
              <span className="text-caption text-muted">{o.description}</span>
            </span>
          </button>
        );
      })}
    </div>
  );
}

export function NumberInput({
  label,
  value,
  onChange,
  min,
  max,
  hint,
}: {
  label: string;
  value: number;
  onChange: (v: number) => void;
  min?: number;
  max?: number;
  hint?: string;
}) {
  return (
    <Field label={label} hint={hint}>
      {(id, d) => (
        <TextInput
          id={id}
          aria-describedby={d}
          type="number"
          min={min}
          max={max}
          value={Number.isFinite(value) ? value : ""}
          onChange={(e) => {
            const n = Number(e.target.value);
            if (e.target.value === "" || Number.isNaN(n)) return;
            onChange(Math.max(min ?? -Infinity, Math.min(max ?? Infinity, n)));
          }}
        />
      )}
    </Field>
  );
}
