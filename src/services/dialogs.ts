// Native file and folder pickers.
import { open } from "@tauri-apps/plugin-dialog";

const PDF_FILTER = { name: "PDF documents", extensions: ["pdf", "PDF"] };
const IMAGE_FILTER = {
  name: "Images",
  extensions: ["png", "jpg", "jpeg", "webp", "bmp", "tif", "tiff", "gif", "PNG", "JPG", "JPEG"],
};

function asArray(value: string | string[] | null): string[] {
  if (!value) return [];
  return Array.isArray(value) ? value : [value];
}

export async function pickPdfs(multiple = true): Promise<string[]> {
  return asArray(await open({ multiple, directory: false, filters: [PDF_FILTER], title: "Choose PDF files" }));
}

export async function pickImages(multiple = true): Promise<string[]> {
  return asArray(await open({ multiple, directory: false, filters: [IMAGE_FILTER], title: "Choose images" }));
}

export async function pickFolder(defaultPath?: string | null): Promise<string | null> {
  const result = await open({ directory: true, multiple: false, defaultPath: defaultPath ?? undefined, title: "Choose a folder" });
  return typeof result === "string" ? result : null;
}
