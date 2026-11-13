import { useEffect, useRef, useState } from "react";
import { CircleAlert, CircleCheck, CircleSlash, FolderOpen, LoaderCircle, ShieldCheck, X } from "lucide-react";
import { ProgressBar } from "@/components/ui/Display";
import { api } from "@/services/api";
import { activeJobs, isFinished, useJobs } from "@/stores/jobs";
import type { JobSnapshot } from "@/types";
import { cn } from "@/utils/cn";

function JobRow({ job }: { job: JobSnapshot }) {
  const cancel = useJobs((s) => s.cancel);
  const output = job.result?.outputFolder ?? job.result?.outputFiles[0];
  return (
    <li className="flex flex-col gap-1.5 px-3 py-2.5">
      <div className="flex items-center gap-2">
        {job.status === "completed" ? (
          <CircleCheck size={15} className="text-success" />
        ) : job.status === "failed" ? (
          <CircleAlert size={15} className="text-danger" />
        ) : job.status === "cancelled" ? (
          <CircleSlash size={15} className="text-muted" />
        ) : (
          <LoaderCircle size={15} className="animate-spin text-accent-text" />
        )}
        <span className="text-secondary font-medium">{job.title}</span>
        <span className="truncate text-caption text-muted">{job.inputNames.join(", ")}</span>
        <span className="ml-auto flex items-center">
          {!isFinished(job) && (
            <button
              type="button"
              onClick={() => void cancel(job.id)}
              className="rounded px-1.5 text-caption text-muted hover:bg-surface-2 hover:text-danger"
            >
              Cancel
            </button>
          )}
          {job.status === "completed" && output && (
            <button
              type="button"
              aria-label="Show in folder"
              onClick={() => void api.revealPath(output)}
              className="rounded p-1 text-muted hover:bg-surface-2 hover:text-fg"
            >
              <FolderOpen size={14} />
            </button>
          )}
        </span>
      </div>
      {!isFinished(job) && <ProgressBar value={job.status === "queued" ? 0 : job.progress} label={`${job.title} progress`} />}
      <p className={cn("truncate text-caption", job.status === "failed" ? "text-danger" : "text-muted")}>
        {job.status === "queued" ? "Waiting for another job to finish" : job.message}
      </p>
    </li>
  );
}

/** Bottom bar: privacy reassurance and a live summary of running jobs. */
export function StatusBar() {
  const jobs = useJobs((s) => s.jobs);
  const clearFinished = useJobs((s) => s.clearFinished);
  const [open, setOpen] = useState(false);
  const panelRef = useRef<HTMLDivElement>(null);
  const active = activeJobs(jobs);
  const all = Object.values(jobs).sort((a, b) => b.createdAt - a.createdAt);
  const running = active.filter((j) => j.status === "running");
  const overall =
    running.length > 0 ? running.reduce((sum, j) => sum + (j.progress ?? 0), 0) / running.length : active.length ? 0 : null;

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && setOpen(false);
    window.addEventListener("mousedown", onDown);
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("mousedown", onDown);
      window.removeEventListener("keydown", onKey);
    };
  }, [open]);

  return (
    <footer className="relative z-10 flex h-7 shrink-0 items-center gap-3 border-t border-line bg-chrome px-3 text-caption text-muted">
      <span className="flex items-center gap-1.5">
        <ShieldCheck size={13} className="text-success" />
        Offline · all processing happens on this device
      </span>

      <div ref={panelRef} className="ml-auto">
        <button
          type="button"
          onClick={() => setOpen((o) => !o)}
          aria-expanded={open}
          disabled={all.length === 0}
          className="flex h-5 items-center gap-2 rounded px-1.5 hover:bg-surface-2 hover:text-fg disabled:hover:bg-transparent disabled:hover:text-muted"
        >
          {active.length > 0 ? (
            <>
              <LoaderCircle size={12} className="animate-spin text-accent-text" />
              <span>
                {active.length === 1 ? active[0].title : `${active.length} jobs`} · {Math.round((overall ?? 0) * 100)}%
              </span>
              <span className="w-20">
                <ProgressBar value={overall} label="Overall progress" />
              </span>
            </>
          ) : all.length ? (
            <span>Jobs ({all.length})</span>
          ) : (
            <span>No jobs</span>
          )}
        </button>

        {open && all.length > 0 && (
          <div className="animate-in absolute bottom-8 right-3 w-[380px] overflow-hidden rounded-xl border border-line bg-surface shadow-pop">
            <div className="flex items-center justify-between border-b border-line px-3 py-2">
              <span className="text-secondary font-semibold text-fg">Jobs</span>
              <div className="flex items-center gap-1">
                {all.some(isFinished) && (
                  <button type="button" onClick={() => void clearFinished()} className="rounded px-1.5 py-0.5 hover:bg-surface-2 hover:text-fg">
                    Clear finished
                  </button>
                )}
                <button type="button" aria-label="Close" onClick={() => setOpen(false)} className="rounded p-1 hover:bg-surface-2 hover:text-fg">
                  <X size={13} />
                </button>
              </div>
            </div>
            <ul className="max-h-[360px] divide-y divide-line overflow-y-auto">
              {all.map((j) => (
                <JobRow key={j.id} job={j} />
              ))}
            </ul>
          </div>
        )}
      </div>
    </footer>
  );
}
