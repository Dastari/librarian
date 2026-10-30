import { act, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { NowPlayingSheet } from "../NowPlayingSheet";
import { playerStore, type PlayItem } from "../store";
import { audioCommands } from "../usePlayer";

const queue: PlayItem[] = [
  { mediaFileId: "file-1", title: "First track", duration: 120, entity: { kind: "track", id: "track-1" } },
  { mediaFileId: "file-2", title: "Second track", duration: 240, entity: { kind: "track", id: "track-2" } },
];

afterEach(() => { act(() => playerStore.stopAudio()); delete audioCommands.retry; });

function Fixture() {
  const [open, setOpen] = useState(false);
  return <>
    <button type="button" onClick={() => setOpen(true)}>Expand player</button>
    <NowPlayingSheet open={open} onClose={() => setOpen(false)} onOpenEntity={vi.fn()} />
  </>;
}

describe("NowPlayingSheet", () => {
  it("selects a queue item with the keyboard and returns focus to the opener on Escape", async () => {
    playerStore.playQueue(queue);
    await renderWithProviders(<Fixture />, { router: false, shell: true });
    const opener = screen.getByRole("button", { name: "Expand player" });
    await userEvent.click(opener);
    const dialog = await screen.findByRole("dialog", { name: "Now playing" });
    const next = within(dialog).getByRole("button", { name: /Second track/ });
    next.focus();
    await userEvent.keyboard("{Enter}");
    expect(playerStore.getSnapshot().index).toBe(1);
    expect(next).toHaveAttribute("aria-current", "true");
    await userEvent.keyboard("{Escape}");
    await vi.waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
    await vi.waitFor(() => expect(opener).toHaveFocus());
  });
});


it("shows transport errors and invokes the audio engine retry action", async () => {
  playerStore.playQueue(queue);
  playerStore.updateTransport({ error: "This media could not be played.", playing: false });
  audioCommands.retry = vi.fn();
  await renderWithProviders(<Fixture />, { router: false, shell: true });
  await userEvent.click(screen.getByRole("button", { name: "Expand player" }));
  const dialog = await screen.findByRole("dialog", { name: "Now playing" });
  expect(within(dialog).getByRole("alert")).toHaveTextContent("This media could not be played.");
  await userEvent.click(within(dialog).getByRole("button", { name: "Retry" }));
  expect(audioCommands.retry).toHaveBeenCalledTimes(1);
  delete audioCommands.retry;
});
