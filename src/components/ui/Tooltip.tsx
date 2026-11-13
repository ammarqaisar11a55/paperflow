import { cloneElement, useCallback, useEffect, useId, useRef, useState, type ReactElement } from "react";
import { createPortal } from "react-dom";

interface TooltipProps {
  label: string;
  /** Keyboard shortcut shown after the label. */
  shortcut?: string;
  side?: "top" | "bottom";
  children: ReactElement<Record<string, unknown>>;
  delay?: number;
}

/**
 * A small, delayed tooltip rendered in a portal so it is never clipped. The
 * trigger gets `aria-describedby`, so screen readers announce the label too.
 */
export function Tooltip({ label, shortcut, side = "bottom", children, delay = 450 }: TooltipProps) {
  const id = useId();
  const [pos, setPos] = useState<{ x: number; y: number; side: "top" | "bottom" } | null>(null);
  const timer = useRef<number | undefined>(undefined);
  const anchor = useRef<HTMLElement | null>(null);

  const hide = useCallback(() => {
    window.clearTimeout(timer.current);
    setPos(null);
  }, []);

  const show = useCallback(
    (el: HTMLElement, immediate = false) => {
      anchor.current = el;
      window.clearTimeout(timer.current);
      timer.current = window.setTimeout(
        () => {
          const r = el.getBoundingClientRect();
          const below = side === "bottom" ? r.bottom + 34 < window.innerHeight : r.top < 40;
          setPos({ x: r.left + r.width / 2, y: below ? r.bottom + 6 : r.top - 6, side: below ? "bottom" : "top" });
        },
        immediate ? 0 : delay,
      );
    },
    [delay, side],
  );

  useEffect(() => () => window.clearTimeout(timer.current), []);
  useEffect(() => {
    if (!pos) return;
    window.addEventListener("scroll", hide, true);
    window.addEventListener("blur", hide);
    return () => {
      window.removeEventListener("scroll", hide, true);
      window.removeEventListener("blur", hide);
    };
  }, [pos, hide]);

  const props = children.props as Record<string, (e: unknown) => void>;
  const trigger = cloneElement(children, {
    "aria-describedby": pos ? id : undefined,
    onMouseEnter: (e: React.MouseEvent<HTMLElement>) => {
      props.onMouseEnter?.(e);
      show(e.currentTarget);
    },
    onMouseLeave: (e: React.MouseEvent<HTMLElement>) => {
      props.onMouseLeave?.(e);
      hide();
    },
    onMouseDown: (e: React.MouseEvent<HTMLElement>) => {
      props.onMouseDown?.(e);
      hide();
    },
    onFocus: (e: React.FocusEvent<HTMLElement>) => {
      props.onFocus?.(e);
      if (e.currentTarget.matches(":focus-visible")) show(e.currentTarget);
    },
    onBlur: (e: React.FocusEvent<HTMLElement>) => {
      props.onBlur?.(e);
      hide();
    },
  });

  return (
    <>
      {trigger}
      {pos &&
        createPortal(
          <div
            id={id}
            role="tooltip"
            className="animate-in pointer-events-none fixed z-[1000] flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-md bg-[#20242c] px-2 py-1 text-caption font-medium text-white shadow-pop dark:bg-[#3a3f4b]"
            style={{
              left: Math.min(Math.max(pos.x, 60), window.innerWidth - 60),
              top: pos.y,
              transform: `translateX(-50%) ${pos.side === "top" ? "translateY(-100%)" : ""}`,
            }}
          >
            {label}
            {shortcut && <kbd className="rounded bg-white/15 px-1 font-sans text-[10.5px] text-white/80">{shortcut}</kbd>}
          </div>,
          document.body,
        )}
    </>
  );
}
