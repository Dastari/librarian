import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EntityHomeShelfListDocument, EntityListEntryDeleteDocument, EntityListEntryUpdateDocument, PersonalEntriesDocument, PlaylistGetDocument, PlaylistPlayMoviesDocument } from "@/graphql/generated/graphql";
import { playerStore } from "@/features/player/store";

import { PersonalListPage } from "../PersonalListPage";
import { connection, expectValidOperation, installTableGeometry, movieFields, OWNER, personalEntry, personalHarness, playlist, respond, STAMP } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

beforeEach(() => installTableGeometry());
afterEach(() => { vi.restoreAllMocks(); playerStore.stopAudio(); playerStore.setVideo(null); });

const FILMS = ["First film", "Second film", "Third film"];

async function mountList(positions = [0, 1024, 2048]) {
  const view = personalHarness();
  const rows = FILMS.map((title, index) => personalEntry(index, { position: positions[index], movie: movieFields(`movie-${index}`, title, { mediaFileId: `file-${index}`, hasFile: true }) }));
  await view.mount(<PersonalListPage listId="list-a" />);
  await respond(await view.next(PlaylistGetDocument), { personalList: playlist({}, rows.length) });
  for (const request of view.requests.filter(({ operation }) => operation.operationName === "EntityHomeShelfList")) await respond(request, connection("homeShelves", []));
  await respond(await view.next(PersonalEntriesDocument), connection("listEntries", rows));
  await screen.findByRole("button", { name: "Actions for Second film" });
  return { ...view, rows };
}

async function choose(view: Awaited<ReturnType<typeof mountList>>, row: string, action: string) {
  const trigger = screen.getByRole("button", { name: `Actions for ${row}` });
  await userEvent.click(trigger);
  const item = await screen.findByRole("menuitem", { name: action });
  item.focus();
  await userEvent.keyboard("{Enter}");
  return trigger;
}

describe("playlist page header", () => {
  it("names the playlist with its kind, item count and length, and offers Play and Shuffle", async () => {
    await mountList();
    expect(screen.getByRole("heading", { level: 1, name: /Evening films/ })).toBeInTheDocument();
    expect(await screen.findByText("Playlist · 3 items · 5h")).toBeInTheDocument();
    // jsdom measures no room for header buttons, so they sit in the overflow menu.
    await userEvent.click(screen.getByRole("button", { name: "More actions for Evening films" }));
    expect(await screen.findByRole("menuitem", { name: "Play" })).toBeEnabled();
    expect(screen.getByRole("menuitem", { name: "Shuffle" })).toBeEnabled();
    expect(screen.getByRole("menuitem", { name: "Pin to Home" })).toBeInTheDocument();
  });
});

