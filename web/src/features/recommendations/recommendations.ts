import type { EntityReleaseRecommendationListQuery } from "@/graphql/generated/graphql";

export type Recommendation = EntityReleaseRecommendationListQuery["releaseRecommendations"]["edges"][number]["node"];

/**
 * What a recommendation is looked up for: a movie, a book, a show, or an artist (an album or a track
 * is answered by its artist's plan). A show's or an artist's answer is one coverage plan for
 * everything missing; `season` or `albumId` narrows what counts as "for this" (the Find releases
 * badge), and the dialog lists the whole plan.
 */
export interface RecommendationTarget {
  showId?: string;
  season?: number;
  /** The whole show: its plan in full. */
  series?: boolean;
  movieId?: string;
  albumId?: string;
  /** The album's or the track's artist: music is answered by the artist's plan. */
  artistId?: string;
  audiobookId?: string;
}

/** The server-side filter for a target's rows. A show's rows are its plan and the plan's releases. */
export function recommendationFilter(target: RecommendationTarget) {
  if (target.movieId) return { movieId: { eq: target.movieId } };
  if (target.artistId) return { artistId: { eq: target.artistId } };
  if (target.albumId) return { albumId: { eq: target.albumId } };
  if (target.audiobookId) return { audiobookId: { eq: target.audiobookId } };
  if (target.showId && (target.series || target.season !== undefined)) return { showId: { eq: target.showId } };
  return null;
}

/** A stable key for a target, so one lookup is asked for once per session. A show is looked up once, whichever season asked. */
export function targetKey(target: RecommendationTarget): string {
  if (target.showId) return `${target.showId}:plan`;
  if (target.artistId) return `${target.artistId}:artist`;
  return [target.movieId, target.albumId, target.audiobookId].filter((part) => part !== undefined).join(":");
}

