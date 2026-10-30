import type { PersonalEntryFragment } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, moviePoster, showPoster } from "@/lib/artwork";

export type PersonalTarget = { kind: "movie" | "show" | "album" | "audiobook" | "track"; id: string };

/** A personal entry points at catalogue identity; it never changes Wanted or imports media. */
export function targetFields(target: PersonalTarget) {
  return { movieId: null, showId: null, albumId: null, audiobookId: null, trackId: null, [`${target.kind}Id`]: target.id };
}

export function entryPresentation(entry: PersonalEntryFragment) {
  if (entry.movie) return { title: entry.movie.title, subtitle: entry.movie.year ? String(entry.movie.year) : "Movie", image: moviePoster(entry.movie.id), href: `/movies/${entry.movie.id}`, square: false };
  if (entry.show) return { title: entry.show.name, subtitle: entry.show.year ? String(entry.show.year) : "Show", image: showPoster(entry.show), href: `/shows/${entry.show.id}`, square: false };
  if (entry.album) return { title: entry.album.name, subtitle: entry.album.artist?.name ?? "Album", image: albumCover(entry.album), href: `/albums/${entry.album.id}`, square: true };
  if (entry.audiobook) return { title: entry.audiobook.title, subtitle: entry.audiobook.authorName ?? "Audiobook", image: audiobookCover(entry.audiobook), href: `/audiobooks/${entry.audiobook.id}`, square: false };
  if (entry.track) return { title: entry.track.title, subtitle: entry.track.artistName ?? "Track", image: entry.track.album ? albumCover(entry.track.album) : undefined, href: `/albums/${entry.track.albumId}`, square: true };
  return null;
}
