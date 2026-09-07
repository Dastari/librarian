import { useCallback, useMemo } from "react";
import { useQuery } from "../lib/graphql/client";
import {
  ContentStatusesDocument,
  type ContentStatus,
  type ContentStatusRequestInput,
  type ContentStatusesQuery,
  type ContentStatusesQueryVariables,
} from "../lib/graphql/generated/graphql";

export type ContentStatusTarget = ContentStatusRequestInput;

/**
 * Resolve the server-authoritative status for a bounded set of media items.
 * Unauthorized or deleted items are deliberately absent from the returned map.
 */
export function useContentStatuses(targets: readonly ContentStatusTarget[]) {
  const inputs = useMemo(() => {
    const unique = new Map<string, ContentStatusRequestInput>();
    for (const target of targets) {
      if (!target.id) continue;
      unique.set(`${target.contentType}:${target.id}`, {
        contentType: target.contentType,
        id: target.id,
      });
    }
    return Array.from(unique.values()).slice(0, 500);
  }, [targets]);

  const query = useQuery<ContentStatusesQuery, ContentStatusesQueryVariables>(
    ContentStatusesDocument,
    {
      variables: { inputs },
      skip: inputs.length === 0,
      fetchPolicy: "cache-and-network",
    },
  );

  const statuses = useMemo(() => {
    const result = new Map<string, ContentStatus>();
    for (const item of query.data?.contentStatuses ?? []) {
      result.set(`${item.contentType}:${item.id}`, item.status);
    }
    return result;
  }, [query.data]);
  const getStatus = useCallback(
    (
      contentType: ContentStatusRequestInput["contentType"],
      id: string,
    ) => statuses.get(`${contentType}:${id}`),
    [statuses],
  );

  return {
    ...query,
    statuses,
    getStatus,
  };
}
