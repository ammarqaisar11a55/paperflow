import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/Button";
import { Field, TextInput } from "@/components/ui/Form";
import { pickFolder } from "@/services/dialogs";
import { prettyPath } from "@/utils/format";

interface OutputControlsProps {
  /** Editable output name, or null when names are derived per file. */
  name: string | null;
  onName: (name: string) => void;
  extension: string | null;
  nameHint?: string;
  dir: string | null;
  onDir: (dir: string) => void;
}

export function OutputControls({ name, onName, extension, nameHint, dir, onDir }: OutputControlsProps) {
  return (
    <div className="flex flex-col gap-4">
      {name !== null && (
        <Field label={extension ? "Output file" : "Output name"} hint={nameHint}>
          {(id, d) => (
            <div className="flex items-center rounded-md border border-line bg-surface focus-within:border-accent focus-within:ring-2 focus-within:ring-accent/20 hover:border-line-strong">
              <input
                id={id}
                aria-describedby={d}
                value={name}
                spellCheck={false}
                onChange={(e) => onName(e.target.value)}
                className="h-8 min-w-0 flex-1 bg-transparent px-2.5 text-body focus:outline-none"
              />
              {extension && <span className="pr-2.5 text-body text-muted">.{extension}</span>}
            </div>
          )}
        </Field>
      )}
      <Field label="Save location">
        {(id) => (
          <div className="flex gap-2">
            <TextInput id={id} readOnly value={dir ? prettyPath(dir) : "Choosing…"} title={dir ?? undefined} className="text-muted" />
            <Button
              icon={FolderOpen}
              onClick={async () => {
                const picked = await pickFolder(dir);
                if (picked) onDir(picked);
              }}
            >
              Change
            </Button>
          </div>
        )}
      </Field>
    </div>
  );
}
