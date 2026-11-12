/**
 * Playing a playlist. `playlistPlayItems` turns an ordered list of playlist targets into the
 * player's items, in order:
 *
 * - tracks and chapters become audio items; movies and episodes become video items;
 * - an album expands to its tracks (disc, then track number), a show to its episodes (season,
 *   then episode, specials last), an audiobook to its files (a book held in one file is one item
 *   that carries its chapter marks, a book in parts is one item per part). Containers expand
 *   through the play queue's own helpers (`player/playables.ts`), so "Add album to queue" and "a
 *   playlist holding that album" always agree;
 * - anything without a file is left out and counted in `skipped`.
 *
 * It only reads. Starting playback is `startPlaylistPlayback`: the items become the play queue
 * (`playQueue`), audio and video in whatever mix the playlist has, and the queue director moves
 * between the audio dock and the video player as it plays. The queue's "Play next" / "Add to
 * queue" resolve playlists through `expandPlaylist` and lists of targets through `playlistPlayItems`.
 */
import type { ApolloClient, TypedDocumentNode } from "@apollo/client";

import { albumPlayItems, audiobookItems, showPlayItems } from "@/features/player/playables";
import { episodePlayItem, moviePlayItem, trackPlayItem } from "@/features/player/playItems";
import { isVideoItem, type PlayItem } from "@/features/player/store";
import {
  PlaylistPlayChaptersDocument, PlaylistPlayEpisodesDocument, PlaylistPlayMoviesDocument, PlaylistPlayTracksDocument,
  type PlaylistPlayChaptersQuery, type PlaylistPlayEpisodesQuery, type PlaylistPlayMoviesQuery, type PlaylistPlayTracksQuery,
} from "@/graphql/generated/graphql";
import { audiobookCover } from "@/lib/artwork";

import { targetKey, type PlaylistTarget } from "./types";
import { fetchPlaylistTargets } from "./usePlaylists";

type MovieNode = PlaylistPlayMoviesQuery["movies"]["edges"][number]["node"];
type EpisodeNode = PlaylistPlayEpisodesQuery["episodes"]["edges"][number]["node"];
type TrackNode = PlaylistPlayTracksQuery["tracks"]["edges"][number]["node"];
type ChapterNode = PlaylistPlayChaptersQuery["chapters"]["edges"][number]["node"];

export interface PlaylistPlayItems {
  /** Playable items, in play order. */
  items: PlayItem[];
  /** Targets that produced nothing, because they have no file yet. */
  skipped: number;
  /** Where each target's first item sits in `items`, or null for a target that produced nothing. */
  starts: (number | null)[];
}

export const isVideoPlayItem = isVideoItem;

export const movieItem = (movie: MovieNode): PlayItem | null => {
  const item = moviePlayItem(movie);
  return item && { ...item, duration: movie.runtime ? movie.runtime * 60 : undefined };
};

export const episodeItem = (episode: EpisodeNode): PlayItem | null => {
  const item = episodePlayItem(episode, { id: episode.showId, name: episode.show?.name ?? "Episode" });
  return item && { ...item, duration: episode.runtime ? episode.runtime * 60 : undefined };
};

export const trackItem = (track: TrackNode): PlayItem | null => trackPlayItem(track);

/**
 * Where a lone chapter stops in its file: its start plus its length, when the file holds more than
 * the chapter. A chapter that runs to the end of the file (or into the next part) needs no bound.
 */
export function chapterEnd(chapter: Pick<ChapterNode, "durationSecs">, offset: number | undefined, fileSecs: number, inLargerFile: boolean): number | undefined {
  const length = chapter.durationSecs ?? 0;
  if (length <= 0 || (!inLargerFile && !offset)) return undefined;
  const end = (offset ?? 0) + length;
  return fileSecs > 0 && end >= fileSecs - 1 ? undefined : end;
}

