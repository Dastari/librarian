import type { EntityRef } from "glass-ux/ai";

import { entityPath } from "./kinds";

/** A record reference with the page it opens, for entity chips and "Open …" actions. */
export type LinkedRef = EntityRef & { href?: string };

export const episodeCode = (season: number, episode: number) => `S${String(season).padStart(2, "0")}E${String(episode).padStart(2, "0")}`;

/** The item a media file (or a pending match) belongs to: the movie, episode, track or chapter. */
export interface ItemLinks {
  movie?: { id: string; title: string; year?: number | null } | null;
  episode?: { id: string; showId: string; season: number; episode: number; title?: string | null; show?: { id: string; name: string } | null } | null;
  track?: { id: string; albumId: string; title: string } | null;
  chapter?: { id: string; audiobookId: string; title?: string | null } | null;
}

export function linkedItem(links: ItemLinks): LinkedRef | null {
  if (links.movie) return { kind: "movie", id: links.movie.id, label: links.movie.title, meta: links.movie.year ? String(links.movie.year) : undefined, href: entityPath("movie", links.movie.id) };
  if (links.episode) {
    const code = episodeCode(links.episode.season, links.episode.episode);
    return { kind: "episode", id: links.episode.id, label: links.episode.show ? `${links.episode.show.name} · ${code}` : code, meta: links.episode.title ?? undefined, href: entityPath("episode", links.episode.id, links.episode.showId) };
  }
  if (links.track) return { kind: "track", id: links.track.id, label: links.track.title, href: entityPath("album", links.track.albumId) };
  if (links.chapter) return { kind: "audiobook", id: links.chapter.audiobookId, label: links.chapter.title ?? "Chapter", href: entityPath("audiobook", links.chapter.audiobookId) };
  return null;
}

export const fileName = (path: string) => path.split(/[\\/]/).pop() ?? path;

/** A media file as a chip: its name, opening the library's Files view filtered to it. */
export function mediaFileRef(file: { id: string; libraryId?: string | null; path: string }): LinkedRef {
  const name = fileName(file.path);
  return { kind: "mediaFile", id: file.id, label: name, href: file.libraryId ? `/libraries/${file.libraryId}/files?q=${encodeURIComponent(name)}` : undefined };
}

export function libraryRef(library: { id: string; name: string }): LinkedRef {
  return { kind: "library", id: library.id, label: library.name, href: entityPath("library", library.id) };
}

/** Upper snake case from the server ("BYTE_IDENTICAL_DUPLICATE", "auto_download") as words. */
export function humanize(code: string): string {
  const words = code.replace(/[_-]+/g, " ").trim().toLowerCase();
  return words.charAt(0).toUpperCase() + words.slice(1);
}
