import type { ApolloClient } from "@apollo/client";
import { describe, expect, it, vi } from "vitest";

import { after, continueAfter } from "../continuation";
import type { PlayItem } from "../store";

vi.mock("@/lib/auth/session", () => ({ session: { onChange: () => () => {} } }));

type Variables = Record<string, unknown>;
type Handlers = Record<string, (variables: Variables) => unknown>;

/** An Apollo client whose queries are answered by operation name. */
function fakeClient(handlers: Handlers) {
  const calls: Array<{ name: string; variables: Variables }> = [];
  const query = vi.fn(async ({ query: document, variables = {} }: { query: { definitions: Array<{ name?: { value: string } }> }; variables?: Variables }) => {
    const name = document.definitions[0]!.name!.value;
    calls.push({ name, variables });
    const handler = handlers[name];
    if (!handler) throw new Error(`Unexpected query ${name}`);
    return { data: handler(variables) };
  });
  return { client: { query } as unknown as ApolloClient, calls };
}

const connection = <T>(nodes: T[], hasNextPage = false) => ({ edges: nodes.map((node) => ({ node })), pageInfo: { hasNextPage, totalCount: nodes.length } });
const trackRow = (id: string, albumId: string, number: number, file: string | null = `file-${id}`) => ({ id, albumId, title: id, trackNumber: number, discNumber: 1, mediaFileId: file, artistName: "Artist", durationSecs: 100 });
const albumRow = (id: string, year: number | null, hasFiles = true) => ({ id, name: id, year, releaseDate: null, artistId: "artist-1", hasFiles });
const item = (kind: PlayItem["entity"]["kind"], id: string, extra: Partial<PlayItem> = {}): PlayItem => ({ mediaFileId: `file-${id}`, title: id, entity: { kind, id }, ...extra });

describe("after", () => {
  const items = [item("track", "a"), item("track", "b"), item("track", "c")];
  it("is what comes after the entry", () => expect(after(items, items[0]!).map((entry) => entry.title)).toEqual(["b", "c"]));
  it("is nothing after the last entry", () => expect(after(items, items[2]!)).toEqual([]));
  it("is nothing, rather than everything, when the entry is no longer there", () => expect(after(items, item("track", "gone"))).toEqual([]));
  it("does not match the same media under another file", () => expect(after(items, { ...items[0]!, mediaFileId: "replaced" })).toEqual([]));
});

describe("what plays after a track", () => {
  const albums = (extra: Handlers = {}) => ({
    AlbumDetail: ({ id }: Variables) => ({ album: { id, name: id, tracks: connection(id === "album-1" ? [trackRow("t1", "album-1", 1), trackRow("t2", "album-1", 2), trackRow("t3", "album-1", 3)] : [trackRow("n1", id as string, 1), trackRow("n2", id as string, 2)]) } }),
    EntityAlbumGet: ({ id }: Variables) => ({ album: { id, artistId: "artist-1" } }),
    ArtistDetail: () => ({ artist: { albums: connection([albumRow("album-3", 2001), albumRow("album-1", 1995), albumRow("album-2", 1999), albumRow("album-0", null)]) } }),
    ...extra,
  });
  const last = (id: string) => item("track", id, { albumId: "album-1" });

  it("carries on through the rest of its album, in order", async () => {
    const { client } = fakeClient(albums());
    const found = await continueAfter(client, last("t1"));
    expect(found.map((entry) => entry.title)).toEqual(["t2", "t3"]);
    expect(found.every((entry) => entry.albumId === "album-1")).toBe(true);
  });

  it("goes on to the artist's next album, by year, when the album is done", async () => {
    const { client } = fakeClient(albums());
    const found = await continueAfter(client, last("t3"));
    expect(found.map((entry) => entry.title)).toEqual(["n1", "n2"]);
    expect(found[0]!.albumId).toBe("album-2");
  });

  it("skips later albums with nothing to play", async () => {
    const { client } = fakeClient(albums({ ArtistDetail: () => ({ artist: { albums: connection([albumRow("album-1", 1995), albumRow("album-2", 1999, false), albumRow("album-3", 2001)]) } }) }));
    const found = await continueAfter(client, last("t3"));
    expect(found[0]!.albumId).toBe("album-3");
  });

  it("stops after the artist's last album", async () => {
    const { client } = fakeClient(albums({ ArtistDetail: () => ({ artist: { albums: connection([albumRow("album-1", 1995)]) } }) }));
    expect(await continueAfter(client, last("t3"))).toEqual([]);
  });

  it("does not guess when the track has no album", async () => {
    const { client, calls } = fakeClient({});
    expect(await continueAfter(client, item("track", "t1"))).toEqual([]);
    expect(calls).toEqual([]);
  });
});

