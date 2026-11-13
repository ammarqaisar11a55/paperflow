import { useEffect, useState, type ReactNode } from "react";
import { CircleCheck, CircleX, FolderOpen, Monitor, Moon, ShieldCheck, Sun, Trash2 } from "lucide-react";
import { Page } from "@/components/layout/Page";
import { Button } from "@/components/ui/Button";
import { Badge, Card } from "@/components/ui/Display";
import { Field, Segmented, Select, Switch, TextInput } from "@/components/ui/Form";
import { PRESETS } from "@/features/compress/CompressTool";
import { languageName, useOcrLanguages } from "@/features/ocr/OcrTool";
import { api, toAppError } from "@/services/api";
import { pickFolder } from "@/services/dialogs";
import { useJobs } from "@/stores/jobs";
import { useSettings } from "@/stores/settings";
import type { AppInfo, EngineInfo, ThemePreference } from "@/types";
import { formatBytes, prettyPath } from "@/utils/format";

function Section({ title, description, children }: { title: string; description?: string; children: ReactNode }) {
  return (
    <section className="grid grid-cols-[240px_minmax(0,1fr)] gap-8 border-b border-line py-7 first:pt-0 last:border-0 max-[1000px]:grid-cols-1 max-[1000px]:gap-3">
      <div>
        <h2 className="text-section font-semibold">{title}</h2>
        {description && <p className="mt-1 text-secondary text-muted">{description}</p>}
      </div>
      <Card className="flex flex-col gap-5 p-5">{children}</Card>
    </section>
  );
}

const ENGINE_PURPOSE: Record<string, string> = {
  ghostscript: "Compression",
  qpdf: "Protect and unlock",
  pdftoppm: "PDF to images, OCR rendering",
  pdftotext: "PDF to text",
  pdfimages: "Extract images",
  tesseract: "OCR",
};

