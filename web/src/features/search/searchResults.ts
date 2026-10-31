import type { LinkProps } from "@tanstack/react-router";

import type { LibrarySearchQuery, SearchCreditsQuery, SearchStartQuery } from "@/graphql/generated/graphql";
import { albumCover, artistImage, audiobookCover, episodeThumb, movieBackdrop, showBackdrop } from "@/lib/artwork";
import { LIBRARY_TYPES } from "@/lib/library-types";

export type SearchHitKind = "movie" | "show" | "episode" | "artist" | "album" | "audiobook";

/** One result tile: every kind is shown the same way, in one grid, best match first. */
export interface SearchHit {
  key: string;
  kind: SearchHitKind;
  title: string;
  meta?: string;
  image: string;
  /** Shown when `image` is missing (an episode without a still shows its show). */
  fallbackImage?: string;
  /** Wide artwork fills the tile; square covers sit on a blurred copy of themselves. */
  art: "wide" | "square";
  tint: string;
  to: LinkProps["to"];
  params: LinkProps["params"];
  /** Why a result that does not contain the query in its title matched (a cast member, a director). */
  via?: string;
}

const KIND_ORDER: Record<SearchHitKind, number> = { movie: 0, show: 1, artist: 2, album: 3, audiobook: 4, episode: 5 };
export const KIND_LABEL: Record<SearchHitKind, string> = { movie: "Movie", show: "Show", episode: "Episode", artist: "Artist", album: "Album", audiobook: "Audiobook" };

/** Case- and accent-insensitive form for matching ("Amélie" matches "ame"). */
export function fold(text: string): string {
  return text.normalize("NFD").replace(/\p{M}/gu, "").toLocaleLowerCase();
}

/** 0 exact, 1 title starts with the query, 2 a word starts with it, 3 contains it, 4 matched another field. */
export function matchRank(title: string, query: string): number {
  const t = fold(title);
  const q = fold(query.trim());
  if (!q) return 4;
  if (t === q) return 0;
  if (t.startsWith(q)) return 1;
  if (t.split(/[^\p{L}\p{N}]+/u).some((word) => word.startsWith(q))) return 2;
  if (t.includes(q)) return 3;
  return 4;
}

type Search = LibrarySearchQuery;
type Credits = SearchCreditsQuery["movieCastCredits"]["edges"][number]["node"];
type Movie = Search["movies"]["edges"][number]["node"];
type Show = Search["shows"]["edges"][number]["node"];

const movieHit = (movie: Movie, via?: string): SearchHit => ({
  key: `movie:${movie.id}`, kind: "movie", title: movie.title, meta: [KIND_LABEL.movie, movie.year].filter(Boolean).join(" · "),
  image: movieBackdrop(movie.id), art: "wide", tint: LIBRARY_TYPES.movies.tintVar, to: "/movies/$movieId", params: { movieId: movie.id }, via,
});
const showHit = (show: Show): SearchHit => ({
  key: `show:${show.id}`, kind: "show", title: show.name, meta: [KIND_LABEL.show, show.year].filter(Boolean).join(" · "),
  image: showBackdrop(show), art: "wide", tint: LIBRARY_TYPES.tv.tintVar, to: "/shows/$showId", params: { showId: show.id },
});

/**
 * Every library match as one ranked list: titles that start with the query first, then
 * titles containing it, then films found through a person or director. Within a rank the
 * kinds keep a fixed order so the grid does not shuffle between keystrokes.
 */