describe("what plays after an episode", () => {
  const episodes = Array.from({ length: 103 }, (_, index) => ({ id: `episode-${index + 1}`, showId: "show-a", season: Math.ceil((index + 1) / 20), episode: index + 1, title: `Episode ${index + 1}`, mediaFileId: `file-${index + 1}` }));
  const showHandlers = (rows = episodes): Handlers => ({
    ShowDetail: () => ({ show: { id: "show-a", name: "Example Show" } }),
    ShowEpisodes: () => ({ episodes: connection(rows.slice(0, 100), rows.length > 100) }),
    ShowEpisodesPage: ({ offset }: Variables) => ({ episodes: connection(rows.slice(offset as number)) }),
  });
  const episode = (number: number) => item("episode", `episode-${number}`, { mediaFileId: `file-${number}`, showId: "show-a" });

  it("carries on beyond the first hundred, through every later season", async () => {
    const { client, calls } = fakeClient(showHandlers());
    const found = await continueAfter(client, episode(101));
    expect(found.map((entry) => entry.entity.id)).toEqual(["episode-102", "episode-103"]);
    expect(calls.map((call) => call.name)).toContain("ShowEpisodesPage");
  });

  it("skips later episodes that have no file", async () => {
    const { client } = fakeClient(showHandlers(episodes.map((row) => (row.id === "episode-102" ? { ...row, mediaFileId: null as unknown as string } : row))));
    expect((await continueAfter(client, episode(101))).map((entry) => entry.entity.id)).toEqual(["episode-103"]);
  });

  it("puts specials after the regular seasons", async () => {
    const rows = [{ ...episodes[0]!, id: "special", season: 0, episode: 1, mediaFileId: "file-special" }, episodes[0]!, episodes[1]!, episodes[2]!];
    const { client } = fakeClient(showHandlers(rows));
    expect((await continueAfter(client, episode(1))).map((entry) => entry.entity.id)).toEqual(["episode-2", "episode-3", "special"]);
  });

  it("stops after the last episode, without wrapping to the first", async () => {
    const { client } = fakeClient(showHandlers());
    expect(await continueAfter(client, episode(103))).toEqual([]);
  });

  it("stops when the episode is no longer in the show", async () => {
    const { client } = fakeClient(showHandlers());
    expect(await continueAfter(client, episode(999))).toEqual([]);
  });

  it("finds the show from the episode when the entry does not say", async () => {
    const { client } = fakeClient({ ...showHandlers(), EpisodeDetail: () => ({ episode: { showId: "show-a" } }) });
    const found = await continueAfter(client, { ...episode(102), showId: undefined });
    expect(found.map((entry) => entry.entity.id)).toEqual(["episode-103"]);
  });
});

describe("what plays after a chapter and a movie", () => {
  it("carries on through the rest of the book", async () => {
    const chapters = [1, 2, 3].map((number) => ({ id: `c${number}`, chapterNumber: number, title: `Chapter ${number}`, startTimeSecs: 0, mediaFileId: `file-c${number}`, durationSecs: 60, mediaFile: null }));
    const { client } = fakeClient({ AudiobookDetail: () => ({ audiobook: { id: "book", title: "Book", authorName: null, fileShape: null, chapters: connection(chapters) } }) });
    const found = await continueAfter(client, item("chapter", "c1", { audiobookId: "book", mediaFileId: "file-c1" }));
    expect(found.map((entry) => entry.entity.id)).toEqual(["c2", "c3"]);
  });

  it("has nothing after a book that plays as one file", async () => {
    const chapters = [1, 2].map((number) => ({ id: `c${number}`, chapterNumber: number, title: null, startTimeSecs: number * 10, mediaFileId: "whole", durationSecs: null, mediaFile: { duration: 100 } }));
    const { client } = fakeClient({ AudiobookDetail: () => ({ audiobook: { id: "book", title: "Book", authorName: null, fileShape: "whole_book", chapters: connection(chapters) } }) });
    expect(await continueAfter(client, item("chapter", "c1", { audiobookId: "book", mediaFileId: "whole" }))).toEqual([]);
  });

  it("has nothing after a movie", async () => {
    const { client, calls } = fakeClient({});
    expect(await continueAfter(client, item("movie", "film"))).toEqual([]);
    expect(calls).toEqual([]);
  });
});
