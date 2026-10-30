import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { EntityHomeShelfCreateDocument, EntityHomeShelfListDocument, EntityHomeShelfUpdateDocument } from "@/graphql/generated/graphql";

import { HomeCustomization } from "../HomeCustomization";
import { DEFAULT_SHELVES } from "../usePersonalLibrary";
import { connection, expectValidOperation, homeShelf, operationName, OWNER, personalHarness, respond } from "./personalHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "admin-self", role: "admin" }, status: "authenticated" }) }));

describe("HomeCustomization generated mutation boundaries", () => {
  it("reorders and hides a personal shelf while preserving its list/library references", async () => {
    const view = personalHarness();
    const close = vi.fn();
    await view.mount(<HomeCustomization open onOpenChange={close} />);
    const first = await view.next(EntityHomeShelfListDocument);
    expect(first.operation.variables.where).toEqual({ userId: { eq: OWNER } });
    const shelf = homeShelf();
    await respond(first, connection("homeShelves", [shelf]));
    await userEvent.click(await screen.findByRole("button", { name: "Move Personal list up" }));
    const reorder = await view.next(EntityHomeShelfUpdateDocument);
    const input = reorder.operation.variables.input;
    expect(reorder.operation.variables.id).toBe(shelf.id);
    expect(input).toEqual({ position: expect.any(Number), visible: true, listId: "list-a", libraryId: null });
    // The new order sits between TV guide (60) and Movie releases (70), rather than
    // dropping the associated list or sending the rendered row's key/label/id.
    expect(input.position).toBeGreaterThan(60);
    expect(input.position).toBeLessThan(70);
    expectValidOperation(reorder.operation);
    expect(screen.getByRole("button", { name: "Done" })).toBeDisabled();
    const moved = homeShelf({ position: input.position });
    await respond(reorder, { updateHomeShelf: { success: true, error: null, homeShelf: moved } });
    await respond(await view.next(EntityHomeShelfListDocument, 1), connection("homeShelves", [moved]));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Show Personal list" })).toBeEnabled());
    await userEvent.click(screen.getByRole("switch", { name: "Show Personal list" }));
    const hide = await view.next(EntityHomeShelfUpdateDocument, 1);
    expect(hide.operation.variables).toEqual({ id: shelf.id, input: { position: moved.position, visible: false, listId: "list-a", libraryId: null } });
    expectValidOperation(hide.operation);
    const hidden = homeShelf({ position: moved.position, visible: false });
    await respond(hide, { updateHomeShelf: { success: true, error: null, homeShelf: hidden } });
    await respond(await view.next(EntityHomeShelfListDocument, 2), connection("homeShelves", [hidden]));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Show Personal list" })).not.toBeChecked());
    expect(close).not.toHaveBeenCalled();
    expect(view.mutations().map(({ operation }) => operation.operationName)).toEqual([operationName(EntityHomeShelfUpdateDocument), operationName(EntityHomeShelfUpdateDocument)]);
  });

  it("persists a previously unsaved built-in shelf for the current administrator as a personal row", async () => {
    const view = personalHarness();
    await view.mount(<HomeCustomization open onOpenChange={vi.fn()} />);
    await respond(await view.next(EntityHomeShelfListDocument), connection("homeShelves", []));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Show Libraries" })).toBeEnabled());
    await userEvent.click(screen.getByRole("switch", { name: "Show Libraries" }));
    const create = await view.next(EntityHomeShelfCreateDocument);
    const input = create.operation.variables.input;
    expect(input).toMatchObject({ userId: OWNER, shelfKey: "libraries", visible: false, position: 10 });
    expect(Object.keys(input).filter((key) => !["userId", "shelfKey", "position", "visible", "listId", "libraryId"].includes(key))).toEqual([]);
    expectValidOperation(create.operation);
    const saved = homeShelf({ id: "shelf-libraries", shelfKey: "libraries", visible: false, position: 10, listId: null, libraryId: null });
    await respond(create, { createHomeShelf: { success: true, error: null, homeShelf: saved } });
    await respond(await view.next(EntityHomeShelfListDocument, 1), connection("homeShelves", [saved]));
    await waitFor(() => expect(screen.getByRole("switch", { name: "Show Libraries" })).not.toBeChecked());
    expect(view.mutations().map(({ operation }) => operation.operationName)).toEqual([operationName(EntityHomeShelfCreateDocument)]);
  });

  it("moves the second shelf ahead of a zero-position head using only nonnegative positions", async () => {
    const view = personalHarness();
    await view.mount(<HomeCustomization open onOpenChange={vi.fn()} />);
    const initial = [
      ...DEFAULT_SHELVES.map((item, index) => homeShelf({ id: `shelf-${item.key}`, shelfKey: item.key, position: index * 10, listId: null, libraryId: null })),
      homeShelf({ position: 80 }),
    ];
    const stored = new Map(initial.map((row) => [row.id, row]));
    await respond(await view.next(EntityHomeShelfListDocument), connection("homeShelves", initial));
    await userEvent.click(await screen.findByRole("button", { name: "Move Libraries up" }));
    for (let index = 0; index < initial.length; index++) {
      const update = await view.next(EntityHomeShelfUpdateDocument, index);
      const { id, input } = update.operation.variables;
      const previous = stored.get(id)!;
      expect(input.position).toBeGreaterThanOrEqual(0);
      expect(Number.isInteger(input.position)).toBe(true);
      expect(input.listId).toBe(previous.listId);
      expect(input.libraryId).toBe(previous.libraryId);
      expectValidOperation(update.operation);
      const saved = { ...previous, ...input };
      stored.set(id, saved);
      await respond(update, { updateHomeShelf: { success: true, error: null, homeShelf: saved } });
      await respond(await view.next(EntityHomeShelfListDocument, index + 1), connection("homeShelves", [...stored.values()]));
    }
    await waitFor(() => expect(screen.getByRole("button", { name: "Done" })).toBeEnabled());
    const ordered = [...stored.values()].sort((left, right) => left.position - right.position);
    expect(ordered.map((row) => row.shelfKey).slice(0, 2)).toEqual(["libraries", "continue"]);
    expect(new Set(ordered.map((row) => row.position)).size).toBe(initial.length);
    expect(stored.get("shelf-a")).toMatchObject({ listId: "list-a", libraryId: null });
    expect(view.mutations()).toHaveLength(initial.length);
  });

});
