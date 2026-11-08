/**
 * Playing a playlist. `playlistPlayItems` turns an ordered list of playlist targets into the
 * player's items, in order:
 *
 * - tracks and chapters become audio items; movies and episodes become video items;
 * - an album expands to its tracks (disc, then track number), a show to its episodes (season,
 *   then episode), an audiobook to its files (a book held in one file is one item that carries its
 *   chapter marks, a book in parts is one item per part);
 * - anything without a file is left out and counted in `skipped`.
 *
 * It only reads. Starting playback is `startPlaylistPlayback`, which uses the player's public API
 * (`playAudio`, `playVideo`, `enqueue`) and nothing else, so the queue can change underneath it.
 * The queue's own "Add album/playlist" actions reuse `playlistPlayItems` and `shufflePlayItems`.
 */
import type { ApolloClient, TypedDocumentNode } from "@apollo/client";

import type { PlayItem } from "@/features/player/store";
import {
  PlaylistAlbumTracksDocument, PlaylistBookChaptersDocument, PlaylistPlayChaptersDocument, PlaylistPlayEpisodesDocument,
  PlaylistPlayMoviesDocument, PlaylistPlayTracksDocument, PlaylistShowEpisodesDocument,
  type PlaylistAlbumTracksQuery, type PlaylistBookChaptersQuery, type PlaylistPlayChaptersQuery, type PlaylistPlayEpisodesQuery,
  type PlaylistPlayMoviesQuery, type PlaylistPlayTracksQuery, type PlaylistShowEpisodesQuery,
} from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, movieBackdrop, moviePoster, showBackdrop, showPoster } from "@/lib/artwork";

import { episodeCode, type PlaylistTarget } from "./types";

type MovieNode = PlaylistPlayMoviesQuery["movies"]["edges"][number]["node"];
type EpisodeNode = PlaylistPlayEpisodesQuery["episodes"]["edges"][number]["node"];
type TrackNode = PlaylistPlayTracksQuery["tracks"]["edges"][number]["node"];
type ChapterNode = PlaylistPlayChaptersQuery["chapters"]["edges"][number]["node"];

export interface PlaylistPlayItems {
  /** Playable items, in play order. */
  items: PlayItem[];
  /** Targets that produced nothing, because they have no file yet. */
  skipped: number;
}

export const isVideoPlayItem = (item: Pick<PlayItem, "entity">) => item.entity.kind === "movie" || item.entity.kind === "episode";

export const movieItem = (movie: MovieNode): PlayItem | null => movie.mediaFileId ? {
  mediaFileId: movie.mediaFileId, title: movie.title, subtitle: movie.year ? String(movie.year) : undefined,
  artwork: moviePoster(movie.id), backdrop: movieBackdrop(movie.id), entity: { kind: "movie", id: movie.id }, href: `/movies/${movie.id}`,
  duration: movie.runtime ? movie.runtime * 60 : undefined,
} : null;

export const episodeItem = (episode: EpisodeNode): PlayItem | null => episode.mediaFileId ? {
  mediaFileId: episode.mediaFileId, title: episode.show?.name ?? "Episode", subtitle: [episodeCode(episode), episode.title].filter(Boolean).join("  "),
  artwork: showPoster({ id: episode.showId }), backdrop: showBackdrop({ id: episode.showId }), entity: { kind: "episode", id: episode.id }, href: `/shows/${episode.showId}`,
  duration: episode.runtime ? episode.runtime * 60 : undefined,
} : null;

export const trackItem = (track: TrackNode): PlayItem | null => track.mediaFileId ? {
  mediaFileId: track.mediaFileId, title: track.title, subtitle: [track.artistName, track.album?.name].filter(Boolean).join(" · ") || undefined,
  artwork: albumCover({ id: track.albumId }), entity: { kind: "track", id: track.id }, href: `/albums/${track.albumId}`, duration: track.durationSecs ?? undefined,
} : null;

