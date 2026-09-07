import { useQuery } from "@apollo/client/react";
import type { OperationVariables, TypedDocumentNode } from "@apollo/client";
import { useCallback, useMemo, useRef, useState } from "react";

import type { PageInput } from "@/graphql/generated/graphql";

import type { Connection } from "./useConnection";

export const INFINITE_PAGE_SIZE = 60;

interface ConnectionLike {
  edges: unknown[];
  pageInfo: { hasNextPage: boolean; totalCount?: number | null };
}

function isConnection(value: unknown): value is ConnectionLike {
  return typeof value === "object" && value !== null && Array.isArray((value as ConnectionLike).edges);
}

/** Appends the next page's edges onto the first connection found in the result. */
function mergePages<TData>(previous: TData, next: TData): TData {
  const merged: Record<string, unknown> = { ...(previous as Record<string, unknown>) };
  for (const [key, value] of Object.entries(next as Record<string, unknown>)) {
    const existing = merged[key];
    if (isConnection(existing) && isConnection(value)) {
      merged[key] = { ...value, edges: [...existing.edges, ...value.edges] };
    }
  }
  return merged as TData;
}

/**
 * Offset-paginated list loaded page by page for infinite scrolling. `variables` must contain a
 * `page` slot; the hook owns it. Changing any other variable restarts from the first page.
 * `options.skip` keeps the hook mounted without querying (used by tabbed pages).
 */
export function useInfiniteConnection<TData, TVariables extends OperationVariables & { page?: PageInput | null }, TNode>(
  document: TypedDocumentNode<TData, TVariables>,
  variables: Omit<TVariables, "page">,
  select: (data: TData) => Connection<TNode> | null | undefined,
  options?: { skip?: boolean },
) {
  const [loadingMore, setLoadingMore] = useState(false);
  const inflight = useRef(false);
  const result = useQuery(document, {
    variables: { ...variables, page: { limit: INFINITE_PAGE_SIZE, offset: 0 } } as TVariables,
    fetchPolicy: "cache-and-network",
    notifyOnNetworkStatusChange: true,
    skip: options?.skip,
  });
  const source = (result.data ?? result.previousData) as TData | undefined;
  const connection = source ? select(source) : undefined;
  const rows = useMemo(() => connection?.edges.map((edge) => edge.node) ?? [], [connection]);
  const totalCount = connection?.pageInfo.totalCount ?? undefined;
  const hasMore = Boolean(connection?.pageInfo.hasNextPage) || (totalCount !== undefined && rows.length < totalCount);

  const loadMore = useCallback(async () => {
    if (inflight.current || !hasMore || !result.data) return false;
    inflight.current = true;
    setLoadingMore(true);
    try {
      await result.fetchMore({
        variables: { page: { limit: INFINITE_PAGE_SIZE, offset: rows.length } } as Partial<TVariables>,
        updateQuery: (previous, { fetchMoreResult }) => (fetchMoreResult ? mergePages(previous, fetchMoreResult) : previous),
      });
      return true;
    } finally {
      inflight.current = false;
      setLoadingMore(false);
    }
  }, [hasMore, result, rows.length]);

  return {
    rows,
    totalCount,
    hasMore,
    loadMore,
    loadingMore,
    loading: !options?.skip && result.loading && !result.data && !result.previousData,
    error: result.error,
    refetch: result.refetch,
  };
}
