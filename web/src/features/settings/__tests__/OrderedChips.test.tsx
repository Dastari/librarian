import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { languageName } from "@/lib/languages";

import { OrderedChips } from "../OrderedChips";

describe("OrderedChips as a free-text list", () => {
  it("numbers the entries in preference order", () => {
    render(<OrderedChips label="Preferred release groups" value={["NTb", "FLUX"]} onChange={() => {}} />);
    const items = screen.getAllByRole("listitem");
    expect(items[0]).toHaveTextContent("1NTb");
    expect(items[1]).toHaveTextContent("2FLUX");
  });

  it("shows the empty label until something is added", async () => {
    const onChange = vi.fn();
    render(<OrderedChips label="Preferred release groups" value={[]} onChange={onChange} emptyLabel="No preference" />);
    expect(screen.getByText("No preference")).toBeInTheDocument();
    const add = screen.getByRole("button", { name: /Add/ });
    expect(add).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox"), "NTb");
    await userEvent.click(screen.getByRole("button", { name: /Add/ }));
    expect(onChange).toHaveBeenCalledWith(["NTb"]);
  });

  it("adds on Enter and trims the value", async () => {
    const onChange = vi.fn();
    render(<OrderedChips label="Preferred release groups" value={["NTb"]} onChange={onChange} />);
    await userEvent.type(screen.getByRole("textbox"), "  FLUX  {Enter}");
    expect(onChange).toHaveBeenCalledWith(["NTb", "FLUX"]);
  });

  it("refuses a duplicate or an empty entry", async () => {
    const onChange = vi.fn();
    render(<OrderedChips label="Preferred release groups" value={["NTb"]} onChange={onChange} />);
    await userEvent.type(screen.getByRole("textbox"), "NTb{Enter}");
    expect(onChange).not.toHaveBeenCalled();
    await userEvent.clear(screen.getByRole("textbox"));
    await userEvent.type(screen.getByRole("textbox"), "   {Enter}");
    expect(onChange).not.toHaveBeenCalled();
  });

  it("moves an entry up and down and removes it", async () => {
    const onChange = vi.fn();
    const { rerender } = render(<OrderedChips label="Groups" value={["NTb", "FLUX", "CtrlHD"]} onChange={onChange} />);
    await userEvent.click(screen.getByRole("button", { name: "Move FLUX up" }));
    expect(onChange).toHaveBeenLastCalledWith(["FLUX", "NTb", "CtrlHD"]);
    await userEvent.click(screen.getByRole("button", { name: "Move FLUX down" }));
    expect(onChange).toHaveBeenLastCalledWith(["NTb", "CtrlHD", "FLUX"]);
    await userEvent.click(screen.getByRole("button", { name: "Remove NTb" }));
    expect(onChange).toHaveBeenLastCalledWith(["FLUX", "CtrlHD"]);

    rerender(<OrderedChips label="Groups" value={["NTb", "FLUX", "CtrlHD"]} onChange={onChange} />);
    expect(screen.getByRole("button", { name: "Move NTb up" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Move CtrlHD down" })).toBeDisabled();
  });
});

describe("OrderedChips with fixed options", () => {
  const options = [
    { key: "2160p", label: "2160p" },
    { key: "1080p", label: "1080p" },
  ];

  it("offers only the options that are not already listed", async () => {
    render(<OrderedChips label="Resolution preference" value={["2160p"]} onChange={() => {}} options={options} />);
    expect(screen.queryByRole("textbox")).toBeNull();
    await userEvent.click(screen.getByRole("button", { name: /Add to Resolution preference/ }));
    const listbox = await screen.findByRole("listbox");
    expect(listbox).toHaveTextContent("1080p");
    expect(listbox.textContent).not.toContain("2160p");
  });

  it("adds the option that was picked", async () => {
    const onChange = vi.fn();
    render(<OrderedChips label="Resolution preference" value={[]} onChange={onChange} options={options} />);
    await userEvent.click(screen.getByRole("button", { name: /Add to Resolution preference/ }));
    await userEvent.click(await screen.findByRole("option", { name: "1080p" }));
    expect(onChange).toHaveBeenCalledWith(["1080p"]);
  });

  it("hides the picker once every option is used", () => {
    render(<OrderedChips label="Resolution preference" value={["2160p", "1080p"]} onChange={() => {}} options={options} />);
    expect(screen.queryByRole("button", { name: /Add to/ })).toBeNull();
  });

  it("renders codes through the label function", () => {
    render(<OrderedChips label="Preferred languages" value={["fr"]} onChange={() => {}} renderLabel={languageName} options={[{ key: "fr", label: "French" }]} />);
    expect(screen.getByRole("listitem")).toHaveTextContent("French");
  });
});
