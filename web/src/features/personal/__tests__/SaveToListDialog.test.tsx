import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EntityListEntryCreateDocument, EntityListEntryDeleteDocument, EntityPersonalListCreateDocument, EntityPersonalListListDocument, PersonalEntriesDocument } from "@/graphql/generated/graphql";

import { SaveToListDialog } from "../SaveToListDialog";
import { connection, createdEntry, expectValidOperation, operationName, OWNER, personalEntry, personalHarness, personalList, respond } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

async function mount(entries?: ReturnType<typeof personalEntry>[]) {
  const harness = personalHarness();
  const close = vi.fn();
  await harness.mount(<SaveToListDialog open onOpenChange={close} target={{ kind: "movie", id: "target-movie" }} title="Night film" />);
  const lists = await harness.next(EntityPersonalListListDocument);
  expect(lists.operation.variables.where).toEqual({ userId: { eq: OWNER } });
  await respond(lists, connection("personalLists", [personalList(), personalList({ id: "smart-a", name: "Smart collection", kind: "SMART" })]));
  await userEvent.click(screen.getByRole("combobox", { name: "List" }));
  expect(screen.queryByRole("option", { name: "Smart collection" })).not.toBeInTheDocument();
  await userEvent.click(await screen.findByRole("option", { name: "Evening films" }));
  const first = await harness.next(PersonalEntriesDocument);
  if (entries) await respond(first, connection("listEntries", entries));
  return { ...harness, close, first };
}

function expectMutationNames(harness: ReturnType<typeof personalHarness>, documents: Parameters<typeof operationName>[0][]) {
  const mutations = harness.mutations();
  expect(mutations.map(({ operation }) => operation.operationName)).toEqual(documents.map(operationName));
  mutations.forEach(({ operation }) => expectValidOperation(operation));
}

describe("SaveToListDialog with real Glass controls and generated operations", () => {
  it("waits for all entries beyond 100 and appends after the greatest stored position", async () => {
    const view = await mount();
    const rows = Array.from({ length: 125 }, (_, index) => personalEntry(index));
    rows[124] = personalEntry(124, { position: 9000 });
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await respond(view.first, connection("listEntries", rows.slice(0, 60), true, rows.length));
    const second = await view.next(PersonalEntriesDocument, 1);
    expect(second.operation.variables.page).toEqual({ limit: 60, offset: 60 });
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await respond(second, connection("listEntries", rows.slice(60, 120), true, rows.length));
    const third = await view.next(PersonalEntriesDocument, 2);
    expect(third.operation.variables.page).toEqual({ limit: 60, offset: 120 });
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    expect(view.mutations()).toEqual([]);
    await respond(third, connection("listEntries", rows.slice(120), false, rows.length));
    await waitFor(() => expect(screen.getByRole("button", { name: "Save" })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    const save = await view.next(EntityListEntryCreateDocument);
    expect(save.operation.variables).toEqual({ input: { userId: OWNER, listId: "list-a", position: 9001, movieId: "target-movie", showId: null, albumId: null, audiobookId: null, trackId: null } });
    await respond(save, createdEntry(personalEntry(125, { movieId: "target-movie", position: 9001 })));
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    // The complete mutation allowlist also excludes every Wanted, acquisition and import action.
    expectMutationNames(view, [EntityListEntryCreateDocument]);
  });

  it("removes an existing member found after the first 100 entries instead of adding a duplicate", async () => {
    const view = await mount();
    const rows = Array.from({ length: 125 }, (_, index) => personalEntry(index));
    rows[124] = personalEntry(124, { movieId: "target-movie" });
    await respond(view.first, connection("listEntries", rows.slice(0, 60), true, 125));
    await respond(await view.next(PersonalEntriesDocument, 1), connection("listEntries", rows.slice(60, 120), true, 125));
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await respond(await view.next(PersonalEntriesDocument, 2), connection("listEntries", rows.slice(120), false, 125));
    await userEvent.click(await screen.findByRole("button", { name: "Remove from list" }));
    const remove = await view.next(EntityListEntryDeleteDocument);
    expect(remove.operation.variables).toEqual({ id: "entry-124" });
    await respond(remove, { deleteListEntry: { success: true, error: null } });
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    expectMutationNames(view, [EntityListEntryDeleteDocument]);
  });

  it("keeps an incomplete failed snapshot read-only until a complete retry", async () => {
    const view = await mount();
    await respond(view.first, connection("listEntries", [personalEntry(0)], false, 125));
    expect(await screen.findByRole("alert")).toHaveTextContent("load every row");
    expect(screen.getByRole("button", { name: "Save" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: /Try again/ }));
    const retry = await view.next(PersonalEntriesDocument, 1);
    expect(retry.operation.variables.page.offset).toBe(0);
    await respond(retry, connection("listEntries", []));
    await waitFor(() => expect(screen.getByRole("button", { name: "Save" })).toBeEnabled());
    expect(view.mutations()).toEqual([]);
    expect(view.close).not.toHaveBeenCalled();
  });

  it("blocks cancellation during save, keeps a failed save open, and permits retry", async () => {
    const view = await mount([]);
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    const first = await view.next(EntityListEntryCreateDocument);
    expect(screen.getByRole("button", { name: "Cancel" })).toBeDisabled();
    await userEvent.keyboard("{Escape}");
    expect(view.close).not.toHaveBeenCalled();
    await respond(first, { createListEntry: { success: false, error: "This list is read-only", listEntry: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("This list is read-only");
    expect(screen.getByRole("dialog", { name: "Save Night film" })).toBeInTheDocument();
    expect(view.close).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    await respond(await view.next(EntityListEntryCreateDocument, 1), createdEntry());
    await waitFor(() => expect(view.close).toHaveBeenCalledWith(false));
    expectMutationNames(view, [EntityListEntryCreateDocument, EntityListEntryCreateDocument]);
  });

  it("creates a manual list and saves catalogue identity without importing or marking it wanted", async () => {
    const view = personalHarness();
    await view.mount(<SaveToListDialog open onOpenChange={vi.fn()} target={{ kind: "track", id: "track-a" }} title="A track" />);
    await respond(await view.next(EntityPersonalListListDocument), connection("personalLists", []));
    await userEvent.type(screen.getByRole("textbox", { name: "New list" }), "Evening tracks");
    await userEvent.click(screen.getByRole("button", { name: "Create" }));
    const create = await view.next(EntityPersonalListCreateDocument);
    expect(create.operation.variables.input).toEqual({ userId: OWNER, name: "Evening tracks", kind: "MANUAL", savedViewId: null, description: null });
    const saved = personalList({ name: "Evening tracks" });
    await respond(create, { createPersonalList: { success: true, error: null, personalList: saved } });
    await respond(await view.next(EntityPersonalListListDocument, 1), connection("personalLists", [saved]));
    await respond(await view.next(PersonalEntriesDocument), connection("listEntries", []));
    await waitFor(() => expect(screen.getByRole("button", { name: "Save" })).toBeEnabled());
    await userEvent.click(screen.getByRole("button", { name: "Save" }));
    const save = await view.next(EntityListEntryCreateDocument);
    expect(save.operation.variables.input).toEqual({ userId: OWNER, listId: "list-a", position: 0, movieId: null, showId: null, albumId: null, audiobookId: null, trackId: "track-a" });
    await respond(save, createdEntry(personalEntry(0, { movieId: null, trackId: "track-a" })));
    expectMutationNames(view, [EntityPersonalListCreateDocument, EntityListEntryCreateDocument]);
  });
});
