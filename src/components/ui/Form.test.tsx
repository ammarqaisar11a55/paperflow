import { useState } from "react";
import { describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PasswordInput, Segmented, Switch } from "./Form";

function Controlled() {
  const [on, setOn] = useState(false);
  return <Switch checked={on} onChange={setOn} label="Keep recent activity" description="File names only" />;
}

describe("form controls", () => {
  it("switch toggles and is labelled", async () => {
    const user = userEvent.setup();
    render(<Controlled />);
    const sw = screen.getByRole("switch", { name: /Keep recent activity/ });
    expect(sw).toHaveAttribute("aria-checked", "false");
    await user.click(sw);
    expect(sw).toHaveAttribute("aria-checked", "true");
    await user.click(screen.getByText("Keep recent activity"));
    expect(sw).toHaveAttribute("aria-checked", "false");
  });

  it("switch knob stays anchored inside its track", () => {
    render(<Switch checked onChange={() => undefined} label="x" />);
    const knob = screen.getByRole("switch").firstElementChild!;
    expect(knob.className).toMatch(/\bleft-0\b/);
  });

  it("segmented control is a radio group", async () => {
    const user = userEvent.setup();
    function S() {
      const [v, setV] = useState<"a" | "b">("a");
      return <Segmented label="Mode" value={v} onChange={setV} options={[{ value: "a", label: "Alpha" }, { value: "b", label: "Beta" }]} />;
    }
    render(<S />);
    await user.click(screen.getByRole("radio", { name: "Beta" }));
    expect(screen.getByRole("radio", { name: "Beta" })).toHaveAttribute("aria-checked", "true");
    expect(screen.getByRole("radio", { name: "Alpha" })).toHaveAttribute("aria-checked", "false");
  });

  it("password field can be revealed", async () => {
    const user = userEvent.setup();
    render(<PasswordInput aria-label="Password" defaultValue="secret" />);
    const input = screen.getByLabelText("Password");
    expect(input).toHaveAttribute("type", "password");
    await user.click(screen.getByRole("button", { name: "Show password" }));
    expect(input).toHaveAttribute("type", "text");
  });
});
