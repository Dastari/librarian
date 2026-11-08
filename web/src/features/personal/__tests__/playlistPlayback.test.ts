import type { ApolloClient } from "@apollo/client";
import { getOperationAST } from "graphql";
import { beforeEach, describe, expect, it, vi } from "vitest";

import type { PlayItem } from "@/features/player/store";

import { expandPlaylist, isVideoPlayItem, playItemTargets, playlistPlayItems, shufflePlayItems, startPlaylistPlayback } from "../playlistPlayback";

// Containers expand through the queue's own helpers; these stand in for them.
const playables = vi.hoisted(() => ({ albumPlayItems: vi.fn(), showPlayItems: vi.fn(), audiobookItems: vi.fn() }));
vi.mock("@/features/player/playables", () => playables);
const members = vi.hoisted(() => ({ fetchPlaylistTargets: vi.fn() }));
vi.mock("../usePlaylists", () => members);

type Rows = Record<string, unknown[]>;

/** A client that answers each generated document by its operation name, recording what was asked. */
function fakeClient(answers: Rows) {
  const asked: { name: string; variables: Record<string, unknown> }[] = [];
  const client = {
    query: async ({ query, variables }: { query: Parameters<typeof getOperationAST>[0]; variables: Record<string, unknown> }) => {
      const name = getOperationAST(query)!.name!.value;
      asked.push({ name, variables });
      const field = name.includes("Movies") ? "movies" : name.includes("Episodes") ? "episodes" : name.includes("Tracks") ? "tracks" : "chapters";
      const rows = (answers[name] ?? []).filter((row) => (variables.ids as string[]).includes((row as { id: string }).id));
      return { data: { [field]: { edges: rows.map((node) => ({ node })), pageInfo: { hasNextPage: false } } } };
    },
  } as unknown as ApolloClient;
  return { client, asked };
}

const album = { id: "a", name: "Record" };
const track = (id: string, trackNumber: number, discNumber: number | null, mediaFileId: string | null = `f-${id}`) => ({ id, title: `Song ${id}`, albumId: "a", discNumber, trackNumber, artistName: "Band", durationSecs: 200, mediaFileId, album });
const episode = (id: string, season: number, number: number, showId = "s", mediaFileId: string | null = `f-${id}`) => ({ id, showId, season, episode: number, title: `Ep ${id}`, runtime: 40, mediaFileId, show: { id: showId, name: "Series" } });
const chapter = (id: string, chapterNumber: number, mediaFileId: string | null, extra: Record<string, unknown> = {}) => ({ id, audiobookId: "b", chapterNumber, title: `Chapter ${chapterNumber}`, startTimeSecs: 0, durationSecs: 600, fileOffsetSecs: null, mediaFileId, mediaFile: mediaFileId ? { id: mediaFileId, duration: 600 } : null, audiobook: { id: "b", title: "Book", authorName: "Author" }, ...extra });
const movie = (id: string, mediaFileId: string | null = `f-${id}`) => ({ id, title: `Film ${id}`, year: 2020, runtime: 90, mediaFileId });

