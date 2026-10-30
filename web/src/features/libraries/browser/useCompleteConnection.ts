import type { OperationVariables, TypedDocumentNode } from "@apollo/client";

import type { PageInput } from "@/graphql/generated/graphql";
import type { Connection } from "./useConnection";
import { useInfiniteConnection } from "./useInfiniteConnection";

/** Drain off-cache, then publish a complete array once. Failed refreshes retain
 * the prior complete snapshot, including records beyond the first 100. */
export function useCompleteConnection<TData, TVariables extends OperationVariables & { page?: PageInput | null }, TNode extends { id: string }>(
  document: TypedDocumentNode<TData, TVariables>,
  variables: Omit<TVariables, "page">,
  select: (data: TData) => Connection<TNode> | null | undefined,
  options?: { skip?: boolean },
) {
  const list = useInfiniteConnection(document, variables, select, { ...options, complete: true });
  return { ...list, complete: !options?.skip && !list.loading && !list.fetching && !list.error };
}
