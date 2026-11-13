import { useFileSelection } from "@/hooks/useFileSelection";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";

export function MergeTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const tool = getTool("merge")!;
  return (
    <ToolWorkspace
      tool={tool}
      selection={selection}
      buildOptions={() => ({})}
      defaultName={() => "Merged document"}
      options={
        <p className="text-secondary text-muted">
          Files are combined top to bottom. Drag them or use the arrows to change the order.
        </p>
      }
    />
  );
}
