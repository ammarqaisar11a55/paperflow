import { useCallback, useEffect, useMemo, useState, type ReactNode } from "react";
import { Star } from "lucide-react";
import { Button, IconButton } from "@/components/ui/Button";
import { Card } from "@/components/ui/Display";
import { Page } from "@/components/layout/Page";
import { DropZone } from "@/components/files/DropZone";
import { FileList } from "@/components/files/FileList";
import { OutputControls } from "@/components/processing/OutputControls";
import { ProcessingPanel } from "@/components/processing/ProcessingPanel";
import type { FileSelection } from "@/hooks/useFileSelection";
import { api, toAppError } from "@/services/api";
import { pickImages, pickPdfs } from "@/services/dialogs";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";
import type { AppError, FileInfo, PdfOperation } from "@/types";
import { stem } from "@/utils/format";
import type { ToolDefinition } from "./registry";

export interface ToolWorkspaceProps {
  tool: ToolDefinition;
  selection: FileSelection;
  /** Tool settings, shown in the side panel. */
  options?: ReactNode;
  /** Extra content under the file list (previews, page pickers). */
  main?: ReactNode;
  /** Options sent with each job; receives the file for per-file jobs. */
  buildOptions: (file: FileInfo | null) => Record<string, unknown>;
  /** Why the tool cannot run yet, or null. */
  blocker?: string | null;
  /** Output extension for single-file results; null when a folder is created. */
  extension?: "pdf" | "txt" | null;
  /** Default output name for a single result. */
  defaultName?: (files: FileInfo[]) => string;
  showReduction?: boolean;
  /** Hide the standard file list (tools with their own file view). */
  hideFileList?: boolean;
}

export function useOutputDir(firstFile: string | null) {
  const { outputLocation, outputDir } = useSettings((s) => s.settings);
  const [dir, setDir] = useState<string | null>(null);
  const [userChose, setUserChose] = useState(false);
  useEffect(() => {
    if (userChose) return;
    let cancelled = false;
    const resolve = async () => {
      if (outputLocation === "custom" && outputDir) return outputDir;
      return api.suggestOutputDir(firstFile);
    };
    resolve()
      .then((d) => !cancelled && setDir(d))
      .catch(() => !cancelled && setDir(null));
    return () => {
      cancelled = true;
    };
  }, [firstFile, outputLocation, outputDir, userChose]);
  return {
    dir,
    setDir: (d: string) => {
      setUserChose(true);
      setDir(d);
    },
    reset: () => setUserChose(false),
  };
}

/**
 * The shared layout and behaviour of every tool: file selection, settings
 * panel, output location, and the processing view once started.
 */
