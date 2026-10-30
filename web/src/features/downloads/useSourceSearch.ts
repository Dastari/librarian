import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { SearchSourcesDocument, type SearchSourcesQuery, type SearchSourcesQueryVariables } from "@/graphql/generated/graphql";
import { useLiveEpoch } from "@/lib/graphql/live/hooks";
import { stableKey } from "@/lib/graphql/live/registry";

type Source = SearchSourcesQuery["searchSources"]["sources"][number];
type Release = Source["releases"][number];
type Continuation = NonNullable<SearchSourcesQueryVariables["input"]["continuations"]>[number];

export function mergeSourcePages(previous: readonly Source[], next: readonly Source[]): Source[] {
  const sources = new Map(previous.map((source) => [source.sourceId, source]));
  for (const source of next) {
    const old = sources.get(source.sourceId);
    const releases = new Map<string, Release>(old?.releases.map((release) => [release.stableId, release]));
    for (const release of source.releases) releases.set(release.stableId, release);
    sources.set(source.sourceId, { ...source, releases: [...releases.values()] });
  }
  return [...sources.values()];
}

/** Opaque continuation tokens stay bound to the original generated input and
 * authenticated client epoch. Later-page failures retain every returned row. */
export function useSourceSearch() {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const generation = useRef(0);
  const scope = useRef(epoch);
  const currentInput = useRef<SearchSourcesQueryVariables | undefined>(undefined);
  const currentData = useRef<SearchSourcesQuery | undefined>(undefined);
  const busy = useRef(false);
  const lastContinuation = useRef<Continuation[]>([]);
  const [revision, setRevision] = useState(0);
  if (scope.current !== epoch) {
    scope.current = epoch; generation.current++;
    currentInput.current = undefined; currentData.current = undefined; lastContinuation.current = []; busy.current = false;
  }
  const [snapshot, setSnapshot] = useState<{ epoch: string; data?: SearchSourcesQuery; loading: boolean; loadingMore: boolean; error?: Error; moreError?: Error }>();
  useEffect(() => () => { generation.current++; busy.current = false; }, [epoch]);

  const search = useCallback(async ({ variables }: { variables: SearchSourcesQueryVariables }) => {
    const version = ++generation.current;
    currentInput.current = variables;
    currentData.current = undefined;
    lastContinuation.current = [];
    setRevision((value) => value + 1);
    busy.current = true;
    setSnapshot({ epoch, loading: true, loadingMore: false });
    try {
      const response = await client.query({ query: SearchSourcesDocument, variables, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
      if (version !== generation.current || scope.current !== epoch) return;
      if (!response.data) throw new Error("Source search unavailable");
      currentData.current = response.data;
      setSnapshot({ epoch, data: response.data, loading: false, loadingMore: false });
    } catch (cause) {
      if (version === generation.current && scope.current === epoch) setSnapshot({ epoch, loading: false, loadingMore: false, error: cause instanceof Error ? cause : new Error(String(cause)) });
    } finally { if (version === generation.current) busy.current = false; }
  }, [client, epoch]);

  const continueSources = useCallback(async (continuations: Continuation[]) => {
    const variables = currentInput.current;
    const previous = currentData.current;
    if (busy.current || !variables || !previous || !continuations.length) return;
    const version = generation.current;
    lastContinuation.current = continuations;
    busy.current = true;
    setSnapshot((value) => ({ epoch, data: value?.epoch === epoch ? value.data : undefined, loading: false, loadingMore: true }));
    try {
      const sourceIds = continuations.map((continuation) => continuation.sourceId);
      const response = await client.query({ query: SearchSourcesDocument, variables: { input: { ...variables.input, sourceIds, continuations } }, fetchPolicy: "no-cache", context: { queryDeduplication: false } });
      if (version !== generation.current || scope.current !== epoch) return;
      if (!response.data) throw new Error("Source continuation unavailable");
      if (response.data.searchSources.sources.some((source) => source.pageInfo.nextCursor && continuations.some((request) => request.sourceId === source.sourceId && request.cursor === source.pageInfo.nextCursor))) throw new Error("Source continuation did not advance");
      const data: SearchSourcesQuery = { searchSources: {
        ...response.data.searchSources,
        sources: mergeSourcePages(previous.searchSources.sources, response.data.searchSources.sources),
      } };
      currentData.current = data;
      setRevision((value) => value + 1);
      setSnapshot({ epoch, data, loading: false, loadingMore: false });
    } catch (cause) {
      if (version === generation.current && scope.current === epoch) setSnapshot({ epoch, data: previous, loading: false, loadingMore: false, moreError: cause instanceof Error ? cause : new Error(String(cause)) });
    } finally { if (version === generation.current) busy.current = false; }
  }, [client, epoch]);

  const current = snapshot?.epoch === epoch ? snapshot : undefined;
  const continuations = useMemo(() => (current?.data?.searchSources.sources ?? []).flatMap((source): Continuation[] =>
    source.pageInfo.nextCursor && source.pageInfo.state !== "EXHAUSTED" && source.pageInfo.hasMore !== false
      ? [{ sourceId: source.sourceId, cursor: source.pageInfo.nextCursor }] : []), [current?.data]);
  const loadMore = useCallback(() => continueSources(continuations), [continueSources, continuations]);
  const retrySource = useCallback((sourceId: string) => {
    const source = currentData.current?.searchSources.sources.find((source) => source.sourceId === sourceId);
    return source?.pageInfo.retryCursor ? continueSources([{ sourceId, cursor: source.pageInfo.retryCursor }]) : Promise.resolve();
  }, [continueSources]);
  const retryContinuation = useCallback(() => continueSources(lastContinuation.current), [continueSources]);
  const queryKey = stableKey([epoch, currentInput.current?.input]);
  return { search, result: { ...current, loading: current?.loading ?? false }, loadMore, retrySource, retryContinuation, revision, hasMore: continuations.length > 0, queryKey };
}
