import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { apiMock } from "@/test/mocks";

vi.mock("@/services/api", async (orig) => ({ ...(await orig<typeof import("@/services/api")>()), api: apiMock }));
vi.mock("@/services/pdf", () => ({
  renderThumbnail: () => new Promise(() => undefined),
  acquirePdf: () => new Promise(() => undefined),
  releasePdf: () => undefined,
}));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => undefined) }));

import { App } from "./App";
import { suggestTools } from "./screens/Dashboard";
import { pdfInfo } from "@/test/mocks";

describe("navigation", () => {
  it("opens on the dashboard and moves between screens", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(screen.getByRole("heading", { name: "Everything you need for PDFs." })).toBeInTheDocument();
    expect(screen.getByText("Your documents stay on your device. Always.")).toBeInTheDocument();

    const nav = screen.getByRole("navigation", { name: "Main" });
    await user.click(screen.getAllByRole("button", { name: "All Tools" }).find((b) => nav.contains(b))!);
    expect(screen.getByRole("heading", { name: "All tools" })).toBeInTheDocument();
    expect(screen.getByText("Security")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /^Merge PDF\s*Combine/ }));
    expect(screen.getByRole("heading", { name: "Merge PDF" })).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Back to tools" }));
    expect(screen.getByRole("heading", { name: "All tools" })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Settings" }));
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
    expect(screen.getByRole("switch", { name: /Keep recent activity/ })).toBeInTheDocument();
  });

  it("Ctrl+, opens settings", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.keyboard("{Control>},{/Control}");
    expect(screen.getByRole("heading", { name: "Settings" })).toBeInTheDocument();
  });
});

describe("dashboard suggestions", () => {
  it("suggests tools that fit the dropped files", () => {
    expect(suggestTools([pdfInfo("a.pdf"), pdfInfo("b.pdf")])[0]).toBe("merge");
    expect(suggestTools([pdfInfo("a.pdf")])).toContain("compress");
    expect(suggestTools([pdfInfo("a.pdf", 1, { encrypted: true })])).toEqual(["unlock"]);
    expect(suggestTools([{ ...pdfInfo("a.png"), kind: "image" }])).toEqual(["images-to-pdf"]);
    expect(suggestTools([{ ...pdfInfo("a.docx"), kind: "word" }, { ...pdfInfo("b.xlsx"), kind: "spreadsheet" }])).toEqual([
      "word-to-pdf",
      "excel-to-pdf",
    ]);
  });
});
