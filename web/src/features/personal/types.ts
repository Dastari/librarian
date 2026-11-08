import type { PersonalEntryFragment } from "@/graphql/generated/graphql";
import { albumCover, audiobookCover, episodeThumb, moviePoster, showPoster } from "@/lib/artwork";

/** Everything a playlist can hold. Albums, shows and audiobooks stand for their playable children. */
export const PLAYLIST_TARGET_KINDS = ["movie", "show", "episode", "album", "track", "audiobook", "chapter"] as const;
export type PlaylistTargetKind = (typeof PLAYLIST_TARGET_KINDS)[number];
export type PlaylistTarget = { kind: PlaylistTargetKind; id: string };
/** The name this had before playlists held episodes and chapters. */
export type PersonalTarget = PlaylistTarget;

export const targetKey = (target: PlaylistTarget) => `${target.kind}:${target.id}`;

/** A personal entry points at catalogue identity; it never changes Wanted or imports media. */
export function targetFields(target: PlaylistTarget) {
  return { movieId: null, showId: null, albumId: null, audiobookId: null, trackId: null, episodeId: null, chapterId: null, [`${target.kind}Id`]: target.id };
}

type EntryTargetIds = Pick<PersonalEntryFragment, "movieId" | "showId" | "albumId" | "audiobookId" | "trackId" | "episodeId" | "chapterId">;

/** What an entry points at. */
export function entryTarget(entry: EntryTargetIds): PlaylistTarget | null {
  for (const kind of PLAYLIST_TARGET_KINDS) {
    const id = entry[`${kind}Id`];
    if (id) return { kind, id };
  }
  return null;
}

export const episodeCode = (episode: { season: number; episode: number }) => `S${String(episode.season).padStart(2, "0")}E${String(episode.episode).padStart(2, "0")}`;

export function entryPresentation(entry: PersonalEntryFragment) {
  if (entry.movie) return { title: entry.movie.title, subtitle: entry.movie.year ? String(entry.movie.year) : "Movie", image: moviePoster(entry.movie.id), href: `/movies/${entry.movie.id}`, square: false };
  if (entry.show) return { title: entry.show.name, subtitle: entry.show.year ? String(entry.show.year) : "Show", image: showPoster(entry.show), href: `/shows/${entry.show.id}`, square: false };
  if (entry.episode) return { title: entry.episode.title ?? `Episode ${entry.episode.episode}`, subtitle: [entry.episode.show?.name, episodeCode(entry.episode)].filter(Boolean).join(" · "), image: episodeThumb(entry.episode.id), href: `/shows/${entry.episode.showId}`, square: false };
  if (entry.album) return { title: entry.album.name, subtitle: entry.album.artist?.name ?? "Album", image: albumCover(entry.album), href: `/albums/${entry.album.id}`, square: true };
  if (entry.audiobook) return { title: entry.audiobook.title, subtitle: entry.audiobook.authorName ?? "Audiobook", image: audiobookCover(entry.audiobook), href: `/audiobooks/${entry.audiobook.id}`, square: false };
  if (entry.track) return { title: entry.track.title, subtitle: entry.track.artistName ?? "Track", image: entry.track.album ? albumCover(entry.track.album) : undefined, href: `/albums/${entry.track.albumId}`, square: true };
  if (entry.chapter) return { title: entry.chapter.title ?? `Chapter ${entry.chapter.chapterNumber}`, subtitle: entry.chapter.audiobook?.title ?? "Chapter", image: audiobookCover({ id: entry.chapter.audiobookId }), href: `/audiobooks/${entry.chapter.audiobookId}`, square: false };
  return null;
}
