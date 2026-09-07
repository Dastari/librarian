/** Season grouping for episode lists. Pure so it can be unit tested without Apollo. */

export interface SeasonEpisodeLike {
  season: number;
  episode: number;
  wanted: boolean;
  ignored?: boolean | null;
  mediaFileId?: string | null;
}

export interface SeasonGroup<TEpisode> {
  season: number;
  label: string;
  episodes: TEpisode[];
  total: number;
  /** Episodes with a linked file. */
  have: number;
  /** Episodes without a file that are not ignored. */
  missing: number;
  /** Missing episodes that are marked wanted. */
  wanted: number;
  ignored: number;
  allIgnored: boolean;
}

export function seasonLabel(season: number): string {
  return season === 0 ? "Specials" : `Season ${season}`;
}

/**
 * Groups episodes by season, ascending, keeping the incoming order inside a season.
 * Counts drive the season header row (have / missing / wanted).
 */
export function groupBySeason<TEpisode extends SeasonEpisodeLike>(episodes: TEpisode[]): Array<SeasonGroup<TEpisode>> {
  const bySeason = new Map<number, TEpisode[]>();
  for (const episode of episodes) {
    const bucket = bySeason.get(episode.season);
    if (bucket) bucket.push(episode);
    else bySeason.set(episode.season, [episode]);
  }
  return [...bySeason.entries()]
    .sort((a, b) => a[0] - b[0])
    .map(([season, list]) => {
      const have = list.filter((episode) => Boolean(episode.mediaFileId)).length;
      const ignored = list.filter((episode) => episode.ignored === true).length;
      const missingList = list.filter((episode) => !episode.mediaFileId && episode.ignored !== true);
      return {
        season,
        label: seasonLabel(season),
        episodes: list,
        total: list.length,
        have,
        missing: missingList.length,
        wanted: missingList.filter((episode) => episode.wanted).length,
        ignored,
        allIgnored: list.length > 0 && ignored === list.length,
      };
    });
}
