import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { confirm } from "glass-ux/alert-dialog";
import { PaletteProvider, useRegisteredCommands } from "glass-ux/command-palette";
import type { ComponentProps } from "react";
import { describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { MediaActions } from "../MediaActions";

const mount = (props: Partial<ComponentProps<typeof MediaActions>> = {}) => renderWithProviders(<MediaActions id="show-1" label="Andor" {...props} />);

describe("MediaActions with the real Glass action host", () => {
  it("renders named actions with icons and one More menu without requiring a palette", async () => {
    const play = vi.fn();
    const enqueue = vi.fn();
    const find = vi.fn();
    const settings = vi.fn();
    const remove = vi.fn();
    await mount({ play: { label: "Play next episode", run: play }, wanted: { value: false, run: vi.fn(async () => {}) }, enqueue, find, settings, refresh: { pending: false, run: vi.fn(async () => {}) }, remove });
    const toolbar = screen.getByRole("toolbar", { name: "Actions for Andor" });
    expect(within(toolbar).getAllByRole("button", { name: /More actions/ })).toHaveLength(1);
    for (const button of within(toolbar).getAllByRole("button")) {
      expect(button.querySelector("svg")).toBeTruthy();
      expect(button.querySelector("svg")).toHaveAttribute("aria-hidden", "true");
      expect(button.querySelector(".fa-btn-label svg")).toBeNull();
    }
    // Playback leads the hero as its own large button, outside the secondary toolbar.
    const playButton = screen.getByRole("button", { name: "Play next episode" });
    expect(toolbar).not.toContainElement(playButton);
    expect(playButton).toHaveAttribute("data-tv-autofocus");
    await userEvent.click(playButton);
    expect(play).toHaveBeenCalledOnce();
    await userEvent.click(within(toolbar).getByRole("button", { name: "Add to queue" }));
    expect(enqueue).toHaveBeenCalledOnce();
    await userEvent.click(within(toolbar).getByRole("button", { name: /More actions/ }));
    const menu = await screen.findByRole("menu", { name: "More actions for Andor" });
    // With Play beside the bar, the third button slot goes to the next priority action.
    expect(within(toolbar).getByRole("button", { name: "Find releases" })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Refresh metadata" })).toBeInTheDocument();
    expect(within(menu).getByRole("menuitem", { name: "Remove from library" })).toBeInTheDocument();
    await userEvent.click(within(menu).getByRole("menuitem", { name: "Download settings" }));
    expect(settings).toHaveBeenCalledOnce();
    expect(find).not.toHaveBeenCalled();
    expect(remove).not.toHaveBeenCalled();
  });

  it("keeps disabled playback discoverable with its reason and never calls it", async () => {
    const play = vi.fn();
    await mount({ play: { label: "Play", run: play, disabledReason: "Loading remaining playback progress" } });
    const button = screen.getByRole("button", { name: "Play" });
    expect(button).toHaveAttribute("aria-disabled", "true");
    button.focus();
    await userEvent.keyboard("{Enter} ");
    await userEvent.click(button);
    expect(play).not.toHaveBeenCalled();
    expect(button).toHaveAccessibleDescription("Loading remaining playback progress");
  });

  it.each(["wanted", "refresh"] as const)("reports rejected %s work as an error without success feedback", async (kind) => {
    let reject: (error: Error) => void = () => {};
    const run = vi.fn(() => new Promise<void>((_resolve, fail) => { reject = fail; }));
    await mount(kind === "wanted" ? { wanted: { value: false, run } } : { refresh: { pending: false, run } });
    const button = kind === "wanted" ? screen.getByRole("button", { name: "Want" }) : undefined;
    if (button) {
      await userEvent.click(button);
      expect(button).toHaveAttribute("data-status", "pending");
    } else {
      await userEvent.click(screen.getByRole("button", { name: /More actions/ }));
      await userEvent.click(await screen.findByRole("menuitem", { name: "Refresh metadata" }));
    }
    await waitFor(() => expect(run).toHaveBeenCalledOnce());
    await act(async () => reject(new Error("Server refused the change")));
    if (button) await waitFor(() => expect(button).toHaveAttribute("data-status", "error"));
    expect((await screen.findAllByText("Server refused the change")).length).toBeGreaterThan(0);
    expect(screen.getAllByText(kind === "wanted" ? "Want failed" : "Refresh metadata failed").length).toBeGreaterThan(0);
    expect(screen.queryByText("Marked as wanted")).toBeNull();
    expect(screen.queryByText("Metadata refreshed")).toBeNull();
    if (button) expect(button).not.toHaveAttribute("data-status", "success");
  });

  it("prevents a second metadata refresh while one is already pending", async () => {
    const run = vi.fn(async () => {});
    await mount({ refresh: { pending: true, run } });
    await userEvent.click(screen.getByRole("button", { name: /More actions/ }));
    const item = await screen.findByRole("menuitem", { name: "Refresh metadata" });
    expect(item).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(item);
    expect(run).not.toHaveBeenCalled();
  });

  it("opens the host's removal confirmation exactly once and waits before deleting", async () => {
    const deleteMedia = vi.fn(async () => {});
    const remove = vi.fn(() => {
      void confirm({ title: "Remove Andor?", description: "Remove this show from your library.", confirmLabel: "Remove show", tone: "danger", onConfirm: deleteMedia });
    });
    await mount({ play: { label: "Play", run: vi.fn() }, wanted: { value: false, run: vi.fn(async () => {}) }, find: vi.fn(), settings: vi.fn(), remove });
    await userEvent.click(screen.getByRole("button", { name: /More actions/ }));
    await userEvent.click(await screen.findByRole("menuitem", { name: "Remove from library" }));
    const dialog = await screen.findByRole("alertdialog", { name: "Remove Andor?" });
    expect(screen.getAllByRole("alertdialog")).toHaveLength(1);
    expect(remove).toHaveBeenCalledOnce();
    expect(deleteMedia).not.toHaveBeenCalled();
    await userEvent.click(within(dialog).getByRole("button", { name: "Remove show" }));
    await waitFor(() => expect(deleteMedia).toHaveBeenCalledOnce());
    expect(remove).toHaveBeenCalledOnce();
    await waitFor(() => expect(screen.queryByRole("alertdialog")).toBeNull());
  });

  it("registers the same available actions with an optional shared palette", async () => {
    function PaletteEntries() {
      const { commands } = useRegisteredCommands();
      return <ul aria-label="Registered media commands">{commands.map((command) => <li key={command.id}>{command.group}: {command.label}{command.disabledReason ? ` (${command.disabledReason})` : ""}</li>)}</ul>;
    }
    await renderWithProviders(<PaletteProvider><MediaActions id="show-1" label="Andor" play={{ label: "Play next episode", disabledReason: "Progress loading", run: vi.fn() }} find={vi.fn()} remove={vi.fn()} /><PaletteEntries /></PaletteProvider>);
    const registry = screen.getByRole("list", { name: "Registered media commands" });
    await waitFor(() => expect(within(registry).getAllByRole("listitem")).toHaveLength(3));
    expect(registry).toHaveTextContent("Andor: Play next episode (Progress loading)");
    expect(registry).toHaveTextContent("Andor: Find releases");
    expect(registry).toHaveTextContent("Andor: Remove from library");
  });
});
