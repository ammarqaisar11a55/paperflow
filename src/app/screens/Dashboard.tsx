import { useCallback, useState } from "react";
import { ArrowRight, Clock, FileText, Images, X } from "lucide-react";
import { Page } from "@/components/layout/Page";
import { DropZone } from "@/components/files/DropZone";
import { ActivityList } from "@/components/tools/ActivityList";
import { ToolCard } from "@/components/tools/ToolCard";
import { Button } from "@/components/ui/Button";
import { EmptyState, SectionTitle } from "@/components/ui/Display";
import { QUICK_ACTIONS, TOOLS, getTool, type ToolId } from "@/features/registry";
import { api } from "@/services/api";
import { pickFiles } from "@/services/dialogs";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import type { FileInfo } from "@/types";
import { pluralize } from "@/utils/format";

/** Tools suggested for files dropped on the dashboard. */
const CONVERTERS: Partial<Record<FileInfo["kind"], ToolId>> = {
  image: "images-to-pdf",
  word: "word-to-pdf",
  presentation: "powerpoint-to-pdf",
  spreadsheet: "excel-to-pdf",
  html: "html-to-pdf",
};

export function suggestTools(files: FileInfo[]): ToolId[] {
  // Documents that are not PDFs: offer the matching conversion(s).
  const converters = [...new Set(files.map((f) => CONVERTERS[f.kind]).filter((t): t is ToolId => !!t))];
  if (files.length && files.every((f) => f.kind !== "pdf")) return converters;
  const pdfs = files.filter((f) => f.kind === "pdf");
  if (pdfs.some((f) => f.encrypted)) return ["unlock"];
  if (pdfs.length > 1) return ["merge", "compress", "rotate", "watermark", "page-numbers", "ocr"];
  return ["compress", "organize", "split", "pdf-to-images", "pdf-to-markdown", "watermark", "ocr", "protect"];
}

export function Dashboard() {
  const openTool = useNavigation((s) => s.openTool);
  const navigate = useNavigation((s) => s.navigate);
  const activity = useJobs((s) => s.activity);
  const [staged, setStaged] = useState<FileInfo[]>([]);

  const stage = useCallback(async (paths: string[]) => {
    if (!paths.length) return;
    const infos = await api.inspectFiles(paths).catch(() => []);
    setStaged(infos.filter((f) => f.kind !== "unsupported" && !f.error));
  }, []);

  const suggestions = suggestTools(staged);
  const images = staged.length > 0 && staged.every((f) => f.kind === "image");

  return (
    <Page>
      <header className="mb-7 mt-2">
        <h1 className="text-hero font-semibold tracking-[-0.02em]">Everything you need for PDFs.</h1>
        <p className="mt-1.5 text-section text-muted">Your documents stay on your device. Always.</p>
      </header>

      {staged.length === 0 ? (
        <DropZone accepts="any" onFiles={stage} onBrowse={async () => stage(await pickFiles("any", true))} dropLabel="Drop files to choose a tool" />
      ) : (
        <div className="animate-in rounded-xl border border-accent/40 bg-accent-soft/40 p-5">
          <div className="flex items-start justify-between gap-4">
            <div className="flex items-center gap-3">
              <span className="flex h-10 w-10 items-center justify-center rounded-lg bg-surface text-accent-text shadow-card">
                {images ? <Images size={19} /> : <FileText size={19} />}
              </span>
              <div>
                <p className="text-card font-semibold">{pluralize(staged.length, images ? "image" : "file")} ready</p>
                <p className="max-w-[640px] truncate text-secondary text-muted">{staged.map((f) => f.name).join(", ")}</p>
              </div>
            </div>
            <Button variant="ghost" size="sm" icon={X} onClick={() => setStaged([])}>
              Clear
            </Button>
          </div>
          <p className="mb-2 mt-4 text-secondary font-medium">What would you like to do?</p>
          <div className="flex flex-wrap gap-2">
            {suggestions.map((id) => {
              const t = getTool(id)!;
              return (
                <Button key={id} icon={t.icon} onClick={() => openTool(id, staged.map((f) => f.path))}>
                  {t.title}
                </Button>
              );
            })}
            <Button variant="ghost" iconRight={ArrowRight} onClick={() => navigate({ view: "tools" })}>
              All tools
            </Button>
          </div>
        </div>
      )}

      <section className="mt-9">
        <SectionTitle>Quick actions</SectionTitle>
        <div className="grid grid-cols-4 gap-3 max-[1180px]:grid-cols-2">
          {QUICK_ACTIONS.map((id) => (
            <ToolCard key={id} tool={getTool(id)!} compact onOpen={() => openTool(id)} />
          ))}
        </div>
      </section>

      <section className="mt-9">
        <SectionTitle
          action={
            <button type="button" onClick={() => navigate({ view: "tools" })} className="flex items-center gap-1 text-secondary font-medium text-accent-text hover:underline">
              Browse by category <ArrowRight size={13} />
            </button>
          }
        >
          All tools
        </SectionTitle>
        <div className="grid grid-cols-[repeat(auto-fill,minmax(230px,1fr))] gap-3">
          {TOOLS.filter((t) => !QUICK_ACTIONS.includes(t.id)).map((t) => (
            <ToolCard key={t.id} tool={t} onOpen={() => openTool(t.id)} />
          ))}
        </div>
      </section>

      <section className="mt-9">
        <SectionTitle
          action={
            activity.length > 5 && (
              <button type="button" onClick={() => navigate({ view: "recent" })} className="flex items-center gap-1 text-secondary font-medium text-accent-text hover:underline">
                View all <ArrowRight size={13} />
              </button>
            )
          }
        >
          Recent activity
        </SectionTitle>
        {activity.length ? (
          <ActivityList entries={activity} limit={5} />
        ) : (
          <div className="rounded-xl border border-line bg-surface">
            <EmptyState icon={Clock} title="Nothing yet">
              Files you process will appear here. Only file names are kept, never their contents.
            </EmptyState>
          </div>
        )}
      </section>
    </Page>
  );
}
