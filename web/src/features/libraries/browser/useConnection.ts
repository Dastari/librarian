import { useQuery } from "@apollo/client/react";
import type { OperationVariables, TypedDocumentNode } from "@apollo/client";
import { useMemo } from "react";

export interface Connection<TNode> {
  edges: Array<{ node: TNode }>;
  pageInfo: { hasNextPage: boolean; totalCount?: number | null };
}

/**
 * Runs a generated list query and unwraps the connection. `previousData` keeps the current rows
 * on screen while the next page or sort loads, so tables never flash empty.
 */
export function useConnection<TData, TVariables extends OperationVariables, TNode>(
  document: TypedDocumentNode<TData, TVariables>,
  variables: TVariables,
  select: (data: TData) => Connection<TNode> | null | undefined,
) {
  const result = useQuery(document, { variables, fetchPolicy: "cache-and-network", notifyOnNetworkStatusChange: true });
  const source = (result.data ?? result.previousData) as TData | undefined;
  const connection = source ? select(source) : undefined;
  const rows = useMemo(() => connection?.edges.map((edge) => edge.node) ?? [], [connection]);
  return {
    rows,
    totalCount: connection?.pageInfo.totalCount ?? undefined,
    hasNextPage: connection?.pageInfo.hasNextPage ?? false,
    loading: result.loading && !result.data && !result.previousData,
    fetching: result.loading,
    error: result.error,
    refetch: result.refetch,
  };
}
