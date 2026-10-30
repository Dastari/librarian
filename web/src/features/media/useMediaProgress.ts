import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";
import { PlaybackProgressForFilesDocument, type PlaybackProgressFieldsFragment } from "@/graphql/generated/graphql";

/** The generated operation returns at most 100 rows: query disjoint file batches. */
export function useMediaProgress(mediaFileIds: readonly string[], userId: string | undefined) {
  const client = useApolloClient();
  const epoch = useLiveEpoch();
  const scopeKey = JSON.stringify([epoch, userId ?? ""]);
  const key = JSON.stringify([epoch, userId ?? "", [...new Set(mediaFileIds)].sort()]);
  const [snapshot, setSnapshot] = useState<{ key: string; scope: string; rows: PlaybackProgressFieldsFragment[]; loading: boolean; error?: Error }>({ key: "", scope: "", rows: [], loading: false });
  const previous = useRef(snapshot);
  previous.current = snapshot;
  const refresh = useRef<Array<() => Promise<unknown>>>([]);
  useEffect(() => {
    const [, owner, ids] = JSON.parse(key) as [string, string, string[]];
    if (!owner || ids.length === 0) {
      refresh.current = [];
      return;
    }
    let disposed = false;
    const chunks = Array.from({ length: Math.ceil(ids.length / 100) }, (_, index) => ids.slice(index * 100, (index + 1) * 100));
    const states: Array<{ rows: PlaybackProgressFieldsFragment[]; loading: boolean; error?: Error }> = chunks.map((files) => ({ rows: previous.current.scope === scopeKey ? previous.current.rows.filter((row) => row.mediaFileId && files.includes(row.mediaFileId)) : [], loading: true }));
    const publish = () => {
      if (!disposed) setSnapshot({ key, scope: scopeKey, rows: states.flatMap((state) => state.rows), loading: states.some((state) => state.loading), error: states.find((state) => state.error)?.error });
    };
    const queries = chunks.map((mediaFileIds) => client.watchQuery({ query: PlaybackProgressForFilesDocument, variables: { userId: owner, mediaFileIds }, fetchPolicy: "cache-and-network", notifyOnNetworkStatusChange: true }));
    refresh.current = queries.map((query) => () => query.refetch());
    const subscriptions = queries.map((query, index) => query.subscribe({
      next: (result) => {
        const rows = result.dataState === "complete" ? result.data.playbackProgresses.edges.map(({ node }) => node) : undefined;
        states[index] = { rows: rows ?? states[index]!.rows, loading: result.loading, error: result.error };
        publish();
      },
      error: (error: Error) => { states[index] = { ...states[index]!, loading: false, error }; publish(); },
    }));
    return () => { disposed = true; subscriptions.forEach((subscription) => subscription.unsubscribe()); refresh.current = []; };
  }, [client, key]);
  const enabled = Boolean(userId && mediaFileIds.length);
  useLiveInvalidation(PlaybackProgressForFilesDocument, { userId, mediaFileIds }, () => Promise.all(refresh.current.map((run) => run())), { enabled, key: `progress:${key}` });
  const current = snapshot.key === key;
  const progressByFile = useMemo(() => {
    const wanted = new Set(mediaFileIds);
    return new Map(current || snapshot.scope === scopeKey ? snapshot.rows.filter((row) => row.mediaFileId && wanted.has(row.mediaFileId)).map((row) => [row.mediaFileId!, row]) : []);
  }, [current, snapshot.rows, scopeKey, key]);
  return { progressByFile, loading: enabled && (!current || snapshot.loading), error: current ? snapshot.error : undefined, refetch: useCallback(() => Promise.all(refresh.current.map((run) => run())), []) };
}
