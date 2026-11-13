import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { apiMock, pdfInfo } from "@/test/mocks";

vi.mock("@/services/api", async (orig) => ({ ...(await orig<typeof import("@/services/api")>()), api: apiMock }));
vi.mock("@/services/pdf", () => ({
  renderThumbnail: () => new Promise(() => undefined),
  acquirePdf: () => new Promise(() => undefined),
  releasePdf: () => undefined,
}));
vi.mock("@/services/dialogs", () => ({ pickPdfs: vi.fn(async () => []), pickImages: vi.fn(async () => []), pickFolder: vi.fn(async () => null) }));

import { MergeTool } from "./merge/MergeTool";
import { CompressTool } from "./compress/CompressTool";
import { ProtectTool } from "./security/SecurityTools";
import { useNavigation } from "@/stores/navigation";
import { useJobs } from "@/stores/jobs";

function handOff(...names: string[]) {
  useNavigation.setState({ handoff: names.map((n) => `/home/test/${n}`) });
}

beforeEach(() => {
  vi.clearAllMocks();
  useJobs.setState({ jobs: {} });
});

describe("tool workspace", () => {
  it("explains what is missing before a tool can run", async () => {
    render(<MergeTool />);
    expect(screen.getByRole("button", { name: "Merge PDFs" })).toBeDisabled();
    expect(screen.getByText("Add a PDF to get started.")).toBeInTheDocument();
  });

  it("merge needs two files and sends them in the chosen order", async () => {
    const user = userEvent.setup();
    handOff("a.pdf", "b.pdf");
    render(<MergeTool />);
    await screen.findByText("a.pdf");
    const button = await screen.findByRole("button", { name: "Merge PDFs" });
    await waitFor(() => expect(button).toBeEnabled());

    // Move b above a.
    const moveUp = screen.getAllByRole("button", { name: "Move up" });
    await user.click(moveUp[1]);
    await user.click(button);

    expect(apiMock.startJob).toHaveBeenCalledWith(
      expect.objectContaining({
        type: "merge",
        inputFiles: ["/home/test/b.pdf", "/home/test/a.pdf"],
        outputDir: "/home/test/Documents",
        outputName: "Merged document",
      }),
    );
  });

  it("protected files must be unlocked first", async () => {
    apiMock.inspectFiles.mockResolvedValueOnce([pdfInfo("locked.pdf", 2, { encrypted: true })]);
    handOff("locked.pdf");
    render(<CompressTool />);
    await screen.findByText("locked.pdf");
    expect(screen.getByText("Protected")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Compress" })).toBeDisabled();
    expect(screen.getByText(/must be unlocked first/)).toBeInTheDocument();
  });

  it("files that are not PDFs are rejected with a reason", async () => {
    apiMock.inspectFiles.mockResolvedValueOnce([{ ...pdfInfo("notes.txt"), kind: "unsupported", pageCount: null, error: "“notes.txt” is not a PDF file." }]);
    handOff("notes.txt");
    render(<CompressTool />);
    expect(await screen.findByRole("alert")).toHaveTextContent("not a PDF");
  });

  it("batch tools start one job per file with the chosen options", async () => {
    const user = userEvent.setup();
    handOff("a.pdf", "b.pdf");
    render(<CompressTool />);
    await screen.findByText("b.pdf");
    await user.click(screen.getByRole("radio", { name: /Extreme/ }));
    const button = screen.getByRole("button", { name: "Compress 2 files" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    expect(apiMock.startJob).toHaveBeenCalledTimes(2);
    expect(apiMock.startJob.mock.calls[0][0]).toMatchObject({ type: "compress", inputFiles: ["/home/test/a.pdf"], options: { preset: "extreme" } });
    // Several files: names come from each file, not from one shared name.
    expect(apiMock.startJob.mock.calls[0][0].outputName).toBeUndefined();
  });

  it("shows progress, then the result with the size saved", async () => {
    const user = userEvent.setup();
    handOff("a.pdf");
    render(<CompressTool />);
    await screen.findByText("a.pdf");
    const button = screen.getByRole("button", { name: "Compress" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);

    const base = {
      id: "job-1",
      type: "compress" as const,
      title: "Compress PDF",
      inputNames: ["a.pdf"],
      currentFile: "a.pdf",
      error: null,
      createdAt: 1,
      finishedAt: null,
    };
    act(() => useJobs.getState().apply({ ...base, status: "running", progress: 0.62, message: "Optimizing page 4 of 6", result: null }));
    expect(await screen.findByRole("progressbar", { name: /Compress PDF progress/ })).toHaveAttribute("aria-valuenow", "62");
    expect(screen.getByText("Optimizing page 4 of 6…")).toBeInTheDocument();

    act(() =>
      useJobs.getState().apply({
        ...base,
        status: "completed",
        progress: 1,
        message: "Done",
        finishedAt: 2,
        result: {
          success: true,
          outputFiles: ["/home/test/a - compressed.pdf"],
          outputFolder: null,
          errors: [],
          warnings: [],
          inputSize: 1000,
          outputSize: 400,
          pageCount: 6,
        },
      }),
    );
    expect(await screen.findByText("Your file is ready")).toBeInTheDocument();
    expect(screen.getByText("a - compressed.pdf")).toBeInTheDocument();
    expect(screen.getByText("−60%")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Show in folder" }));
    expect(apiMock.revealPath).toHaveBeenCalledWith("/home/test/a - compressed.pdf");
  });

  it("shows a clear error with a suggestion and a retry", async () => {
    const user = userEvent.setup();
    handOff("a.pdf");
    render(<CompressTool />);
    await screen.findByText("a.pdf");
    const button = screen.getByRole("button", { name: "Compress" });
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    act(() =>
      useJobs.getState().apply({
        id: "job-1",
        type: "compress",
        title: "Compress PDF",
        inputNames: ["a.pdf"],
        status: "failed",
        progress: 0.1,
        message: "Ghostscript is not available.",
        currentFile: null,
        result: null,
        error: { kind: "missingEngine", message: "Ghostscript is not available.", hint: "Install the “ghostscript” package.", detail: "gs: not found" },
        createdAt: 1,
        finishedAt: 2,
      }),
    );
    expect(await screen.findByText("That didn’t work")).toBeInTheDocument();
    expect(screen.getByText("Install the “ghostscript” package.")).toBeInTheDocument();
    expect(screen.queryByText("gs: not found")).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Technical details" }));
    expect(screen.getByText("gs: not found")).toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: "Retry" }));
    expect(apiMock.startJob).toHaveBeenCalledTimes(2);
  });

  it("protect requires matching passwords and never sends them anywhere else", async () => {
    const user = userEvent.setup();
    handOff("a.pdf");
    render(<ProtectTool />);
    await screen.findByText("a.pdf");
    const button = screen.getByRole("button", { name: "Protect" });
    await user.type(screen.getByLabelText("Password to open"), "s3cret");
    await user.type(screen.getByLabelText("Confirm password"), "s3cre");
    expect(button).toBeDisabled();
    await user.type(screen.getByLabelText("Confirm password"), "t");
    await waitFor(() => expect(button).toBeEnabled());
    await user.click(button);
    expect(apiMock.startJob).toHaveBeenCalledWith(expect.objectContaining({ type: "protect", options: expect.objectContaining({ userPassword: "s3cret" }) }));
    expect(apiMock.saveSettings).not.toHaveBeenCalled();
  });
});
