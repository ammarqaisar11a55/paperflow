import { CircleAlert, CircleCheck, CircleSlash, FolderOpen, X } from "lucide-react";
import { useState } from "react";
import { IconButton } from "@/components/ui/Button";
import { api, toAppError } from "@/services/api";
import { useJobs } from "@/stores/jobs";
import type { ActivityEntry } from "@/types";
import { basename, dirname, prettyPath, relativeTime } from "@/utils/format";

const STATUS = {
  completed: { icon: CircleCheck, label: "Completed", className: "text-success" },
  failed: { icon: CircleAlert, label: "Failed", className: "text-danger" },
  cancelled: { icon: CircleSlash, label: "Cancelled", className: "text-muted" },
};

/** Recent operations: file name, tool, status, time and where the output went. */
export function ActivityList({ entries, limit }: { entries: ActivityEntry[]; limit?: number }) {
  const remove = useJobs((s) => s.removeActivity);
  const [error, setError] = useState<string | null>(null);
  const shown = limit ? entries.slice(0, limit) : entries;

  return (
    <div className="overflow-hidden rounded-xl border border-line bg-surface shadow-card">
      {error && <p className="border-b border-line bg-danger-soft px-4 py-2 text-secondary text-danger">{error}</p>}
      <table className="w-full table-fixed text-left">
        <thead>
          <tr className="border-b border-line text-caption font-medium uppercase tracking-[0.05em] text-muted">
            <th className="w-[34%] px-4 py-2 font-medium">File</th>
            <th className="w-[18%] px-3 py-2 font-medium">Operation</th>
            <th className="w-[14%] px-3 py-2 font-medium">Status</th>
            <th className="w-[13%] px-3 py-2 font-medium">When</th>
            <th className="px-3 py-2 font-medium">Saved to</th>
          </tr>
        </thead>
        <tbody className="divide-y divide-line">
          {shown.map((e) => {
            const s = STATUS[e.status];
            const out = e.outputFolder ?? e.outputFiles[0] ?? null;
            const where = e.outputFolder ?? (e.outputFiles[0] ? dirname(e.outputFiles[0]) : null);
            return (
              <tr key={e.id} className="group text-secondary hover:bg-surface-2/50">
                <td className="truncate px-4 py-2.5 font-medium text-fg" title={e.fileNames.join(", ")}>
                  {e.fileNames[0]}
                  {e.fileNames.length > 1 && <span className="font-normal text-muted"> +{e.fileNames.length - 1}</span>}
                </td>
                <td className="truncate px-3 py-2.5 text-muted">{e.title}</td>
                <td className="px-3 py-2.5">
                  <span className={`flex items-center gap-1.5 ${s.className}`} title={e.error ?? undefined}>
                    <s.icon size={14} />
                    {s.label}
                  </span>
                </td>
                <td className="truncate px-3 py-2.5 text-muted" title={new Date(e.timestamp).toLocaleString()}>
                  {relativeTime(e.timestamp)}
                </td>
                <td className="px-3 py-2.5">
                  <div className="flex items-center gap-1">
                    <span className="min-w-0 flex-1 truncate text-muted" title={where ?? undefined}>
                      {where ? prettyPath(where) : "—"}
                    </span>
                    {out && (
                      <IconButton
                        icon={FolderOpen}
                        size="sm"
                        label={`Show ${basename(out)}`}
                        className="opacity-0 group-hover:opacity-100"
                        onClick={() =>
                          (e.outputFolder ? api.openPath(out) : api.revealPath(out)).catch((err) => setError(toAppError(err).message))
                        }
                      />
                    )}
                    <IconButton icon={X} size="sm" label="Remove from list" className="opacity-0 group-hover:opacity-100" onClick={() => void remove(e.id)} />
                  </div>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