describe("playlist page reorder and row menu focus", () => {
  it("moves the second entry before a zero-position head with one write, leaving the tie to the server", async () => {
    const view = await mountList();
    await choose(view, "Second film", "Move up");
    const update = await view.next(EntityListEntryUpdateDocument);
    expect(update.operation.variables).toEqual({ id: "entry-1", input: { position: 0 } });
    expectValidOperation(update.operation);
    await respond(update, { updateListEntry: { success: true, error: null, listEntry: { ...view.rows[1]!, position: 0, userId: OWNER, createdAt: STAMP, updatedAt: STAMP } } });
    const order = [view.rows[1]!, view.rows[0]!, view.rows[2]!].map((row, index) => ({ ...row, position: index * 1024 }));
    await respond(await view.next(PersonalEntriesDocument, 1), connection("listEntries", order));
    const table = screen.getByRole("grid", { name: "Evening films" });
    await waitFor(() => expect(within(table).getAllByRole("row")[1]).toHaveTextContent("Second film"));
    expect(view.mutations()).toHaveLength(1);
  });

  it("puts a moved entry halfway between its new neighbours", async () => {
    const view = await mountList();
    await choose(view, "Third film", "Move up");
    expect((await view.next(EntityListEntryUpdateDocument)).operation.variables).toEqual({ id: "entry-2", input: { position: 512 } });
  });

  it("sends the next neighbour's own position when no gap is left", async () => {
    const view = await mountList([0, 1, 2]);
    await choose(view, "Third film", "Move up");
    expect((await view.next(EntityListEntryUpdateDocument)).operation.variables).toEqual({ id: "entry-2", input: { position: 1 } });
  });

  it("keeps the row-menu opener and says why while a reorder is pending and after refusal", async () => {
    const view = await mountList();
    const trigger = await choose(view, "Second film", "Move up");
    const update = await view.next(EntityListEntryUpdateDocument);
    await waitFor(() => expect(trigger).toHaveFocus());
    await userEvent.click(trigger);
    expect(await screen.findByRole("menuitem", { name: "Move up" })).toHaveAttribute("aria-disabled", "true");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(trigger).toHaveFocus());
    await respond(update, { updateListEntry: { success: false, error: "The list could not be changed", listEntry: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("The list could not be changed");
    expect(screen.getByRole("button", { name: "Actions for Second film" })).toBe(trigger);
    await userEvent.click(trigger);
    await waitFor(() => expect(screen.getByRole("menuitem", { name: "Move up" })).not.toHaveAttribute("aria-disabled", "true"));
    expect(view.mutations()).toHaveLength(1);
  });
});

describe("playlist page playing", () => {
  it("plays from the chosen entry: its video first, the rest queued in order", async () => {
    const view = await mountList();
    await choose(view, "Second film", "Play from here");
    const resolve = await view.next(PlaylistPlayMoviesDocument);
    expect(resolve.operation.variables).toEqual({ ids: ["movie-1", "movie-2"] });
    await respond(resolve, { movies: { edges: ["movie-2", "movie-1"].map((id) => ({ node: { __typename: "Movie", id, title: id, year: 2026, runtime: 100, mediaFileId: `file-${id.at(-1)}` } })) } });
    await waitFor(() => expect(playerStore.getSnapshot().video?.entity.id).toBe("movie-1"));
    expect(playerStore.getSnapshot().queue.map((item) => item.entity.id)).toEqual(["movie-2"]);
  });
});

describe("playlist page bulk selection", () => {
  it("confirms selected removal once, attempts every selected item and reports partial failure", async () => {
    const view = await mountList();
    await userEvent.click(screen.getByRole("checkbox", { name: "Select First film" }));
    await userEvent.click(screen.getByRole("checkbox", { name: "Select Second film" }));
    await userEvent.click(screen.getByRole("button", { name: "Actions for 2 selected items" }));
    expect(await screen.findByRole("menuitem", { name: "Play 2 selected" })).toBeInTheDocument();
    expect(screen.getByRole("menuitem", { name: "Add to playlist…" })).toBeInTheDocument();
    await userEvent.click(await screen.findByRole("menuitem", { name: "Remove 2 from playlist" }));
    const confirm = within(await screen.findByRole("alertdialog"));
    expect(view.mutations()).toHaveLength(0);
    expect(confirm.getByText("Media and files remain in the library.")).toBeInTheDocument();
    await userEvent.click(confirm.getByRole("button", { name: "Remove items" }));
    const first = await view.next(EntityListEntryDeleteDocument);
    expect(first.operation.variables).toEqual({ id: "entry-0" });
    await respond(first, { deleteListEntry: { success: false, error: "Entry is locked" } });
    const second = await view.next(EntityListEntryDeleteDocument, 1);
    expect(second.operation.variables).toEqual({ id: "entry-1" });
    await respond(second, { deleteListEntry: { success: true, error: null } });
    await respond(await view.next(PersonalEntriesDocument, 1), connection("listEntries", [view.rows[0]!, view.rows[2]!]));
    expect(await screen.findByRole("alert")).toHaveTextContent("Removed 1 of 2 items");
    expect(view.mutations()).toHaveLength(2);
    expect(screen.getByText("Third film")).toBeInTheDocument();
  });
});
