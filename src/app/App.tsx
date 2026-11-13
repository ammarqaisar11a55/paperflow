import { AppShell } from "@/components/layout/AppShell";
import { TOOL_SCREENS } from "@/features";
import { useAppBootstrap, useGlobalShortcuts, useThemeEffect } from "@/hooks/useAppEffects";
import { useNavigation } from "@/stores/navigation";
import { Dashboard } from "./screens/Dashboard";
import { AllTools, Favorites, Recent } from "./screens/Library";
import { SettingsScreen } from "./screens/Settings";
import { Viewer } from "./screens/Viewer";

function Screen() {
  const route = useNavigation((s) => s.route);
  switch (route.view) {
    case "tools":
      return <AllTools />;
    case "recent":
      return <Recent />;
    case "favorites":
      return <Favorites />;
    case "settings":
      return <SettingsScreen />;
    case "viewer":
      return <Viewer />;
    case "tool": {
      const Tool = route.toolId ? TOOL_SCREENS[route.toolId] : undefined;
      // Keyed so switching tools starts fresh.
      return Tool ? <Tool key={route.toolId} /> : <Dashboard />;
    }
    default:
      return <Dashboard />;
  }
}

export function App() {
  useThemeEffect();
  useAppBootstrap();
  useGlobalShortcuts();
  return (
    <AppShell>
      <Screen />
    </AppShell>
  );
}
