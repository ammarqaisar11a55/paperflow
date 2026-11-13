// Copies the PDF.js data files (character maps, standard fonts and image
// decoders) into public/pdfjs so the viewer renders every PDF offline.
// Runs automatically before `npm run dev` and `npm run build`.
import { cpSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const root = dirname(require.resolve("pdfjs-dist/package.json"));
const target = join(process.cwd(), "public", "pdfjs");
mkdirSync(target, { recursive: true });

for (const dir of ["cmaps", "standard_fonts", "wasm", "iccs"]) {
  const from = join(root, dir);
  if (existsSync(from)) {
    cpSync(from, join(target, dir), { recursive: true });
  }
}
console.log("PDF.js assets copied to public/pdfjs");
