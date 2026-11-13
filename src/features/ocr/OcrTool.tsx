import { useEffect, useState } from "react";
import { Languages, TriangleAlert } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Notice } from "@/components/ui/Display";
import { Field, Select } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { api, toAppError } from "@/services/api";
import { useSettings } from "@/stores/settings";

/** Display names for the Tesseract language codes PaperFlow bundles. */
export const LANGUAGE_NAMES: Record<string, string> = {
  eng: "English",
  deu: "German",
  fra: "French",
  spa: "Spanish",
  ita: "Italian",
  por: "Portuguese",
  nld: "Dutch",
  pol: "Polish",
  tur: "Turkish",
  rus: "Russian",
  ara: "Arabic",
  urd: "Urdu",
  hin: "Hindi",
  chi_sim: "Chinese (Simplified)",
  chi_tra: "Chinese (Traditional)",
  jpn: "Japanese",
  kor: "Korean",
};

export const languageName = (code: string) => LANGUAGE_NAMES[code] ?? code;

export function useOcrLanguages() {
  const [languages, setLanguages] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    api
      .ocrLanguages()
      .then(setLanguages)
      .catch((e) => {
        setLanguages([]);
        setError(toAppError(e).message);
      });
  }, []);
  return { languages, error };
}

export function OcrTool() {
  const selection = useFileSelection({ accepts: "pdf" });
  const preferred = useSettings((s) => s.settings.ocrLanguage);
  const { languages, error } = useOcrLanguages();
  const [primary, setPrimary] = useState(preferred);
  const [secondary, setSecondary] = useState("");
  const [dpi, setDpi] = useState(300);

  useEffect(() => {
    if (languages && languages.length && !languages.includes(primary)) setPrimary(languages[0]);
  }, [languages, primary]);

  const language = [primary, secondary].filter(Boolean).join("+");
  const blocker = languages === null ? "Loading languages…" : !languages.length ? (error ?? "No OCR languages are installed.") : null;
  const pages = selection.files.reduce((n, f) => n + (f.pageCount ?? 0), 0);

  return (
    <ToolWorkspace
      tool={getTool("ocr")!}
      selection={selection}
      blocker={blocker}
      buildOptions={() => ({ language, dpi })}
      options={
        <>
          {error ? (
            <Notice tone="warning" icon={TriangleAlert} title="OCR is unavailable">
              {error}
            </Notice>
          ) : (
            <>
              <Field label="Document language">
                {(id) => (
                  <Select id={id} value={primary} onChange={(e) => setPrimary(e.target.value)}>
                    {(languages ?? []).map((l) => (
                      <option key={l} value={l}>
                        {languageName(l)}
                      </option>
                    ))}
                  </Select>
                )}
              </Field>
              <Field label="Second language (optional)">
                {(id) => (
                  <Select id={id} value={secondary} onChange={(e) => setSecondary(e.target.value)}>
                    <option value="">None</option>
                    {(languages ?? [])
                      .filter((l) => l !== primary)
                      .map((l) => (
                        <option key={l} value={l}>
                          {languageName(l)}
                        </option>
                      ))}
                  </Select>
                )}
              </Field>
              <Field label="Scan quality" hint="Higher resolution reads small print better but takes longer.">
                {(id, d) => (
                  <Select id={id} aria-describedby={d} value={dpi} onChange={(e) => setDpi(Number(e.target.value))}>
                    <option value={200}>Fast (200 dpi)</option>
                    <option value={300}>Balanced (300 dpi)</option>
                    <option value={400}>Accurate (400 dpi)</option>
                  </Select>
                )}
              </Field>
            </>
          )}
          <Notice tone="neutral" icon={Languages}>
            Recognition runs entirely on this computer
            {pages > 0 ? ` and takes a few seconds per page (${pages} in total).` : "."} Pages are stored as images with an invisible text layer.
          </Notice>
        </>
      }
    />
  );
}
