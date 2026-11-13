import { create } from "zustand";
import { api } from "@/services/api";
import type { Settings, ThemePreference } from "@/types";

export const DEFAULT_SETTINGS: Settings = {
  theme: "system",
  outputLocation: "sameAsSource",
  outputDir: null,
  compressionPreset: "recommended",
  imageDpi: 150,
  imageFormat: "png",
  overwrite: "rename",
  ocrLanguage: "eng",
  favorites: [],
  sidebarCollapsed: false,
  recordActivity: true,
};

const THEME_CACHE_KEY = "paperflow.theme";

interface SettingsState {
  settings: Settings;
  loaded: boolean;
  load: () => Promise<void>;
  update: (patch: Partial<Settings>) => Promise<void>;
  toggleFavorite: (toolId: string) => Promise<void>;
}

export const useSettings = create<SettingsState>((set, get) => ({
  settings: { ...DEFAULT_SETTINGS, theme: cachedTheme() },
  loaded: false,
  load: async () => {
    try {
      const settings = await api.getSettings();
      set({ settings, loaded: true });
      cacheTheme(settings.theme);
    } catch {
      set({ loaded: true });
    }
  },
  update: async (patch) => {
    const previous = get().settings;
    const next = { ...previous, ...patch };
    set({ settings: next });
    if (patch.theme) cacheTheme(patch.theme);
    try {
      const saved = await api.saveSettings(next);
      set({ settings: saved });
    } catch {
      set({ settings: previous });
    }
  },
  toggleFavorite: async (toolId) => {
    const favs = get().settings.favorites;
    const favorites = favs.includes(toolId) ? favs.filter((f) => f !== toolId) : [...favs, toolId];
    await get().update({ favorites });
  },
}));

function cachedTheme(): ThemePreference {
  try {
    const v = localStorage.getItem(THEME_CACHE_KEY);
    return v === "light" || v === "dark" || v === "system" ? v : "system";
  } catch {
    return "system";
  }
}

function cacheTheme(theme: ThemePreference) {
  try {
    localStorage.setItem(THEME_CACHE_KEY, theme);
  } catch {
    /* the cache only avoids a flash at start-up */
  }
}

export function resolveTheme(pref: ThemePreference, systemDark: boolean): "light" | "dark" {
  if (pref === "system") return systemDark ? "dark" : "light";
  return pref;
}
