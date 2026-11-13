import { useEffect, useState } from "react";
import { LoaderCircle } from "lucide-react";
import { useFileSelection } from "@/hooks/useFileSelection";
import { Field, TextInput } from "@/components/ui/Form";
import { ToolWorkspace } from "@/features/ToolWorkspace";
import { getTool } from "@/features/registry";
import { api, toAppError } from "@/services/api";
import type { PdfMetadata } from "@/types";

const EMPTY: PdfMetadata = { title: "", author: "", subject: "", keywords: "", creator: "", producer: "" };
const FIELDS: { key: keyof PdfMetadata; label: string; placeholder?: string }[] = [
  { key: "title", label: "Title" },
  { key: "author", label: "Author" },
  { key: "subject", label: "Subject" },
  { key: "keywords", label: "Keywords", placeholder: "Separated by commas" },
  { key: "creator", label: "Creator", placeholder: "Application that made the original" },
  { key: "producer", label: "Producer" },
];

export function MetadataTool() {
  const selection = useFileSelection({ accepts: "pdf", single: true });
  const file = selection.files[0];
  const [meta, setMeta] = useState<PdfMetadata>(EMPTY);
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    setMeta(EMPTY);
    setLoadError(null);
    if (!file || file.encrypted) return;
    let cancelled = false;
    setLoading(true);
    api
      .readMetadata(file.path)
      .then((m) => !cancelled && setMeta(m))
      .catch((e) => !cancelled && setLoadError(toAppError(e).message))
      .finally(() => !cancelled && setLoading(false));
    return () => {
      cancelled = true;
    };
  }, [file]);

  return (
    <ToolWorkspace
      tool={getTool("metadata")!}
      selection={selection}
      blocker={loading ? "Reading the current metadata…" : loadError}
      buildOptions={() => ({ ...meta })}
      options={
        <>
          {loading && (
            <p className="flex items-center gap-2 text-secondary text-muted">
              <LoaderCircle size={14} className="animate-spin" /> Reading current values…
            </p>
          )}
          {FIELDS.map((f) => (
            <Field key={f.key} label={f.label}>
              {(id) => (
                <TextInput
                  id={id}
                  value={meta[f.key]}
                  placeholder={f.placeholder}
                  disabled={!file || loading}
                  onChange={(e) => setMeta((m) => ({ ...m, [f.key]: e.target.value }))}
                />
              )}
            </Field>
          ))}
          <p className="text-caption text-muted">Empty fields are removed from the document.</p>
        </>
      }
    />
  );
}