export function ToolWorkspace({
  tool,
  selection,
  options,
  main,
  buildOptions,
  blocker,
  extension = "pdf",
  defaultName,
  showReduction,
  hideFileList,
}: ToolWorkspaceProps) {
  const { files, add, clear } = selection;
  const takeHandoff = useNavigation((s) => s.takeHandoff);
  const favorites = useSettings((s) => s.settings.favorites);
  const overwrite = useSettings((s) => s.settings.overwrite);
  const toggleFavorite = useSettings((s) => s.toggleFavorite);
  const startJob = useJobs((s) => s.start);
  const output = useOutputDir(files[0]?.path ?? null);

  const [name, setName] = useState("");
  const [nameEdited, setNameEdited] = useState(false);
  const [jobIds, setJobIds] = useState<string[] | null>(null);
  const [startErrors, setStartErrors] = useState<AppError[]>([]);
  const [submitting, setSubmitting] = useState(false);

  useEffect(() => {
    const handed = takeHandoff();
    if (handed.length) void add(handed);
  }, [takeHandoff, add]);

  const perFile = tool.inputs === "batch";
  const singleResult = !perFile || files.length === 1;
  const showName = singleResult && files.length > 0;

  const suggested = useMemo(() => {
    if (!files.length) return "";
    if (defaultName) return defaultName(files);
    const base = stem(files[0].path);
    return extension ? `${base} - ${tool.operation === "pdfToText" ? "text" : suffixFor(tool.id)}` : base;
  }, [files, defaultName, extension, tool]);

  useEffect(() => {
    if (!nameEdited) setName(suggested);
  }, [suggested, nameEdited]);

  const browse = useCallback(async () => {
    const paths = tool.accepts === "pdf" ? await pickPdfs(tool.inputs !== "one") : await pickImages(true);
    await add(paths);
  }, [tool, add]);

  const encrypted = files.some((f) => f.encrypted) && tool.operation !== "unlock";
  const minFiles = tool.operation === "merge" ? 2 : 1;
  const problem =
    files.length === 0
      ? `Add ${tool.accepts === "pdf" ? "a PDF" : "an image"} to get started.`
      : files.length < minFiles
        ? "Add at least two PDFs to merge."
        : encrypted
          ? "Protected files must be unlocked first (use Unlock PDF)."
          : !output.dir
            ? "Choose where to save the result."
            : (blocker ?? null);

  const submit = useCallback(async () => {
    if (problem || !output.dir) return;
    setSubmitting(true);
    setStartErrors([]);
    const ops: PdfOperation[] = perFile
      ? files.map((f) => ({
          type: tool.operation,
          inputFiles: [f.path],
          outputDir: output.dir!,
          outputName: files.length === 1 && name.trim() ? name.trim() : undefined,
          options: buildOptions(f),
          overwrite,
        }))
      : [
          {
            type: tool.operation,
            inputFiles: files.map((f) => f.path),
            outputDir: output.dir!,
            outputName: name.trim() || undefined,
            options: buildOptions(files.length === 1 ? files[0] : null),
            overwrite,
          },
        ];
    const ids: string[] = [];
    const errors: AppError[] = [];
    for (const op of ops) {
      try {
        ids.push(await startJob(op));
      } catch (e) {
        errors.push(toAppError(e));
      }
    }
    setJobIds(ids);
    setStartErrors(errors);
    setSubmitting(false);
  }, [problem, output.dir, perFile, files, tool, name, buildOptions, overwrite, startJob]);

  // Ctrl+Enter starts the tool.
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && jobIds === null) {
        e.preventDefault();
        void submit();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o" && jobIds === null) {
        e.preventDefault();
        void browse();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [submit, browse, jobIds]);

  const favorite = favorites.includes(tool.id);
  const header = (
    <IconButton
      icon={Star}
      label={favorite ? "Remove from favorites" : "Add to favorites"}
      active={favorite}
      onClick={() => void toggleFavorite(tool.id)}
      className={favorite ? "[&_svg]:fill-current" : ""}
    />
  );

  if (jobIds !== null) {
    return (
      <Page title={tool.title} description={tool.description} back="Back to tools" actions={header}>
        <ProcessingPanel
          jobIds={jobIds}
          startErrors={startErrors}
          showReduction={showReduction}
          onBack={() => setJobIds(null)}
          onRetry={() => {
            setJobIds(null);
            void submit();
          }}
          onAnother={() => {
            setJobIds(null);
            clear();
            setNameEdited(false);
            output.reset();
          }}
        />
      </Page>
    );
  }

  const Icon = tool.icon;
  return (
    <Page title={tool.title} description={tool.description} back="Back to tools" actions={header}>
      <div className="grid grid-cols-[minmax(0,1fr)_340px] items-start gap-6 max-[1100px]:grid-cols-1">
        <div className="flex min-w-0 flex-col gap-4">
          <DropZone
            accepts={tool.accepts}
            multiple={tool.inputs !== "one"}
            onFiles={add}
            onBrowse={browse}
            dropLabel={`Drop to add to ${tool.title}`}
            compact={files.length > 0}
          />
          {!hideFileList && <FileList selection={selection} orderable={tool.inputs === "many"} />}
          {main}
        </div>

        <Card className="sticky top-0 flex flex-col">
          <div className="flex items-center gap-2.5 border-b border-line px-5 py-3.5">
            <Icon size={17} className="text-accent" />
            <h2 className="text-card font-semibold">Settings</h2>
          </div>
          <div className="flex flex-col gap-5 px-5 py-4">
            {options}
            {options && <div className="h-px bg-line" />}
            <OutputControls
              name={showName ? name : null}
              onName={(n) => {
                setName(n);
                setNameEdited(true);
              }}
              extension={extension}
              nameHint={!extension ? "A new folder with this name will hold the results." : undefined}
              dir={output.dir}
              onDir={output.setDir}
            />
            {perFile && files.length > 1 && (
              <p className="text-caption text-muted">Each file is processed separately and saved with its own name.</p>
            )}
          </div>
          <div className="border-t border-line px-5 py-4">
            <Button variant="primary" size="lg" className="w-full" disabled={!!problem} loading={submitting} onClick={() => void submit()}>
              {tool.action}
              {perFile && files.length > 1 ? ` ${files.length} files` : ""}
            </Button>
            <p className="mt-2 min-h-4 text-center text-caption text-muted">{problem ?? "Ctrl+Enter"}</p>
          </div>
        </Card>
      </div>
    </Page>
  );
}

export function suffixFor(id: string): string {
  const map: Record<string, string> = {
    merge: "merged",
    compress: "compressed",
    rotate: "rotated",
    "extract-pages": "extract",
    organize: "organized",
    "images-to-pdf": "images",
    watermark: "watermarked",
    "page-numbers": "numbered",
    metadata: "edited",
    protect: "protected",
    unlock: "unlocked",
    ocr: "searchable",
  };
  return map[id] ?? "output";
}
