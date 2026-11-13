import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { apiMock } from "@/test/mocks";

vi.mock("@/services/api", async (orig) => ({ ...(await orig<typeof import("@/services/api")>()), api: apiMock }));

import { TitleBar } from "./TitleBar";
import { useNavigation } from "@/stores/navigation";
import { useSettings } from "@/stores/settings";

function setup(maximized = false) {
  const handlers = { onMinimize: vi.fn(), onToggleMaximize: vi.fn(), onClose: vi.fn() };
  render(<TitleBar maximized={maximized} focused {...handlers} />);
  return handlers;
}

describe("TitleBar", () => {
  it("has labelled window controls that call the window", async () => {
    const user = userEvent.setup();
    const h = setup();
    await user.click(screen.getByRole("button", { name: "Minimize" }));
    await user.click(screen.getByRole("button", { name: "Maximize" }));
    await user.click(screen.getByRole("button", { name: "Close" }));
    expect(h.onMinimize).toHaveBeenCalledOnce();
    expect(h.onToggleMaximize).toHaveBeenCalledOnce();
    expect(h.onClose).toHaveBeenCalledOnce();
  });

  it("offers Restore when the window is maximized", () => {
    setup(true);
    expect(screen.getByRole("button", { name: "Restore" })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Maximize" })).not.toBeInTheDocument();
  });

  it("is a drag region with interactive controls outside it", () => {
    const { container } = render(<TitleBar maximized={false} focused onMinimize={vi.fn()} onToggleMaximize={vi.fn()} onClose={vi.fn()} />);
    const header = container.querySelector("header")!;
    expect(header).toHaveAttribute("data-tauri-drag-region");
    for (const button of header.querySelectorAll("button")) {
      expect(button).not.toHaveAttribute("data-tauri-drag-region");
    }
  });

  it("searching opens the tool catalogue", async () => {
    const user = userEvent.setup();
    setup();
    await user.type(screen.getByRole("searchbox", { name: "Search tools" }), "merge");
    expect(useNavigation.getState().route.view).toBe("tools");
    expect(useNavigation.getState().search).toBe("merge");
  });

  it("cycles the theme", async () => {
    const user = userEvent.setup();
    setup();
    const start = useSettings.getState().settings.theme;
    await user.click(screen.getByRole("button", { name: /^Theme:/ }));
    expect(useSettings.getState().settings.theme).not.toBe(start);
  });
});
