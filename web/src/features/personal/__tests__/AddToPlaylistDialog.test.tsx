import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EntityListEntryCreateDocument, EntityListEntryDeleteDocument, EntityPersonalListCreateDocument, PlaylistListDocument, PlaylistMembersDocument } from "@/graphql/generated/graphql";

import { AddToPlaylistDialog } from "../AddToPlaylistDialog";
import type { PlaylistTarget } from "../types";
import { connection, createdEntry, expectValidOperation, operationName, OWNER, personalEntry, personalHarness, personalList, playlist, respond } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

const member = (id: string, position: number, target: Partial<Record<"movieId" | "trackId" | "episodeId", string>>) => ({
  __typename: "ListEntry" as const, id, position, movieId: null, showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: null, ...target,
});

async function mount(targets: PlaylistTarget[], title: string, lists = [playlist({}, 2), playlist({ id: "smart-a", name: "Smart collection", kind: "SMART", savedViewId: "view-a" }, 0)]) {
  const harness = personalHarness();
  const close = vi.fn();
  await harness.mount(<AddToPlaylistDialog open onOpenChange={close} targets={targets} title={title} />);
  const request = await harness.next(PlaylistListDocument);
  expect(request.operation.variables.where).toEqual({ userId: { eq: OWNER } });
  await respond(request, connection("personalLists", lists));
  return { ...harness, close };
}

async function choosePlaylist(name: string) {
  await userEvent.click(screen.getByRole("combobox", { name: "Playlist" }));
  await userEvent.click(await screen.findByRole("option", { name }));
}

function expectMutations(harness: ReturnType<typeof personalHarness>, documents: Parameters<typeof operationName>[0][]) {
  const mutations = harness.mutations();
  expect(mutations.map(({ operation }) => operation.operationName)).toEqual(documents.map(operationName));
  mutations.forEach(({ operation }) => expectValidOperation(operation));
}