/** One chapter on its own. A chapter inside a file that holds more than itself starts where it does. */
export function chapterItem(chapter: ChapterNode): PlayItem | null {
  if (!chapter.mediaFileId) return null;
  const fileSecs = chapter.mediaFile?.duration ?? 0;
  const inLargerFile = fileSecs > (chapter.durationSecs ?? 0) + 5;
  const offset = chapter.fileOffsetSecs ?? (inLargerFile && chapter.startTimeSecs > 0 ? chapter.startTimeSecs : undefined);
  const end = chapterEnd(chapter, offset, fileSecs, inLargerFile);
  return {
    mediaFileId: chapter.mediaFileId, title: chapter.audiobook?.title ?? "Audiobook", subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`,
    artwork: audiobookCover({ id: chapter.audiobookId }), entity: { kind: "chapter", id: chapter.id }, audiobookId: chapter.audiobookId,
    href: `/audiobooks/${chapter.audiobookId}`, duration: chapter.durationSecs ?? undefined, startPosition: offset || undefined, endPosition: end,
  };
}

const CHUNK = 100;
const chunks = <T,>(values: readonly T[]) => Array.from({ length: Math.ceil(values.length / CHUNK) }, (_, index) => values.slice(index * CHUNK, (index + 1) * CHUNK));

async function byIds<TData, TNode extends { id: string }>(client: ApolloClient, document: TypedDocumentNode<TData, { ids: string[] }>, ids: readonly string[], nodes: (data: TData) => TNode[]): Promise<Map<string, TNode>> {
  const found = new Map<string, TNode>();
  for (const part of chunks([...new Set(ids)])) {
    const { data } = await client.query({ query: document, variables: { ids: part }, fetchPolicy: "network-only" });
    for (const node of nodes(data as TData)) found.set(node.id, node);
  }
  return found;
}

const idsOf = (targets: readonly PlaylistTarget[], kind: PlaylistTarget["kind"]) => targets.filter((target) => target.kind === kind).map((target) => target.id);

/** Each distinct container expanded once, by the queue's own helper for it. */
async function expanded(client: ApolloClient, targets: readonly PlaylistTarget[], kind: "album" | "show" | "audiobook", expand: (client: ApolloClient, id: string) => Promise<PlayItem[]>): Promise<Map<string, PlayItem[]>> {
  const ids = [...new Set(idsOf(targets, kind))];
  return new Map(await Promise.all(ids.map(async (id) => [id, await expand(client, id)] as const)));
}

/** The player's items for an ordered list of targets. See the module comment for the rules. */
export async function playlistPlayItems(client: ApolloClient, targets: readonly PlaylistTarget[]): Promise<PlaylistPlayItems> {
  const [movies, episodes, tracks, chapters, albums, shows, books] = await Promise.all([
    byIds(client, PlaylistPlayMoviesDocument, idsOf(targets, "movie"), (data) => data.movies.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayEpisodesDocument, idsOf(targets, "episode"), (data) => data.episodes.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayTracksDocument, idsOf(targets, "track"), (data) => data.tracks.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayChaptersDocument, idsOf(targets, "chapter"), (data) => data.chapters.edges.map((edge) => edge.node)),
    expanded(client, targets, "album", albumPlayItems),
    expanded(client, targets, "show", showPlayItems),
    expanded(client, targets, "audiobook", audiobookItems),
  ]);
  const items: PlayItem[] = [];
  const starts: (number | null)[] = [];
  let skipped = 0;
  for (const target of targets) {
    const found: PlayItem[] = (() => {
      switch (target.kind) {
        case "movie": return [movies.get(target.id)].flatMap((movie) => movie ? [movieItem(movie)] : []);
        case "episode": return [episodes.get(target.id)].flatMap((episode) => episode ? [episodeItem(episode)] : []);
        case "track": return [tracks.get(target.id)].flatMap((track) => track ? [trackItem(track)] : []);
        case "chapter": return [chapters.get(target.id)].flatMap((chapter) => chapter ? [chapterItem(chapter)] : []);
        case "album": return albums.get(target.id) ?? [];
        case "show": return shows.get(target.id) ?? [];
        case "audiobook": return books.get(target.id) ?? [];
      }
    })().filter((item): item is PlayItem => item !== null);
    starts.push(found.length ? items.length : null);
    if (found.length) items.push(...found);
    else skipped += 1;
  }
  return { items, skipped, starts };
}

/** What a playlist plays as: its entries in order, expanded as `playlistPlayItems` says. Read off-cache. */
export async function expandPlaylist(client: ApolloClient, listId: string): Promise<PlayItem[]> {
  return (await playlistPlayItems(client, await fetchPlaylistTargets(client, listId))).items;
}

/** A fair shuffle (Fisher–Yates). `random` is injectable for tests. */
export function shufflePlayItems<T>(items: readonly T[], random: () => number = Math.random): T[] {
  const next = [...items];
  for (let index = next.length - 1; index > 0; index -= 1) {
    const other = Math.floor(random() * (index + 1));
    [next[index], next[other]] = [next[other]!, next[index]!];
  }
  return next;
}

/** The slice of the player this needs: `usePlayer().play`, which replaces the queue and starts at an entry. */
export interface PlaylistPlayer {
  play: (items: PlayItem[], index?: number) => void;
}

/**
 * Makes the items the play queue, in order, and starts at `index`. Audio and video may be mixed:
 * the queue director opens the video player when the playing entry is video and returns to the dock
 * when it is audio, so nothing waits behind the first item.
 */
export function startPlaylistPlayback(player: PlaylistPlayer, items: readonly PlayItem[], index = 0): boolean {
  if (!items[index]) return false;
  player.play([...items], index);
  return true;
}

/**
 * The targets a list of player items stand for, in order, for "Save queue as playlist": a track is
 * a track, a movie a movie, an episode an episode and a chapter a chapter, except that a file
 * carrying a book's chapter marks (the whole book in one file, or one part of a book in parts) is
 * the audiobook, so its parts save as one entry that plays the whole book. A playlist holds each
 * target once, so a repeat is dropped.
 */
export function playItemTargets(items: readonly Pick<PlayItem, "entity" | "audiobookId" | "chapters">[]): PlaylistTarget[] {
  const seen = new Set<string>();
  return items.flatMap((item) => {
    const target: PlaylistTarget = item.entity.kind === "chapter" && item.audiobookId && item.chapters
      ? { kind: "audiobook", id: item.audiobookId }
      : { kind: item.entity.kind, id: item.entity.id };
    const key = targetKey(target);
    if (seen.has(key)) return [];
    seen.add(key);
    return [target];
  });
}