export function rankHits(query: string, data: Search | undefined, credits: readonly Credits[] = []): SearchHit[] {
  if (!data) return [];
  const hits: Array<SearchHit & { rank: number }> = [];
  const seen = new Set<string>();
  const add = (hit: SearchHit, rank: number) => {
    if (seen.has(hit.key)) return;
    seen.add(hit.key);
    hits.push({ ...hit, rank });
  };
  for (const { node } of data.movies.edges) {
    const rank = matchRank(node.title, query);
    add(movieHit(node, rank === 4 && node.director ? `Directed by ${node.director}` : undefined), rank);
  }
  for (const { node } of data.shows.edges) add(showHit(node), matchRank(node.name, query));
  for (const { node } of data.artists.edges) {
    add({ key: `artist:${node.id}`, kind: "artist", title: node.name, meta: KIND_LABEL.artist, image: artistImage(node), art: "square", tint: LIBRARY_TYPES.music.tintVar, to: "/artists/$artistId", params: { artistId: node.id } }, matchRank(node.name, query));
  }
  for (const { node } of data.albums.edges) {
    add({ key: `album:${node.id}`, kind: "album", title: node.name, meta: [KIND_LABEL.album, node.year].filter(Boolean).join(" · "), image: albumCover(node), art: "square", tint: LIBRARY_TYPES.music.tintVar, to: "/albums/$albumId", params: { albumId: node.id } }, matchRank(node.name, query));
  }
  for (const { node } of data.audiobooks.edges) {
    const rank = matchRank(node.title, query);
    add({ key: `audiobook:${node.id}`, kind: "audiobook", title: node.title, meta: [KIND_LABEL.audiobook, node.authorName].filter(Boolean).join(" · "), image: audiobookCover(node), art: "square", tint: LIBRARY_TYPES.audiobooks.tintVar, to: "/audiobooks/$audiobookId", params: { audiobookId: node.id }, via: rank === 4 && node.authorName ? `By ${node.authorName}` : undefined }, rank);
  }
  for (const { node } of data.episodes.edges) {
    const code = `S${String(node.season).padStart(2, "0")}E${String(node.episode).padStart(2, "0")}`;
    add({ key: `episode:${node.id}`, kind: "episode", title: node.title ?? `Episode ${node.episode}`, meta: [node.show?.name, code].filter(Boolean).join(" · "), image: episodeThumb(node.id), fallbackImage: showBackdrop({ id: node.showId }), art: "wide", tint: LIBRARY_TYPES.tv.tintVar, to: "/shows/$showId", params: { showId: node.showId } }, matchRank(node.title ?? "", query));
  }
  const names = new Map(data.people.edges.map(({ node }) => [node.id, node.name]));
  for (const credit of credits) {
    if (!credit.movie) continue;
    const name = names.get(credit.personId);
    add(movieHit(credit.movie, name ? (credit.characterName ? `${name} as ${credit.characterName}` : `With ${name}`) : undefined), 4);
  }
  return hits
    .sort((a, b) => a.rank - b.rank || KIND_ORDER[a.kind] - KIND_ORDER[b.kind] || a.title.localeCompare(b.title))
    .map(({ rank: _rank, ...hit }) => hit);
}

/** Newest films and shows interleaved, for the grid before anything is typed. */
export function startHits(data: SearchStartQuery | undefined): SearchHit[] {
  if (!data) return [];
  const movies = data.movies.edges.map(({ node }) => movieHit(node));
  const shows = data.shows.edges.map(({ node }) => showHit(node));
  const out: SearchHit[] = [];
  for (let index = 0; index < Math.max(movies.length, shows.length); index += 1) {
    if (movies[index]) out.push(movies[index]!);
    if (shows[index]) out.push(shows[index]!);
  }
  return out;
}

/**
 * Search terms to offer under the keyboard: people whose names match, then directors and
 * authors, then the best titles. Choosing one replaces the query, as on a TV app.
 */
export function suggestTerms(query: string, data: Search | undefined, hits: readonly SearchHit[], limit = 8): string[] {
  const q = fold(query.trim());
  if (!q || !data) return [];
  const terms: string[] = [];
  const seen = new Set([q]);
  const offer = (term: string | null | undefined) => {
    if (!term || terms.length >= limit) return;
    const key = fold(term);
    if (seen.has(key) || !key.includes(q)) return;
    seen.add(key);
    terms.push(term);
  };
  for (const { node } of data.people.edges) offer(node.name);
  for (const { node } of data.movies.edges) offer(node.director);
  for (const { node } of data.audiobooks.edges) offer(node.authorName);
  for (const hit of hits) offer(hit.kind === "episode" ? null : hit.title);
  return terms;
}