describe("AddToPlaylistDialog", () => {
  it("offers your playlists and New playlist…, never a smart collection", async () => {
    await mount([{ kind: "movie", id: "m1" }], "Night film");
    await userEvent.click(screen.getByRole("combobox", { name: "Playlist" }));
    expect(await screen.findByRole("option", { name: "Evening films · 2 items" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "New playlist…" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: /Smart collection/ })).not.toBeInTheDocument();
  });

  it("appends a movie after the greatest stored position once every entry has loaded", async () => {
    const view = await mount([{ kind: "movie", id: "m1" }], "Night film");
    await choosePlaylist("Evening films · 2 items");
    const ui = await view.next(PlaylistMembersDocument);
    expect(ui.operation.variables.listId).toBe("list-a");
    expect(screen.getByRole("button", { name: "Add" })).toBeDisabled();
    await respond(ui, connection("listEntries", [member("e1", 0, { movieId: "other" }), member("e2", 9000, { trackId: "t" })]));
    await waitFor(() => expect(screen.getByRole("button", { name: "Add" })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await respond(await view.next(PlaylistMembersDocument, 1), connection("listEntries", [member("e1", 0, { movieId: "other" }), member("e2", 9000, { trackId: "t" })]));
    const create = await view.next(EntityListEntryCreateDocument);
    expect(create.operation.variables).toEqual({ input: { userId: OWNER, listId: "list-a", position: 10024, movieId: "m1", showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: null } });
    await respond(create, createdEntry(personalEntry(2, { movieId: "m1", position: 10024 })));
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    // Only playlist entries are written: no Wanted, acquisition or import action is reachable from here.
    expectMutations(view, [EntityListEntryCreateDocument]);
  });

  it("adds many in order from the end, skipping what the playlist already holds and what is listed twice", async () => {
    const targets: PlaylistTarget[] = [{ kind: "track", id: "t1" }, { kind: "episode", id: "ep1" }, { kind: "track", id: "t2" }, { kind: "track", id: "t1" }];
    const view = await mount(targets, "4 items");
    await choosePlaylist("Evening films · 2 items");
    const rows = connection("listEntries", [member("e1", 2048, { trackId: "t2" })]);
    await respond(await view.next(PlaylistMembersDocument), rows);
    await waitFor(() => expect(screen.getByRole("button", { name: "Add" })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await respond(await view.next(PlaylistMembersDocument, 1), rows);
    const first = await view.next(EntityListEntryCreateDocument);
    expect(first.operation.variables.input).toMatchObject({ trackId: "t1", position: 3072, episodeId: null });
    await respond(first, createdEntry(personalEntry(1, { movieId: null, trackId: "t1" })));
    const second = await view.next(EntityListEntryCreateDocument, 1);
    expect(second.operation.variables.input).toMatchObject({ episodeId: "ep1", position: 4096, trackId: null });
    await respond(second, createdEntry(personalEntry(2, { movieId: null, episodeId: "ep1" })));
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    expectMutations(view, [EntityListEntryCreateDocument, EntityListEntryCreateDocument]);
  });

  it("removes a single title that is already in the chosen playlist instead of adding it twice", async () => {
    const view = await mount([{ kind: "track", id: "t1" }], "A song");
    await choosePlaylist("Evening films · 2 items");
    await respond(await view.next(PlaylistMembersDocument), connection("listEntries", [member("e1", 0, { trackId: "t1" })]));
    await userEvent.click(await screen.findByRole("button", { name: "Remove from playlist" }));
    const remove = await view.next(EntityListEntryDeleteDocument);
    expect(remove.operation.variables).toEqual({ id: "e1" });
    await respond(remove, { deleteListEntry: { success: true, error: null } });
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    expectMutations(view, [EntityListEntryDeleteDocument]);
  });

  it("starts a playlist and puts the titles in it in order", async () => {
    const view = await mount([{ kind: "chapter", id: "c1" }, { kind: "album", id: "a1" }], "2 items");
    await choosePlaylist("New playlist…");
    expect(screen.getByRole("button", { name: "Create and add" })).toBeDisabled();
    await userEvent.type(screen.getByRole("textbox", { name: /Name/ }), "Evening listening");
    await userEvent.click(screen.getByRole("button", { name: "Create and add" }));
    const create = await view.next(EntityPersonalListCreateDocument);
    expect(create.operation.variables.input).toEqual({ userId: OWNER, name: "Evening listening", kind: "MANUAL", savedViewId: null, description: null });
    await respond(create, { createPersonalList: { success: true, error: null, personalList: personalList({ name: "Evening listening" }) } });
    await respond(await view.next(PlaylistMembersDocument), connection("listEntries", []));
    const first = await view.next(EntityListEntryCreateDocument);
    expect(first.operation.variables.input).toMatchObject({ listId: "list-a", chapterId: "c1", position: 0 });
    await respond(first, createdEntry(personalEntry(0, { movieId: null, chapterId: "c1" })));
    const second = await view.next(EntityListEntryCreateDocument, 1);
    expect(second.operation.variables.input).toMatchObject({ albumId: "a1", position: 1024, chapterId: null });
    await respond(second, createdEntry(personalEntry(1, { movieId: null, albumId: "a1" })));
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    expectMutations(view, [EntityPersonalListCreateDocument, EntityListEntryCreateDocument, EntityListEntryCreateDocument]);
  });

  it("offers only a new playlist when there is none yet", async () => {
    await mount([{ kind: "movie", id: "m1" }], "Night film", []);
    expect(await screen.findByRole("textbox", { name: /Name/ })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Create and add" })).toBeInTheDocument();
  });

  it("blocks cancellation while saving, keeps a failed save open and allows a retry", async () => {
    const view = await mount([{ kind: "movie", id: "m1" }], "Night film");
    await choosePlaylist("Evening films · 2 items");
    const rows = connection("listEntries", []);
    await respond(await view.next(PlaylistMembersDocument), rows);
    await waitFor(() => expect(screen.getByRole("button", { name: "Add" })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await respond(await view.next(PlaylistMembersDocument, 1), rows);
    const first = await view.next(EntityListEntryCreateDocument);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    await userEvent.keyboard("{Escape}");
    expect(view.close).not.toHaveBeenCalled();
    await respond(first, { createListEntry: { success: false, error: "This list is read-only", listEntry: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("This list is read-only");
    expect(view.close).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Add" }));
    await respond(await view.next(PlaylistMembersDocument, 2), rows);
    await respond(await view.next(EntityListEntryCreateDocument, 1), createdEntry());
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
  });
});
