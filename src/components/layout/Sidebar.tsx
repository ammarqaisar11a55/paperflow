import { BookOpen, Clock, House, LayoutGrid, PanelLeftClose, PanelLeftOpen, ShieldCheck, Star, type LucideIcon } from "lucide-react";
import { Tooltip } from "@/components/ui/Tooltip";
import { useNavigation, type View } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";
import { cn } from "@/utils/cn";

const ITEMS: { view: View; label: string; icon: LucideIcon }[] = [
  { view: "home", label: "Home", icon: House },
  { view: "tools", label: "All Tools", icon: LayoutGrid },
  { view: "viewer", label: "Viewer", icon: BookOpen },
  { view: "recent", label: "Recent", icon: Clock },
  { view: "favorites", label: "Favorites", icon: Star },
];

export function Sidebar() {
  const route = useNavigation((s) => s.route);
  const navigate = useNavigation((s) => s.navigate);
  const collapsed = useSettings((s) => s.settings.sidebarCollapsed);
  const update = useSettings((s) => s.update);

  const isActive = (view: View) => route.view === view || (view === "tools" && route.view === "tool");

  return (
    <nav
      aria-label="Main"
      className={cn(
        "flex shrink-0 flex-col border-r border-line bg-chrome transition-[width] duration-150",
        collapsed ? "w-[72px]" : "w-[240px]",
      )}
    >
      <ul className="flex flex-col gap-0.5 p-3">
        {ITEMS.map(({ view, label, icon: Icon }) => {
          const active = isActive(view);
          const button = (
            <button
              type="button"
              aria-current={active ? "page" : undefined}
              aria-label={collapsed ? label : undefined}
              onClick={() => navigate({ view })}
              className={cn(
                "flex h-9 w-full items-center gap-3 rounded-md px-3 text-body transition-colors",
                collapsed && "justify-center px-0",
                active ? "bg-accent-soft font-medium text-accent-text" : "text-fg/85 hover:bg-surface-2 hover:text-fg",
              )}
            >
              <Icon size={17} strokeWidth={active ? 2 : 1.8} className="shrink-0" />
              {!collapsed && <span className="truncate">{label}</span>}
            </button>
          );
          return <li key={view}>{collapsed ? <Tooltip label={label}>{button}</Tooltip> : button}</li>;
        })}
      </ul>

      <div className="mt-auto flex flex-col gap-2 p-3">
        {!collapsed && (
          <div className="rounded-lg bg-surface-2 p-3">
            <div className="flex items-center gap-2 text-secondary font-semibold text-fg">
              <ShieldCheck size={15} className="text-success" />
              Private by design
            </div>
            <p className="mt-1 text-caption text-muted">Documents are processed on this computer and never uploaded.</p>
          </div>
        )}
        <Tooltip label={collapsed ? "Expand sidebar" : "Collapse sidebar"}>
          <button
            type="button"
            aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
            onClick={() => update({ sidebarCollapsed: !collapsed })}
            className={cn(
              "flex h-8 items-center gap-2 rounded-md px-3 text-secondary text-muted hover:bg-surface-2 hover:text-fg",
              collapsed && "justify-center px-0",
            )}
          >
            {collapsed ? <PanelLeftOpen size={16} /> : <PanelLeftClose size={16} />}
            {!collapsed && "Collapse"}
          </button>
        </Tooltip>
      </div>
    </nav>
  );
}