describe("playlistPlayItems", () => {
  beforeEach(() => vi.clearAllMocks());
  it("keeps playlist order across kinds, tracks and chapters as audio, movies and episodes as video", async () => {
    const { client } = fakeClient({
      PlaylistPlayMovies: [movie("m1")], PlaylistPlayEpisodes: [episode("e1", 1, 1)],
      PlaylistPlayTracks: [track("t1", 1, 1)], PlaylistPlayChapters: [chapter("c1", 1, "f-c1")],
    });
    const { items, skipped } = await playlistPlayItems(client, [{ kind: "track", id: "t1" }, { kind: "movie", id: "m1" }, { kind: "chapter", id: "c1" }, { kind: "episode", id: "e1" }]);
    expect(items.map((item) => `${item.entity.kind}:${item.entity.id}`)).toEqual(["track:t1", "movie:m1", "chapter:c1", "episode:e1"]);
    expect(items.map(isVideoPlayItem)).toEqual([false, true, false, true]);
    expect(skipped).toBe(0);
    expect(items[0]).toMatchObject({ mediaFileId: "f-t1", href: "/albums/a", subtitle: "Band · Record", duration: 200 });
    expect(items[1]).toMatchObject({ duration: 5400, href: "/movies/m1" });
    expect(items[3]).toMatchObject({ title: "Series", subtitle: "S01 · E01  Ep e1", href: "/shows/s" });
  });

  it("expands an album, a show and an audiobook through the queue's helpers, once each, in the playlist's order", async () => {
    const queued = (kind: PlayItem["entity"]["kind"], id: string): PlayItem => ({ mediaFileId: `f-${id}`, title: id, entity: { kind, id } });
    playables.showPlayItems.mockResolvedValue([queued("episode", "e1"), queued("episode", "e2")]);
    playables.albumPlayItems.mockResolvedValue([queued("track", "t1")]);
    playables.audiobookItems.mockResolvedValue([queued("chapter", "c1")]);
    const { client } = fakeClient({});
    const { items, starts } = await playlistPlayItems(client, [{ kind: "show", id: "s" }, { kind: "album", id: "a" }, { kind: "audiobook", id: "b" }, { kind: "album", id: "a" }]);
    expect(items.map((item) => item.entity.id)).toEqual(["e1", "e2", "t1", "c1", "t1"]);
    expect(starts).toEqual([0, 2, 3, 4]);
    expect(playables.albumPlayItems).toHaveBeenCalledTimes(1);
    expect(playables.albumPlayItems).toHaveBeenCalledWith(client, "a");
    expect(playables.showPlayItems).toHaveBeenCalledWith(client, "s");
    expect(playables.audiobookItems).toHaveBeenCalledWith(client, "b");
  });

  it("carries the album and the show so playback can continue like any queued track or episode", async () => {
    const { client } = fakeClient({ PlaylistPlayTracks: [track("t1", 1, 1)], PlaylistPlayEpisodes: [episode("e1", 1, 1)] });
    const { items } = await playlistPlayItems(client, [{ kind: "track", id: "t1" }, { kind: "episode", id: "e1" }]);
    expect(items[0]).toMatchObject({ albumId: "a" });
    expect(items[1]).toMatchObject({ showId: "s" });
  });

  it("leaves out what has no file and counts it", async () => {
    playables.albumPlayItems.mockResolvedValue([]);
    const { client } = fakeClient({ PlaylistPlayTracks: [track("t1", 1, 1, null), track("t2", 2, 1)] });
    const { items, skipped, starts } = await playlistPlayItems(client, [{ kind: "track", id: "t1" }, { kind: "track", id: "t2" }, { kind: "album", id: "a" }, { kind: "movie", id: "gone" }]);
    expect(items.map((item) => item.entity.id)).toEqual(["t2"]);
    expect(skipped).toBe(3);
    expect(starts).toEqual([null, 0, null, null]);
  });

  it("starts a lone chapter where it begins inside a larger file", async () => {
    const { client } = fakeClient({ PlaylistPlayChapters: [chapter("c2", 2, "f-book", { startTimeSecs: 600, mediaFile: { id: "f-book", duration: 3600 } }), chapter("c1", 1, "f-own", { startTimeSecs: 0 })] });
    const { items } = await playlistPlayItems(client, [{ kind: "chapter", id: "c2" }, { kind: "chapter", id: "c1" }]);
    expect(items[0]!.startPosition).toBe(600);
    expect(items[1]!.startPosition).toBeUndefined();
  });

  it("asks for each kind once, in batches of at most a hundred ids, and not at all for kinds it has none of", async () => {
    const { client, asked } = fakeClient({});
    const targets = Array.from({ length: 230 }, (_, index) => ({ kind: "track" as const, id: `t${index}` }));
    await playlistPlayItems(client, [...targets, { kind: "track", id: "t0" }]);
    expect(asked.map((call) => call.name)).toEqual(["PlaylistPlayTracks", "PlaylistPlayTracks", "PlaylistPlayTracks"]);
    expect(asked.map((call) => (call.variables.ids as string[]).length)).toEqual([100, 100, 30]);
  });
});

