import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { GlassSegmented, GlassSwitch } from "../GlassControls";

const items = [
  { key: "mine", label: "My shows" },
  { key: "all", label: "All shows" },
  { key: "none", label: "Nothing" },
];

describe("GlassSegmented", () => {
  it("is a group with exactly one pressed option", () => {
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="all" onChange={() => {}} />);
    const group = screen.getByRole("group", { name: "Guide scope" });
    expect(group).toBeInTheDocument();
    const radios = screen.getAllByRole("button");
    expect(radios).toHaveLength(3);
    expect(screen.getByRole("button", { name: "All shows" })).toHaveAttribute("aria-pressed", "true");
    expect(radios.filter((radio) => radio.getAttribute("aria-pressed") === "true")).toHaveLength(1);
  });

  it("reports the picked key", async () => {
    const onChange = vi.fn();
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="mine" onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "All shows" }));
    expect(onChange).toHaveBeenCalledWith("all");
  });

  it("moves focus with arrow keys, wraps, and activates with Enter", async () => {
    const onChange = vi.fn();
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="mine" onChange={onChange} />);
    screen.getByRole("button", { name: "My shows" }).focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(screen.getByRole("button", { name: "All shows" })).toHaveFocus();
    expect(onChange).not.toHaveBeenCalled();
    await userEvent.keyboard("{Enter}");
    expect(onChange).toHaveBeenLastCalledWith("all");
    await userEvent.keyboard("{ArrowLeft}{ArrowLeft}{Enter}");
    expect(onChange).toHaveBeenLastCalledWith("none");
  });

  it("keeps a single tab stop", () => {
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="all" onChange={() => {}} />);
    expect(screen.getAllByRole("button").filter((button) => button.tabIndex === 0)).toHaveLength(1);
  });
});

describe("GlassSwitch", () => {
  it("exposes the checked state and toggles it", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<GlassSwitch checked={false} onChange={onChange} ariaLabel="Monitored" />);
    const control = screen.getByRole("switch", { name: "Monitored" });
    expect(control).not.toBeChecked();
    await userEvent.click(control);
    expect(onChange).toHaveBeenCalledWith(true);

    rerender(<GlassSwitch checked onChange={onChange} ariaLabel="Monitored" />);
    expect(screen.getByRole("switch", { name: "Monitored" })).toBeChecked();
    await userEvent.click(screen.getByRole("switch", { name: "Monitored" }));
    expect(onChange).toHaveBeenLastCalledWith(false);
  });

  it("does nothing while disabled", async () => {
    const onChange = vi.fn();
    render(<GlassSwitch checked={false} onChange={onChange} ariaLabel="Monitored" disabled />);
    const control = screen.getByRole("switch");
    expect(control).toBeDisabled();
    await userEvent.click(control);
    expect(onChange).not.toHaveBeenCalled();
  });
});
