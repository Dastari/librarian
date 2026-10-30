import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { ContentStatusesDocument, type ContentStatus, type ContentStatusType } from "@/graphql/generated/graphql";
import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";
import { useSession } from "@/lib/auth/useSession";

// The contentStatuses resolver rejects requests above 500 inputs. Keep each
// observable active so media mutations can still invalidate its computed values.
export const CONTENT_STATUS_BATCH_SIZE = 500;

interface Snapshot {
  key: string;
  scope: string;
  statuses: Map<string, ContentStatus>;
  loading: boolean;
  error?: Error;
}

export function useContentStatusState(contentType: ContentStatusType, ids: string[]) {
  const client = useApolloClient();
  const { user, status } = useSession();
  const epoch = useLiveEpoch();
  const refresh = useRef<Array<() => Promise<unknown>>>([]);
  const clientScope = useRef({ client, version: 0 });
  if (clientScope.current.client !== client) clientScope.current = { client, version: clientScope.current.version + 1 };
  const idsKey = JSON.stringify([...new Set(ids)]);
  const principal = user ? `${user.id}:${user.role}` : status;
  const scopeKey = JSON.stringify([clientScope.current.version, principal, epoch, contentType]);
  const key = JSON.stringify([scopeKey, idsKey]);
  const batches = useMemo(() => {
    const inputs = [...new Set(ids)].map((id) => ({ contentType, id }));
    return Array.from({ length: Math.ceil(inputs.length / CONTENT_STATUS_BATCH_SIZE) }, (_, index) => inputs.slice(index * CONTENT_STATUS_BATCH_SIZE, (index + 1) * CONTENT_STATUS_BATCH_SIZE));
  }, [contentType, idsKey]);
  const [snapshot, setSnapshot] = useState<Snapshot>({ key: "", scope: "", statuses: new Map(), loading: false });
  const previous = useRef(snapshot);
  previous.current = snapshot;
  useEffect(() => {
    if (!batches.length) return;
    const results = batches.map((inputs) => ({
      statuses: new Map<string, ContentStatus>(previous.current.scope === scopeKey ? inputs.flatMap(({ id }) => {
        const status = previous.current.statuses.get(id);
        return status ? [[id, status] as const] : [];
      }) : []), loading: true, error: undefined as Error | undefined,
    }));
    let active = true;
    const publish = () => {
      if (!active) return;
      setSnapshot({
        key, scope: scopeKey,
        statuses: new Map(results.flatMap((result) => [...result.statuses])),
        loading: results.some((result) => result.loading),
        error: results.find((result) => result.error)?.error,
      });
    };
    const queries = batches.map((inputs) => client.watchQuery({
      query: ContentStatusesDocument,
      variables: { inputs },
      // A new principal must not briefly see the previous user's cached verdicts.
      // Network-only still writes to and observes Apollo's reactive cache.
      fetchPolicy: "network-only",
      context: { queryDeduplication: false },
      notifyOnNetworkStatusChange: true,
    }));
    refresh.current = queries.map((query) => () => query.refetch());
    const subscriptions = queries.map((query, index) => query.subscribe({
      next: (result) => {
        const statuses = new Map<string, ContentStatus>();
        for (const status of result.data?.contentStatuses ?? []) {
          if (status?.id && status.status) statuses.set(status.id, status.status);
        }
        results[index] = {
          statuses: result.data?.contentStatuses ? statuses : results[index]!.statuses,
          loading: result.loading,
          error: result.error,
        };
        publish();
      },
      error: (cause: unknown) => {
        results[index] = { ...results[index]!, loading: false, error: cause instanceof Error ? cause : new Error(String(cause)) };
        publish();
      },
    }));
    return () => { active = false; refresh.current = []; for (const subscription of subscriptions) subscription.unsubscribe(); };
  }, [client, batches, key]);
  const refreshAll = useCallback(() => Promise.all(refresh.current.map((run) => run())), []);
  const refetch = useCallback(() => refreshAll().catch(() => {}), [refreshAll]);
  useLiveInvalidation(ContentStatusesDocument, { contentType, ids: idsKey }, refreshAll, { enabled: batches.length > 0, key: `status:${key}` });
  const current = useMemo<Snapshot>(() => {
    if (ids.length === 0) return { key, scope: scopeKey, statuses: new Map(), loading: false };
    if (snapshot.key === key) return snapshot;
    const wanted = new Set(ids);
    return { key, scope: scopeKey, statuses: new Map(snapshot.scope === scopeKey ? [...snapshot.statuses].filter(([id]) => wanted.has(id)) : []), loading: true };
  }, [idsKey, key, scopeKey, snapshot]);
  return { ...current, refetch };
}

/** Existing consumers keep the Map API; filtered views can use the richer state above. */
export function useContentStatuses(contentType: ContentStatusType, ids: string[]) {
  return useContentStatusState(contentType, ids).statuses;
}
