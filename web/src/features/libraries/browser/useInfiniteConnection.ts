import type { OperationVariables, TypedDocumentNode } from "@apollo/client";
import { useCallback } from "react";

import type { PageInput } from "@/graphql/generated/graphql";
import { useSettingsConnection } from "@/features/settings/useSettingsConnection";
import type { Connection } from "./useConnection";

export const INFINITE_PAGE_SIZE = 60;

/** One snapshot controller owns both pagination and subscription refresh. A
 * refresh stages the loaded window off-cache and commits it atomically. */
export function useInfiniteConnection<TData, TVariables extends OperationVariables & { page?: PageInput | null }, TNode extends { id: string }>(
  document: TypedDocumentNode<TData, TVariables>,
  variables: Omit<TVariables, "page">,
  select: (data: TData) => Connection<TNode> | null | undefined,
  options?: { skip?: boolean; complete?: boolean },
) {
  const list = useSettingsConnection(document, variables, (data) => {
    const connection = select(data);
    if (!connection) throw new Error("The server returned no connection");
    return connection;
  }, { infinite: !options?.complete, batchSize: INFINITE_PAGE_SIZE, skip: options?.skip, initialPartial: options?.complete });
  const loadMore = useCallback(async () => {
    if (list.fetching || list.loadingMore || !list.hasMore) return false;
    await list.loadMore();
    return true;
  }, [list.fetching, list.loadingMore, list.hasMore, list.loadMore]);
  return {
    ...list, loadMore,
    loadRevision: list.revision, error: list.error ?? list.moreError,
  };
}
