import { useState } from "react";
import {
  ChevronDown,
  CircleAlert,
  CircleCheck,
  CircleSlash,
  ExternalLink,
  FolderOpen,
  LoaderCircle,
  RefreshCw,
  RotateCcw,
  TriangleAlert,
} from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Card, ProgressBar } from "@/components/ui/Display";
import { api, toAppError } from "@/services/api";
import { isFinished, useJobs } from "@/stores/jobs";
import type { AppError, JobSnapshot } from "@/types";
import { cn } from "@/utils/cn";
import { basename, formatBytes, pluralize, reduction } from "@/utils/format";

interface ProcessingPanelProps {
  jobIds: string[];
  /** Errors from starting jobs (validation), shown like job failures. */
  startErrors: AppError[];
  onAnother: () => void;
  onBack: () => void;
  onRetry: () => void;
  showReduction?: boolean;
}

function useOpen() {
  const [error, setError] = useState<string | null>(null);
  const run = (fn: () => Promise<void>) => {
    setError(null);
    fn().catch((e) => setError(toAppError(e).message));
  };
  return { error, open: (p: string) => run(() => api.openPath(p)), reveal: (p: string) => run(() => api.revealPath(p)) };
}

function ErrorDetails({ error }: { error: AppError }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex flex-col gap-1">
      <p className="text-body font-medium text-fg">{error.message}</p>
      {error.hint && <p className="text-secondary text-muted">{error.hint}</p>}
      {error.detail && (
        <div>
          <button
            type="button"
            onClick={() => setOpen((o) => !o)}
            className="mt-1 flex items-center gap-1 text-caption text-muted hover:text-fg"
            aria-expanded={open}
          >
            <ChevronDown size={12} className={cn("transition-transform", open && "rotate-180")} />
            Technical details
          </button>
          {open && (
            <pre className="selectable mt-1.5 max-h-40 overflow-auto whitespace-pre-wrap rounded-md bg-surface-2 p-2 text-caption text-muted">
              {error.detail}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

function JobResultRow({ job, showReduction }: { job: JobSnapshot; showReduction?: boolean }) {
  const { open, reveal, error } = useOpen();
  const cancel = useJobs((s) => s.cancel);
  const r = job.result;
  const files = r?.outputFiles ?? [];
  const single = files.length === 1 ? files[0] : null;
  const pct = r ? reduction(r.inputSize, r.outputSize) : 0;

  return (
    <div className="flex flex-col gap-2 px-5 py-4">
      <div className="flex items-start gap-3">
        <div className="mt-0.5">
          {job.status === "completed" ? (
            <CircleCheck size={18} className="text-success" />
          ) : job.status === "failed" ? (
            <CircleAlert size={18} className="text-danger" />
          ) : job.status === "cancelled" ? (
            <CircleSlash size={18} className="text-muted" />
          ) : (
            <LoaderCircle size={18} className="animate-spin text-accent" />
          )}
        </div>
        <div className="flex min-w-0 flex-1 flex-col gap-1.5">
          <div className="flex items-baseline justify-between gap-3">
            <p className="truncate text-body font-medium">{job.inputNames.join(", ")}</p>
            {!isFinished(job) && (
              <span className="tabular text-secondary text-muted">
                {job.status === "queued" ? "Queued" : job.progress === null ? "" : `${Math.round(job.progress * 100)}%`}
              </span>
            )}
          </div>

          {!isFinished(job) && (
            <>
              <ProgressBar value={job.status === "queued" ? 0 : job.progress} label={`${job.title} progress`} />
              <div className="flex items-center justify-between">
                <p className="text-secondary text-muted">{job.status === "queued" ? "Waiting for another job to finish…" : `${job.message}…`}</p>
                <button type="button" onClick={() => void cancel(job.id)} className="rounded px-1.5 text-secondary text-muted hover:bg-surface-2 hover:text-danger">
                  Cancel
                </button>
              </div>
            </>
          )}

          {job.status === "completed" && r && (
            <>
              <p className="text-secondary text-muted">
                {single ? (
                  <>
                    Saved as <span className="selectable font-medium text-fg">{basename(single)}</span>
                  </>
                ) : (
                  <>
                    {pluralize(files.length, "file")} saved in{" "}
                    <span className="selectable font-medium text-fg">{basename(r.outputFolder ?? files[0] ?? "")}</span>
                  </>
                )}
                {r.pageCount !== null && ` · ${pluralize(r.pageCount, "page")}`}
              </p>
              {showReduction && (
                <div className="flex items-center gap-3 text-secondary">
                  <span className="text-muted">{formatBytes(r.inputSize)}</span>
                  <span className="text-muted">→</span>
                  <span className="font-semibold">{formatBytes(r.outputSize)}</span>
                  {pct > 0 && <span className="rounded bg-success-soft px-1.5 font-semibold text-success">−{pct}%</span>}
                </div>
              )}
              {r.warnings.map((w) => (
                <p key={w} className="flex items-start gap-1.5 text-secondary text-warning">
                  <TriangleAlert size={14} className="mt-0.5 shrink-0" />
                  {w}
                </p>
              ))}
              <div className="mt-1 flex flex-wrap gap-2">
                {single && (
                  <Button size="sm" icon={ExternalLink} onClick={() => open(single)}>
                    Open file
                  </Button>
                )}
                {(r.outputFolder || single) && (
                  <Button size="sm" icon={FolderOpen} onClick={() => (r.outputFolder ? open(r.outputFolder) : reveal(single!))}>
                    {r.outputFolder ? "Open folder" : "Show in folder"}
                  </Button>
                )}
              </div>
              {error && <p className="text-caption text-danger">{error}</p>}
            </>
          )}

          {job.status === "failed" && job.error && <ErrorDetails error={job.error} />}
          {job.status === "cancelled" && <p className="text-secondary text-muted">Cancelled. No files were written.</p>}
        </div>
      </div>
    </div>
  );
}

/** The unified processing experience: progress, completion and errors. */
export function ProcessingPanel({ jobIds, startErrors, onAnother, onBack, onRetry, showReduction }: ProcessingPanelProps) {
  const all = useJobs((s) => s.jobs);
  const jobs = jobIds.map((id) => all[id]).filter(Boolean);
  const done = jobs.length > 0 && jobs.every(isFinished);
  const failed = jobs.filter((j) => j.status === "failed").length + startErrors.length;
  const completed = jobs.filter((j) => j.status === "completed").length;
  const cancelled = jobs.filter((j) => j.status === "cancelled").length;
  const finished = (done || jobs.length === 0) && (jobs.length > 0 || startErrors.length > 0);
  const title = jobs[0]?.title ?? "Processing";

  let heading: string;
  if (!finished) heading = jobs.length > 1 ? `Processing ${pluralize(jobs.length, "file")}` : `${title} in progress`;
  else if (failed === 0 && cancelled === 0) heading = completed > 1 ? `All ${completed} files are ready` : "Your file is ready";
  else if (completed === 0 && failed > 0) heading = "That didn’t work";
  else if (completed === 0) heading = "Cancelled";
  else heading = `${completed} of ${completed + failed + cancelled} files are ready`;

  return (
    <div className="animate-in mx-auto flex max-w-[720px] flex-col gap-5">
      <Card className="overflow-hidden">
        <div className="flex items-center gap-3 border-b border-line px-5 py-4">
          <div
            className={cn(
              "flex h-9 w-9 items-center justify-center rounded-lg",
              !finished ? "bg-accent-soft text-accent" : failed && !completed ? "bg-danger-soft text-danger" : "bg-success-soft text-success",
            )}
          >
            {!finished ? <LoaderCircle size={18} className="animate-spin" /> : failed && !completed ? <CircleAlert size={18} /> : <CircleCheck size={18} />}
          </div>
          <div>
            <h2 className="text-section font-semibold">{heading}</h2>
            <p className="text-secondary text-muted">{title} · processed on this device</p>
          </div>
        </div>
        <div className="divide-y divide-line">
          {startErrors.map((e, i) => (
            <div key={i} className="flex gap-3 px-5 py-4">
              <CircleAlert size={18} className="mt-0.5 shrink-0 text-danger" />
              <ErrorDetails error={e} />
            </div>
          ))}
          {jobs.map((j) => (
            <JobResultRow key={j.id} job={j} showReduction={showReduction} />
          ))}
        </div>
      </Card>

      {finished && (
        <div className="flex items-center justify-between">
          <Button variant="ghost" icon={RotateCcw} onClick={onBack}>
            Back to settings
          </Button>
          <div className="flex gap-2">
            {failed + cancelled > 0 && (
              <Button icon={RefreshCw} onClick={onRetry}>
                Retry
              </Button>
            )}
            <Button variant="primary" onClick={onAnother}>
              Process another document
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}
