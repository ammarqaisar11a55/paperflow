import type { ReactNode } from "react";
import { ArrowLeft } from "lucide-react";
import { useNavigation } from "@/stores/navigation";
import { cn } from "@/utils/cn";

interface PageProps {
  title?: string;
  description?: string;
  back?: string;
  actions?: ReactNode;
  children: ReactNode;
  wide?: boolean;
  className?: string;
}

/** Scrollable content area with the standard heading. */
export function Page({ title, description, back, actions, children, wide, className }: PageProps) {
  const goBack = useNavigation((s) => s.back);
  return (
    <div className="h-full overflow-y-auto">
      <div className={cn("mx-auto px-10 pb-12 pt-7", wide ? "max-w-[1400px]" : "max-w-[1200px]", className)}>
        {back && (
          <button
            type="button"
            onClick={goBack}
            className="mb-4 -ml-1.5 flex items-center gap-1.5 rounded-md px-1.5 py-1 text-secondary text-muted hover:bg-surface-2 hover:text-fg"
          >
            <ArrowLeft size={14} />
            {back}
          </button>
        )}
        {title && (
          <div className="mb-7 flex items-end justify-between gap-6">
            <div>
              <h1 className="text-title font-semibold tracking-[-0.015em]">{title}</h1>
              {description && <p className="mt-1 text-body text-muted">{description}</p>}
            </div>
            {actions && <div className="flex shrink-0 items-center gap-2">{actions}</div>}
          </div>
        )}
        {children}
      </div>
    </div>
  );
}
