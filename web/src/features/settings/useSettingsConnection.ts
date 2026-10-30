import type { OperationVariables, TypedDocumentNode } from "@apollo/client";
import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { operationKey } from "@/lib/graphql/live/documents";
import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";
import type { KeysetPageInput, PageInput } from "@/graphql/generated/graphql";

interface Connection<T> {
  edges: Array<{ node: T }>;
  pageInfo: { hasNextPage: boolean; totalCount?: number | null; endCursor?: string | null };
}
interface Snapshot<T> {
  rows: T[];
  offset: number;
  cursor?: string;
  hasMore: boolean;
  totalCount?: number;
}
const EMPTY: never[] = [];

/**
 * Settings are small enough to filter locally, but one server page is not a collection.
 * Publish complete snapshots atomically; a failed refresh keeps the last complete one.
 * Logs opt into incremental server batches instead. Offsets count server edges, not unique IDs.
 */
export function useSettingsConnection<TData, TVariables extends OperationVariables & { page?: PageInput | KeysetPageInput | null }, TNode extends { id: string }>(
  document: TypedDocumentNode<TData, TVariables>,
  variables: Omit<NoInfer<TVariables>, "page">,
  select: (data: TData) => Connection<TNode>,
  options?: { infinite?: boolean; batchSize?: number; skip?: boolean; live?: boolean; keyset?: boolean; initialPartial?: boolean },
) {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const batchSize = options?.batchSize ?? 100;
  const infinite = options?.infinite ?? false;
  const keyset = options?.keyset ?? false;
  const key = JSON.stringify([epoch, operationKey(document), variables, batchSize, infinite, keyset, Boolean(options?.skip)]);
  const inputs = useRef({ variables, select });
  inputs.current = { variables, select };
  const generation = useRef(0);
  const busy = useRef(false);
  const [snapshot, setSnapshot] = useState<{ key: string; value: Snapshot<TNode> }>();
  const current = snapshot?.key === key ? snapshot.value : undefined;
  const currentRef = useRef(current);
  currentRef.current = current;
  const [status, setStatus] = useState<{ key: string; fetching: boolean; loadingMore: boolean; error?: Error; moreError?: Error }>({ key, fetching: true, loadingMore: false });
  const active = status.key === key ? status : { key, fetching: true, loadingMore: false };

  const request = useCallback(async (offset: number, after?: string): Promise<Connection<TNode>> => {
    const selectResult = inputs.current.select;
    const result = await client.query({
      query: document,
      variables: { ...inputs.current.variables, page: keyset ? { limit: batchSize, after: after ?? null, includeTotalCount: true } : { limit: batchSize, offset } } as TVariables,
      fetchPolicy: "no-cache",
      context: { queryDeduplication: false },
    });
    if (!result.data) throw new Error("The server returned no collection");
    const connection = selectResult(result.data as TData);
    if (keyset && connection.pageInfo.hasNextPage && (!connection.pageInfo.endCursor || connection.pageInfo.endCursor === after)) throw new Error("Cursor paging did not advance");
    return connection;
  }, [batchSize, client, document, keyset]);

  const refetch = useCallback(async (refreshOptions?: { reset?: boolean; throwOnError?: boolean }) => {
    if (options?.skip) return;
    const version = ++generation.current;
    busy.current = true;
    setStatus({ key, fetching: true, loadingMore: false });
    // A live refresh replaces the loaded window atomically. Clear explicitly returns to its head.
    const target = refreshOptions?.reset ? 0 : currentRef.current?.offset ?? 0;
    const nodes = new Map<string, TNode>();
    let offset = 0;
    let after: string | undefined;
    let connection: Connection<TNode> | undefined;
    try {
      do {
        connection = await request(offset, after);
        if (version !== generation.current) return;
        if (!connection.edges.length && connection.pageInfo.hasNextPage) throw new Error("The server returned an incomplete collection. Retry to reconnect.");
        connection.edges.forEach(({ node }) => nodes.set(node.id, node));
        offset += connection.edges.length;
        after = connection.pageInfo.endCursor ?? undefined;
      } while (connection.pageInfo.hasNextPage && (!infinite || offset < target));
      if (!infinite && connection.pageInfo.totalCount != null && nodes.size < connection.pageInfo.totalCount) {
        throw new Error("The collection changed while loading. Retry to load every row.");
      }
      setSnapshot({ key, value: { rows: [...nodes.values()], offset, cursor: after, hasMore: connection.pageInfo.hasNextPage, totalCount: connection.pageInfo.totalCount ?? undefined } });
      setStatus({ key, fetching: false, loadingMore: false });
    } catch (cause) {
      // Browsers may retain an explicitly incomplete first load for reading.
      // Existing complete snapshots and settings always keep atomic semantics.
      if (version === generation.current && options?.initialPartial && !currentRef.current && connection && nodes.size) {
        setSnapshot({ key, value: { rows: [...nodes.values()], offset, cursor: after, hasMore: connection.pageInfo.hasNextPage, totalCount: connection.pageInfo.totalCount ?? undefined } });
      }
      if (version === generation.current) setStatus({ key, fetching: false, loadingMore: false, error: cause instanceof Error ? cause : new Error(String(cause)) });
      if (refreshOptions?.throwOnError) throw cause;
    } finally {
      if (version === generation.current) busy.current = false;
    }
  }, [batchSize, infinite, key, request, options?.skip, options?.initialPartial]);

  useLiveInvalidation(document, variables, () => refetch({ throwOnError: true }), {
    enabled: !options?.skip && options?.live !== false,
    key: `snapshot:${key}`, shared: false,
  });

  useEffect(() => {
    void refetch();
    return () => { generation.current++; busy.current = false; };
  }, [refetch]);

  const loadMore = useCallback(async () => {
    const previous = currentRef.current;
    if (!infinite || busy.current || !previous?.hasMore) return;
    const version = generation.current;
    busy.current = true;
    setStatus({ key, fetching: false, loadingMore: true });
    try {
      const connection = await request(previous.offset, previous.cursor);
      if (version !== generation.current) return;
      if (!connection.edges.length && connection.pageInfo.hasNextPage) throw new Error("The server returned an incomplete batch. Retry to reconnect.");
      const nodes = new Map(previous.rows.map((node) => [node.id, node]));
      connection.edges.forEach(({ node }) => nodes.set(node.id, node));
      setSnapshot({ key, value: { rows: [...nodes.values()], offset: previous.offset + connection.edges.length, cursor: connection.pageInfo.endCursor ?? undefined, hasMore: connection.pageInfo.hasNextPage, totalCount: connection.pageInfo.totalCount ?? undefined } });
      setStatus({ key, fetching: false, loadingMore: false });
    } catch (cause) {
      if (version !== generation.current) return;
      const error = cause instanceof Error ? cause : new Error(String(cause));
      setStatus({ key, fetching: false, loadingMore: false, moreError: error });
      throw error;
    } finally {
      if (version === generation.current) busy.current = false;
    }
  }, [infinite, key, request]);

  return useMemo(() => ({
    rows: current?.rows ?? EMPTY,
    queryKey: key,
    revision: generation.current,
    totalCount: current?.totalCount,
    hasMore: current?.hasMore ?? false,
    loading: !options?.skip && active.fetching && !current,
    fetching: !options?.skip && active.fetching,
    loadingMore: active.loadingMore,
    error: active.error,
    moreError: active.moreError,
    refetch,
    loadMore,
  }), [active.error, active.fetching, active.loadingMore, active.moreError, current, loadMore, refetch]);
}
