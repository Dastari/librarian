import type { PersonalEntryFragment } from "@/graphql/generated/graphql";

/**
 * How long a playlist runs, from what each entry knows of itself: a track's or chapter's length,
 * a movie's or episode's runtime, an album's or audiobook's total. A show has no total of its own,
 * so a playlist holding one reports `partial` and the figure is "at least".
 */
export function playlistDuration(entries: readonly PersonalEntryFragment[]): { seconds: number; partial: boolean } {
  let seconds = 0;
  let partial = false;
  for (const entry of entries) {
    const known = entry.track ? entry.track.durationSecs
      : entry.chapter ? entry.chapter.durationSecs
      : entry.movie ? (entry.movie.runtime ? entry.movie.runtime * 60 : null)
      : entry.episode ? (entry.episode.runtime ? entry.episode.runtime * 60 : null)
      : entry.album ? entry.album.totalDurationSecs
      : entry.audiobook ? entry.audiobook.totalDurationSecs
      : null;
    if (known) seconds += known;
    else partial = true;
  }
  return { seconds, partial };
}
