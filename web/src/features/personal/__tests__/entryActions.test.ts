import { defineActions, resolveActions } from "glass-ux/actions";
import { describe, expect, it, vi } from "vitest";

import { ENTRY_OWN_ACTIONS, liftActions } from "../entryActions";

type Target = { id: string; wanted: boolean };
type Entry = { id: string; movie: Target | null; album: Target | null };

const run = vi.fn();
const set = defineActions<Target>("movie", [
  { id: "play", label: "Play", group: "open", run: vi.fn() },
  { id: "refresh", label: (rows) => (rows.length === 1 ? "Refresh metadata" : `Refresh ${rows.length}`), group: "manage", bulk: true, run, successMessage: (rows) => `Refreshed ${rows.length}` },
  { id: "unwant", label: "Stop wanting", group: "manage", visible: (rows) => rows.every((row) => row.wanted), run: vi.fn() },
]);
const lifted = liftActions<Entry, Target>(set, (entry) => entry.movie);
const movie = (id: string, wanted = true): Entry => ({ id: `e-${id}`, movie: { id, wanted }, album: null });
const album: Entry = { id: "e-album", movie: null, album: { id: "al", wanted: false } };

describe("a target's actions on a playlist entry", () => {
  it("leaves out what the entry already does for itself", () => {
    expect(lifted.map((action) => action.id)).toEqual(["movie-refresh", "movie-unwant"]);
    expect(ENTRY_OWN_ACTIONS.has("play")).toBe(true);
  });

  it("runs on the entries' targets and labels for them", async () => {
    const refresh = lifted[0]!;
    const rows = [movie("a"), movie("b")];
    expect((refresh.label as (rows: readonly Entry[]) => string)(rows)).toBe("Refresh 2");
    expect(refresh.successMessage?.(rows)).toBe("Refreshed 2");
    await refresh.run?.(rows, {} as never);
    expect(run).toHaveBeenCalledWith([{ id: "a", wanted: true }, { id: "b", wanted: true }], {});
  });

  it("shows only when every selected entry points at one, and the target's own rule agrees", () => {
    const shown = (rows: Entry[]) => resolveActions(defineActions<Entry>("entry", lifted), rows, "menu").flatMap((group) => group.actions.map((action) => action.id));
    expect(shown([movie("a")])).toEqual(["movie-refresh", "movie-unwant"]);
    expect(shown([movie("a", false)])).toEqual(["movie-refresh"]);
    expect(shown([album])).toEqual([]);
    expect(shown([movie("a"), album])).toEqual([]);
  });
});
