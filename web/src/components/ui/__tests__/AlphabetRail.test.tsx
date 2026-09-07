import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { AlphabetRail } from "../AlphabetRail";

describe("AlphabetRail", () => {
  it("offers # plus every letter", () => {
    render(<AlphabetRail value={null} onChange={() => {}} />);
    const buttons = screen.getAllByRole("button");
    expect(buttons).toHaveLength(27);
    expect(buttons[0]).toHaveTextContent("#");
    expect(buttons[26]).toHaveTextContent("Z");
    expect(screen.getByRole("group", { name: "Jump to letter" })).toBeInTheDocument();
  });

  it("marks the current letter as pressed", () => {
    render(<AlphabetRail value="M" onChange={() => {}} />);
    expect(screen.getByRole("button", { name: "M" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "N" })).toHaveAttribute("aria-pressed", "false");
  });

  it("reports the letter that was picked", async () => {
    const onChange = vi.fn();
    render(<AlphabetRail value={null} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "Q" }));
    expect(onChange).toHaveBeenCalledWith("Q");
  });

  it("dims letters with no content but keeps them selectable", async () => {
    const onChange = vi.fn();
    render(<AlphabetRail value={null} onChange={onChange} available={new Set(["A", "B"])} />);
    expect(screen.getByRole("button", { name: "A" }).className).not.toContain("text-muted/40");
    const empty = screen.getByRole("button", { name: "C" });
    expect(empty.className).toContain("text-muted/40");
    await userEvent.click(empty);
    expect(onChange).toHaveBeenCalledWith("C");
  });
});
