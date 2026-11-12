import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";

import type { PlaylistCoverEntryFragment } from "@/graphql/generated/graphql";

/** The artwork one entry contributes to a playlist's picture. Artwork is keyed by id, so no more is needed. */
export function entryCover(entry: PlaylistCoverEntryFragment): string | undefined {
  if (entry.movieId) return moviePoster(entry.movieId);
  if (entry.showId) return showPoster({ id: entry.showId });
  if (entry.albumId) return albumCover({ id: entry.albumId });
  if (entry.audiobookId) return audiobookCover({ id: entry.audiobookId });
  if (entry.track) return albumCover({ id: entry.track.albumId });
  if (entry.episode) return showPoster({ id: entry.episode.showId });
  if (entry.chapter) return audiobookCover({ id: entry.chapter.audiobookId });
  return undefined;
}

/** Up to four different covers, in playlist order. Several entries from one album make one tile. */
export function playlistCovers(entries: readonly PlaylistCoverEntryFragment[] | undefined): string[] {
  const covers: string[] = [];
  for (const entry of entries ?? []) {
    const cover = entryCover(entry);
    if (cover && !covers.includes(cover)) covers.push(cover);
    if (covers.length === 4) break;
  }
  return covers;
}
