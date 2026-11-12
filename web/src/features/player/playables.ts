import type { ApolloClient } from "@apollo/client";

import {
  AlbumDetailDocument,
  AlbumTracksPageDocument,
  AudiobookChaptersPageDocument,
  AudiobookDetailDocument,
  AudiobookPartsDocument,
  ShowDetailDocument,
  ShowEpisodesDocument,
  ShowEpisodesPageDocument,
} from "@/graphql/generated/graphql";

import { audiobookPlayItems, episodePlayItem, isPartsBook, trackPlayItem } from "./playItems";
import type { PlayItem } from "./store";

interface Connection<T> {
  edges: Array<{ node: T }>;
  pageInfo: { hasNextPage: boolean };
}

/** Every page of a connection, a page at a time, refusing to loop if the server's paging stalls. */
async function collect<T>(first: Connection<T>, more: (offset: number) => Promise<Connection<T>>): Promise<T[]> {
  const rows = first.edges.map((edge) => edge.node);
  let page = first;
  while (page.pageInfo.hasNextPage) {
    page = await more(rows.length);
    if (!page.edges.length) break;
    rows.push(...page.edges.map((edge) => edge.node));
  }
  return rows;
}

const read = { fetchPolicy: "no-cache", context: { queryDeduplication: false } } as const;

export async function albumPlayItems(client: ApolloClient, albumId: string): Promise<PlayItem[]> {
  const { data } = await client.query({ query: AlbumDetailDocument, variables: { id: albumId }, ...read });
  const album = data?.album;
  if (!album) throw new Error("Album not found");
  const tracks = await collect(album.tracks, async (offset) => {
    const page = await client.query({ query: AlbumTracksPageDocument, variables: { id: albumId, offset }, ...read });
    if (!page.data?.album) throw new Error("Album tracks unavailable");
    return page.data.album.tracks;
  });
  return tracks
    .sort((a, b) => (a.discNumber ?? 1) - (b.discNumber ?? 1) || a.trackNumber - b.trackNumber)
    .map((track) => trackPlayItem({ ...track, album }))
    .filter((item): item is PlayItem => item !== null);
}

export async function audiobookItems(client: ApolloClient, audiobookId: string): Promise<PlayItem[]> {
  const { data } = await client.query({ query: AudiobookDetailDocument, variables: { id: audiobookId }, ...read });
  const book = data?.audiobook;
  if (!book) throw new Error("Audiobook not found");
  const chapters = (await collect(book.chapters, async (offset) => {
    const page = await client.query({ query: AudiobookChaptersPageDocument, variables: { id: audiobookId, offset }, ...read });
    if (!page.data?.audiobook) throw new Error("Audiobook chapters unavailable");
    return page.data.audiobook.chapters;
  })).sort((a, b) => a.chapterNumber - b.chapterNumber);
  let parts: Array<{ id: string; chapterId?: string | null; duration?: number | null }> = [];
  if (isPartsBook(book, chapters) && chapters.length) {
    const found = await client.query({ query: AudiobookPartsDocument, variables: { chapterIds: chapters.map((chapter) => chapter.id) }, ...read });
    parts = (found.data?.mediaFiles.edges ?? []).map((edge) => edge.node).sort((a, b) => a.path.localeCompare(b.path, undefined, { numeric: true }));
  }
  return audiobookPlayItems(book, chapters, parts);
}

export async function showPlayItems(client: ApolloClient, showId: string): Promise<PlayItem[]> {
  const detail = await client.query({ query: ShowDetailDocument, variables: { id: showId }, ...read });
  const show = detail.data?.show;
  if (!show) throw new Error("Show not found");
  const first = await client.query({ query: ShowEpisodesDocument, variables: { showId }, ...read });
  if (!first.data) throw new Error("Episodes unavailable");
  const episodes = await collect(first.data.episodes, async (offset) => {
    const page = await client.query({ query: ShowEpisodesPageDocument, variables: { showId, offset }, ...read });
    if (!page.data) throw new Error("Episodes unavailable");
    return page.data.episodes;
  });
  // Seasons in order, with specials (season 0) after the regular seasons rather than before them.
  const season = (value: number) => (value === 0 ? Number.MAX_SAFE_INTEGER : value);
  return episodes
    .sort((a, b) => season(a.season) - season(b.season) || a.episode - b.episode)
    .map((episode) => episodePlayItem(episode, show))
    .filter((item): item is PlayItem => item !== null);
}
