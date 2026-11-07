import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { defineActions } from "glass-ux/actions";
import { PaletteProvider, useRegisteredCommands } from "glass-ux/command-palette";
import { IconDownload, IconPlus, IconRefresh, IconSettings, IconTrash } from "glass-ux/icons";
import { describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { MediaActions } from "../MediaActions";

const target = { id: "show-1" };

function actionSet(run = vi.fn()) {
  return defineActions<typeof target>("media-test", [
    { id: "enqueue", label: "Add to queue", icon: IconPlus, group: "open", priority: 1, run },
    { id: "find", label: "Find releases", icon: IconDownload, group: "manage", priority: 2, run },
    { id: "refresh", label: "Refresh metadata", icon: IconRefresh, group: "manage", priority: 3, disabledReason: () => "Already refreshing", run },
    { id: "settings", label: "Show settings", icon: IconSettings, group: "manage", priority: 4, run },
    { id: "remove", label: "Remove from library…", icon: IconTrash, group: "danger", priority: 9, run },
  ]);
}

describe("MediaActions", () => {
  it("leads with Play, outside the toolbar of the title's own actions", async () => {
    const play = vi.fn();
    const run = vi.fn();
    await renderWithProviders(<MediaActions target={target} label="Andor" actions={actionSet(run)} play={{ label: "Play next episode", run: play }} />);
    const toolbar = screen.getByRole("toolbar", { name: "Actions for Andor" });
    const playButton = screen.getByRole("button", { name: "Play next episode" });
    expect(toolbar).not.toContainElement(playButton);
    expect(playButton).toHaveAttribute("data-tv-autofocus");
    await userEvent.click(playButton);
    expect(play).toHaveBeenCalledOnce();
    await userEvent.click(within(toolbar).getByRole("button", { name: "Add to queue" }));
    await waitFor(() => expect(run).toHaveBeenCalledOnce());
  });

  it("keeps the rest in one More menu, with a busy action disabled", async () => {
    await renderWithProviders(<MediaActions target={target} label="Andor" actions={actionSet()} />);
    expect(screen.getByRole("button", { name: "Refresh metadata" })).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(screen.getByRole("button", { name: /More actions/ }));
    expect(await screen.findByRole("menuitem", { name: "Remove from library…" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Show settings" })).toBeInTheDocument();
  });

  it("keeps disabled playback discoverable with its reason and never calls it", async () => {
    const play = vi.fn();
    await renderWithProviders(<MediaActions target={target} label="Andor" actions={actionSet()} play={{ label: "Play", run: play, disabledReason: "Loading remaining playback progress" }} />);
    const button = screen.getByRole("button", { name: "Play" });
    expect(button).toHaveAttribute("aria-disabled", "true");
    await userEvent.click(button);
    expect(play).not.toHaveBeenCalled();
    expect(button).toHaveAccessibleDescription("Loading remaining playback progress");
  });

  it("registers the same actions with a shared palette", async () => {
    function PaletteEntries() {
      const { commands } = useRegisteredCommands();
      return <ul aria-label="Commands">{commands.map((command) => <li key={command.id}>{command.label}</li>)}</ul>;
    }
    await renderWithProviders(<PaletteProvider><MediaActions target={target} label="Andor" actions={actionSet()} /><PaletteEntries /></PaletteProvider>);
    await waitFor(() => expect(screen.getByRole("list", { name: "Commands" })).toHaveTextContent("Find releases"));
  });
});
