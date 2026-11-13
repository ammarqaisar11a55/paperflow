import { useEffect, useRef } from "react";
import { Monitor, Moon, Search, Settings as SettingsIcon, Sun, X } from "lucide-react";
import { IconButton } from "@/components/ui/Button";
import { Kbd } from "@/components/ui/Display";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";
import type { ThemePreference } from "@/types";
import { cn } from "@/utils/cn";
import { AppLogo } from "./AppLogo";
import { WindowControls } from "./WindowControls";

const THEME_NEXT: Record<ThemePreference, ThemePreference> = { system: "light", light: "dark", dark: "system" };
const THEME_LABEL: Record<ThemePreference, string> = {
  system: "Theme: System",
  light: "Theme: Light",
  dark: "Theme: Dark",
};
const THEME_ICON = { system: Monitor, light: Sun, dark: Moon };

interface TitleBarProps {
  maximized: boolean;
  focused: boolean;
  onMinimize: () => void;
  onToggleMaximize: () => void;
  onClose: () => void;
}

/**
 * The application header and window title bar in one: brand, global tool
 * search, theme and settings, and the window controls. Empty areas drag the
 * window; double-clicking them maximizes or restores it.
 */
export function TitleBar({ maximized, focused, onMinimize, onToggleMaximize, onClose }: TitleBarProps) {
  const search = useNavigation((s) => s.search);
  const setSearch = useNavigation((s) => s.setSearch);
  const navigate = useNavigation((s) => s.navigate);
  const view = useNavigation((s) => s.route.view);
  const theme = useSettings((s) => s.settings.theme);
  const updateSettings = useSettings((s) => s.update);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "k") {
        e.preventDefault();
        inputRef.current?.focus();
        inputRef.current?.select();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const ThemeIcon = THEME_ICON[theme];

  return (
    <header
      data-tauri-drag-region
      className={cn(
        "relative z-20 flex h-11 shrink-0 items-center gap-3 border-b border-line bg-chrome pl-3 pr-2.5 transition-opacity",
        !focused && "[&>*]:opacity-75",
      )}
    >
      <div data-tauri-drag-region className="flex w-[216px] shrink-0 items-center gap-2">
        <AppLogo size={22} />
        <span data-tauri-drag-region className="text-card font-semibold tracking-[-0.01em]">
          PaperFlow
        </span>
      </div>

      <div data-tauri-drag-region className="flex flex-1 justify-center">
        <div className="relative w-full max-w-[420px]">
          <Search size={15} className="pointer-events-none absolute left-2.5 top-[7px] text-muted" />
          <input
            ref={inputRef}
            type="search"
            value={search}
            aria-label="Search tools"
            placeholder="Search tools…"
            onChange={(e) => {
              setSearch(e.target.value);
              if (e.target.value && view !== "tools") navigate({ view: "tools" });
            }}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setSearch("");
                inputRef.current?.blur();
              }
            }}
            className="h-[30px] w-full rounded-md border border-transparent bg-surface-2 pl-8 pr-14 text-body text-fg placeholder:text-muted transition-colors hover:border-line focus:border-accent focus:bg-surface focus:outline-none focus:ring-2 focus:ring-accent/20 [&::-webkit-search-cancel-button]:hidden"
          />
          {search ? (
            <button
              type="button"
              aria-label="Clear search"
              onClick={() => setSearch("")}
              className="absolute right-1.5 top-[5px] flex h-5 w-5 items-center justify-center rounded text-muted hover:bg-surface-3 hover:text-fg"
            >
              <X size={13} />
            </button>
          ) : (
            <span className="pointer-events-none absolute right-2 top-[6px]">
              <Kbd>Ctrl K</Kbd>
            </span>
          )}
        </div>
      </div>

      <div data-tauri-drag-region className="flex shrink-0 items-center gap-1">
        <IconButton icon={ThemeIcon} label={THEME_LABEL[theme]} onClick={() => updateSettings({ theme: THEME_NEXT[theme] })} />
        <IconButton
          icon={SettingsIcon}
          label="Settings"
          shortcut="Ctrl ,"
          active={view === "settings"}
          onClick={() => navigate({ view: "settings" })}
        />
        <div data-tauri-drag-region className="mx-2 h-5 w-px bg-line" />
        <WindowControls maximized={maximized} onMinimize={onMinimize} onToggleMaximize={onToggleMaximize} onClose={onClose} />
      </div>
    </header>
  );
}
