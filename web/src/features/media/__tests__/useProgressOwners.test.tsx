import { act, renderHook, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";

import type { PlaybackProgressOwnersQuery } from "@/graphql/generated/graphql";

import { gridReset, groupProgressOwners, ownerOf, useProgressOwners } from "../useProgressOwners";
import { mediaHookHarness } from "./mediaHookHarness";

vi.mock("@/lib/auth/useSession", () => ({ useSession: () => ({ user: { id: "user-a" } }), useIsAdmin: () => false }));

type Row = PlaybackProgressOwnersQuery["playbackProgresses"]["edges"][number]["node"];
const row = (mediaFileId: string, file: Partial<NonNullable<Row["mediaFile"]>>, progress: { currentPosition?: number; isWatched?: boolean } = {}): Row => ({
  __typename: "PlaybackProgress", id: `p-${mediaFileId}`, userId: "user-a", mediaFileId, currentPosition: progress.currentPosition ?? 30, duration: 100, progressPercent: 30, isWatched: progress.isWatched ?? false,
  watchedAt: null, createdAt: "2026-01-01", updatedAt: "2026-01-01",
  mediaFile: { __typename: "MediaFile", id: mediaFileId, movieId: null, episode: null, track: null, chapter: null, ...file },
}) as unknown as Row;
const page = (...nodes: Row[]) => ({ playbackProgresses: { __typename: "PlaybackProgressConnection", edges: nodes.map((node) => ({ __typename: "PlaybackProgressEdge", node })) } });

describe("which title a progress row belongs to", () => {
  it("names the movie, show, album or book that owns the file", () => {
    expect(ownerOf(row("f1", { movieId: "m1" }))).toEqual({ kind: "movie", id: "m1" });
    expect(ownerOf(row("f2", { episode: { __typename: "Episode", id: "e1", showId: "s1" } } as never))).toEqual({ kind: "show", id: "s1" });
    expect(ownerOf(row("f3", { track: { id: "t1", albumId: "a1" } } as never))).toEqual({ kind: "album", id: "a1" });
    expect(ownerOf(row("f4", { chapter: { id: "c1", audiobookId: "b1" } } as never))).toEqual({ kind: "audiobook", id: "b1" });
    expect(ownerOf({ mediaFile: null })).toBeNull();
  });

  it("collects each title's files with saved progress, ignoring rows with none", () => {
    const owners = groupProgressOwners([
      row("f1", { episode: { __typename: "Episode", id: "e1", showId: "s1" } } as never, { isWatched: true, currentPosition: 0 }),
      row("f2", { episode: { id: "e2", showId: "s1" } } as never),
      row("f3", { episode: { id: "e3", showId: "s1" } } as never, { currentPosition: 0 }),
      row("f4", { movieId: "m1" }),
    ]);
    expect(owners.get("show:s1")).toEqual(["f1", "f2"]);
    expect(owners.get("movie:m1")).toEqual(["f4"]);
    expect(owners.has("album:a1")).toBe(false);
  });
});

describe("the grid's Mark as unwatched", () => {
  const owners = (over: Partial<Parameters<typeof gridReset>[1]> = {}) => ({ filesFor: (kind: string, id: string) => (kind === "show" && id === "s1" ? ["f1", "f2"] : []), loading: false, error: undefined, ...over });

  it("offers a title's progress files and nothing for an unwatched one", () => {
    const reset = gridReset<{ id: string }>("show", owners() as never);
    expect(reset.files([{ id: "s1" }])).toEqual(["f1", "f2"]);
    expect(reset.files([{ id: "s2" }])).toEqual([]);
    expect(reset.files([{ id: "s1" }, { id: "s1" }])).toEqual(["f1", "f2"]);
  });

  it("waits while the history loads and says when it could not", () => {
    expect(gridReset<{ id: string }>("show", owners({ loading: true }) as never).disabledReason?.([])).toBe("Loading watch history");
    expect(gridReset<{ id: string }>("show", owners({ error: new Error("x") }) as never).disabledReason?.([])).toBe("Watch history could not be loaded");
    expect(gridReset<{ id: string }>("show", owners() as never).disabledReason?.([])).toBeUndefined();
  });
});

describe("reading the history", () => {
  it("reads pages until one is short, then answers per title", async () => {
    const { requests, wrapper } = mediaHookHarness();
    const { result } = renderHook(() => useProgressOwners(), { wrapper });
    await waitFor(() => expect(requests).toHaveLength(1));
    expect(requests[0]!.operation.variables).toMatchObject({ userId: "user-a", page: { limit: 100, offset: 0 } });
    const full = Array.from({ length: 100 }, (_, index) => row(`f${index}`, index < 3 ? { movieId: `m${index}` } : { episode: { __typename: "Episode", id: `e${index}`, showId: "s1" } } as never));
    await act(async () => requests[0]!.resolve(page(...full)));
    await waitFor(() => expect(requests).toHaveLength(2));
    expect(requests[1]!.operation.variables).toMatchObject({ page: { offset: 100 } });
    await act(async () => requests[1]!.resolve(page(row("last", { movieId: "m9" }))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.filesFor("movie", "m9")).toEqual(["last"]);
    expect(result.current.filesFor("show", "s1")).toHaveLength(97);
    expect(requests).toHaveLength(2);
  });
});
