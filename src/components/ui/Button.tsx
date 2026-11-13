import { forwardRef, type ButtonHTMLAttributes, type ReactNode } from "react";
import type { LucideIcon } from "lucide-react";
import { LoaderCircle } from "lucide-react";
import { cn } from "@/utils/cn";
import { Tooltip } from "./Tooltip";

type Variant = "primary" | "secondary" | "ghost" | "danger" | "subtle";
type Size = "sm" | "md" | "lg";

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
  icon?: LucideIcon;
  iconRight?: LucideIcon;
  loading?: boolean;
  children?: ReactNode;
}

const variants: Record<Variant, string> = {
  primary:
    "bg-accent text-accent-contrast hover:bg-accent-hover shadow-card disabled:bg-accent/45 disabled:shadow-none",
  secondary:
    "bg-surface text-fg border border-line hover:border-line-strong hover:bg-surface-2 shadow-card disabled:text-muted",
  ghost: "text-fg hover:bg-surface-2 disabled:text-muted",
  subtle: "bg-surface-2 text-fg hover:bg-surface-3 disabled:text-muted",
  danger: "bg-danger text-white hover:brightness-95 disabled:opacity-50",
};

const sizes: Record<Size, string> = {
  sm: "h-7 px-2.5 text-secondary gap-1.5 rounded-md",
  md: "h-8 px-3 text-body gap-2 rounded-md",
  lg: "h-10 px-5 text-body gap-2 rounded-lg font-semibold",
};

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "secondary", size = "md", icon: Icon, iconRight: IconRight, loading, className, children, disabled, ...rest },
  ref,
) {
  const iconSize = size === "sm" ? 14 : 16;
  return (
    <button
      ref={ref}
      type="button"
      disabled={disabled || loading}
      className={cn(
        "inline-flex shrink-0 select-none items-center justify-center font-medium transition-colors duration-100 disabled:cursor-not-allowed",
        variants[variant],
        sizes[size],
        className,
      )}
      {...rest}
    >
      {loading ? <LoaderCircle size={iconSize} className="animate-spin" /> : Icon && <Icon size={iconSize} strokeWidth={1.9} />}
      {children}
      {IconRight && <IconRight size={iconSize} strokeWidth={1.9} />}
    </button>
  );
});

interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  label: string;
  shortcut?: string;
  size?: "sm" | "md";
  active?: boolean;
  tooltipSide?: "top" | "bottom";
}

/** A square icon-only button with an accessible label and a tooltip. */
export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { icon: Icon, label, shortcut, size = "md", active, className, tooltipSide, ...rest },
  ref,
) {
  const dim = size === "sm" ? "h-7 w-7" : "h-8 w-8";
  return (
    <Tooltip label={label} shortcut={shortcut} side={tooltipSide}>
      <button
        ref={ref}
        type="button"
        aria-label={label}
        aria-pressed={active}
        className={cn(
          "inline-flex shrink-0 items-center justify-center rounded-md transition-colors duration-100 disabled:cursor-not-allowed disabled:opacity-40",
          active ? "bg-accent-soft text-accent-text" : "text-muted hover:bg-surface-2 hover:text-fg",
          dim,
          className,
        )}
        {...rest}
      >
        <Icon size={size === "sm" ? 15 : 17} strokeWidth={1.8} />
      </button>
    </Tooltip>
  );
});
