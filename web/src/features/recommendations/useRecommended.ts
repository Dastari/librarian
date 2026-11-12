import { useMutation } from "@apollo/client/react";
import { useEffect, useMemo } from "react";

import { EntityReleaseRecommendationListDocument, LookupRecommendationsDocument } from "@/graphql/generated/graphql";
import { useIsAdmin } from "@/lib/auth/useSession";
import { useLiveQuery } from "@/lib/graphql/live/hooks";

import { lookupInput, recommendationFilter, recommended, targetKey, type Recommendation, type RecommendationTarget, type Recommended } from "./recommendations";

/** Lookups already asked for in this session, so opening a page again does not ask again. */
const asked = new Set<string>();

/** Forget the lookups asked for (tests). */
export function forgetAskedLookups() {
  asked.clear();
}

/**
 * A target's recommended releases, live, for Find releases: its badge and its table. While something
 * is missing (`missing > 0`) and nothing is stored yet, the server is asked once per session to look
 * (it answers within its hourly search budget, and the rows arrive live). Administrators only, like
 * every other way of finding and grabbing a release.
 */
export function useRecommended(target: RecommendationTarget, missing: number, enabled = true): Recommended & { rows: Recommendation[]; refetch: () => void; loaded: boolean } {
  const isAdmin = useIsAdmin();
  const filter = recommendationFilter(target);
  const key = targetKey(target);
  const active = enabled && isAdmin && Boolean(filter) && missing > 0;
  const list = useLiveQuery(EntityReleaseRecommendationListDocument, {
    variables: { where: filter, orderBy: [{ foundAt: "DESC" }], page: { limit: 200, offset: 0 } },
    skip: !active,
  });
  const data = list.data;
  const rows = useMemo(() => data?.releaseRecommendations.edges.map((edge) => edge.node) ?? [], [data]);
  const [lookup] = useMutation(LookupRecommendationsDocument);
  const loaded = Boolean(list.data) && !list.loading;
  // A plan is answered once its plan row exists; rows of older kinds do not count.
  const answered = target.showId || target.artistId ? rows.some((row) => row.scope === "plan") : rows.length > 0;

  useEffect(() => {
    const input = lookupInput(target);
    if (!active || !loaded || answered || !input || asked.has(key)) return;
    asked.add(key);
    void lookup({ variables: { input } }).catch(() => asked.delete(key));
    // `target` is covered by `key`.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [active, loaded, answered, key, lookup]);

  const view = useMemo<Recommended>(
    () => (active ? recommended(rows, target) : { plan: null, releases: [], forTarget: 0, searching: false, none: null }),
    // `target` is covered by `key` and the narrowing fields.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [active, rows, key, target.season, target.series, target.albumId],
  );
  return { ...view, rows, loaded, refetch: () => void list.refetch() };
}
