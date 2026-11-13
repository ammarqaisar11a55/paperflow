import { ArrowRight, Star } from "lucide-react";
import type { ToolDefinition } from "@/features/registry";
import { useSettings } from "@/stores/settings";
import { cn } from "@/utils/cn";

interface ToolCardProps {
  tool: ToolDefinition;
  onOpen: () => void;
  compact?: boolean;
}

/** A tool in the catalogue: icon, title, description and a favourite toggle. */
export function ToolCard({ tool, onOpen, compact }: ToolCardProps) {
  const favorite = useSettings((s) => s.settings.favorites.includes(tool.id));
  const toggleFavorite = useSettings((s) => s.toggleFavorite);
  const Icon = tool.icon;

  if (compact) {
    return (
      <button
        type="button"
        onClick={onOpen}
        className="group flex items-center gap-3 rounded-xl border border-line bg-surface px-4 py-3.5 text-left shadow-card transition-[border-color,box-shadow] duration-150 hover:border-accent/50 hover:shadow-raised"
      >
        <span className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-accent-soft text-accent">
          <Icon size={18} strokeWidth={1.8} />
        </span>
        <span className="text-card font-semibold">{tool.title}</span>
        <ArrowRight size={15} className="ml-auto text-muted opacity-0 transition-opacity group-hover:opacity-100" />
      </button>
    );
  }

  return (
    <div className="group relative">
      <button
        type="button"
        onClick={onOpen}
        className="flex h-full w-full flex-col items-start gap-3 rounded-xl border border-line bg-surface p-5 text-left shadow-card transition-[border-color,box-shadow,transform] duration-150 hover:border-accent/50 hover:shadow-raised"
      >
        <span className="flex h-10 w-10 items-center justify-center rounded-lg bg-accent-soft text-accent">
          <Icon size={19} strokeWidth={1.8} />
        </span>
        <span className="flex flex-col gap-1">
          <span className="text-card font-semibold">{tool.title}</span>
          <span className="text-secondary text-muted">{tool.description}</span>
        </span>
        <span className="mt-auto flex items-center gap-1 pt-1 text-secondary font-medium text-accent opacity-0 transition-opacity group-hover:opacity-100 group-focus-within:opacity-100">
          Open tool <ArrowRight size={14} />
        </span>
      </button>
      <button
        type="button"
        aria-label={favorite ? `Remove ${tool.title} from favorites` : `Add ${tool.title} to favorites`}
        aria-pressed={favorite}
        onClick={() => void toggleFavorite(tool.id)}
        className={cn(
          "absolute right-3 top-3 flex h-7 w-7 items-center justify-center rounded-md transition-[opacity,color]",
          favorite ? "text-warning opacity-100" : "text-muted opacity-0 hover:bg-surface-2 hover:text-fg group-hover:opacity-100 focus-visible:opacity-100",
        )}
      >
        <Star size={15} className={favorite ? "fill-current" : ""} />
      </button>
    </div>
  );
}