/** One chapter on its own. A chapter inside a file that holds more than itself starts where it does. */
export function chapterItem(chapter: ChapterNode): PlayItem | null {
  if (!chapter.mediaFileId) return null;
  const fileSecs = chapter.mediaFile?.duration ?? 0;
  const inLargerFile = fileSecs > (chapter.durationSecs ?? 0) + 5;
  const offset = chapter.fileOffsetSecs ?? (inLargerFile && chapter.startTimeSecs > 0 ? chapter.startTimeSecs : undefined);
  return {
    mediaFileId: chapter.mediaFileId, title: chapter.audiobook?.title ?? "Audiobook", subtitle: chapter.title ?? `Chapter ${chapter.chapterNumber}`,
    artwork: audiobookCover({ id: chapter.audiobookId }), entity: { kind: "chapter", id: chapter.id }, audiobookId: chapter.audiobookId,
    href: `/audiobooks/${chapter.audiobookId}`, duration: chapter.durationSecs ?? undefined, startPosition: offset || undefined,
  };
}

/** A whole book: one item for each file its chapters live in, with the chapter marks inside it. */
export function bookItems(chapters: readonly ChapterNode[]): PlayItem[] {
  const ordered = [...chapters].sort((a, b) => a.chapterNumber - b.chapterNumber);
  const files = [...new Set(ordered.flatMap((chapter) => chapter.mediaFileId ? [chapter.mediaFileId] : []))];
  return files.flatMap((file) => {
    const here = ordered.filter((chapter) => chapter.mediaFileId === file);
    const first = here[0]!;
    const shared = here.length > 1;
    const name = (chapter: ChapterNode) => chapter.title ?? `Chapter ${chapter.chapterNumber}`;
    return [{
      mediaFileId: file, title: first.audiobook?.title ?? "Audiobook", subtitle: shared ? first.audiobook?.authorName ?? undefined : name(first),
      artwork: audiobookCover({ id: first.audiobookId }), entity: { kind: "chapter", id: first.id }, audiobookId: first.audiobookId, href: `/audiobooks/${first.audiobookId}`,
      duration: shared ? first.mediaFile?.duration ?? undefined : first.durationSecs ?? undefined,
      chapters: shared ? here.map((chapter) => ({ start: chapter.fileOffsetSecs ?? chapter.startTimeSecs, title: name(chapter) })) : undefined,
    } satisfies PlayItem];
  });
}

const CHUNK = 100;
const chunks = <T,>(values: readonly T[]) => Array.from({ length: Math.ceil(values.length / CHUNK) }, (_, index) => values.slice(index * CHUNK, (index + 1) * CHUNK));

/** Drains a children query: every page for a set of parents, off-cache so a playlist never plays stale. */
async function children<TData, TNode>(client: ApolloClient, document: TypedDocumentNode<TData, { ids: string[]; offset: number }>, ids: readonly string[], read: (data: TData) => { nodes: TNode[]; more: boolean }): Promise<TNode[]> {
  const out: TNode[] = [];
  for (const part of chunks(ids)) {
    for (let offset = 0; ; offset += CHUNK) {
      const { data } = await client.query({ query: document, variables: { ids: part, offset }, fetchPolicy: "network-only" });
      const page = read(data as TData);
      out.push(...page.nodes);
      if (!page.more) break;
    }
  }
  return out;
}

async function byIds<TData, TNode extends { id: string }>(client: ApolloClient, document: TypedDocumentNode<TData, { ids: string[] }>, ids: readonly string[], nodes: (data: TData) => TNode[]): Promise<Map<string, TNode>> {
  const found = new Map<string, TNode>();
  for (const part of chunks([...new Set(ids)])) {
    const { data } = await client.query({ query: document, variables: { ids: part }, fetchPolicy: "network-only" });
    for (const node of nodes(data as TData)) found.set(node.id, node);
  }
  return found;
}

