import type { ApolloClient } from "@apollo/client";
import { getOperationAST } from "graphql";
import { describe, expect, it, vi } from "vitest";

import type { PlayItem } from "@/features/player/store";

import { isVideoPlayItem, playItemTargets, playlistPlayItems, shufflePlayItems, startPlaylistPlayback } from "../playlistPlayback";

type Rows = Record<string, unknown[]>;

/** A client that answers each generated document by its operation name, recording what was asked. */
function fakeClient(answers: Rows) {
  const asked: { name: string; variables: Record<string, unknown> }[] = [];
  const client = {
    query: async ({ query, variables }: { query: Parameters<typeof getOperationAST>[0]; variables: Record<string, unknown> }) => {
      const name = getOperationAST(query)!.name!.value;
      asked.push({ name, variables });
      const field = name.includes("Movies") ? "movies" : name.includes("Episodes") ? "episodes" : name.includes("Tracks") ? "tracks" : "chapters";
      const rows = (answers[name] ?? []).filter((row) => {
        const ids = variables.ids as string[];
        const parent = name === "PlaylistAlbumTracks" ? (row as { albumId: string }).albumId : name === "PlaylistShowEpisodes" ? (row as { showId: string }).showId : name === "PlaylistBookChapters" ? (row as { audiobookId: string }).audiobookId : (row as { id: string }).id;
        return ids.includes(parent);
      });
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
    expect(items[3]).toMatchObject({ title: "Series", subtitle: "S01E01  Ep e1", href: "/shows/s" });
  });

  it("expands an album to its tracks by disc then number, and a show to its episodes by season then number", async () => {
    const { client } = fakeClient({
      PlaylistAlbumTracks: [track("t3", 1, 2), track("t2", 2, 1), track("t1", 1, 1), track("t0", 0, null)],
      PlaylistShowEpisodes: [episode("e3", 2, 1), episode("e2", 1, 2), episode("e1", 1, 1)],
    });
    const { items } = await playlistPlayItems(client, [{ kind: "show", id: "s" }, { kind: "album", id: "a" }]);
    expect(items.map((item) => item.entity.id)).toEqual(["e1", "e2", "e3", "t0", "t1", "t2", "t3"]);
  });

  it("leaves out what has no file and counts it", async () => {
    const { client } = fakeClient({
      PlaylistPlayTracks: [track("t1", 1, 1, null), track("t2", 2, 1)], PlaylistAlbumTracks: [track("t9", 1, 1, null)],
    });
    const { items, skipped } = await playlistPlayItems(client, [{ kind: "track", id: "t1" }, { kind: "track", id: "t2" }, { kind: "album", id: "a" }, { kind: "movie", id: "gone" }]);
    expect(items.map((item) => item.entity.id)).toEqual(["t2"]);
    expect(skipped).toBe(3);
  });

  it("plays an audiobook held in one file as one item with its chapter marks, and a book in parts as one item per part", async () => {
    const whole = fakeClient({ PlaylistBookChapters: [chapter("c1", 1, "f-book", { startTimeSecs: 0 }), chapter("c2", 2, "f-book", { startTimeSecs: 600 }), chapter("c3", 3, "f-book", { startTimeSecs: 1200 })] });
    const one = await playlistPlayItems(whole.client, [{ kind: "audiobook", id: "b" }]);
    expect(one.items).toHaveLength(1);
    expect(one.items[0]).toMatchObject({ mediaFileId: "f-book", title: "Book", audiobookId: "b", chapters: [{ start: 0, title: "Chapter 1" }, { start: 600, title: "Chapter 2" }, { start: 1200, title: "Chapter 3" }] });
    const parts = fakeClient({ PlaylistBookChapters: [chapter("c2", 2, "f-2"), chapter("c1", 1, "f-1")] });
    const many = await playlistPlayItems(parts.client, [{ kind: "audiobook", id: "b" }]);
    expect(many.items.map((item) => item.mediaFileId)).toEqual(["f-1", "f-2"]);
    expect(many.items[0]!.chapters).toBeUndefined();
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
  const player = () => ({ playAudio: vi.fn(), playVideo: vi.fn(), enqueue: vi.fn() });

  it("hands audio to the queue in order", () => {
    const p = player();
    const go = vi.fn();
    expect(startPlaylistPlayback(p, go, [audio("a"), video("m"), audio("b")])).toBe(true);
    expect(p.playAudio).toHaveBeenCalledWith([audio("a"), video("m"), audio("b")]);
    expect(p.playVideo).not.toHaveBeenCalled();
    expect(go).not.toHaveBeenCalled();
  });

  it("opens the video player for a video first and queues the rest behind it", () => {
    const p = player();
    const go = vi.fn();
    startPlaylistPlayback(p, go, [video("m"), audio("a")]);
    expect(p.playVideo).toHaveBeenCalledWith(video("m"));
    expect(go).toHaveBeenCalledWith("f-m");
    expect(p.enqueue).toHaveBeenCalledWith([audio("a")]);
  });

  it("does nothing for nothing", () => {
    const p = player();
    expect(startPlaylistPlayback(p, vi.fn(), [])).toBe(false);
    expect(p.playAudio).not.toHaveBeenCalled();
  });
});

describe("playItemTargets", () => {
  it("turns queue items into playlist targets in order, once each", () => {
    const item = (kind: PlayItem["entity"]["kind"], id: string) => ({ entity: { kind, id } });
    expect(playItemTargets([item("track", "t1"), item("movie", "m1"), item("track", "t1"), item("chapter", "c1"), item("episode", "e1")])).toEqual([
      { kind: "track", id: "t1" }, { kind: "movie", id: "m1" }, { kind: "chapter", id: "c1" }, { kind: "episode", id: "e1" },
    ]);
  });
});
