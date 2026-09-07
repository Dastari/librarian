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
  it("is a radio group with exactly one checked option", () => {
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="all" onChange={() => {}} />);
    const group = screen.getByRole("radiogroup", { name: "Guide scope" });
    expect(group).toBeInTheDocument();
    const radios = screen.getAllByRole("radio");
    expect(radios).toHaveLength(3);
    expect(screen.getByRole("radio", { name: "All shows" })).toBeChecked();
    expect(radios.filter((radio) => radio.getAttribute("aria-checked") === "true")).toHaveLength(1);
  });

  it("reports the picked key", async () => {
    const onChange = vi.fn();
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="mine" onChange={onChange} />);
    await userEvent.click(screen.getByRole("radio", { name: "All shows" }));
    expect(onChange).toHaveBeenCalledWith("all");
  });

  it("moves through the options with the arrow keys and wraps around", async () => {
    const onChange = vi.fn();
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="mine" onChange={onChange} />);
    const selected = screen.getByRole("radio", { name: "My shows" });
    selected.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(onChange).toHaveBeenLastCalledWith("all");
    await userEvent.keyboard("{ArrowLeft}");
    expect(onChange).toHaveBeenLastCalledWith("none");
  });

  it("keeps a single tab stop", () => {
    render(<GlassSegmented ariaLabel="Guide scope" items={items} value="all" onChange={() => {}} />);
    expect(screen.getByRole("radio", { name: "All shows" })).toHaveAttribute("tabindex", "0");
    expect(screen.getByRole("radio", { name: "My shows" })).toHaveAttribute("tabindex", "-1");
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
