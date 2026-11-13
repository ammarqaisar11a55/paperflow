import { useState } from "react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Switch } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { OptionCards } from "@/features/shared/controls";
import { useSettings } from "@/stores/settings";
import type { CompressionPreset } from "@/types";

export const PRESETS: { value: CompressionPreset; title: string; description: string; badge?: string }[] = [
  { value: "extreme", title: "Extreme", description: "Smallest file. Images at 72 dpi, best for screens and email." },
  { value: "recommended", title: "Recommended", description: "Good quality, much smaller. Images at 150 dpi.", badge: "Default" },
  { value: "light", title: "Light", description: "Print quality. Images at 300 dpi, modest savings." },
];

export function CompressTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const defaultPreset = useSettings((s) => s.settings.compressionPreset);
  const [preset, setPreset] = useState<CompressionPreset>(defaultPreset);
  const [grayscale, setGrayscale] = useState(false);

  return (
    <ToolWorkspace
      tool={getTool("compress")!}
      selection={selection}
      showReduction
      buildOptions={() => ({ preset, grayscale })}
      options={
        <>
          <OptionCards label="Compression level" value={preset} onChange={setPreset} options={PRESETS} />
          <Switch checked={grayscale} onChange={setGrayscale} label="Convert to grayscale" description="Removes colour for extra savings." />
        </>
      }
    />
  );
}
