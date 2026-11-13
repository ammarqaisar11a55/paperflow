import { forwardRef, useId, useState, type InputHTMLAttributes, type ReactNode, type SelectHTMLAttributes } from "react";
import { ChevronDown, Eye, EyeOff } from "lucide-react";
import { cn } from "@/utils/cn";

const control =
  "h-8 w-full rounded-md border border-line bg-surface px-2.5 text-body text-fg placeholder:text-muted/70 transition-colors hover:border-line-strong focus:border-accent focus:outline-none focus:ring-2 focus:ring-accent/20 disabled:opacity-60";

interface FieldProps {
  label: string;
  hint?: ReactNode;
  error?: string | null;
  children: (id: string, describedBy: string | undefined) => ReactNode;
  className?: string;
  trailing?: ReactNode;
}

/** Label + control + hint/error, wired up for accessibility. */
export function Field({ label, hint, error, children, className, trailing }: FieldProps) {
  const id = useId();
  const descId = `${id}-desc`;
  return (
    <div className={cn("flex flex-col gap-1.5", className)}>
      <div className="flex items-center justify-between gap-2">
        <label htmlFor={id} className="text-secondary font-medium text-fg">
          {label}
        </label>
        {trailing}
      </div>
      {children(id, hint || error ? descId : undefined)}
      {error ? (
        <p id={descId} role="alert" className="text-caption text-danger">
          {error}
        </p>
      ) : hint ? (
        <p id={descId} className="text-caption text-muted">
          {hint}
        </p>
      ) : null}
    </div>
  );
}

export const TextInput = forwardRef<HTMLInputElement, InputHTMLAttributes<HTMLInputElement> & { invalid?: boolean }>(
  function TextInput({ className, invalid, ...rest }, ref) {
    return (
      <input
        ref={ref}
        spellCheck={false}
        autoComplete="off"
        className={cn(control, invalid && "border-danger focus:border-danger focus:ring-danger/20", className)}
        {...rest}
      />
    );
  },
);

export function PasswordInput(props: InputHTMLAttributes<HTMLInputElement>) {
  const [visible, setVisible] = useState(false);
  return (
    <div className="relative">
      <TextInput {...props} type={visible ? "text" : "password"} className={cn("pr-9", props.className)} />
      <button
        type="button"
        onClick={() => setVisible((v) => !v)}
        aria-label={visible ? "Hide password" : "Show password"}
        className="absolute right-1 top-1 flex h-6 w-7 items-center justify-center rounded text-muted hover:text-fg"
      >
        {visible ? <EyeOff size={15} /> : <Eye size={15} />}
      </button>
    </div>
  );
}

export function Select({ className, children, ...rest }: SelectHTMLAttributes<HTMLSelectElement>) {
  return (
    <div className="relative">
      <select className={cn(control, "appearance-none pr-8", className)} {...rest}>
        {children}
      </select>
      <ChevronDown size={15} className="pointer-events-none absolute right-2.5 top-2 text-muted" />
    </div>
  );
}

interface SwitchProps {
  checked: boolean;
  onChange: (checked: boolean) => void;
  label: string;
  description?: string;
  disabled?: boolean;
}

export function Switch({ checked, onChange, label, description, disabled }: SwitchProps) {
  const id = useId();
  return (
    <div className="flex items-start justify-between gap-4">
      <label htmlFor={id} className="flex flex-col">
        <span className="text-body text-fg">{label}</span>
        {description && <span className="text-caption text-muted">{description}</span>}
      </label>
      <button
        id={id}
        type="button"
        role="switch"
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onChange(!checked)}
        className={cn(
          "relative mt-0.5 h-5 w-9 shrink-0 rounded-full transition-colors disabled:opacity-50",
          checked ? "bg-accent" : "bg-line-strong",
        )}
      >
        <span
          className={cn(
            "absolute top-0.5 h-4 w-4 rounded-full bg-white shadow-card transition-transform",
            checked ? "translate-x-[18px]" : "translate-x-0.5",
          )}
        />
      </button>
    </div>
  );
}

interface SegmentedProps<T extends string> {
  value: T;
  onChange: (value: T) => void;
  options: { value: T; label: string; icon?: React.ComponentType<{ size?: number }> }[];
  label: string;
  size?: "sm" | "md";
  className?: string;
}

/** A compact radio group styled as a segmented control. */
export function Segmented<T extends string>({ value, onChange, options, label, size = "md", className }: SegmentedProps<T>) {
  return (
    <div role="radiogroup" aria-label={label} className={cn("flex rounded-md bg-surface-2 p-0.5", className)}>
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
              "flex flex-1 items-center justify-center gap-1.5 rounded-[5px] px-2.5 font-medium transition-colors",
              size === "sm" ? "h-6 text-caption" : "h-7 text-secondary",
              selected ? "bg-surface text-fg shadow-card" : "text-muted hover:text-fg",
            )}
          >
            {Icon && <Icon size={14} />}
            {o.label}
          </button>
        );
      })}
    </div>
  );
}

interface SliderProps {
  value: number;
  onChange: (v: number) => void;
  min: number;
  max: number;
  step?: number;
  label: string;
  format?: (v: number) => string;
}

export function Slider({ value, onChange, min, max, step = 1, label, format = String }: SliderProps) {
  const id = useId();
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <label htmlFor={id} className="text-secondary font-medium">
          {label}
        </label>
        <span className="tabular text-secondary text-muted">{format(value)}</span>
      </div>
      <input
        id={id}
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="h-1.5 w-full cursor-pointer appearance-none rounded-full bg-surface-3 accent-[var(--accent)]"
      />
    </div>
  );
}

export function ColorInput({ value, onChange, label }: { value: string; onChange: (v: string) => void; label: string }) {
  return (
    <label className="flex h-8 items-center gap-2 rounded-md border border-line bg-surface px-1.5 hover:border-line-strong">
      <input
        type="color"
        value={value}
        aria-label={label}
        onChange={(e) => onChange(e.target.value)}
        className="h-5 w-6 cursor-pointer rounded border-0 bg-transparent p-0"
      />
      <span className="tabular text-secondary uppercase text-muted">{value}</span>
    </label>
  );
}
