import { create } from "zustand";
import type { ToolId } from "@/features/registry";

export type View = "home" | "tools" | "recent" | "favorites" | "settings" | "viewer" | "tool";

export interface Route {
  view: View;
  toolId?: ToolId;
}

interface NavigationState {
  route: Route;
  history: Route[];
  /** Files handed to the next screen (e.g. dropped on the dashboard). */
  handoff: string[];
  /** Global tool search text (title bar). */
  search: string;
  navigate: (route: Route, files?: string[]) => void;
  openTool: (toolId: ToolId, files?: string[]) => void;
  back: () => void;
  takeHandoff: () => string[];
  setSearch: (search: string) => void;
}

export const useNavigation = create<NavigationState>((set, get) => ({
  route: { view: "home" },
  history: [],
  handoff: [],
  search: "",
  navigate: (route, files) =>
    set((s) => ({
      route,
      history: [...s.history, s.route].slice(-30),
      handoff: files ?? [],
    })),
  openTool: (toolId, files) => get().navigate({ view: "tool", toolId }, files),
  back: () =>
    set((s) => {
      const history = [...s.history];
      const previous = history.pop() ?? { view: "home" as View };
      return { route: previous, history, handoff: [] };
    }),
  takeHandoff: () => {
    const files = get().handoff;
    if (files.length) set({ handoff: [] });
    return files;
  },
  setSearch: (search) => set({ search }),
}));
