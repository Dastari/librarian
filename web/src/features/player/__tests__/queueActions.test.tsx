import type { ApolloClient } from "@apollo/client";
import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { playlistAdapter, savableItems } from "../playlistAdapter";
import { resolveQueueSources, useQueueActions } from "../queueActions";
import { playerStore, type PlayItem } from "../store";

vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));

const apollo = vi.hoisted(() => ({ client: { query: vi.fn(), mutate: vi.fn() } }));
vi.mock("@apollo/client/react", () => ({ useApolloClient: () => apollo.client }));

const track = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "track", id } });
const episode = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "episode", id } });
const chapter = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "chapter", id } });
const movie = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "movie", id } });
const titles = () => playerStore.getSnapshot().queue.map((entry) => entry.title);
const client = apollo.client as unknown as ApolloClient;

beforeEach(() => {
  vi.clearAllMocks();
  playerStore.forgetPerson();
  playerStore.stopAudio();
});
afterEach(() => playerStore.stopAudio());

describe("Play next and Add to queue", () => {
  const actions = (toSources: Parameters<typeof useQueueActions<{ id: string }>>[0], options?: Parameters<typeof useQueueActions<{ id: string }>>[1]) =>
    renderHook(() => useQueueActions<{ id: string }>(toSources, options)).result.current;
  const byId = (list: ReturnType<typeof actions>, id: string) => list.find((action) => action.id === id)!;
  const context = {} as never;

  it("offers both, for one row or a selection", () => {
    const list = actions(() => []);
    expect(list.map((action) => [action.id, action.label, action.bulk])).toEqual([["play-next", "Play next", true], ["enqueue", "Add to queue", true]]);
  });

  it("puts the rows' items after what is playing, or at the end", async () => {
    playerStore.playQueue(["a", "b"].map(track));
    const list = actions((rows) => [rows.map((row) => track(row.id))]);
    await byId(list, "play-next").run!([{ id: "x" }, { id: "y" }], context);
    expect(titles()).toEqual(["a", "x", "y", "b"]);
    await byId(list, "enqueue").run!([{ id: "z" }], context);
    expect(titles()).toEqual(["a", "x", "y", "b", "z"]);
  });

  it("expands an entity to its playable children, in the order the rows are given", async () => {
    apollo.client.query.mockImplementation(async ({ query: document, variables }: { query: { definitions: Array<{ name: { value: string } }> }; variables: { id: string } }) => {
      const name = document.definitions[0]!.name.value;
      if (name === "AlbumDetail") return { data: { album: { id: variables.id, name: variables.id, tracks: { edges: [1, 2].map((number) => ({ node: { id: `${variables.id}-${number}`, albumId: variables.id, title: `${variables.id}-${number}`, trackNumber: number, discNumber: 1, mediaFileId: `file-${variables.id}-${number}`, artistName: null, durationSecs: 60 } })), pageInfo: { hasNextPage: false } } } } };
      throw new Error(name);
    });
    playerStore.playQueue([track("now")]);
    const list = actions((rows) => rows.map((row) => ({ kind: "album" as const, id: row.id })));
    await byId(list, "enqueue").run!([{ id: "one" }, { id: "two" }], context);
    expect(titles()).toEqual(["now", "one-1", "one-2", "two-1", "two-2"]);
  });

  it("says so when there is nothing to play, rather than queueing nothing", async () => {
    const list = actions(() => [[]]);
    await expect(byId(list, "enqueue").run!([{ id: "x" }], context)).rejects.toThrow("Nothing here is ready to play");
  });

  it("follows the entity's own rules for when it applies", () => {
    const list = actions(() => [], { visible: (rows) => rows.length === 1, disabledReason: () => "No file" });
    expect(byId(list, "enqueue").visible!([{ id: "a" }, { id: "b" }])).toBe(false);
    expect(byId(list, "enqueue").disabledReason!([{ id: "a" }])).toBe("No file");
  });

  it("resolves items already in hand without asking the server", async () => {
    expect(await resolveQueueSources(client, [[track("a")], [track("b")]])).toHaveLength(2);
    expect(apollo.client.query).not.toHaveBeenCalled();
  });
});

describe("the playlist adapter", () => {
  it("saves tracks and movies in queue order and reports what lists cannot hold yet", async () => {
    apollo.client.mutate.mockImplementation(async ({ variables }: { variables: { input: Record<string, unknown> } }) => (
      "kind" in variables.input
        ? { data: { createPersonalList: { success: true, error: null, personalList: { id: "list-1" } } } }
        : { data: { createListEntry: { success: true, error: null } } }
    ));
    const queue = [track("t1"), episode("e1"), movie("m1"), chapter("c1"), track("t2")];
    const result = await playlistAdapter.createFromItems(client, { userId: "user-1", name: "  Road trip ", items: queue });
    expect(result).toEqual({ listId: "list-1", saved: 3, skipped: 2 });
    const inputs = apollo.client.mutate.mock.calls.map(([call]) => call.variables.input);
    expect(inputs[0]).toEqual({ userId: "user-1", name: "Road trip", kind: "MANUAL", description: null, savedViewId: null });
    expect(inputs.slice(1)).toEqual([
      { userId: "user-1", listId: "list-1", position: 0, trackId: "t1" },
      { userId: "user-1", listId: "list-1", position: 1, movieId: "m1" },
      { userId: "user-1", listId: "list-1", position: 2, trackId: "t2" },
    ]);
  });

  it("creates nothing when nothing in the queue can be saved", async () => {
    await expect(playlistAdapter.createFromItems(client, { userId: "user-1", name: "Shows", items: [episode("e1"), chapter("c1")] })).rejects.toThrow(/Episodes and chapters/);
    expect(apollo.client.mutate).not.toHaveBeenCalled();
  });

  it("reports a failed save rather than a half-made playlist as success", async () => {
    apollo.client.mutate.mockResolvedValueOnce({ data: { createPersonalList: { success: true, error: null, personalList: { id: "list-1" } } } })
      .mockResolvedValueOnce({ data: { createListEntry: { success: false, error: "nope" } } });
    await expect(playlistAdapter.createFromItems(client, { userId: "user-1", name: "X", items: [track("t1")] })).rejects.toThrow();
  });

  it("knows which items it can save", () => {
    expect(savableItems([track("t"), episode("e"), movie("m"), chapter("c")]).map((entry) => entry.entity.kind)).toEqual(["track", "movie"]);
  });

  it("expands a playlist to its entries' play items, in position order", async () => {
    apollo.client.query.mockResolvedValue({ data: { listEntries: {
      edges: [
        { node: { id: "1", movie: { id: "m1", title: "Film", year: 1999, libraryId: "l", mediaFileId: "file-m1" }, track: null, albumId: null, audiobookId: null, showId: null } },
        { node: { id: "2", movie: null, track: { id: "t1", title: "Song", albumId: "album", artistName: "Band", mediaFileId: "file-t1", durationSecs: 90, album: { id: "album", coverUrl: null } }, albumId: null, audiobookId: null, showId: null } },
        { node: { id: "3", movie: null, track: { id: "t2", title: "No file", albumId: "album", artistName: null, mediaFileId: null, durationSecs: null, album: null }, albumId: null, audiobookId: null, showId: null } },
      ],
      pageInfo: { hasNextPage: false, totalCount: 3 },
    } } });
    const items = await playlistAdapter.expand(client, "list-1");
    expect(items.map((entry) => `${entry.entity.kind}:${entry.title}`)).toEqual(["movie:Film", "track:Song"]);
  });
});
