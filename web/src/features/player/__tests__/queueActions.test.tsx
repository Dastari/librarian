import type { ApolloClient } from "@apollo/client";
import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { resolveQueueSources, useQueueActions } from "../queueActions";
import { playerStore, type PlayItem } from "../store";

vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));

const apollo = vi.hoisted(() => ({ client: { query: vi.fn(), mutate: vi.fn() } }));
vi.mock("@apollo/client/react", () => ({ useApolloClient: () => apollo.client }));

const track = (id: string): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind: "track", id } });
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

const personal = vi.hoisted(() => ({ fetchPlaylistTargets: vi.fn() }));
vi.mock("@/features/personal/usePlaylists", async (original) => ({ ...(await original<typeof import("@/features/personal/usePlaylists")>()), fetchPlaylistTargets: personal.fetchPlaylistTargets }));

describe("playlists as a queue source", () => {
  /** Answers each generated document by operation name with the rows it has for the requested ids. */
  const answer = (rows: Record<string, Array<{ id: string }>>) => apollo.client.query.mockImplementation(async ({ query: document, variables }: { query: { definitions: Array<{ name: { value: string } }> }; variables: { ids: string[] } }) => {
    const name = document.definitions[0]!.name.value;
    const field = name.includes("Movies") ? "movies" : name.includes("Episodes") ? "episodes" : name.includes("Tracks") ? "tracks" : "chapters";
    return { data: { [field]: { edges: (rows[name] ?? []).filter((row) => variables.ids.includes(row.id)).map((node) => ({ node })), pageInfo: { hasNextPage: false } } } };
  });
  const rows = {
    PlaylistPlayMovies: [{ id: "m1", title: "Film", year: 1999, runtime: 90, mediaFileId: "file-m1" }],
    PlaylistPlayEpisodes: [{ id: "e1", showId: "s", season: 1, episode: 2, title: "Pilot", runtime: 40, mediaFileId: "file-e1", show: { id: "s", name: "Series" } }],
    PlaylistPlayTracks: [{ id: "t1", title: "Song", albumId: "a", discNumber: 1, trackNumber: 1, artistName: "Band", durationSecs: 90, mediaFileId: "file-t1", album: { id: "a", name: "Record" } }, { id: "t2", title: "No file", albumId: "a", discNumber: 1, trackNumber: 2, artistName: null, durationSecs: null, mediaFileId: null, album: null }],
    PlaylistPlayChapters: [{ id: "c1", audiobookId: "b", chapterNumber: 1, title: "One", startTimeSecs: 0, durationSecs: 600, fileOffsetSecs: null, mediaFileId: "file-c1", mediaFile: { id: "file-c1", duration: 600 }, audiobook: { id: "b", title: "Book", authorName: "Author" } }],
  };

  it("expands a playlist to every kind it holds, in position order, leaving out what has no file", async () => {
    personal.fetchPlaylistTargets.mockResolvedValue([{ kind: "movie", id: "m1" }, { kind: "track", id: "t2" }, { kind: "episode", id: "e1" }, { kind: "chapter", id: "c1" }, { kind: "track", id: "t1" }]);
    answer(rows);
    const items = await resolveQueueSources(client, [{ kind: "playlist", id: "list-1" }]);
    expect(personal.fetchPlaylistTargets).toHaveBeenCalledWith(client, "list-1");
    expect(items.map((entry) => `${entry.entity.kind}:${entry.entity.id}`)).toEqual(["movie:m1", "episode:e1", "chapter:c1", "track:t1"]);
  });

  it("queues a playlist after what is playing: Play next and Add to queue with a playlist source", async () => {
    personal.fetchPlaylistTargets.mockResolvedValue([{ kind: "episode", id: "e1" }, { kind: "track", id: "t1" }]);
    answer(rows);
    playerStore.playQueue([track("now"), track("later")]);
    const list = renderHook(() => useQueueActions<{ id: string }>((selected) => selected.map((row) => ({ kind: "playlist" as const, id: row.id })))).result.current;
    await list.find((action) => action.id === "play-next")!.run!([{ id: "list-1" }], {} as never);
    expect(playerStore.getSnapshot().queue.map((entry) => entry.entity.id)).toEqual(["now", "e1", "t1", "later"]);
    await list.find((action) => action.id === "enqueue")!.run!([{ id: "list-1" }], {} as never);
    expect(playerStore.getSnapshot().queue.map((entry) => entry.entity.id)).toEqual(["now", "e1", "t1", "later", "e1", "t1"]);
  });

  it("resolves a list of targets, episodes and chapters included, as one source", async () => {
    answer(rows);
    const items = await resolveQueueSources(client, [{ kind: "targets", targets: [{ kind: "chapter", id: "c1" }, { kind: "episode", id: "e1" }] }, { kind: "movie", id: "m1" }]);
    expect(items.map((entry) => entry.entity.id)).toEqual(["c1", "e1", "m1"]);
  });
});
