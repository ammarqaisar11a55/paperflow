import { describe, expect, it } from "vitest";
import { CATEGORIES, QUICK_ACTIONS, TOOLS, getTool, searchTools } from "./registry";
import { resolveTheme } from "@/stores/settings";
import { splitOutputCount } from "./split/SplitTool";
import { passwordProblem } from "./security/SecurityTools";
import { pageLabel } from "./watermark/PageNumbersTool";

describe("tool registry", () => {
  it("has every tool from the specification, once", () => {
    const ids = TOOLS.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toHaveLength(21);
    for (const t of TOOLS) expect(CATEGORIES).toContain(t.category);
    for (const q of QUICK_ACTIONS) expect(getTool(q)).toBeDefined();
  });

  it("finds tools by what people call them", () => {
    expect(searchTools("smaller").map((t) => t.id)).toEqual(["compress"]);
    expect(searchTools("password").map((t) => t.id)).toEqual(expect.arrayContaining(["protect", "unlock"]));
    expect(searchTools("jpg").map((t) => t.id)).toEqual(expect.arrayContaining(["pdf-to-images", "images-to-pdf"]));
    expect(searchTools("  ")).toHaveLength(TOOLS.length);
    expect(searchTools("spreadsheet").map((t) => t.id)).toEqual(["excel-to-pdf"]);
    expect(searchTools("docx").map((t) => t.id)).toEqual(["word-to-pdf"]);
    expect(searchTools("markdown").map((t) => t.id)).toEqual(["pdf-to-markdown"]);
    expect(searchTools("blockchain")).toHaveLength(0);
  });
});

describe("tool helpers", () => {
  it("resolves the theme preference", () => {
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("light", true)).toBe("light");
  });

  it("predicts how many files a split creates", () => {
    expect(splitOutputCount("every", 7, "", 1)).toBe(7);
    expect(splitOutputCount("chunks", 7, "", 3)).toBe(3);
    expect(splitOutputCount("ranges", 7, "1-2, 3-5, 6-", 1)).toBe(3);
    expect(splitOutputCount("ranges", 7, "9", 1)).toBeNull();
  });

  it("checks new passwords", () => {
    expect(passwordProblem("", "")).toMatch(/Enter/);
    expect(passwordProblem("abc", "abd")).toMatch(/match/);
    expect(passwordProblem("a\nb", "a\nb")).toMatch(/line breaks/);
    expect(passwordProblem("correct horse", "correct horse")).toBeNull();
  });

  it("formats page number labels like the backend", () => {
    expect(pageLabel("Page ", 3, 9, "number", "")).toBe("Page 3");
    expect(pageLabel("", 3, 9, "numberOfTotal", ".")).toBe("3 of 9.");
  });
});
