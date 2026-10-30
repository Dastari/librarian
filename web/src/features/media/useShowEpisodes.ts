import { useApolloClient, useQuery } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";
import { ShowEpisodesDocument, ShowEpisodesPageDocument } from "@/graphql/generated/graphql";

/** Load the complete ordered show; page failures retain readable episodes and can retry. */
export function useShowEpisodes(showId: string) {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const scopeKey = `${epoch}:${showId}`;
  const activation = useRef<{ showId: string; revalidate: boolean } | undefined>(undefined);
  if (activation.current?.showId !== scopeKey) activation.current = { showId: scopeKey, revalidate: Boolean(showId && client.readQuery({ query: ShowEpisodesDocument, variables: { showId } })) };
  // The app's cache-and-network default would replace a cached complete show
  // with page one. Cached shows revalidate through the atomic snapshot below.
  const query = useQuery(ShowEpisodesDocument, { variables: { showId }, skip: !showId, fetchPolicy: "cache-first", notifyOnNetworkStatusChange: true });
  const connection = query.data?.episodes;
  const rows = useMemo(() => [...new Map(connection?.edges.filter(({ node }) => node.showId === showId).map(({ node }) => [node.id, node]) ?? []).values()], [connection, showId]);
  const inFlight = useRef(new Set<string>());
  const currentShow = useRef(scopeKey);
  const generation = useRef(0);
  if (currentShow.current !== scopeKey) { currentShow.current = scopeKey; generation.current += 1; }
  const refreshing = useRef<{ showId: string; version: number } | undefined>(undefined);
  const [failure, setFailure] = useState<{ showId: string; error: Error }>();
  const [paging, setPaging] = useState<{ showId: string; key: string }>();
  const pageError = failure?.showId === scopeKey ? failure.error : undefined;
  const loadingMore = paging?.showId === scopeKey;
  const loadMore = useCallback(async () => {
    const version = generation.current;
    const key = `${showId}:${version}:${rows.length}`;
    if (refreshing.current?.showId === scopeKey || inFlight.current.has(key) || query.loading || !connection?.pageInfo.hasNextPage) return;
    inFlight.current.add(key);
    setPaging({ showId: scopeKey, key });
    try {
      // Validate before writing the watched connection. Throwing in Apollo's
      // updateQuery callback escapes the request promise and cannot be recovered.
      const { data } = await client.query({ query: ShowEpisodesPageDocument, variables: { showId, offset: rows.length }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
      if (currentShow.current !== scopeKey || generation.current !== version) return;
      const next = data?.episodes;
      if (!next) throw new Error("Could not load the remaining episodes");
      const seen = new Set(rows.map((node) => node.id));
      const added = [...new Map(next.edges.filter(({ node }) => node.showId === showId && !seen.has(node.id)).map((edge) => [edge.node.id, edge])).values()];
      if (!added.length && (next.pageInfo.hasNextPage || (next.pageInfo.totalCount ?? 0) > rows.length)) throw new Error("Episode paging did not advance. Retry to load the remaining seasons.");
      query.updateQuery((_previous, options) => {
        if (currentShow.current !== scopeKey || generation.current !== version || !options.complete) return;
        const previous = options.previousData;
        return { ...previous, episodes: { ...next, edges: [...previous.episodes.edges, ...added] } };
      });
      setFailure(undefined);
    } catch (error) {
      if (currentShow.current === scopeKey && generation.current === version) setFailure({ showId: scopeKey, error: error instanceof Error ? error : new Error("Could not load the remaining episodes") });
    } finally {
      inFlight.current.delete(key);
      setPaging((current) => current?.key === key ? undefined : current);
    }
  }, [client, query.updateQuery, query.loading, connection, rows, showId, scopeKey]);
  const refetch = useCallback(async () => {
    if (currentShow.current !== scopeKey) return;
    const version = ++generation.current;
    const key = `${showId}:refresh:${version}`;
    refreshing.current = { showId: scopeKey, version };
    setPaging({ showId: scopeKey, key });
    setFailure(undefined);
    try {
      if (!connection) return await query.refetch();
      // Revalidate off-cache, then replace the connection once. Existing later
      // seasons and focused controls stay mounted even if a new page fails.
      const first = await client.query({ query: ShowEpisodesDocument, variables: { showId }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
      if (!first.data) throw new Error("Could not refresh episodes");
      let next = first.data.episodes;
      const fresh = new Map(next.edges.map((edge) => [edge.node.id, edge]));
      let offset = next.edges.length;
      while (next.pageInfo.hasNextPage) {
        const result = await client.query({ query: ShowEpisodesPageDocument, variables: { showId, offset }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
        if (currentShow.current !== scopeKey || generation.current !== version) return;
        if (!result.data) throw new Error("Could not refresh the remaining episodes");
        next = result.data.episodes;
        const before = fresh.size;
        for (const edge of next.edges) if (edge.node.showId === showId) fresh.set(edge.node.id, edge);
        if (fresh.size === before && (next.pageInfo.hasNextPage || (next.pageInfo.totalCount ?? 0) > fresh.size)) throw new Error("Episode paging did not advance. Retry to refresh the remaining seasons.");
        offset += next.edges.length;
      }
      if (currentShow.current !== scopeKey || generation.current !== version) return;
      if (next.pageInfo.totalCount != null && fresh.size < next.pageInfo.totalCount) throw new Error("Episode collection changed while refreshing");
      client.writeQuery({ query: ShowEpisodesDocument, variables: { showId }, data: { ...first.data, episodes: { ...next, edges: [...fresh.values()] } } });
    } catch (error) {
      if (currentShow.current === scopeKey && generation.current === version) setFailure({ showId: scopeKey, error: error instanceof Error ? error : new Error("Could not refresh episodes") });
      throw error;
    } finally {
      if (refreshing.current?.version === version) refreshing.current = undefined;
      setPaging((current) => current?.key === key ? undefined : current);
    }
  }, [client, connection, query.refetch, showId, scopeKey]);
  useLiveInvalidation(ShowEpisodesDocument, { showId }, refetch, { enabled: Boolean(showId), key: `show-episodes:${scopeKey}` });
  const revalidate = useRef(refetch);
  revalidate.current = refetch;
  useEffect(() => {
    if (!activation.current?.revalidate) return;
    activation.current.revalidate = false;
    void revalidate.current().catch(() => {});
  }, [scopeKey]);
  useEffect(() => { if (!pageError) void loadMore(); }, [loadMore, pageError]);
  return { ...query, rows, loadingMore, error: query.error ?? pageError, complete: Boolean(connection && !connection.pageInfo.hasNextPage && !query.loading && !loadingMore && !pageError), refetch };
}