export function SettingsScreen() {
  const settings = useSettings((s) => s.settings);
  const update = useSettings((s) => s.update);
  const clearActivity = useJobs((s) => s.clearActivity);
  const activityCount = useJobs((s) => s.activity.length);
  const { languages, error: ocrError } = useOcrLanguages();
  const [engines, setEngines] = useState<EngineInfo[] | null>(null);
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [tempSize, setTempSize] = useState<number | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    api.engineStatus().then(setEngines).catch(() => setEngines([]));
    api.appInfo().then(setInfo).catch(() => undefined);
    api.tempUsage().then(setTempSize).catch(() => undefined);
  }, []);

  const flash = (m: string) => {
    setMessage(m);
    window.setTimeout(() => setMessage(null), 3000);
  };

  return (
    <Page title="Settings" description="Preferences are saved on this computer and kept between sessions.">
      {message && (
        <div role="status" className="animate-in mb-4 rounded-lg bg-success-soft px-3 py-2 text-secondary text-success">
          {message}
        </div>
      )}

      <Section title="Appearance" description="Follow the system, or choose light or dark.">
        <Segmented<ThemePreference>
          label="Theme"
          value={settings.theme}
          onChange={(theme) => void update({ theme })}
          options={[
            { value: "system", label: "System", icon: Monitor },
            { value: "light", label: "Light", icon: Sun },
            { value: "dark", label: "Dark", icon: Moon },
          ]}
          className="max-w-[360px]"
        />
        <Switch
          checked={settings.sidebarCollapsed}
          onChange={(sidebarCollapsed) => void update({ sidebarCollapsed })}
          label="Compact sidebar"
          description="Show only icons in the sidebar (Ctrl+B)."
        />
      </Section>

      <Section title="Processing" description="Defaults used by the tools. You can change them for each job.">
        <Field label="Default output folder">
          {(id) => (
            <div className="flex flex-col gap-2">
              <Segmented
                label="Default output folder"
                value={settings.outputLocation}
                onChange={(outputLocation) => void update({ outputLocation })}
                options={[
                  { value: "sameAsSource", label: "Next to the original" },
                  { value: "custom", label: "A fixed folder" },
                ]}
                className="max-w-[360px]"
              />
              {settings.outputLocation === "custom" && (
                <div className="flex gap-2">
                  <TextInput id={id} readOnly value={settings.outputDir ? prettyPath(settings.outputDir) : "No folder chosen"} className="text-muted" />
                  <Button
                    icon={FolderOpen}
                    onClick={async () => {
                      const dir = await pickFolder(settings.outputDir);
                      if (dir) void update({ outputDir: dir });
                    }}
                  >
                    Choose
                  </Button>
                </div>
              )}
            </div>
          )}
        </Field>
        <div className="grid grid-cols-2 gap-4">
          <Field label="Compression preset">
            {(id) => (
              <Select id={id} value={settings.compressionPreset} onChange={(e) => void update({ compressionPreset: e.target.value as typeof settings.compressionPreset })}>
                {PRESETS.map((p) => (
                  <option key={p.value} value={p.value}>
                    {p.title}
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label="Image resolution" hint="For PDF to Images.">
            {(id, d) => (
              <Select id={id} aria-describedby={d} value={settings.imageDpi} onChange={(e) => void update({ imageDpi: Number(e.target.value) })}>
                {[72, 96, 150, 200, 300, 600].map((v) => (
                  <option key={v} value={v}>
                    {v} dpi
                  </option>
                ))}
              </Select>
            )}
          </Field>
          <Field label="Image format">
            {(id) => (
              <Select id={id} value={settings.imageFormat} onChange={(e) => void update({ imageFormat: e.target.value as "png" | "jpeg" })}>
                <option value="png">PNG (lossless)</option>
                <option value="jpeg">JPEG (smaller)</option>
              </Select>
            )}
          </Field>
          <Field label="When a file already exists" hint="Your original files are never replaced.">
            {(id, d) => (
              <Select id={id} aria-describedby={d} value={settings.overwrite} onChange={(e) => void update({ overwrite: e.target.value as "rename" | "overwrite" })}>
                <option value="rename">Keep both (add a number)</option>
                <option value="overwrite">Replace the existing file</option>
              </Select>
            )}
          </Field>
        </div>
      </Section>

      <Section title="Privacy" description="PaperFlow works entirely offline.">
        <div className="flex gap-3 rounded-lg bg-success-soft/60 p-3">
          <ShieldCheck size={18} className="mt-0.5 shrink-0 text-success" />
          <ul className="flex flex-col gap-1 text-secondary">
            <li>Documents are processed on this computer and never uploaded.</li>
            <li>No account, no telemetry, no analytics, no network access.</li>
            <li>Passwords are used once and never stored or logged.</li>
          </ul>
        </div>
        <Switch
          checked={settings.recordActivity}
          onChange={(recordActivity) => void update({ recordActivity })}
          label="Keep recent activity"
          description="File names, tools used and output locations. Never contents."
        />
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-body">Recent activity</p>
            <p className="text-caption text-muted">{activityCount} entries</p>
          </div>
          <Button icon={Trash2} disabled={!activityCount} onClick={() => void clearActivity().then(() => flash("Recent activity cleared."))}>
            Clear
          </Button>
        </div>
        <div className="flex items-center justify-between gap-4">
          <div>
            <p className="text-body">Temporary files</p>
            <p className="text-caption text-muted">
              {tempSize === null ? "…" : tempSize === 0 ? "None — cleaned after every job" : `${formatBytes(tempSize)} in use`}
            </p>
          </div>
          <Button
            icon={Trash2}
            onClick={() =>
              api
                .clearTemp()
                .then((freed) => {
                  setTempSize(0);
                  flash(freed ? `Freed ${formatBytes(freed)}.` : "No temporary files to remove.");
                })
                .catch((e) => flash(toAppError(e).message))
            }
          >
            Clear
          </Button>
        </div>
      </Section>

      <Section title="OCR" description="Text recognition uses language data installed with PaperFlow.">
        <Field label="Default language">
          {(id) => (
            <Select id={id} value={settings.ocrLanguage} disabled={!languages?.length} onChange={(e) => void update({ ocrLanguage: e.target.value })}>
              {(languages ?? [settings.ocrLanguage]).map((l) => (
                <option key={l} value={l}>
                  {languageName(l)}
                </option>
              ))}
            </Select>
          )}
        </Field>
        <div>
          <p className="mb-2 text-secondary font-medium">Installed language data</p>
          {ocrError ? (
            <p className="text-secondary text-danger">{ocrError}</p>
          ) : (
            <div className="flex flex-wrap gap-1.5">
              {(languages ?? []).map((l) => (
                <Badge key={l}>
                  {languageName(l)} <span className="opacity-60">{l}</span>
                </Badge>
              ))}
            </div>
          )}
          {info?.tessdata && <p className="mt-2 truncate text-caption text-muted">{info.tessdata}</p>}
        </div>
      </Section>

      <Section title="About" description="Free and open-source software.">
        <div className="flex items-center gap-4">
          <img src="/icon.png" alt="" className="h-12 w-12" />
          <div>
            <p className="text-section font-semibold">PaperFlow {info?.version}</p>
            <p className="text-secondary text-muted">Your PDFs. Your Device. Your Privacy.</p>
          </div>
        </div>
        <dl className="grid grid-cols-[140px_1fr] gap-y-1.5 text-secondary">
          <dt className="text-muted">License</dt>
          <dd>{info?.license ?? "MIT"}</dd>
          <dt className="text-muted">Repository</dt>
          <dd className="selectable">{info?.repository ?? "github.com/ammarqaisar11a55/paperflow"}</dd>
          <dt className="text-muted">Package</dt>
          <dd>{info ? (info.sandboxed ? "Snap (strict confinement)" : "Native") : "…"}</dd>
        </dl>
        <div>
          <p className="mb-2 text-secondary font-medium">Processing engines</p>
          <ul className="divide-y divide-line rounded-lg border border-line">
            {(engines ?? []).map((e) => (
              <li key={e.binary} className="flex items-center gap-3 px-3 py-2 text-secondary">
                {e.available ? <CircleCheck size={15} className="text-success" /> : <CircleX size={15} className="text-danger" />}
                <span className="w-28 font-medium">{e.label}</span>
                <span className="w-44 text-muted">{ENGINE_PURPOSE[e.engine] ?? e.binary}</span>
                <span className="selectable truncate text-caption text-muted">{e.available ? (e.version ?? e.binary) : "Not installed"}</span>
              </li>
            ))}
          </ul>
        </div>
        <p className="text-caption text-muted">
          Built with Tauri, React, PDF.js (Apache-2.0), lopdf (MIT), Ghostscript (AGPL-3.0), Poppler (GPL-2.0), qpdf (Apache-2.0), Tesseract
          (Apache-2.0), Inter (OFL-1.1) and Lucide (ISC). See docs/DEPENDENCIES.md for details.
        </p>
      </Section>
    </Page>
  );
}
