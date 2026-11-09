import type { EntityReleaseRecommendationListQuery } from "@/graphql/generated/graphql";

export type Recommendation = EntityReleaseRecommendationListQuery["releaseRecommendations"]["edges"][number]["node"];

/** What a recommendation is looked up for: a season (with its episodes), a movie, an album, a book, or a whole show. */
export interface RecommendationTarget {
  showId?: string;
  season?: number;
  /** `series_pack` recommendations belong to the show, not a season. */
  series?: boolean;
  movieId?: string;
  albumId?: string;
  audiobookId?: string;
}

/** The server-side filter for a target's rows. */
export function recommendationFilter(target: RecommendationTarget) {
  if (target.movieId) return { movieId: { eq: target.movieId } };
  if (target.albumId) return { albumId: { eq: target.albumId } };
  if (target.audiobookId) return { audiobookId: { eq: target.audiobookId } };
  if (target.showId && target.series) return { showId: { eq: target.showId }, scope: { eq: "series_pack" } };
  if (target.showId && target.season !== undefined) return { showId: { eq: target.showId }, season: { eq: target.season } };
  return null;
}

/** A stable key for a target, so one lookup is asked for once per session. */
export function targetKey(target: RecommendationTarget): string {
  return [target.showId, target.series ? "series" : target.season, target.movieId, target.albumId, target.audiobookId].filter((part) => part !== undefined).join(":");
}

/** The lookup input for a target. A whole-show target (the complete-series pack) names the show alone. */
export function lookupInput(target: RecommendationTarget) {
  if (target.movieId) return { movieId: target.movieId };
  if (target.albumId) return { albumId: target.albumId };
  if (target.audiobookId) return { audiobookId: target.audiobookId };
  if (target.showId && target.series) return { showId: target.showId };
  if (target.showId && target.season !== undefined) return { showId: target.showId, season: target.season };
  return null;
}

/** "covers 8 of 10 missing", or nothing when a single item is missing. */
export function coverageLabel(row: Pick<Recommendation, "coversCount" | "missingCount">): string | null {
  const covers = row.coversCount ?? 0;
  const missing = row.missingCount ?? 0;
  return missing > 1 && covers > 0 ? `covers ${covers} of ${missing} missing` : null;
}

const SCOPE_LABEL: Record<string, string> = {
  episode: "Episode",
  season_pack: "Season pack",
  series_pack: "Complete series",
  movie: "Movie",
  album: "Album",
  track: "Track",
  book: "Audiobook",
};

export const scopeLabel = (scope: string): string => SCOPE_LABEL[scope] ?? scope;

/** Ready rows first, the ones covering most first; a `searching` or `none` row only if nothing is ready. */
export function visibleRows(rows: readonly Recommendation[]): { ready: Recommendation[]; searching: boolean; none: Recommendation | null } {
  const ready = rows.filter((row) => row.status === "ready").sort((a, b) => (b.coversCount ?? 0) - (a.coversCount ?? 0) || (a.episodeId ?? "").localeCompare(b.episodeId ?? ""));
  return {
    ready,
    searching: ready.length === 0 && rows.some((row) => row.status === "searching"),
    none: ready.length === 0 ? rows.find((row) => row.status === "none") ?? null : null,
  };
}
