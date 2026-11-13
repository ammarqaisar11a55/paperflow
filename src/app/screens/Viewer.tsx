import { useCallback, useEffect, useState } from "react";
import { Page } from "@/components/layout/Page";
import { DropZone } from "@/components/files/DropZone";
import { PdfViewer } from "@/components/pdf/PdfViewer";
import { useWindowState } from "@/hooks/useWindowState";
import { api, toAppError } from "@/services/api";
import { pickPdfs } from "@/services/dialogs";
import { useDrop } from "@/stores/drop";
import { useJobs } from "@/stores/jobs";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";
import { describePages, stem } from "@/utils/format";

export function Viewer() {
  const takeHandoff = useNavigation((s) => s.takeHandoff);
  const startJob = useJobs((s) => s.start);
  const overwrite = useSettings((s) => s.settings.overwrite);
  const win = useWindowState();
  const [path, setPath] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const register = useDrop((s) => s.register);

  const open = useCallback(async (paths: string[]) => {
    const [first] = paths;
    if (!first) return;
    const [info] = await api.inspectFiles([first]);
    if (info?.kind !== "pdf" || info.error) {
      setError(info?.error ?? "Only PDF files can be viewed.");
      return;
    }
    if (info.encrypted) {
      setError(`“${info.name}” is password-protected. Unlock it first with Unlock PDF.`);
      return;
    }
    setError(null);
    setPath(first);
  }, []);

  useEffect(() => {
    const handed = takeHandoff();
    if (handed.length) void open(handed);
  }, [takeHandoff, open]);

  // While a document is open, dropping another PDF replaces it.
  useEffect(() => (path ? register((p) => void open(p), "Drop to open in the viewer") : undefined), [path, register, open]);

  // Leave full screen when the viewer closes.
  useEffect(() => () => void win.setFullscreen(false), []); // eslint-disable-line react-hooks/exhaustive-deps

  const browse = async () => open(await pickPdfs(false));

  if (!path) {
    return (
      <Page title="Viewer" description="Read, search and navigate PDFs. Rendering happens on this computer.">
        <DropZone accepts="pdf" multiple={false} onFiles={(p) => void open(p)} onBrowse={browse} dropLabel="Drop a PDF to open it" />
        {error && <p className="mt-3 text-secondary text-warning">{error}</p>}
      </Page>
    );
  }

  return (
    <PdfViewer
      path={path}
      fullscreen={win.fullscreen}
      onFullscreen={(on) => void win.setFullscreen(on)}
      onClose={() => {
        void win.setFullscreen(false);
        setPath(null);
      }}
      onOpenAnother={browse}
      onExtract={async (pages) => {
        try {
          const dir = await api.suggestOutputDir(path);
          await startJob({
            type: "extractPages",
            inputFiles: [path],
            outputDir: dir,
            outputName: `${stem(path)} - pages ${describePages(pages)}`,
            options: { pages: pages.join(",") },
            overwrite,
          });
        } catch (e) {
          setError(toAppError(e).message);
        }
      }}
    />
  );
}
