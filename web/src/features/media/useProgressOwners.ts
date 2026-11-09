import { useApolloClient } from "@apollo/client/react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { PlaybackProgressOwnersDocument, type PlaybackProgressOwnersQuery } from "@/graphql/generated/graphql";
import { useSession } from "@/lib/auth/useSession";
import { useLiveEpoch, useLiveInvalidation } from "@/lib/graphql/live/hooks";

import { hasSavedProgress, type ProgressReset } from "./progressReset";

type OwnerRow = PlaybackProgressOwnersQuery["playbackProgresses"]["edges"][number]["node"];

/** The titles that own files, as library grids list them. */
export type OwnerKind = "movie" | "show" | "album" | "audiobook";

const PAGE = 100;
/** A person's history is read up to this many pages (10,000 files); beyond it the grid stays correct for what it saw. */
const MAX_PAGES = 100;

const ownerKey = (kind: OwnerKind, id: string) => `${kind}:${id}`;

/** The title a progress row's file belongs to: a movie, a show (by episode), an album (by track) or a book (by chapter). */
export function ownerOf(row: Pick<OwnerRow, "mediaFile">): { kind: OwnerKind; id: string } | null {
  const file = row.mediaFile;
  if (!file) return null;
  if (file.movieId) return { kind: "movie", id: file.movieId };
  if (file.episode?.showId) return { kind: "show", id: file.episode.showId };
  if (file.track?.albumId) return { kind: "album", id: file.track.albumId };
  if (file.chapter?.audiobookId) return { kind: "audiobook", id: file.chapter.audiobookId };
  return null;
}

/** Each title's files that hold this person's progress, once each. */
export function groupProgressOwners(rows: readonly OwnerRow[]): Map<string, string[]> {
  const owners = new Map<string, Set<string>>();
  for (const row of rows) {
    if (!row.mediaFileId || !hasSavedProgress(row)) continue;
    const owner = ownerOf(row);
    if (!owner) continue;
    const key = ownerKey(owner.kind, owner.id);
    owners.set(key, (owners.get(key) ?? new Set()).add(row.mediaFileId));
  }
  return new Map([...owners].map(([key, files]) => [key, [...files]]));
}

/**
 * Which titles in a library grid have this person's watch history, and the files that hold it.
 * One read of their progress rows (with each file's owner) serves every grid, so a poster or row
 * offers "Mark as unwatched" only where there is something to reset.
 */
export function useProgressOwners(enabled = true) {
  const client = useApolloClient();
  const { user } = useSession();
  const userId = user?.id;
  const epoch = useLiveEpoch();
  const [snapshot, setSnapshot] = useState<{ key: string; rows: OwnerRow[]; loading: boolean; error?: Error }>({ key: "", rows: [], loading: false });
  const refresh = useRef<Array<() => Promise<unknown>>>([]);
  const active = Boolean(enabled && userId);
  const key = JSON.stringify([epoch, userId ?? ""]);

  useEffect(() => {
    if (!active || !userId) {
      refresh.current = [];
      return;
    }
    let disposed = false;
    const pages: Array<{ rows: OwnerRow[]; loading: boolean; error?: Error }> = [];
    const queries: Array<{ refetch: () => Promise<unknown> }> = [];
    const subscriptions: Array<{ unsubscribe: () => void }> = [];
    const opened = new Set<number>();
    const publish = () => {
      if (disposed) return;
      setSnapshot({ key, rows: pages.flatMap((page) => page?.rows ?? []), loading: pages.some((page) => page?.loading), error: pages.find((page) => page?.error)?.error });
    };
    const open = (index: number) => {
      if (disposed || index >= MAX_PAGES || opened.has(index)) return;
      opened.add(index);
      pages[index] = { rows: [], loading: true };
      const query = client.watchQuery({ query: PlaybackProgressOwnersDocument, variables: { userId, page: { limit: PAGE, offset: index * PAGE } }, fetchPolicy: "cache-and-network", notifyOnNetworkStatusChange: true });
      queries.push(query);
      subscriptions.push(query.subscribe({
        next: (result) => {
          const rows = result.dataState === "complete" ? result.data.playbackProgresses.edges.map(({ node }) => node) : undefined;
          pages[index] = { rows: rows ?? pages[index]!.rows, loading: result.loading, error: result.error };
          // A full page may have another behind it.
          if (rows && rows.length === PAGE) open(index + 1);
          publish();
        },
        error: (error: Error) => { pages[index] = { ...pages[index]!, loading: false, error }; publish(); },
      }));
    };
    refresh.current = [() => Promise.all(queries.map((query) => query.refetch()))];
    open(0);
    return () => { disposed = true; subscriptions.forEach((subscription) => subscription.unsubscribe()); refresh.current = []; };
  }, [client, key, active, userId]);

  useLiveInvalidation(PlaybackProgressOwnersDocument, { userId }, () => Promise.all(refresh.current.map((run) => run())), { enabled: active, key: `progress-owners:${key}` });

  const current = snapshot.key === key;
  const owners = useMemo(() => (current ? groupProgressOwners(snapshot.rows) : new Map<string, string[]>()), [current, snapshot.rows]);
  const loading = active && (!current || (snapshot.loading && snapshot.rows.length === 0));
  const error = current ? snapshot.error : undefined;
  const filesFor = useCallback((kind: OwnerKind, id: string): readonly string[] => owners.get(ownerKey(kind, id)) ?? [], [owners]);
  return { filesFor, loading, error, ready: active && current && !error };
}

/** The reset a grid hands its entity actions: the files holding progress for the titles it is run on. */
export function gridReset<T extends { id: string }>(kind: OwnerKind, owners: Pick<ReturnType<typeof useProgressOwners>, "filesFor" | "loading" | "error">): ProgressReset<T> {
  return {
    files: (rows) => [...new Set(rows.flatMap((row) => owners.filesFor(kind, row.id)))],
    disabledReason: () => (owners.error ? "Watch history could not be loaded" : owners.loading ? "Loading watch history" : undefined),
  };
}

/** `gridReset` for a grid, kept stable between renders. */
export function useGridReset<T extends { id: string }>(kind: OwnerKind): ProgressReset<T> {
  const { filesFor, loading, error } = useProgressOwners();
  return useMemo(() => gridReset<T>(kind, { filesFor, loading, error }), [kind, filesFor, loading, error]);
}
