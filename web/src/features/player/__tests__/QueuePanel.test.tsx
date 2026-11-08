import { act, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { createRef } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { renderWithProviders } from "@/test";

import { playlistAdapter } from "../playlistAdapter";
import { QueuePanel } from "../QueuePanel";
import { playerStore, usePlayerState, type PlayItem } from "../store";
import { useTableLayout } from "./tableLayout";

vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));
vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "user-1" }, status: "authenticated" }) }));
vi.mock("../playlistAdapter", async (original) => ({
  ...(await original<typeof import("../playlistAdapter")>()),
  playlistAdapter: { createFromItems: vi.fn(async () => ({ listId: "list-1", saved: 2, skipped: 0 })), expand: vi.fn() },
}));

const track = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: `Song ${id}`, entity: { kind: "track", id } });
const titles = () => playerStore.getSnapshot().queue.map((entry) => entry.title);

function Panel() {
  const state = usePlayerState();
  return <QueuePanel state={state} listRef={createRef()} fill={false} />;
}

useTableLayout();
beforeEach(() => {
  vi.clearAllMocks();
  playerStore.forgetPerson();
  playerStore.stopAudio();
  playerStore.playQueue(["a", "b", "c", "d"].map(track));
});
afterEach(() => { act(() => playerStore.stopAudio()); });

const grid = () => screen.findByRole("grid", { name: "Play queue" });
const row = async (name: RegExp) => within(await grid()).findByRole("row", { name });

describe("QueuePanel", () => {
  it("lists the queue in order and marks the playing entry", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    for (const name of ["Song a", "Song b", "Song c", "Song d"]) expect(await row(new RegExp(name))).toBeInTheDocument();
    expect(within(await row(/Song a/)).getByText("Song a")).toHaveClass("text-brand");
    expect(within(await row(/Song b/)).getByText("Song b")).not.toHaveClass("text-brand");
  });

  it("plays an entry that is opened", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    (await row(/Song c/)).focus();
    await userEvent.keyboard("{Enter}");
    expect(playerStore.getSnapshot().index).toBe(2);
  });

  it("reorders from the keyboard: Space lifts the grip's row, arrows move it, Space drops it", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    const grip = within(await row(/Song d/)).getByRole("button", { name: "Reorder Song d" });
    grip.focus();
    await userEvent.keyboard(" {ArrowUp}{ArrowUp} ");
    await waitFor(() => expect(titles()).toEqual(["Song a", "Song d", "Song b", "Song c"]));
    // The entry that was playing is still the one playing.
    expect(playerStore.getSnapshot().queue[playerStore.getSnapshot().index]!.title).toBe("Song a");
  });

  it("keeps the playing entry playing when another is dragged past it", async () => {
    playerStore.jumpTo(1);
    await renderWithProviders(<Panel />, { router: false, shell: true });
    const grip = within(await row(/Song d/)).getByRole("button", { name: "Reorder Song d" });
    grip.focus();
    await userEvent.keyboard(" {ArrowUp}{ArrowUp}{ArrowUp} ");
    await waitFor(() => expect(titles()).toEqual(["Song d", "Song a", "Song b", "Song c"]));
    expect(playerStore.getSnapshot().queue[playerStore.getSnapshot().index]!.title).toBe("Song b");
  });

  it("moves, plays next and removes from a row's menu", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    await userEvent.pointer({ keys: "[MouseRight]", target: await row(/Song d/) });
    expect(screen.getAllByRole("menuitem").map((item) => item.textContent)).toEqual(["Play", "Play next", "Move up", "Remove from queue"]);
    await userEvent.click(screen.getByRole("menuitem", { name: "Play next" }));
    await waitFor(() => expect(titles()).toEqual(["Song a", "Song d", "Song b", "Song c"]));
    await userEvent.pointer({ keys: "[MouseRight]", target: await row(/Song c/) });
    await userEvent.click(await screen.findByRole("menuitem", { name: "Remove from queue" }));
    await waitFor(() => expect(titles()).toEqual(["Song a", "Song d", "Song b"]));
  });

  it("clears what is after the playing entry, and only then", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    await userEvent.click(screen.getByRole("button", { name: "Clear up next" }));
    expect(titles()).toEqual(["Song a"]);
    expect(screen.getByRole("button", { name: "Clear up next" })).toBeDisabled();
  });

  it("saves the queue as a playlist with the name given, entries in order", async () => {
    await renderWithProviders(<Panel />, { router: false, shell: true });
    await userEvent.click(screen.getByRole("button", { name: "Save as playlist" }));
    const dialog = await screen.findByRole("dialog", { name: "Save queue as playlist" });
    const name = within(dialog).getByRole("textbox", { name: "Name" });
    await userEvent.clear(name);
    await userEvent.type(name, "Evening");
    await userEvent.click(within(dialog).getByRole("button", { name: "Save playlist" }));
    await waitFor(() => expect(playlistAdapter.createFromItems).toHaveBeenCalledTimes(1));
    const [, input] = vi.mocked(playlistAdapter.createFromItems).mock.calls[0]!;
    expect(input).toMatchObject({ userId: "user-1", name: "Evening" });
    expect(input.items.map((entry) => entry.title)).toEqual(["Song a", "Song b", "Song c", "Song d"]);
    await waitFor(() => expect(screen.queryByRole("dialog", { name: "Save queue as playlist" })).not.toBeInTheDocument());
  });

  it("will not save a queue that holds only what playlists cannot hold yet", async () => {
    playerStore.playQueue([{ mediaFileId: "file-e", title: "Episode", entity: { kind: "episode", id: "e" } }]);
    await renderWithProviders(<Panel />, { router: false, shell: true });
    await userEvent.click(screen.getByRole("button", { name: "Save as playlist" }));
    const dialog = await screen.findByRole("dialog", { name: "Save queue as playlist" });
    expect(within(dialog).getByRole("button", { name: "Save playlist" })).toBeDisabled();
    expect(within(dialog).getByText(/cannot hold episodes or chapters yet/)).toBeInTheDocument();
  });
});