describe("shufflePlayItems", () => {
  it("is a permutation that follows the random source and leaves its input alone", () => {
    const input = [1, 2, 3, 4, 5];
    const out = shufflePlayItems(input, () => 0);
    expect(out).toEqual([2, 3, 4, 5, 1]);
    expect([...out].sort()).toEqual(input);
    expect(input).toEqual([1, 2, 3, 4, 5]);
    expect(shufflePlayItems([], () => 0.5)).toEqual([]);
  });
});

describe("startPlaylistPlayback", () => {
  const audio = (id: string): PlayItem => ({ mediaFileId: `f-${id}`, title: id, entity: { kind: "track", id } });
  const video = (id: string): PlayItem => ({ mediaFileId: `f-${id}`, title: id, entity: { kind: "movie", id } });

  it("makes the whole mixed list the queue, in order, whatever comes first", () => {
    const player = { play: vi.fn() };
    expect(startPlaylistPlayback(player, [audio("a"), video("m"), audio("b")])).toBe(true);
    expect(player.play).toHaveBeenCalledWith([audio("a"), video("m"), audio("b")], 0);
    player.play.mockClear();
    startPlaylistPlayback(player, [video("m"), audio("a")]);
    expect(player.play).toHaveBeenCalledWith([video("m"), audio("a")], 0);
  });

  it("starts at the entry asked for with the rest of the list still in the queue", () => {
    const player = { play: vi.fn() };
    startPlaylistPlayback(player, [audio("a"), video("m"), audio("b")], 1);
    expect(player.play).toHaveBeenCalledWith([audio("a"), video("m"), audio("b")], 1);
  });

  it("does nothing for nothing", () => {
    const player = { play: vi.fn() };
    expect(startPlaylistPlayback(player, [])).toBe(false);
    expect(startPlaylistPlayback(player, [audio("a")], 3)).toBe(false);
    expect(player.play).not.toHaveBeenCalled();
  });
});

describe("expandPlaylist", () => {
  it("reads the playlist's targets in order and plays them as items", async () => {
    members.fetchPlaylistTargets.mockResolvedValue([{ kind: "movie", id: "m1" }, { kind: "episode", id: "e1" }, { kind: "chapter", id: "c1" }]);
    const { client } = fakeClient({ PlaylistPlayMovies: [movie("m1")], PlaylistPlayEpisodes: [episode("e1", 1, 1)], PlaylistPlayChapters: [chapter("c1", 1, "f-c1")] });
    const items = await expandPlaylist(client, "list-1");
    expect(members.fetchPlaylistTargets).toHaveBeenCalledWith(client, "list-1");
    expect(items.map((item) => `${item.entity.kind}:${item.entity.id}`)).toEqual(["movie:m1", "episode:e1", "chapter:c1"]);
  });
});

describe("playItemTargets", () => {
  it("turns queue items into playlist targets in order, once each", () => {
    const item = (kind: PlayItem["entity"]["kind"], id: string) => ({ entity: { kind, id } });
    expect(playItemTargets([item("track", "t1"), item("movie", "m1"), item("track", "t1"), item("chapter", "c1"), item("episode", "e1")])).toEqual([
      { kind: "track", id: "t1" }, { kind: "movie", id: "m1" }, { kind: "chapter", id: "c1" }, { kind: "episode", id: "e1" },
    ]);
  });

  it("keeps an episode and a chapter, which a playlist can hold now", () => {
    expect(playItemTargets([{ entity: { kind: "episode", id: "e1" } }, { entity: { kind: "chapter", id: "c1" } }])).toEqual([{ kind: "episode", id: "e1" }, { kind: "chapter", id: "c1" }]);
  });

  it("saves a book's files, which carry its chapter marks, as the audiobook once", () => {
    const part = (id: string) => ({ entity: { kind: "chapter" as const, id }, audiobookId: "b", chapters: [{ start: 0, title: "One" }] });
    expect(playItemTargets([part("c1"), part("c9"), { entity: { kind: "chapter", id: "c2" }, audiobookId: "b" }])).toEqual([{ kind: "audiobook", id: "b" }, { kind: "chapter", id: "c2" }]);
  });
});
