import { useQuery } from "@apollo/client/react";
import { useMemo } from "react";

import { ContentStatusesDocument } from "@/graphql/generated/graphql";
import type { ContentStatus, ContentStatusType } from "@/graphql/generated/graphql";

/**
 * Batch-resolves the computed `ContentStatus` for a page of items. The backend owns the
 * reducer (see design doc Q64); the frontend only displays what it returns.
 */
export function useContentStatuses(contentType: ContentStatusType, ids: string[]) {
  const inputs = useMemo(() => ids.map((id) => ({ contentType, id })), [contentType, ids]);
  const { data, previousData } = useQuery(ContentStatusesDocument, {
    variables: { inputs },
    skip: inputs.length === 0,
    fetchPolicy: "cache-and-network",
  });
  return useMemo(() => {
    const map = new Map<string, ContentStatus>();
    for (const item of (data ?? previousData)?.contentStatuses ?? []) map.set(item.id, item.status);
    return map;
  }, [data, previousData]);
}