const idsOf = (targets: readonly PlaylistTarget[], kind: PlaylistTarget["kind"]) => targets.filter((target) => target.kind === kind).map((target) => target.id);
const trackOrder = (a: TrackNode, b: TrackNode) => (a.discNumber ?? 1) - (b.discNumber ?? 1) || a.trackNumber - b.trackNumber;
const episodeOrder = (a: EpisodeNode, b: EpisodeNode) => a.season - b.season || a.episode - b.episode;

/** The player's items for an ordered list of targets. See the module comment for the rules. */
export async function playlistPlayItems(client: ApolloClient, targets: readonly PlaylistTarget[]): Promise<PlaylistPlayItems> {
  const [movies, episodes, tracks, chapters, albumTracks, showEpisodes, bookChapters] = await Promise.all([
    byIds(client, PlaylistPlayMoviesDocument, idsOf(targets, "movie"), (data) => data.movies.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayEpisodesDocument, idsOf(targets, "episode"), (data) => data.episodes.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayTracksDocument, idsOf(targets, "track"), (data) => data.tracks.edges.map((edge) => edge.node)),
    byIds(client, PlaylistPlayChaptersDocument, idsOf(targets, "chapter"), (data) => data.chapters.edges.map((edge) => edge.node)),
    children(client, PlaylistAlbumTracksDocument, [...new Set(idsOf(targets, "album"))], (data: PlaylistAlbumTracksQuery) => ({ nodes: data.tracks.edges.map((edge) => edge.node), more: data.tracks.pageInfo.hasNextPage })),
    children(client, PlaylistShowEpisodesDocument, [...new Set(idsOf(targets, "show"))], (data: PlaylistShowEpisodesQuery) => ({ nodes: data.episodes.edges.map((edge) => edge.node), more: data.episodes.pageInfo.hasNextPage })),
    children(client, PlaylistBookChaptersDocument, [...new Set(idsOf(targets, "audiobook"))], (data: PlaylistBookChaptersQuery) => ({ nodes: data.chapters.edges.map((edge) => edge.node), more: data.chapters.pageInfo.hasNextPage })),
  ]);
  const items: PlayItem[] = [];
  let skipped = 0;
  for (const target of targets) {
    const found: PlayItem[] = (() => {
      switch (target.kind) {
        case "movie": return [movies.get(target.id)].flatMap((movie) => movie ? [movieItem(movie)] : []);
        case "episode": return [episodes.get(target.id)].flatMap((episode) => episode ? [episodeItem(episode)] : []);
        case "track": return [tracks.get(target.id)].flatMap((track) => track ? [trackItem(track)] : []);
        case "chapter": return [chapters.get(target.id)].flatMap((chapter) => chapter ? [chapterItem(chapter)] : []);
        case "album": return albumTracks.filter((track) => track.albumId === target.id).sort(trackOrder).map(trackItem);
        case "show": return showEpisodes.filter((episode) => episode.showId === target.id).sort(episodeOrder).map(episodeItem);
        case "audiobook": return bookItems(bookChapters.filter((chapter) => chapter.audiobookId === target.id));
      }
    })().filter((item): item is PlayItem => item !== null);
    if (found.length) items.push(...found);
    else skipped += 1;
  }
  return { items, skipped };
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

/** The slice of the player this needs: the same methods `usePlayer()` returns. */
export interface PlaylistPlayer {
  playAudio: (items: PlayItem[], index?: number) => void;
  playVideo: (item: PlayItem) => void;
  enqueue: (items: PlayItem[]) => void;
}

/**
 * Puts the items in the queue in order and starts the first. Audio first: the queue plays it.
 * Video first: the video player takes it (`navigate` opens /watch) and the rest wait in the queue.
 */
export function startPlaylistPlayback(player: PlaylistPlayer, navigate: (mediaFileId: string) => void, items: readonly PlayItem[]): boolean {
  const [first, ...rest] = items;
  if (!first) return false;
  if (isVideoPlayItem(first)) {
    player.playVideo(first);
    navigate(first.mediaFileId);
    if (rest.length) player.enqueue(rest);
  } else player.playAudio([...items]);
  return true;
}

