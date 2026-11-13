// Native file and folder pickers.
import { open } from "@tauri-apps/plugin-dialog";
import { ALL_KINDS, dialogFilter, KINDS, type InputKind } from "@/features/fileKinds";

function asArray(value: string | string[] | null): string[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

/** Picks files of one kind (or of every supported kind for "any"). */
export async function pickFiles(kind: InputKind | "any", multiple = true): Promise<string[]> {
  const kinds = kind === "any" ? ALL_KINDS : [kind];
  const filters = kinds.map(dialogFilter);
  if (kind === "any") {
    filters.unshift({ name: "All supported files", extensions: filters.flatMap((f) => f.extensions) });
  }
  const title = kind === "any" ? "Choose files" : `Choose ${KINDS[kind].many}`;
  return asArray(await open({ multiple, directory: false, filters, title }));
}

export const pickPdfs = (multiple = true) => pickFiles("pdf", multiple);
export const pickImages = (multiple = true) => pickFiles("image", multiple);

export async function pickFolder(defaultPath?: string | null): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, defaultPath: defaultPath ?? undefined, title: "Choose a folder" });
  return typeof result === "string" ? result : null;
}
