import { screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { EntityHomeShelfListDocument, EntityListEntryUpdateDocument, EntityPersonalListGetDocument, PersonalEntriesDocument } from "@/graphql/generated/graphql";

import { PersonalListPage } from "../PersonalListPage";
import { connection, expectValidOperation, installTableGeometry, OWNER, personalEntry, personalHarness, personalList, respond, STAMP } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

beforeEach(() => installTableGeometry());
afterEach(() => vi.restoreAllMocks());

async function mountList() {
  const view = personalHarness();
  const rows = ["First film", "Second film", "Third film"].map((title, index) => personalEntry(index, {
    position: index * 1024,
    movie: { id: `movie-${index}`, title, year: 2026, libraryId: "library-a", mediaFileId: null },
  }));
  await view.mount(<PersonalListPage listId="list-a" />);
  await respond(await view.next(EntityPersonalListGetDocument), { personalList: personalList() });
  await respond(await view.next(EntityHomeShelfListDocument), connection("homeShelves", []));
  await respond(await view.next(PersonalEntriesDocument), connection("listEntries", rows));
  await screen.findByRole("button", { name: "Actions for Second film" });
  return { ...view, rows };
}

async function moveSecondUp() {
  const view = await mountList();
  const trigger = screen.getByRole("button", { name: "Actions for Second film" });
  await userEvent.click(trigger);
  const up = await screen.findByRole("menuitem", { name: "Move up" });
  up.focus();
  await userEvent.keyboard("{Enter}");
  const update = await view.next(EntityListEntryUpdateDocument);
  return { ...view, trigger, update };
}

describe("PersonalListPage reorder and row menu focus", () => {
  it("moves the second entry before a zero-position head without a negative generated update", async () => {
    const view = await moveSecondUp();
    const stored = new Map(view.rows.map((row) => [row.id, row]));
    for (let index = 0; index < view.rows.length; index++) {
      const update = index === 0 ? view.update : await view.next(EntityListEntryUpdateDocument, index);
      const { id, input } = update.operation.variables;
      expect(input.position).toBeGreaterThanOrEqual(0);
      expect(Number.isInteger(input.position)).toBe(true);
      expectValidOperation(update.operation);
      const row = { ...stored.get(id)!, position: input.position };
      stored.set(id, row);
      expect(screen.getByRole("button", { name: "Actions for Second film" })).toBe(view.trigger);
      await respond(update, { updateListEntry: { success: true, error: null, listEntry: { ...row, userId: OWNER, createdAt: STAMP, updatedAt: STAMP } } });
    }
    const order = [...stored.values()].sort((left, right) => left.position - right.position);
    expect(order.map((row) => row.id)).toEqual(["entry-1", "entry-0", "entry-2"]);
    await respond(await view.next(PersonalEntriesDocument, 1), connection("listEntries", order));
    const table = screen.getByRole("grid", { name: "Evening films" });
    await waitFor(() => expect(within(table).getAllByRole("row")[1]).toHaveTextContent("Second film"));
    expect(view.mutations()).toHaveLength(3);
  });

  it("retains the row-menu opener and returns focus while a reorder is pending and after refusal", async () => {
    const view = await moveSecondUp();
    await waitFor(() => expect(view.trigger).toHaveFocus());
    expect(screen.getByRole("button", { name: "Actions for Second film" })).toBe(view.trigger);
    await userEvent.click(view.trigger);
    const disabledUp = await screen.findByRole("menuitem", { name: "Move up" });
    expect(disabledUp).toHaveAttribute("aria-disabled", "true");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(view.trigger).toHaveFocus());
    await respond(view.update, { updateListEntry: { success: false, error: "The list could not be changed", listEntry: null } });
    expect(await screen.findByRole("alert")).toHaveTextContent("The list could not be changed");
    expect(screen.getByRole("button", { name: "Actions for Second film" })).toBe(view.trigger);
    expect(view.trigger).toHaveFocus();
    await userEvent.click(view.trigger);
    await waitFor(() => expect(screen.getByRole("menuitem", { name: "Move up" })).not.toHaveAttribute("aria-disabled", "true"));
    expect(view.mutations()).toHaveLength(1);
  });
});