/** The lookup input for a target. A show (any season of it) is looked up whole. */
export function lookupInput(target: RecommendationTarget) {
  if (target.movieId) return { movieId: target.movieId };
  if (target.artistId) return { artistId: target.artistId };
  if (target.albumId) return { albumId: target.albumId };
  if (target.audiobookId) return { audiobookId: target.audiobookId };
  if (target.showId && (target.series || target.season !== undefined)) return { showId: target.showId };
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
  discography: "Discography",
  book: "Audiobook",
  plan: "Plan",
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

/** A JSON array of numbers stored as text, or an empty list. */
function numbers(json: string | null | undefined): number[] {
  if (!json) return [];
  try {
    const value: unknown = JSON.parse(json);
    return Array.isArray(value) ? value.filter((item): item is number => typeof item === "number") : [];
  } catch {
    return [];
  }
}

/** The seasons a release in a plan covers. */
export const coversSeasons = (row: Pick<Recommendation, "coversSeasons" | "season">): number[] => {
  const seasons = numbers(row.coversSeasons);
  return seasons.length > 0 ? seasons : row.season !== null && row.season !== undefined ? [row.season] : [];
};

/** A show's plan as stored: its own row, its releases in order, and whether a lookup is pending or found nothing. */
export interface PlanView {
  plan: Recommendation | null;
  releases: Recommendation[];
  searching: boolean;
  none: Recommendation | null;
}

/**
 * The show's current plan from its rows. With `season`, only the releases covering that season.
 * Rows from before plans existed (no plan row) are left out.
 */
export function planView(rows: readonly Recommendation[], season?: number): PlanView {
  const plan = rows.find((row) => row.scope === "plan" && row.status === "ready") ?? null;
  const releases = plan
    ? rows
        .filter((row) => row.planId === plan.id && row.status === "ready")
        .filter((row) => season === undefined || coversSeasons(row).includes(season))
        .sort((a, b) => (a.planPosition ?? 0) - (b.planPosition ?? 0))
    : [];
  return {
    plan: plan && releases.length > 0 ? plan : null,
    releases,
    searching: !plan && rows.some((row) => row.scope === "plan" && row.status === "searching"),
    none: !plan ? rows.find((row) => row.scope === "plan" && row.status === "none") ?? null : null,
  };
}

/** "Recommended: 3 releases cover 24 of 44 missing". */
export function planSummary(plan: Pick<Recommendation, "coversCount" | "missingCount">, releases: number): string {
  const covers = plan.coversCount ?? 0;
  const missing = plan.missingCount ?? 0;
  const noun = releases === 1 ? "release covers" : "releases cover";
  return missing > 0 ? `${releases} ${noun} ${covers} of ${missing} missing` : `${releases} ${noun} what is missing`;
}

/** "adds 22": what one release contributes to its plan. */
export function addsLabel(row: Pick<Recommendation, "coversCount">): string | null {
  const covers = row.coversCount ?? 0;
  return covers > 0 ? `adds ${covers}` : null;
}

/** The ids a JSON array of strings stored as text holds. */
function strings(json: string | null | undefined): string[] {
  if (!json) return [];
  try {
    const value: unknown = JSON.parse(json);
    return Array.isArray(value) ? value.filter((item): item is string => typeof item === "string") : [];
  } catch {
    return [];
  }
}

/** The albums a release in an artist's plan covers missing tracks of. */
export const coversAlbums = (row: Pick<Recommendation, "coversAlbums" | "albumId">): string[] => {
  const albums = strings(row.coversAlbums);
  return albums.length > 0 ? albums : row.albumId ? [row.albumId] : [];
};

/** What Find releases offers for a target: the plan (a show's or an artist's), its releases, or one release. */
export interface Recommended {
  /** The plan's own row, when the answer is a plan with releases. */
  plan: Recommendation | null;
  /** The recommended releases, in the plan's order (or the single best one). */
  releases: Recommendation[];
  /** How many of them are for exactly what was opened (a season, an album): the badge count. */
  forTarget: number;
  searching: boolean;
  none: Recommendation | null;
}

/**
 * The recommended releases among a target's rows. For a show or an artist, the plan's releases
 * (rows of older rules without a plan row are left out); otherwise the ready rows, best first.
 */
export function recommended(rows: readonly Recommendation[], target: RecommendationTarget): Recommended {
  if (target.showId || target.artistId) {
    const view = planView(rows);
    const forTarget = view.releases.filter((row) => {
      if (target.season !== undefined && !target.series) return coversSeasons(row).includes(target.season);
      if (target.artistId && target.albumId) return coversAlbums(row).includes(target.albumId);
      return true;
    }).length;
    return { plan: view.plan, releases: view.releases, forTarget, searching: view.searching, none: view.none };
  }
  const shown = visibleRows(rows);
  return { plan: null, releases: shown.ready, forTarget: shown.ready.length, searching: shown.searching, none: shown.none };
}

/**
 * What a recommended release covers, in the words of its row: `S02 · adds 6`, `Seasons 1–3 · adds 22`,
 * `S03E05`, `Discography · adds 31 tracks`, `Album · adds 10 tracks`.
 */
export function coverageCell(row: Pick<Recommendation, "scope" | "coversCount" | "coversSeasons" | "season" | "targetLabel" | "artistId">): string {
  const adds = row.coversCount ?? 0;
  if (row.artistId) {
    const noun = adds === 1 ? "track" : "tracks";
    return `${scopeLabel(row.scope)}${adds > 0 ? ` · adds ${adds} ${noun}` : ""}`;
  }
  const seasons = coversSeasons(row);
  const pad = (season: number) => `S${String(season).padStart(2, "0")}`;
  let what: string;
  if (row.scope === "episode") {
    const code = /S\d{2}E\d{2}(?:\s+S\d{2}E\d{2})*/.exec(row.targetLabel)?.[0];
    what = code ?? (seasons[0] !== undefined ? pad(seasons[0]) : "Episode");
  } else if (row.scope === "series_pack") {
    what = seasons.length > 1 ? `Seasons ${seasons[0]}–${seasons[seasons.length - 1]}` : "Complete series";
  } else {
    what = seasons.length > 1 ? `Seasons ${seasons[0]}–${seasons[seasons.length - 1]}` : seasons[0] !== undefined ? pad(seasons[0]) : scopeLabel(row.scope);
  }
  return row.scope === "episode" || adds <= 1 ? what : `${what} · adds ${adds}`;
}
